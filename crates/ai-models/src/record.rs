//! The model record: identity, pinned revision, artifacts, source, licence and state.
//! Identity is (model id, revision, format, variant, hash); never a file name or path (plan §3.2).

use serde::{Deserialize, Serialize};

use crate::error::ModelError;
use crate::format::ModelFormat;
use crate::hash::Sha256Hex;
use crate::license::LicenseInfo;
use crate::state::{ModelState, SourceStatus, SourceTrust};

/// Most artifacts, capabilities and similar list entries one record may carry.
pub const MAX_LIST: usize = 64;

fn check_text(field: &'static str, s: &str, max: usize, allow: impl Fn(char) -> bool) -> Result<(), ModelError> {
    if s.is_empty() {
        return Err(ModelError::Invalid { field, reason: "empty" });
    }
    if s.len() > max {
        return Err(ModelError::Invalid { field, reason: "too long" });
    }
    if !s.chars().all(allow) {
        return Err(ModelError::Invalid { field, reason: "forbidden character" });
    }
    Ok(())
}

/// Stable model identifier such as `silero/vad`: lowercase letters, digits, `. _ - /`;
/// no empty or `..` segment, no leading or trailing `/`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ModelId(String);

impl ModelId {
    /// Validates an id.
    pub fn new(s: &str) -> Result<Self, ModelError> {
        check_text("model id", s, 128, |c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-' | '/'))?;
        if s.split('/').any(|seg| seg.is_empty() || seg == "." || seg == "..") {
            return Err(ModelError::Invalid { field: "model id", reason: "empty or dot segment" });
        }
        Ok(Self(s.to_owned()))
    }

    /// The id text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ModelId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(&String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

/// A pinned revision (commit hash, tag or version). Floating names are refused.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Revision(String);

impl Revision {
    /// Names that move; `latest` is never acceptable (ADR 0008 §2).
    const FLOATING: [&'static str; 6] = ["latest", "main", "master", "head", "stable", "nightly"];

    /// Validates a revision.
    pub fn new(s: &str) -> Result<Self, ModelError> {
        check_text("revision", s, 128, |c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'))?;
        if Self::FLOATING.contains(&s.to_ascii_lowercase().as_str()) {
            return Err(ModelError::FloatingRevision(s.to_owned()));
        }
        Ok(Self(s.to_owned()))
    }

    /// The revision text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Revision {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(&String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

/// One file of a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// Base name only, as the source calls it (never a path).
    pub file_name: String,
    pub format: ModelFormat,
    pub sha256: Sha256Hex,
    pub size_bytes: u64,
    /// For example `fp16`; `None` is "unknown", never guessed.
    #[serde(default)]
    pub precision: Option<String>,
    /// For example `q4_k_m`.
    #[serde(default)]
    pub quantization: Option<String>,
}

impl Artifact {
    /// Checks the fields; a zero size or an unknown format is refused.
    pub fn validate(&self) -> Result<(), ModelError> {
        check_text("file name", &self.file_name, 255, |c| !c.is_control() && c != '/' && c != '\\')?;
        if self.file_name == "." || self.file_name == ".." {
            return Err(ModelError::Invalid { field: "file name", reason: "dot name" });
        }
        if self.size_bytes == 0 {
            return Err(ModelError::Invalid { field: "size", reason: "zero" });
        }
        if self.format == ModelFormat::Unknown {
            return Err(ModelError::Invalid { field: "format", reason: "unknown" });
        }
        for (field, v) in [("precision", &self.precision), ("quantization", &self.quantization)] {
            if let Some(v) = v {
                check_text(field, v, 32, |c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))?;
            }
        }
        Ok(())
    }

    /// Identity key: format, variant and hash. Two artifacts with the same key are one copy.
    pub fn identity(&self) -> (ModelFormat, Option<&str>, &Sha256Hex) {
        (self.format, self.quantization.as_deref().or(self.precision.as_deref()), &self.sha256)
    }
}

/// Where the model comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub url: String,
    pub trust: SourceTrust,
    pub status: SourceStatus,
}

impl Source {
    /// `https` only, bounded, no spaces or control characters, no credentials in the URL.
    pub fn validate(&self) -> Result<(), ModelError> {
        let rest = self.url.strip_prefix("https://").ok_or(ModelError::InsecureUrl)?;
        check_text("source url", rest, 2048, |c| c.is_ascii_graphic())?;
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        if authority.is_empty() || authority.contains('@') {
            return Err(ModelError::Invalid { field: "source url", reason: "no host or credentials in URL" });
        }
        Ok(())
    }
}

/// A model revision in the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRecord {
    pub id: ModelId,
    pub family: String,
    pub revision: Revision,
    pub source: Source,
    pub license: LicenseInfo,
    pub artifacts: Vec<Artifact>,
    /// Capability ids the model serves (data, not code).
    pub capabilities: Vec<String>,
    pub state: ModelState,
    /// Declared needs; `None` is "unknown".
    #[serde(default)]
    pub vram_mib: Option<u64>,
    #[serde(default)]
    pub ram_mib: Option<u64>,
}

impl ModelRecord {
    /// Checks every field and bound of the record.
    pub fn validate(&self) -> Result<(), ModelError> {
        check_text("family", &self.family, 64, |c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' '))?;
        self.source.validate()?;
        if self.artifacts.is_empty() || self.artifacts.len() > MAX_LIST {
            return Err(ModelError::Invalid { field: "artifacts", reason: "need 1 to 64" });
        }
        for a in &self.artifacts {
            a.validate()?;
        }
        if self.capabilities.is_empty() || self.capabilities.len() > MAX_LIST {
            return Err(ModelError::Invalid { field: "capabilities", reason: "need 1 to 64" });
        }
        for c in &self.capabilities {
            check_text("capability", c, 64, |c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))?;
        }
        Ok(())
    }

    /// Total bytes of all artifacts (saturating; hostile sizes cannot overflow).
    pub fn total_size(&self) -> u64 {
        self.artifacts.iter().fold(0u64, |s, a| s.saturating_add(a.size_bytes))
    }

    /// Formats in this record that need the modelscan step before activation.
    pub fn needs_scan(&self) -> bool {
        self.artifacts.iter().any(|a| a.format.scan_required())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::license::LicenseText;

    pub fn sample(id: &str) -> ModelRecord {
        let t = |s| LicenseText::new(s).unwrap();
        ModelRecord {
            id: ModelId::new(id).unwrap(),
            family: "test".into(),
            revision: Revision::new("v1.0").unwrap(),
            source: Source { url: "https://example.org/m".into(), trust: SourceTrust::Canonical, status: SourceStatus::Ok },
            license: LicenseInfo { weights: t("MIT"), dataset: t("MIT"), code: t("MIT"), dependencies: t("MIT") },
            artifacts: vec![Artifact {
                file_name: "model.onnx".into(),
                format: ModelFormat::Onnx,
                sha256: Sha256Hex::parse(&"a".repeat(64)).unwrap(),
                size_bytes: 10,
                precision: Some("fp16".into()),
                quantization: None,
            }],
            capabilities: vec!["audio.vad".into()],
            state: ModelState::Discovered,
            vram_mib: None,
            ram_mib: Some(100),
        }
    }

    #[test]
    fn sample_is_valid() {
        sample("silero/vad").validate().unwrap();
    }

    #[test]
    fn floating_revisions_are_refused() {
        for r in ["latest", "LATEST", "main", "Head"] {
            assert!(matches!(Revision::new(r), Err(ModelError::FloatingRevision(_))), "{r}");
        }
        assert!(Revision::new("").is_err() && Revision::new("a b").is_err());
        assert!(Revision::new("4f9c1e2").is_ok());
    }

    #[test]
    fn model_ids_cannot_escape() {
        for bad in ["", "/a", "a/", "a//b", "../x", "a/../b", "A", "a b", "a\\b"] {
            assert!(ModelId::new(bad).is_err(), "{bad}");
        }
        assert!(ModelId::new(&"a".repeat(129)).is_err());
        assert!(ModelId::new("org/model-1.2_x").is_ok());
    }

    #[test]
    fn artifact_rules() {
        let mut r = sample("a/b");
        let mut bad = r.artifacts[0].clone();
        bad.file_name = "../evil".into();
        assert!(bad.validate().is_err());
        bad.file_name = "..".into();
        assert!(bad.validate().is_err());
        bad = r.artifacts[0].clone();
        bad.size_bytes = 0;
        assert!(bad.validate().is_err());
        bad = r.artifacts[0].clone();
        bad.format = ModelFormat::Unknown;
        assert!(bad.validate().is_err());
        r.artifacts.clear();
        assert!(r.validate().is_err());
    }

    #[test]
    fn source_must_be_plain_https() {
        let mk = |u: &str| Source { url: u.into(), trust: SourceTrust::Validated, status: SourceStatus::Ok };
        assert!(mk("http://x.org/m").validate().is_err());
        assert!(mk("file:///etc/passwd").validate().is_err());
        assert!(mk("https://user:pw@x.org/m").validate().is_err());
        assert!(mk("https:///m").validate().is_err());
        assert!(mk("https://x.org/a b").validate().is_err());
        assert!(mk("https://huggingface.co/org/m").validate().is_ok());
    }

    #[test]
    fn sizes_cannot_overflow() {
        let mut r = sample("a/b");
        let mut b = r.artifacts[0].clone();
        b.size_bytes = u64::MAX;
        r.artifacts.push(b.clone());
        r.artifacts.push(b);
        assert_eq!(r.total_size(), u64::MAX);
    }

    #[test]
    fn unknown_json_fields_are_refused() {
        let j = serde_json::to_string(&sample("a/b")).unwrap();
        let back: ModelRecord = serde_json::from_str(&j).unwrap();
        assert_eq!(back, sample("a/b"));
        let evil = j.replacen('{', "{\"extra\":1,", 1);
        assert!(serde_json::from_str::<ModelRecord>(&evil).is_err());
        let floating = j.replace("v1.0", "latest");
        assert!(serde_json::from_str::<ModelRecord>(&floating).is_err());
    }

    #[test]
    fn pickle_artifacts_need_scan() {
        let mut r = sample("a/b");
        assert!(!r.needs_scan());
        r.artifacts[0].format = ModelFormat::Pickle;
        assert!(r.needs_scan());
    }
}
