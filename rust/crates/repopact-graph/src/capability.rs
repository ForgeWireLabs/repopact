//! Explicit ROG capability declaration (WI063 adoption/backfill/clean-
//! clone checkpoint, Decision 0051). Resolves ROG-039's exact gap in
//! Decision 0044's original signal (`rog/manifest.json` exists): that
//! signal alone cannot distinguish "the graph was deliberately never
//! enabled" from "the graph was enabled and its committed durable
//! artifact then disappeared." This module reads/writes a narrow,
//! versioned, schema-validated, committed JSON record --
//! `governance/rog-capability.json` -- directly; it is never indexed as
//! a graph node and never given a `RecordKind` variant, since doing so
//! would recreate the closed-enum-in-a-durable-graph hazard Decisions
//! 0048/0049 exist to avoid, for a fact that has no reason to appear in
//! the graph at all.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CAPABILITY_RECORD_RELATIVE_PATH: &str = "governance/rog-capability.json";
const SCHEMA_POINTER: &str = "../repopact/schemas/rog-capability.schema.json";
pub const CURRENT_CAPABILITY_RECORD_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RogSetting {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CapabilitiesSection {
    rog: RogSetting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CapabilityRecord {
    #[serde(rename = "$schema")]
    schema: String,
    version: u32,
    capabilities: CapabilitiesSection,
}

/// The five orthogonal capability states (Decision 0051 section 1).
/// Never conflated with [`crate::status::Freshness`],
/// [`crate::overlay::GraphCoverageState`], or
/// [`crate::overlay::DurableFreshness`] -- those describe the *quality*
/// of a graph capability says should exist; this describes whether one
/// should exist at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    /// No capability record, no `rog/`. Valid; ROG disabled.
    LegacyAbsent,
    /// No capability record, `rog/` exists. Valid; graph binding --
    /// every graph produced under Decisions 0044-0050 stays exactly
    /// this state, requiring no migration.
    LegacyEnabled,
    /// Capability record says `disabled`.
    ExplicitDisabled,
    /// Capability record says `enabled`, `rog/` exists.
    ExplicitEnabled,
    /// Capability record says `enabled`, `rog/` is absent. A hard,
    /// binding failure (ROG-039) -- never reported as `LegacyAbsent`.
    EnabledMissing,
}

impl CapabilityState {
    /// Whether this state means the graph is binding: its manifest/
    /// freshness/validation requirements apply, and a defect is a real
    /// failure rather than "no graph, valid."
    pub fn is_binding(self) -> bool {
        matches!(
            self,
            Self::LegacyEnabled | Self::ExplicitEnabled | Self::EnabledMissing
        )
    }

    /// Whether the repository is valid *as far as capability alone is
    /// concerned* (independent of the bound graph's own freshness/
    /// corruption, which is checked separately when binding).
    pub fn is_valid_by_itself(self) -> bool {
        !matches!(self, Self::EnabledMissing)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    Io(String),
    Malformed(String),
}

impl std::fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(f, "capability record I/O error: {message}"),
            Self::Malformed(message) => write!(f, "capability record malformed: {message}"),
        }
    }
}

fn capability_path(repository_root: &Path) -> PathBuf {
    repository_root.join(CAPABILITY_RECORD_RELATIVE_PATH)
}

/// Read the capability declaration, if one exists. `Ok(None)` means no
/// record exists -- itself a meaningful, permanently-valid state
/// (Decision 0051 section 2), not an error. A malformed/unparseable
/// record IS an error: an adopter that committed a capability record at
/// all is making an explicit claim, and a broken claim must not be
/// silently treated as "no claim."
pub fn read_declaration(repository_root: &Path) -> Result<Option<RogSetting>, CapabilityError> {
    let path = capability_path(repository_root);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(CapabilityError::Io(error.to_string())),
    };
    let record: CapabilityRecord = serde_json::from_slice(&bytes)
        .map_err(|error| CapabilityError::Malformed(error.to_string()))?;
    Ok(Some(record.capabilities.rog))
}

/// Compute the current [`CapabilityState`] from the declaration (if
/// any) and whether `rog/` physically exists. Pure function, no I/O of
/// its own -- callers already have both inputs from their own checks.
pub fn resolve_state(declaration: Option<RogSetting>, rog_dir_exists: bool) -> CapabilityState {
    match (declaration, rog_dir_exists) {
        (None, false) => CapabilityState::LegacyAbsent,
        (None, true) => CapabilityState::LegacyEnabled,
        (Some(RogSetting::Disabled), _) => CapabilityState::ExplicitDisabled,
        (Some(RogSetting::Enabled), true) => CapabilityState::ExplicitEnabled,
        (Some(RogSetting::Enabled), false) => CapabilityState::EnabledMissing,
    }
}

/// Convenience: read the declaration and `rog/` existence for
/// `repository_root` and resolve the current [`CapabilityState`] in one
/// call.
pub fn current_state(repository_root: &Path) -> Result<CapabilityState, CapabilityError> {
    let declaration = read_declaration(repository_root)?;
    let rog_dir_exists = crate::durable::rog_root(repository_root)
        .join("manifest.json")
        .is_file();
    Ok(resolve_state(declaration, rog_dir_exists))
}

fn write_declaration(repository_root: &Path, setting: RogSetting) -> Result<(), CapabilityError> {
    let record = CapabilityRecord {
        schema: SCHEMA_POINTER.to_owned(),
        version: CURRENT_CAPABILITY_RECORD_VERSION,
        capabilities: CapabilitiesSection { rog: setting },
    };
    let path = capability_path(repository_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| CapabilityError::Io(error.to_string()))?;
    }
    let mut json = serde_json::to_string_pretty(&record)
        .map_err(|error| CapabilityError::Malformed(error.to_string()))?;
    json.push('\n');
    std::fs::write(&path, json).map_err(|error| CapabilityError::Io(error.to_string()))
}

/// Persist `enabled`. Called only after a durable graph install has
/// already succeeded (Decision 0051 section 3) -- never before. Callers
/// (`durable::write`) are responsible for that ordering; this function
/// itself performs no graph-state check, since by the time it is
/// called the graph is already known good.
pub fn persist_enabled(repository_root: &Path) -> Result<(), CapabilityError> {
    write_declaration(repository_root, RogSetting::Enabled)
}

/// Persist `disabled`. Callers (`disable`) are responsible for removing
/// `rog/` first, so a failure partway through this call never claims
/// `disabled` while a stale graph still sits on disk.
pub fn persist_disabled(repository_root: &Path) -> Result<(), CapabilityError> {
    write_declaration(repository_root, RogSetting::Disabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("repopact-capability-{name}-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn no_record_reads_as_none() {
        let root = temp_root("no-record");
        assert_eq!(read_declaration(&root).unwrap(), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn write_then_read_round_trips() {
        let root = temp_root("round-trip");
        persist_enabled(&root).unwrap();
        assert_eq!(read_declaration(&root).unwrap(), Some(RogSetting::Enabled));
        persist_disabled(&root).unwrap();
        assert_eq!(read_declaration(&root).unwrap(), Some(RogSetting::Disabled));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_record_is_an_error_not_none() {
        let root = temp_root("malformed");
        std::fs::create_dir_all(root.join("governance")).unwrap();
        std::fs::write(
            root.join(CAPABILITY_RECORD_RELATIVE_PATH),
            "{ not valid json",
        )
        .unwrap();
        assert!(matches!(
            read_declaration(&root),
            Err(CapabilityError::Malformed(_))
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn state_resolution_matrix() {
        assert_eq!(resolve_state(None, false), CapabilityState::LegacyAbsent);
        assert_eq!(resolve_state(None, true), CapabilityState::LegacyEnabled);
        assert_eq!(
            resolve_state(Some(RogSetting::Disabled), false),
            CapabilityState::ExplicitDisabled
        );
        assert_eq!(
            resolve_state(Some(RogSetting::Disabled), true),
            CapabilityState::ExplicitDisabled
        );
        assert_eq!(
            resolve_state(Some(RogSetting::Enabled), true),
            CapabilityState::ExplicitEnabled
        );
        assert_eq!(
            resolve_state(Some(RogSetting::Enabled), false),
            CapabilityState::EnabledMissing
        );
    }

    #[test]
    fn only_enabled_missing_is_invalid_by_itself() {
        assert!(CapabilityState::LegacyAbsent.is_valid_by_itself());
        assert!(CapabilityState::LegacyEnabled.is_valid_by_itself());
        assert!(CapabilityState::ExplicitDisabled.is_valid_by_itself());
        assert!(CapabilityState::ExplicitEnabled.is_valid_by_itself());
        assert!(!CapabilityState::EnabledMissing.is_valid_by_itself());
    }

    #[test]
    fn binding_matches_the_five_state_table() {
        assert!(!CapabilityState::LegacyAbsent.is_binding());
        assert!(CapabilityState::LegacyEnabled.is_binding());
        assert!(!CapabilityState::ExplicitDisabled.is_binding());
        assert!(CapabilityState::ExplicitEnabled.is_binding());
        assert!(CapabilityState::EnabledMissing.is_binding());
    }

    #[test]
    fn written_record_matches_the_committed_schema_shape() {
        let root = temp_root("schema-shape");
        persist_enabled(&root).unwrap();
        let text = std::fs::read_to_string(root.join(CAPABILITY_RECORD_RELATIVE_PATH)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["version"], 1);
        assert_eq!(value["capabilities"]["rog"], "enabled");
        assert_eq!(value["$schema"], SCHEMA_POINTER);
        assert!(text.ends_with('\n'));
        std::fs::remove_dir_all(root).unwrap();
    }
}
