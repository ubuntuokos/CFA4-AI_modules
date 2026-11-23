//! The four licence parts of a model (ADR 0008 §2, owner decision 2026-11-07). CFA4 does not
//! distribute models, so the weights' and dataset's licence never blocks: it warns and asks for
//! a one-time acceptance. `code` and `dependencies` come from the worker's publisher; an unknown
//! value there still blocks. The model card shows what the source claims; CFA4 gives no legal verdict (D2).

use serde::{Deserialize, Serialize};

use crate::error::ModelError;

/// One licence field as the source states it (an SPDX id or free text).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LicenseText(String);

impl LicenseText {
    /// Longest accepted text.
    pub const MAX: usize = 256;

    /// Trims and validates (no empty text, no control characters, bounded length).
    pub fn new(s: &str) -> Result<Self, ModelError> {
        let t = s.trim();
        if t.is_empty() || t.len() > Self::MAX || t.chars().any(char::is_control) {
            return Err(ModelError::Invalid { field: "license", reason: "empty, too long or with control characters" });
        }
        Ok(Self(t.to_owned()))
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `NOASSERTION`, `unknown` and `no` (any case) are the values that warn.
    pub fn is_unclear(&self) -> bool {
        let t = self.0.to_ascii_lowercase();
        matches!(t.as_str(), "noassertion" | "unknown" | "no" | "none")
    }
}

/// Which part a finding is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicensePart {
    Weights,
    Dataset,
    Code,
    Dependencies,
}

/// The four parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LicenseInfo {
    pub weights: LicenseText,
    pub dataset: LicenseText,
    pub code: LicenseText,
    pub dependencies: LicenseText,
}

impl LicenseInfo {
    /// Parts that need the user's one-time acceptance (weights and dataset, when unclear).
    pub fn warnings(&self) -> Vec<LicensePart> {
        let mut v = Vec::new();
        if self.weights.is_unclear() {
            v.push(LicensePart::Weights);
        }
        if self.dataset.is_unclear() {
            v.push(LicensePart::Dataset);
        }
        v
    }

    /// Parts that block: only the publisher's `code` and `dependencies`, when unclear.
    pub fn blockers(&self) -> Vec<LicensePart> {
        let mut v = Vec::new();
        if self.code.is_unclear() {
            v.push(LicensePart::Code);
        }
        if self.dependencies.is_unclear() {
            v.push(LicensePart::Dependencies);
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lic(w: &str, d: &str, c: &str, p: &str) -> LicenseInfo {
        let t = |s| LicenseText::new(s).unwrap();
        LicenseInfo { weights: t(w), dataset: t(d), code: t(c), dependencies: t(p) }
    }

    #[test]
    fn unclear_weights_and_dataset_warn_but_never_block() {
        let l = lic("NOASSERTION", "unknown", "MIT", "Apache-2.0");
        assert_eq!(l.warnings(), vec![LicensePart::Weights, LicensePart::Dataset]);
        assert!(l.blockers().is_empty());
    }

    #[test]
    fn unclear_code_blocks() {
        let l = lic("MIT", "CC-BY-4.0", "No", "NOASSERTION");
        assert!(l.warnings().is_empty());
        assert_eq!(l.blockers(), vec![LicensePart::Code, LicensePart::Dependencies]);
    }

    #[test]
    fn hostile_text_is_refused() {
        assert!(LicenseText::new("").is_err());
        assert!(LicenseText::new("   ").is_err());
        assert!(LicenseText::new("a\u{0}b").is_err());
        assert!(LicenseText::new(&"x".repeat(LicenseText::MAX + 1)).is_err());
        assert!(LicenseText::new(" MIT ").is_ok());
    }
}
