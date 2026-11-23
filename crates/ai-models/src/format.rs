//! Real file format of a model, from its first bytes. The extension alone is never enough:
//! a `.safetensors` file that is a ZIP is a pickle container (plan §5, "import steps").

use serde::{Deserialize, Serialize};

/// Largest safetensors JSON header accepted (the format's own reference limit is 100 MB).
pub const SAFETENSORS_MAX_HEADER: u64 = 100 * 1024 * 1024;

/// What a model file really is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelFormat {
    Onnx,
    Safetensors,
    Gguf,
    /// Pickle stream or the ZIP container of `torch.save`: can run code on load.
    Pickle,
    /// Keras HDF5.
    Hdf5,
    /// Unknown or not enough bytes: never activated.
    Unknown,
}

impl ModelFormat {
    /// Formats that must pass the modelscan step before activation (plan §6b, D11).
    pub fn scan_required(self) -> bool {
        matches!(self, Self::Pickle | Self::Hdf5)
    }

    /// Formats CFA4 checks structurally itself instead of with modelscan.
    pub fn structural_check(self) -> bool {
        matches!(self, Self::Onnx | Self::Safetensors | Self::Gguf)
    }

    /// Detects the format from the first bytes (at least 16 are useful) and the file extension.
    ///
    /// ONNX has no magic number: it needs the `.onnx` extension and the protobuf tag of the first
    /// `ModelProto` field (`ir_version`, byte `0x08`).
    pub fn detect(header: &[u8], extension: Option<&str>) -> Self {
        if header.starts_with(b"GGUF") {
            return Self::Gguf;
        }
        if header.starts_with(b"\x89HDF\r\n\x1a\n") {
            return Self::Hdf5;
        }
        if header.starts_with(b"PK\x03\x04") || is_pickle_start(header) {
            return Self::Pickle;
        }
        if let Some(len) = header.get(..8).and_then(|b| <[u8; 8]>::try_from(b).ok()).map(u64::from_le_bytes)
            && len > 0
            && len <= SAFETENSORS_MAX_HEADER
            && header.get(8) == Some(&b'{')
        {
            return Self::Safetensors;
        }
        let ext = extension.map(str::to_ascii_lowercase);
        if ext.as_deref() == Some("onnx") && header.first() == Some(&0x08) {
            return Self::Onnx;
        }
        Self::Unknown
    }
}

fn is_pickle_start(h: &[u8]) -> bool {
    // Protocol 2+ pickles start with PROTO (0x80) and a protocol byte 2..=5.
    matches!((h.first(), h.get(1)), (Some(0x80), Some(2..=5)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn safetensors() -> Vec<u8> {
        let mut v = 2u64.to_le_bytes().to_vec();
        v.extend_from_slice(b"{}");
        v
    }

    #[test]
    fn detects_by_content() {
        assert_eq!(ModelFormat::detect(b"GGUF\x03\0\0\0", None), ModelFormat::Gguf);
        assert_eq!(ModelFormat::detect(&safetensors(), Some("bin")), ModelFormat::Safetensors);
        assert_eq!(ModelFormat::detect(&[0x08, 0x07, 0x12], Some("onnx")), ModelFormat::Onnx);
        assert_eq!(ModelFormat::detect(b"\x89HDF\r\n\x1a\n..", None), ModelFormat::Hdf5);
    }

    #[test]
    fn extension_is_not_enough() {
        // A ZIP named .safetensors is a pickle container.
        assert_eq!(ModelFormat::detect(b"PK\x03\x04rest", Some("safetensors")), ModelFormat::Pickle);
        assert_eq!(ModelFormat::detect(&[0x80, 0x04, 0x95], Some("safetensors")), ModelFormat::Pickle);
        // .onnx extension with other content is unknown.
        assert_eq!(ModelFormat::detect(b"hello world", Some("onnx")), ModelFormat::Unknown);
        assert_eq!(ModelFormat::detect(&[0x08], None), ModelFormat::Unknown);
    }

    #[test]
    fn hostile_headers_do_not_panic() {
        assert_eq!(ModelFormat::detect(&[], None), ModelFormat::Unknown);
        assert_eq!(ModelFormat::detect(&[0xff; 7], None), ModelFormat::Unknown);
        let mut huge = u64::MAX.to_le_bytes().to_vec();
        huge.push(b'{');
        assert_eq!(ModelFormat::detect(&huge, None), ModelFormat::Unknown);
        let mut zero = 0u64.to_le_bytes().to_vec();
        zero.push(b'{');
        assert_eq!(ModelFormat::detect(&zero, None), ModelFormat::Unknown);
    }

    #[test]
    fn scan_rules() {
        assert!(ModelFormat::Pickle.scan_required());
        assert!(ModelFormat::Hdf5.scan_required());
        assert!(!ModelFormat::Safetensors.scan_required());
        assert!(ModelFormat::Onnx.structural_check());
        assert!(!ModelFormat::Unknown.structural_check() && !ModelFormat::Unknown.scan_required());
    }
}
