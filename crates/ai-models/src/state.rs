//! Stored model states (plan §3.1: 13 stored; the rest of the order's 19 are derived),
//! source trust (§3.2) and source status (D9/a).

use serde::{Deserialize, Serialize};

use crate::error::ModelError;

/// The state kept in the model record. `IN_USE`, `AVAILABLE`, `VERIFIED` and `ADMITTED` are
/// derived from `model_use` and the verification record, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelState {
    Discovered,
    Installable,
    /// The user accepted the licence warning.
    Approved,
    Acquiring,
    Paused,
    Verifying,
    Installed,
    UpdateAvailable,
    Incompatible,
    Blocked,
    Quarantined,
    Retired,
    Removing,
    Removed,
}

impl ModelState {
    /// The snake_case name (same as the serialized form).
    pub fn name(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Installable => "installable",
            Self::Approved => "approved",
            Self::Acquiring => "acquiring",
            Self::Paused => "paused",
            Self::Verifying => "verifying",
            Self::Installed => "installed",
            Self::UpdateAvailable => "update_available",
            Self::Incompatible => "incompatible",
            Self::Blocked => "blocked",
            Self::Quarantined => "quarantined",
            Self::Retired => "retired",
            Self::Removing => "removing",
            Self::Removed => "removed",
        }
    }

    /// Whether a module may use the model (a partial, unverified or quarantined model never).
    pub fn usable(self) -> bool {
        matches!(self, Self::Installed | Self::UpdateAvailable | Self::Retired)
    }

    /// The allowed next states. A failed or cancelled download goes back to `installable`;
    /// the installed copy is never touched by a failed update (separate revision record).
    pub fn next(self) -> &'static [ModelState] {
        use ModelState::*;
        match self {
            Discovered => &[Installable, Blocked],
            Installable => &[Approved, Blocked, Retired],
            Approved => &[Acquiring, Installable],
            Acquiring => &[Paused, Verifying, Installable],
            Paused => &[Acquiring, Installable],
            Verifying => &[Installed, Quarantined, Blocked, Installable],
            Installed => &[UpdateAvailable, Incompatible, Retired, Removing, Quarantined],
            UpdateAvailable => &[Installed, Incompatible, Retired, Removing],
            Incompatible => &[Installed, Retired, Removing],
            Blocked => &[Installable, Discovered, Removing],
            Quarantined => &[Installed, Removing],
            Retired => &[Removing, Installed],
            Removing => &[Removed, Installed],
            Removed => &[Discovered, Installable],
        }
    }

    /// Checks and performs a transition.
    pub fn transition(self, to: ModelState) -> Result<ModelState, ModelError> {
        if self.next().contains(&to) { Ok(to) } else { Err(ModelError::BadTransition { from: self.name(), to: to.name() }) }
    }
}

/// How far the origin of a model is trusted (plan §3.2). Only an unknown or blocked origin may
/// stop an install; the licence never does (D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTrust {
    Canonical,
    Validated,
    Attributed,
    /// Installable only with the user's explicit approval.
    Unverified,
    Blocked,
}

impl SourceTrust {
    /// Whether the user's explicit confirmation is needed before acquiring.
    pub fn needs_confirmation(self) -> bool {
        matches!(self, Self::Unverified)
    }

    /// Whether acquiring is refused outright.
    pub fn refused(self) -> bool {
        matches!(self, Self::Blocked)
    }
}

/// State of the origin, kept apart from the model state (CFA3 comparison, D9/a). A temporary
/// outage is not a discontinuation; only verified information moves it to a final value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Ok,
    TemporarilyUnreachable,
    PermanentlyUnreachable,
    Discontinued,
    Archived,
    Withdrawn,
}

impl SourceStatus {
    /// The model is gone or going: the UI offers keep or remove and suggests substitutes (AC-23, AC-26).
    pub fn gone(self) -> bool {
        matches!(self, Self::PermanentlyUnreachable | Self::Discontinued | Self::Archived | Self::Withdrawn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [ModelState; 14] = [
        ModelState::Discovered,
        ModelState::Installable,
        ModelState::Approved,
        ModelState::Acquiring,
        ModelState::Paused,
        ModelState::Verifying,
        ModelState::Installed,
        ModelState::UpdateAvailable,
        ModelState::Incompatible,
        ModelState::Blocked,
        ModelState::Quarantined,
        ModelState::Retired,
        ModelState::Removing,
        ModelState::Removed,
    ];

    #[test]
    fn no_activation_without_verification() {
        // Only `verifying` leads to `installed` from the acquisition path (AC-07/08).
        for s in [ModelState::Discovered, ModelState::Installable, ModelState::Approved, ModelState::Acquiring, ModelState::Paused] {
            assert!(s.transition(ModelState::Installed).is_err(), "{s:?}");
        }
        assert!(ModelState::Verifying.transition(ModelState::Installed).is_ok());
    }

    #[test]
    fn quarantine_is_left_only_by_override_or_removal() {
        assert_eq!(ModelState::Quarantined.next(), &[ModelState::Installed, ModelState::Removing]);
    }

    #[test]
    fn partial_models_are_not_usable() {
        for s in [ModelState::Acquiring, ModelState::Paused, ModelState::Verifying, ModelState::Quarantined, ModelState::Blocked] {
            assert!(!s.usable());
        }
        assert!(ModelState::Installed.usable() && ModelState::Retired.usable());
    }

    #[test]
    fn serde_name_matches_name() {
        for s in ALL {
            let j = serde_json::to_string(&s).unwrap();
            assert_eq!(j, format!("\"{}\"", s.name()));
        }
    }

    #[test]
    fn source_status_semantics() {
        assert!(!SourceStatus::TemporarilyUnreachable.gone());
        assert!(!SourceStatus::Ok.gone());
        for s in [SourceStatus::PermanentlyUnreachable, SourceStatus::Discontinued, SourceStatus::Archived, SourceStatus::Withdrawn] {
            assert!(s.gone());
        }
        assert!(SourceTrust::Unverified.needs_confirmation() && !SourceTrust::Unverified.refused());
        assert!(SourceTrust::Blocked.refused());
    }
}
