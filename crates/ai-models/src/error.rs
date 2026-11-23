//! Errors of the model manager. Every hostile input is an error value, never a panic (`AGENTS.md` §0).

use thiserror::Error;

/// Why a model record, a hash or a file was refused.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ModelError {
    /// A field is missing, empty, too long or has a forbidden character.
    #[error("invalid {field}: {reason}")]
    Invalid { field: &'static str, reason: &'static str },
    /// A floating revision (`latest`, `main`, ...) is not a pinned revision.
    #[error("revision `{0}` is not pinned (floating names such as `latest` are refused)")]
    FloatingRevision(String),
    /// The value is not a SHA-256 in lowercase hexadecimal.
    #[error("not a SHA-256 (64 lowercase hex characters)")]
    BadHash,
    /// A source URL that is not `https`.
    #[error("source URL must be https")]
    InsecureUrl,
    /// A stored state may not move to the requested one.
    #[error("model state {from} cannot become {to}")]
    BadTransition { from: &'static str, to: &'static str },
    /// The same model id is already in the catalogue with another record.
    #[error("model `{0}` is already in the catalogue")]
    Duplicate(String),
    /// Reading a file failed (message only, so the error stays comparable).
    #[error("read error: {0}")]
    Io(String),
}
