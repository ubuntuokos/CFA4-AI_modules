//! The catalogue: records with search and filter (plan §4). In memory here; the PostgreSQL
//! `ai.model*` tables (MM-F2, PG-3) answer the same [`Filter`] in the database. A missing datum
//! is `None` ("unknown"), never invented. Nothing here touches the network: opening the
//! catalogue, filtering or suggesting never downloads (AC-03, AC-21).

use std::collections::BTreeMap;

use crate::error::ModelError;
use crate::format::ModelFormat;
use crate::hash::Sha256Hex;
use crate::record::{ModelId, ModelRecord};
use crate::state::ModelState;

/// A catalogue query. Every field that is `None` does not restrict.
#[derive(Debug, Default, Clone)]
pub struct Filter {
    /// Case-insensitive substring of the id or the family.
    pub text: Option<String>,
    pub capability: Option<String>,
    pub format: Option<ModelFormat>,
    pub state: Option<ModelState>,
    /// Only models a module can use now.
    pub usable_only: bool,
    /// Largest VRAM need in MiB; a model with unknown need is kept (it is "unknown", not "too big").
    pub max_vram_mib: Option<u64>,
}

/// Records by id.
#[derive(Debug, Default)]
pub struct Catalog {
    records: BTreeMap<ModelId, ModelRecord>,
}

impl Catalog {
    /// An empty catalogue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the catalogue is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Adds a validated record; a second record with the same id is refused (a new revision
    /// is a new id-revision pair kept in the PostgreSQL schema, MM-F6).
    pub fn insert(&mut self, rec: ModelRecord) -> Result<(), ModelError> {
        rec.validate()?;
        if self.records.contains_key(&rec.id) {
            return Err(ModelError::Duplicate(rec.id.as_str().to_owned()));
        }
        self.records.insert(rec.id.clone(), rec);
        Ok(())
    }

    /// One record.
    pub fn get(&self, id: &ModelId) -> Option<&ModelRecord> {
        self.records.get(id)
    }

    /// Moves a record to a new state if the transition is allowed.
    pub fn set_state(&mut self, id: &ModelId, to: ModelState) -> Result<(), ModelError> {
        let rec = self.records.get_mut(id).ok_or(ModelError::Invalid { field: "model id", reason: "not in the catalogue" })?;
        rec.state = rec.state.transition(to)?;
        Ok(())
    }

    /// Records matching the filter, in id order.
    pub fn search(&self, f: &Filter) -> Vec<&ModelRecord> {
        let text = f.text.as_ref().map(|t| t.to_lowercase());
        self.records
            .values()
            .filter(|r| {
                text.as_ref().is_none_or(|t| r.id.as_str().contains(t.as_str()) || r.family.to_lowercase().contains(t.as_str()))
                    && f.capability.as_ref().is_none_or(|c| r.capabilities.contains(c))
                    && f.format.is_none_or(|fm| r.artifacts.iter().any(|a| a.format == fm))
                    && f.state.is_none_or(|s| r.state == s)
                    && (!f.usable_only || r.state.usable())
                    && f.max_vram_mib.is_none_or(|m| r.vram_mib.is_none_or(|v| v <= m))
            })
            .collect()
    }

    /// Records that hold an artifact with this hash: one physical copy per hash (D4), so a
    /// model shared by several records must not be removed from under another capability.
    pub fn owners_of(&self, hash: &Sha256Hex) -> Vec<&ModelId> {
        self.records.values().filter(|r| r.artifacts.iter().any(|a| &a.sha256 == hash)).map(|r| &r.id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::tests::sample;

    fn cat() -> Catalog {
        let mut c = Catalog::new();
        let mut a = sample("silero/vad");
        a.vram_mib = Some(100);
        let mut b = sample("acme/upscaler");
        b.capabilities = vec!["video.upscale".into()];
        b.family = "Acme Net".into();
        b.vram_mib = Some(9000);
        b.artifacts[0].sha256 = Sha256Hex::parse(&"b".repeat(64)).unwrap();
        let mut d = sample("acme/unknown-need");
        d.capabilities = vec!["video.upscale".into()];
        d.artifacts[0].sha256 = Sha256Hex::parse(&"c".repeat(64)).unwrap();
        for r in [a, b, d] {
            c.insert(r).unwrap();
        }
        c
    }

    #[test]
    fn filters() {
        let c = cat();
        assert_eq!(c.search(&Filter::default()).len(), 3);
        let by_cap = Filter { capability: Some("video.upscale".into()), ..Filter::default() };
        assert_eq!(c.search(&by_cap).len(), 2);
        let by_text = Filter { text: Some("ACME NET".into()), ..Filter::default() };
        assert_eq!(c.search(&by_text).len(), 1);
        // Unknown VRAM need is kept, a known too-large one is dropped.
        let small = Filter { max_vram_mib: Some(1000), ..Filter::default() };
        assert_eq!(c.search(&small).len(), 2);
        let none = Filter { usable_only: true, ..Filter::default() };
        assert!(c.search(&none).is_empty());
    }

    #[test]
    fn duplicate_and_invalid_are_refused() {
        let mut c = cat();
        assert!(matches!(c.insert(sample("silero/vad")), Err(ModelError::Duplicate(_))));
        let mut bad = sample("x/y");
        bad.revision = serde_json::from_str("\"v1\"").unwrap();
        bad.artifacts.clear();
        assert!(c.insert(bad).is_err());
    }

    #[test]
    fn state_changes_follow_the_table() {
        let mut c = cat();
        let id = ModelId::new("silero/vad").unwrap();
        assert!(c.set_state(&id, ModelState::Installed).is_err());
        c.set_state(&id, ModelState::Installable).unwrap();
        assert!(c.set_state(&ModelId::new("nope/x").unwrap(), ModelState::Installable).is_err());
    }

    #[test]
    fn shared_hash_is_found() {
        let mut c = cat();
        let mut e = sample("other/shared");
        e.artifacts[0].sha256 = Sha256Hex::parse(&"b".repeat(64)).unwrap();
        c.insert(e).unwrap();
        assert_eq!(c.owners_of(&Sha256Hex::parse(&"b".repeat(64)).unwrap()).len(), 2);
    }
}
