//! Where models live on disk (plan §5, AC-25, AC-15).
//!
//! The user chooses the model store; there is no default that is used silently. Until a location
//! is chosen, [`Stores::require`] refuses, so neither import nor download can start. The XDG path
//! is only a [suggestion](suggest_root). Under the chosen root CFA4 keeps three areas
//! (`cfa4-models/`, `.staging/`, `.quarantine/`) on the same file system, so promotion from staging
//! is one rename, not a copy. The runtime cache is a separate, disposable directory.
//! Nothing is created outside the chosen root, and an existing copy is never overwritten.

use std::fs;
use std::path::{Path, PathBuf};

use rustix::fs::statvfs;
use serde::{Deserialize, Serialize};

use crate::error::ModelError;
use crate::hash::{Sha256Hex, sha256_file};

/// Free space kept beyond a model's size (staging log, file-system slack).
pub const SPACE_MARGIN: u64 = 512 * 1024 * 1024;

const STORE_DIR: &str = "cfa4-models";
const STAGING_DIR: &str = ".staging";
const QUARANTINE_DIR: &str = ".quarantine";

fn io(e: impl std::fmt::Display) -> ModelError {
    ModelError::Io(e.to_string())
}

/// Free space of the file system holding `path`, in bytes (what an unprivileged user can use).
pub fn free_space(path: &Path) -> Result<u64, ModelError> {
    let s = statvfs(path).map_err(io)?;
    Ok(s.f_bavail.saturating_mul(s.f_frsize))
}

/// The XDG location offered as a suggestion; the user must accept or replace it.
pub fn suggest_root(xdg_data_home: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    let base = match (xdg_data_home, home) {
        (Some(x), _) if Path::new(x).is_absolute() => PathBuf::from(x),
        (_, Some(h)) if Path::new(h).is_absolute() => Path::new(h).join(".local/share"),
        _ => return None,
    };
    Some(base.join("cfa4"))
}

/// The disposable runtime cache; deleting it never touches a store.
pub fn cache_dir(xdg_cache_home: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    let base = match (xdg_cache_home, home) {
        (Some(x), _) if Path::new(x).is_absolute() => PathBuf::from(x),
        (_, Some(h)) if Path::new(h).is_absolute() => Path::new(h).join(".cache"),
        _ => return None,
    };
    Some(base.join("cfa4/models"))
}

/// What happened to a staged file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Promoted {
    /// Moved into the store at this path.
    Stored(PathBuf),
    /// The store already had this hash; the staged copy was dropped (one copy per hash, D4).
    AlreadyPresent(PathBuf),
}

/// One chosen model store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreLocation {
    root: PathBuf,
}

impl StoreLocation {
    /// Checks a root the user chose: absolute, an existing directory, writable, and with room for
    /// `needed_bytes` plus [`SPACE_MARGIN`]. Creates the three areas under it, nothing else.
    pub fn choose(root: &Path, needed_bytes: u64) -> Result<Self, ModelError> {
        if !root.is_absolute() {
            return Err(ModelError::Invalid { field: "store location", reason: "must be an absolute path" });
        }
        let root = root.canonicalize().map_err(io)?;
        if !root.is_dir() {
            return Err(ModelError::Invalid { field: "store location", reason: "not a directory" });
        }
        let need = needed_bytes.saturating_add(SPACE_MARGIN);
        if free_space(&root)? < need {
            return Err(ModelError::Invalid { field: "store location", reason: "not enough free space" });
        }
        let loc = Self { root };
        for d in [loc.store_dir(), loc.staging_dir(), loc.quarantine_dir()] {
            fs::create_dir_all(&d).map_err(|_| ModelError::Invalid { field: "store location", reason: "not writable" })?;
        }
        Ok(loc)
    }

    /// The chosen root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Durable store.
    pub fn store_dir(&self) -> PathBuf {
        self.root.join(STORE_DIR)
    }

    /// Where downloads and imports land first.
    pub fn staging_dir(&self) -> PathBuf {
        self.root.join(STAGING_DIR)
    }

    /// Where a model with a scan finding or a failed check waits.
    pub fn quarantine_dir(&self) -> PathBuf {
        self.root.join(QUARANTINE_DIR)
    }

    /// Store path of a hash: `cfa4-models/<first two hex>/<hash>`. The name never comes from the source.
    pub fn store_path(&self, hash: &Sha256Hex) -> PathBuf {
        let h = hash.as_str();
        self.store_dir().join(h.get(..2).unwrap_or("00")).join(h)
    }

    fn staged(&self, name: &str) -> Result<PathBuf, ModelError> {
        if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') && name.len() <= 2 {
            return Err(ModelError::Invalid { field: "staged name", reason: "must be a plain file name" });
        }
        Ok(self.staging_dir().join(name))
    }

    /// Verifies a staged file against `expected` by hashing it, then moves it into the store with
    /// one rename. A wrong hash leaves the store untouched and the file in staging.
    pub fn promote(&self, staged_name: &str, expected: &Sha256Hex) -> Result<Promoted, ModelError> {
        let src = self.staged(staged_name)?;
        let (actual, _) = sha256_file(&src)?;
        if &actual != expected {
            return Err(ModelError::BadHash);
        }
        let dst = self.store_path(expected);
        if dst.exists() {
            fs::remove_file(&src).map_err(io)?;
            return Ok(Promoted::AlreadyPresent(dst));
        }
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }
        fs::rename(&src, &dst).map_err(io)?;
        Ok(Promoted::Stored(dst))
    }

    /// Moves a staged file to quarantine under its hash (never overwriting) and returns the path.
    pub fn quarantine(&self, staged_name: &str, hash: &Sha256Hex) -> Result<PathBuf, ModelError> {
        let src = self.staged(staged_name)?;
        let dst = self.quarantine_dir().join(hash.as_str());
        if dst.exists() {
            return Err(ModelError::Duplicate(hash.as_str().to_owned()));
        }
        fs::rename(&src, &dst).map_err(io)?;
        Ok(dst)
    }
}

/// The user's stores. Empty until the user chooses one.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Stores {
    locations: Vec<StoreLocation>,
}

impl Stores {
    /// No store chosen yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a chosen store (several are allowed; a repeated root is ignored).
    pub fn add(&mut self, loc: StoreLocation) {
        if !self.locations.contains(&loc) {
            self.locations.push(loc);
        }
    }

    /// The store for a model operation. With none chosen the operation is refused: no silent default.
    pub fn require(&self, index: usize) -> Result<&StoreLocation, ModelError> {
        self.locations.get(index).ok_or(ModelError::Invalid { field: "store location", reason: "none chosen: the user must choose one first" })
    }

    /// The chosen stores.
    pub fn all(&self) -> &[StoreLocation] {
        &self.locations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::sha256_reader;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn tmp() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!("cfa4-mm-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_staged(loc: &StoreLocation, name: &str, data: &[u8]) -> Sha256Hex {
        fs::write(loc.staging_dir().join(name), data).unwrap();
        sha256_reader(data).unwrap().0
    }

    #[test]
    fn no_store_chosen_means_no_operation() {
        let s = Stores::new();
        assert!(s.require(0).is_err());
    }

    #[test]
    fn xdg_is_only_a_suggestion() {
        assert_eq!(suggest_root(Some("/data"), None), Some(PathBuf::from("/data/cfa4")));
        assert_eq!(suggest_root(None, Some("/home/u")), Some(PathBuf::from("/home/u/.local/share/cfa4")));
        assert_eq!(suggest_root(Some("relative"), None), None);
        assert_eq!(cache_dir(None, Some("/home/u")), Some(PathBuf::from("/home/u/.cache/cfa4/models")));
    }

    #[test]
    fn choose_checks_the_place() {
        assert!(StoreLocation::choose(Path::new("relative/dir"), 0).is_err());
        assert!(StoreLocation::choose(Path::new("/nonexistent/cfa4"), 0).is_err());
        let d = tmp();
        let f = d.join("file");
        fs::write(&f, b"x").unwrap();
        assert!(StoreLocation::choose(&f, 0).is_err());
        // More than the disk can hold is refused (AC-25), without overflow.
        assert!(StoreLocation::choose(&d, u64::MAX).is_err());
        let d = tmp();
        let loc = StoreLocation::choose(&d, 1).unwrap();
        assert!(loc.store_dir().is_dir() && loc.staging_dir().is_dir() && loc.quarantine_dir().is_dir());
        // Nothing but the three areas was created.
        let names: Vec<_> = fs::read_dir(loc.root()).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        assert_eq!(names.len(), 3, "{names:?}");
    }

    #[test]
    fn promote_is_a_rename_after_hash_check() {
        let d = tmp();
        let loc = StoreLocation::choose(&d, 0).unwrap();
        let h = write_staged(&loc, "m.part", b"model bytes");
        let Promoted::Stored(p) = loc.promote("m.part", &h).unwrap() else { panic!("not stored") };
        assert_eq!(fs::read(&p).unwrap(), b"model bytes");
        assert!(!loc.staging_dir().join("m.part").exists());
    }

    #[test]
    fn wrong_hash_leaves_the_store_untouched() {
        let d = tmp();
        let loc = StoreLocation::choose(&d, 0).unwrap();
        write_staged(&loc, "m.part", b"model bytes");
        let other = sha256_reader(&b"other"[..]).unwrap().0;
        assert_eq!(loc.promote("m.part", &other), Err(ModelError::BadHash));
        assert!(loc.staging_dir().join("m.part").exists());
        assert!(!loc.store_path(&other).exists());
    }

    #[test]
    fn same_hash_is_one_copy() {
        let d = tmp();
        let loc = StoreLocation::choose(&d, 0).unwrap();
        let h = write_staged(&loc, "a.part", b"same");
        loc.promote("a.part", &h).unwrap();
        write_staged(&loc, "b.part", b"same");
        assert!(matches!(loc.promote("b.part", &h), Ok(Promoted::AlreadyPresent(_))));
        assert!(!loc.staging_dir().join("b.part").exists());
    }

    #[test]
    fn staged_names_cannot_escape() {
        let d = tmp();
        let loc = StoreLocation::choose(&d, 0).unwrap();
        let h = sha256_reader(&b""[..]).unwrap().0;
        for bad in ["../x", "a/b", "", "..", "."] {
            assert!(loc.promote(bad, &h).is_err(), "{bad}");
        }
    }

    #[test]
    fn quarantine_never_overwrites() {
        let d = tmp();
        let loc = StoreLocation::choose(&d, 0).unwrap();
        let h = write_staged(&loc, "bad.part", b"suspicious");
        let q = loc.quarantine("bad.part", &h).unwrap();
        assert!(q.exists());
        write_staged(&loc, "bad2.part", b"suspicious");
        assert!(matches!(loc.quarantine("bad2.part", &h), Err(ModelError::Duplicate(_))));
    }

    #[test]
    fn clearing_the_cache_leaves_the_store() {
        let d = tmp();
        let cache = d.join("cache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("tmp"), b"x").unwrap();
        let store_root = tmp();
        let loc = StoreLocation::choose(&store_root, 0).unwrap();
        let h = write_staged(&loc, "m.part", b"keep me");
        let Promoted::Stored(p) = loc.promote("m.part", &h).unwrap() else { panic!("not stored") };
        fs::remove_dir_all(&cache).unwrap();
        assert!(p.exists());
    }
}
