//! The import steps (MM-F3, plan §5 and §6b): a staged file becomes an installed model only after
//! size, SHA-256, real format, structural check and, where required, the scan (D11, AC-28).
//!
//! No network here: the file is already in staging (the user copied it, or the managed download
//! put it there on the user's command). The scanner is a trait; the real one runs modelscan in a
//! closed process and is installed only after the user's one-time approval. A file that is not
//! verified is never promoted.
//!
//! Outcomes: [`ImportOutcome::Installed`], [`ImportOutcome::AlreadyPresent`] (same hash, one copy),
//! [`ImportOutcome::Quarantined`] (scan findings, no matching override) and
//! [`ImportOutcome::Refused`] (the file stays in staging, the store is untouched).

use std::fs;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ModelError;
use crate::format::{ModelFormat, SAFETENSORS_MAX_HEADER};
use crate::hash::{Sha256Hex, sha256_file};
use crate::location::{Promoted, StoreLocation};
use crate::record::{Artifact, ModelId};

/// Severity of a scan finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// One scanner finding (what was found and how bad).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub description: String,
}

/// Result of the scan step (`ai.verification`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanOutcome {
    /// No finding. This is not a statement that the file is harmless.
    Clean,
    Issues(Vec<Finding>),
    /// The scanner does not cover the format.
    Unsupported,
    Error(String),
}

/// A scanner that reads the file and nothing else. Implementations run in a separate, closed
/// process (no network, no program start) and never load the model.
pub trait Scanner {
    fn scan(&self, path: &Path) -> ScanOutcome;
}

/// The user's explicit, logged override of findings for one model and one file hash (D11).
/// It never carries over to another hash or revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanOverride {
    pub model: ModelId,
    pub sha256: Sha256Hex,
    /// Finding ids the user confirmed.
    pub finding_ids: Vec<String>,
    /// When the user confirmed (Unix seconds, from the caller's clock).
    pub confirmed_at_unix: u64,
}

/// What the import checked; kept with the model record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verification {
    pub sha256: Sha256Hex,
    pub size_bytes: u64,
    pub format: ModelFormat,
    pub structure_ok: bool,
    pub scan: Option<ScanOutcome>,
    pub overridden: Option<ScanOverride>,
}

/// Why a staged file was refused (it stays in staging).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    SizeMismatch,
    HashMismatch,
    /// The bytes are another format than the record says (the extension is not enough).
    FormatMismatch(ModelFormat),
    BadStructure,
    /// Pickle, HDF5 or SavedModel need the scanner, and it is not installed.
    ScannerMissing,
    /// The scanner does not cover the format or failed: unverified files are not activated.
    NotScanned,
}

/// What an import ended in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportOutcome {
    Installed { path: std::path::PathBuf, verification: Verification },
    AlreadyPresent { path: std::path::PathBuf, verification: Verification },
    Quarantined { path: std::path::PathBuf, verification: Verification, findings: Vec<Finding> },
    Refused { reason: Refusal, verification: Option<Verification> },
}

fn io(e: impl std::fmt::Display) -> ModelError {
    ModelError::Io(e.to_string())
}

/// Reads the head of a file (at most `n` bytes).
fn head(path: &Path, n: usize) -> Result<Vec<u8>, ModelError> {
    let f = fs::File::open(path).map_err(io)?;
    let mut buf = Vec::new();
    f.take(n as u64).read_to_end(&mut buf).map_err(io)?;
    Ok(buf)
}

/// Structural check done by CFA4 itself for the formats modelscan does not cover.
fn structure_ok(path: &Path, format: ModelFormat, size: u64) -> Result<bool, ModelError> {
    match format {
        ModelFormat::Safetensors => {
            let h = head(path, 8)?;
            let Some(len) = h.get(..8).and_then(|b| <[u8; 8]>::try_from(b).ok()).map(u64::from_le_bytes) else {
                return Ok(false);
            };
            if len == 0 || len > SAFETENSORS_MAX_HEADER || len.saturating_add(8) > size {
                return Ok(false);
            }
            let total = usize::try_from(len.saturating_add(8)).map_err(io)?;
            let all = head(path, total)?;
            let Some(json) = all.get(8..) else { return Ok(false) };
            Ok(matches!(serde_json::from_slice::<serde_json::Value>(json), Ok(serde_json::Value::Object(_))))
        }
        ModelFormat::Gguf => {
            let h = head(path, 8)?;
            let version = h.get(4..8).and_then(|b| <[u8; 4]>::try_from(b).ok()).map(u32::from_le_bytes);
            Ok(size >= 24 && matches!(version, Some(1..=3)))
        }
        // The ONNX tag byte was part of detection; a deeper parse comes with the ONNX worker.
        ModelFormat::Onnx => Ok(true),
        // Scanned instead.
        ModelFormat::Pickle | ModelFormat::Hdf5 => Ok(true),
        ModelFormat::Unknown => Ok(false),
    }
}

/// Imports a staged file against the artifact record. `scanner` is `None` when modelscan is not
/// installed; then pickle and HDF5 imports are refused and the other formats still work.
/// `overrides` are the user's logged confirmations; only one that matches this model and this hash counts.
pub fn import_staged(
    loc: &StoreLocation,
    staged_name: &str,
    model: &ModelId,
    artifact: &Artifact,
    scanner: Option<&dyn Scanner>,
    overrides: &[ScanOverride],
) -> Result<ImportOutcome, ModelError> {
    let path = loc.staging_dir().join(staged_name);
    if staged_name.is_empty() || staged_name.contains(['/', '\\']) || staged_name == ".." || staged_name == "." {
        return Err(ModelError::Invalid { field: "staged name", reason: "must be a plain file name" });
    }
    let (sha256, size_bytes) = sha256_file(&path)?;
    let refuse = |reason, v| Ok(ImportOutcome::Refused { reason, verification: v });
    let ext = path.extension().and_then(|e| e.to_str());
    let format = ModelFormat::detect(&head(&path, 64)?, ext);
    let mut v = Verification { sha256: sha256.clone(), size_bytes, format, structure_ok: false, scan: None, overridden: None };

    if size_bytes != artifact.size_bytes {
        return refuse(Refusal::SizeMismatch, Some(v));
    }
    if sha256 != artifact.sha256 {
        return refuse(Refusal::HashMismatch, Some(v));
    }
    if format != artifact.format {
        return refuse(Refusal::FormatMismatch(format), Some(v));
    }
    v.structure_ok = structure_ok(&path, format, size_bytes)?;
    if !v.structure_ok {
        return refuse(Refusal::BadStructure, Some(v));
    }

    if format.scan_required() {
        let Some(scanner) = scanner else {
            return refuse(Refusal::ScannerMissing, Some(v));
        };
        let outcome = scanner.scan(&path);
        v.scan = Some(outcome.clone());
        match outcome {
            ScanOutcome::Clean => {}
            ScanOutcome::Unsupported | ScanOutcome::Error(_) => return refuse(Refusal::NotScanned, Some(v)),
            ScanOutcome::Issues(findings) => {
                let confirmed = overrides.iter().find(|o| &o.model == model && o.sha256 == sha256 && findings.iter().all(|f| o.finding_ids.contains(&f.id)));
                match confirmed {
                    Some(o) => v.overridden = Some(o.clone()),
                    None => {
                        let q = loc.quarantine(staged_name, &sha256)?;
                        return Ok(ImportOutcome::Quarantined { path: q, verification: v, findings });
                    }
                }
            }
        }
    }

    Ok(match loc.promote(staged_name, &sha256)? {
        Promoted::Stored(path) => ImportOutcome::Installed { path, verification: v },
        Promoted::AlreadyPresent(path) => ImportOutcome::AlreadyPresent { path, verification: v },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::sha256_reader;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct Fixed(ScanOutcome);
    impl Scanner for Fixed {
        fn scan(&self, _: &Path) -> ScanOutcome {
            self.0.clone()
        }
    }

    fn tmp() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!("cfa4-imp-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn safetensors_bytes() -> Vec<u8> {
        let json = br#"{"w":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#;
        let mut v = (json.len() as u64).to_le_bytes().to_vec();
        v.extend_from_slice(json);
        v.extend_from_slice(&[0, 0, 0, 0]);
        v
    }

    fn pickle_bytes() -> Vec<u8> {
        let mut v = vec![0x80, 0x04];
        v.extend_from_slice(b"generated harmless stream");
        v
    }

    fn art(name: &str, data: &[u8], format: ModelFormat) -> Artifact {
        let (sha256, size_bytes) = sha256_reader(data).unwrap();
        Artifact { file_name: name.into(), format, sha256, size_bytes, precision: None, quantization: None }
    }

    fn setup(name: &str, data: &[u8]) -> StoreLocation {
        let loc = StoreLocation::choose(&tmp(), 0).unwrap();
        fs::write(loc.staging_dir().join(name), data).unwrap();
        loc
    }

    fn id() -> ModelId {
        ModelId::new("test/model").unwrap()
    }

    #[test]
    fn good_safetensors_installs_without_a_scanner() {
        let d = safetensors_bytes();
        let loc = setup("m.safetensors", &d);
        let a = art("m.safetensors", &d, ModelFormat::Safetensors);
        let out = import_staged(&loc, "m.safetensors", &id(), &a, None, &[]).unwrap();
        assert!(matches!(out, ImportOutcome::Installed { ref verification, .. } if verification.structure_ok && verification.scan.is_none()));
    }

    #[test]
    fn wrong_hash_size_and_format_are_refused_and_stay_in_staging() {
        let d = safetensors_bytes();
        let loc = setup("m.safetensors", &d);
        let mut a = art("m.safetensors", &d, ModelFormat::Safetensors);
        a.sha256 = Sha256Hex::parse(&"a".repeat(64)).unwrap();
        assert!(matches!(import_staged(&loc, "m.safetensors", &id(), &a, None, &[]).unwrap(), ImportOutcome::Refused { reason: Refusal::HashMismatch, .. }));
        let mut a = art("m.safetensors", &d, ModelFormat::Safetensors);
        a.size_bytes += 1;
        assert!(matches!(import_staged(&loc, "m.safetensors", &id(), &a, None, &[]).unwrap(), ImportOutcome::Refused { reason: Refusal::SizeMismatch, .. }));
        let a = art("m.safetensors", &d, ModelFormat::Onnx);
        assert!(matches!(
            import_staged(&loc, "m.safetensors", &id(), &a, None, &[]).unwrap(),
            ImportOutcome::Refused { reason: Refusal::FormatMismatch(ModelFormat::Safetensors), .. }
        ));
        assert!(loc.staging_dir().join("m.safetensors").exists());
    }

    #[test]
    fn fake_extension_is_caught_by_the_bytes() {
        // A pickle named .safetensors claiming to be safetensors.
        let d = pickle_bytes();
        let loc = setup("m.safetensors", &d);
        let a = art("m.safetensors", &d, ModelFormat::Safetensors);
        assert!(matches!(
            import_staged(&loc, "m.safetensors", &id(), &a, None, &[]).unwrap(),
            ImportOutcome::Refused { reason: Refusal::FormatMismatch(ModelFormat::Pickle), .. }
        ));
    }

    #[test]
    fn broken_structure_is_refused() {
        // Header length says more than the file holds.
        let mut d = 1000u64.to_le_bytes().to_vec();
        d.extend_from_slice(b"{}");
        let loc = setup("m.safetensors", &d);
        let a = art("m.safetensors", &d, ModelFormat::Safetensors);
        assert!(matches!(import_staged(&loc, "m.safetensors", &id(), &a, None, &[]).unwrap(), ImportOutcome::Refused { reason: Refusal::BadStructure, .. }));
    }

    #[test]
    fn pickle_without_the_scanner_is_refused_but_other_formats_work() {
        let d = pickle_bytes();
        let loc = setup("m.pt", &d);
        let a = art("m.pt", &d, ModelFormat::Pickle);
        assert!(matches!(import_staged(&loc, "m.pt", &id(), &a, None, &[]).unwrap(), ImportOutcome::Refused { reason: Refusal::ScannerMissing, .. }));
        assert!(loc.staging_dir().join("m.pt").exists());
    }

    #[test]
    fn clean_scan_installs() {
        let d = pickle_bytes();
        let loc = setup("m.pt", &d);
        let a = art("m.pt", &d, ModelFormat::Pickle);
        let out = import_staged(&loc, "m.pt", &id(), &a, Some(&Fixed(ScanOutcome::Clean)), &[]).unwrap();
        assert!(matches!(out, ImportOutcome::Installed { .. }));
    }

    #[test]
    fn unsupported_or_failed_scan_does_not_activate() {
        let d = pickle_bytes();
        let loc = setup("m.pt", &d);
        let a = art("m.pt", &d, ModelFormat::Pickle);
        for s in [ScanOutcome::Unsupported, ScanOutcome::Error("boom".into())] {
            let out = import_staged(&loc, "m.pt", &id(), &a, Some(&Fixed(s)), &[]).unwrap();
            assert!(matches!(out, ImportOutcome::Refused { reason: Refusal::NotScanned, .. }));
        }
    }

    fn issues() -> ScanOutcome {
        ScanOutcome::Issues(vec![Finding { id: "F1".into(), severity: Severity::High, description: "generated sample".into() }])
    }

    #[test]
    fn findings_quarantine_and_an_override_is_per_hash() {
        let d = pickle_bytes();
        let loc = setup("m.pt", &d);
        let a = art("m.pt", &d, ModelFormat::Pickle);
        let scanner = Fixed(issues());
        let out = import_staged(&loc, "m.pt", &id(), &a, Some(&scanner), &[]).unwrap();
        let ImportOutcome::Quarantined { path, findings, .. } = out else { panic!("not quarantined") };
        assert!(path.exists() && findings.len() == 1);
        assert!(!loc.store_path(&a.sha256).exists());

        // An override for another hash does not help.
        let other = ScanOverride { model: id(), sha256: Sha256Hex::parse(&"b".repeat(64)).unwrap(), finding_ids: vec!["F1".into()], confirmed_at_unix: 1 };
        fs::write(loc.staging_dir().join("m2.pt"), &d).unwrap();
        fs::remove_file(&path).unwrap();
        let out = import_staged(&loc, "m2.pt", &id(), &a, Some(&scanner), &[other]).unwrap();
        assert!(matches!(out, ImportOutcome::Quarantined { .. }));
    }

    #[test]
    fn override_for_this_model_hash_and_findings_installs_and_is_recorded() {
        let d = pickle_bytes();
        let loc = setup("m.pt", &d);
        let a = art("m.pt", &d, ModelFormat::Pickle);
        let ov = ScanOverride { model: id(), sha256: a.sha256.clone(), finding_ids: vec!["F1".into()], confirmed_at_unix: 42 };
        let out = import_staged(&loc, "m.pt", &id(), &a, Some(&Fixed(issues())), std::slice::from_ref(&ov)).unwrap();
        let ImportOutcome::Installed { verification, .. } = out else { panic!("not installed") };
        assert_eq!(verification.overridden, Some(ov));
    }

    #[test]
    fn override_for_another_model_or_missing_finding_does_not_count() {
        let d = pickle_bytes();
        let a = art("m.pt", &d, ModelFormat::Pickle);
        let wrong_model =
            ScanOverride { model: ModelId::new("other/model").unwrap(), sha256: a.sha256.clone(), finding_ids: vec!["F1".into()], confirmed_at_unix: 1 };
        let wrong_finding = ScanOverride { model: id(), sha256: a.sha256.clone(), finding_ids: vec!["F9".into()], confirmed_at_unix: 1 };
        for ov in [wrong_model, wrong_finding] {
            let loc = setup("m.pt", &d);
            let out = import_staged(&loc, "m.pt", &id(), &a, Some(&Fixed(issues())), &[ov]).unwrap();
            assert!(matches!(out, ImportOutcome::Quarantined { .. }));
        }
    }

    #[test]
    fn staged_names_cannot_escape() {
        let d = safetensors_bytes();
        let loc = setup("m.safetensors", &d);
        let a = art("m.safetensors", &d, ModelFormat::Safetensors);
        assert!(import_staged(&loc, "../x", &id(), &a, None, &[]).is_err());
    }
}
