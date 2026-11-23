//! CFA4 model manager, phase MM-F1 (`docs/ai/CFA4-MODEL-MANAGER-001.md`; layer L2, no UI, no FFI,
//! no network, no `unsafe`).
//!
//! This crate is the registry: what a model *is* (identity, pinned revision, artifacts, source,
//! licence, state) and how to find it. It never downloads. Download is a `cfa4-tasks` task type
//! that calls `cfa4-fetch` on the user's explicit command (plan §5); the store, staging and
//! quarantine areas (MM-F2) and the import steps (MM-F3) build on these types.
//!
//! Rules kept here:
//! - identity is the SHA-256 and the pinned revision, never a file name; `latest` is refused;
//! - the real format comes from the bytes, not the extension; pickle and HDF5 need the
//!   modelscan step before activation ([`ModelFormat::scan_required`]);
//! - the licence of weights and dataset warns and never blocks (owner decision 2026-11-07);
//! - a model that is not verified is never usable ([`ModelState::usable`]).

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod catalog;
pub mod error;
pub mod format;
pub mod hash;
pub mod license;
pub mod record;
pub mod state;

pub use catalog::{Catalog, Filter};
pub use error::ModelError;
pub use format::ModelFormat;
pub use hash::{Sha256Hex, sha256_file, sha256_reader};
pub use license::{LicenseInfo, LicensePart, LicenseText};
pub use record::{Artifact, ModelId, ModelRecord, Revision, Source};
pub use state::{ModelState, SourceStatus, SourceTrust};
