//! SHA-256 identity of a model file. The hash, never the file name, says two files are the same.

use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ModelError;

/// A SHA-256 as 64 lowercase hexadecimal characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Sha256Hex(String);

impl Sha256Hex {
    /// Parses a hash; uppercase and wrong lengths are refused.
    pub fn parse(s: &str) -> Result<Self, ModelError> {
        if s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) { Ok(Self(s.to_owned())) } else { Err(ModelError::BadHash) }
    }

    /// The hex text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Sha256Hex {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// Hashes a reader in 1 MiB steps (bounded memory) and returns the hash and the byte count.
pub fn sha256_reader<R: Read>(mut r: R) -> Result<(Sha256Hex, u64), ModelError> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut total: u64 = 0;
    loop {
        let n = r.read(&mut buf).map_err(|e| ModelError::Io(e.to_string()))?;
        if n == 0 {
            break;
        }
        let chunk = buf.get(..n).ok_or_else(|| ModelError::Io("short buffer".into()))?;
        hasher.update(chunk);
        total = total.saturating_add(n as u64);
    }
    let hex: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok((Sha256Hex(hex), total))
}

/// Hashes a file.
pub fn sha256_file(path: &Path) -> Result<(Sha256Hex, u64), ModelError> {
    let f = std::fs::File::open(path).map_err(|e| ModelError::Io(e.to_string()))?;
    sha256_reader(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        let (h, n) = sha256_reader(&b"abc"[..]).unwrap();
        assert_eq!(h.as_str(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(n, 3);
    }

    #[test]
    fn refuses_bad_hashes() {
        assert_eq!(Sha256Hex::parse("ABC"), Err(ModelError::BadHash));
        assert_eq!(Sha256Hex::parse(&"A".repeat(64)), Err(ModelError::BadHash));
        assert_eq!(Sha256Hex::parse(&"g".repeat(64)), Err(ModelError::BadHash));
        assert!(Sha256Hex::parse(&"a".repeat(64)).is_ok());
        assert!(serde_json::from_str::<Sha256Hex>("\"zz\"").is_err());
    }

    #[test]
    fn missing_file_is_an_error() {
        assert!(sha256_file(Path::new("/nonexistent/cfa4-model")).is_err());
    }
}
