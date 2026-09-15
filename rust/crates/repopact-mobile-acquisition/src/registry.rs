//! Decision 0057 §"Registry model" / §"Atomicity and crash safety": the
//! app-owned, native-owned workspace registry. A single JSON document,
//! mutated only through atomic temp-file-then-rename writes, serialized
//! through one native coordinator lock. Never persists credentials, never
//! silently resets on corruption.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::{AcquisitionError, AcquisitionResult, ErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionKind {
    SafDirectory,
    SafArchive,
    /// Reserved for Stage 2 (Decision 0056/0057); not produced by this work
    /// item.
    RemoteGit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitState {
    NonGit,
    GitMetadataPresent,
    /// Reserved for Stage 2.
    EmbeddedGitManaged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Allocating,
    Importing,
    Ready,
    Exporting,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportState {
    NeverExported,
    Exported,
    ChangedSinceExport,
    DivergenceUnknown,
    Diverged,
}

/// Bounded, non-cryptographic source-state snapshot captured at import time
/// (Decision 0057 §"Source-divergence detection"). Never treated as proof;
/// only as a best-effort basis for warning about obvious divergence before
/// a write-back.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFingerprint {
    pub relative_path_count: u64,
    pub aggregate_bytes: u64,
    /// Provider-reported identifiers, when available (e.g. a SAF document
    /// tree's last-modified marker). Opaque strings; never a credential.
    pub provider_markers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    pub workspace_id: String,
    pub display_name: String,
    pub acquisition_kind: AcquisitionKind,
    /// Opaque, non-credential source descriptor (Decision 0057). Never a
    /// raw content:// URI exposed to JS beyond what the record itself
    /// already is; never a token/secret.
    pub source_reference: String,
    pub git_state: GitState,
    pub lifecycle_state: LifecycleState,
    pub created_at: String,
    pub imported_at: Option<String>,
    pub last_export_state: ExportState,
    pub source_fingerprint: Option<SourceFingerprint>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RegistryDocument {
    #[serde(default)]
    workspaces: Vec<WorkspaceRecord>,
}

/// Native-owned, atomic, single-writer workspace registry. All reads and
/// writes are serialized through this coordinator's lock -- the frontend
/// never mutates the registry file directly.
#[derive(Debug)]
pub struct WorkspaceRegistry {
    path: PathBuf,
    document: Mutex<RegistryDocument>,
}

impl WorkspaceRegistry {
    /// Loads the registry at `path`, creating an empty one if it does not
    /// exist yet. A registry file that exists but fails to parse is a loud,
    /// typed failure -- it is never silently replaced with an empty
    /// registry, which would orphan every already-imported workspace's
    /// identity.
    pub fn open(path: impl Into<PathBuf>) -> AcquisitionResult<Self> {
        let path = path.into();
        let document = if path.is_file() {
            let raw = fs::read(&path).map_err(|error| {
                AcquisitionError::new(
                    ErrorCode::InternalIo,
                    format!("unable to read registry '{}': {error}", path.display()),
                )
            })?;
            serde_json::from_slice(&raw).map_err(|error| {
                AcquisitionError::new(
                    ErrorCode::InternalIo,
                    format!(
                        "registry '{}' is corrupt and was not modified: {error}",
                        path.display()
                    ),
                )
            })?
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    AcquisitionError::new(ErrorCode::InternalIo, error.to_string())
                })?;
            }
            RegistryDocument::default()
        };
        Ok(Self {
            path,
            document: Mutex::new(document),
        })
    }

    pub fn list(&self) -> Vec<WorkspaceRecord> {
        self.document
            .lock()
            .expect("registry poisoned")
            .workspaces
            .clone()
    }

    pub fn get(&self, workspace_id: &str) -> Option<WorkspaceRecord> {
        self.document
            .lock()
            .expect("registry poisoned")
            .workspaces
            .iter()
            .find(|record| record.workspace_id == workspace_id)
            .cloned()
    }

    /// Inserts or replaces a record and durably persists the registry
    /// before returning, via atomic temp-file-then-rename (Decision 0057
    /// §"Atomicity and crash safety").
    pub fn upsert(&self, record: WorkspaceRecord) -> AcquisitionResult<()> {
        let mut guard = self.document.lock().expect("registry poisoned");
        if let Some(existing) = guard
            .workspaces
            .iter_mut()
            .find(|existing| existing.workspace_id == record.workspace_id)
        {
            *existing = record;
        } else {
            guard.workspaces.push(record);
        }
        self.persist(&guard)
    }

    pub fn remove(&self, workspace_id: &str) -> AcquisitionResult<()> {
        let mut guard = self.document.lock().expect("registry poisoned");
        guard
            .workspaces
            .retain(|record| record.workspace_id != workspace_id);
        self.persist(&guard)
    }

    fn persist(&self, document: &RegistryDocument) -> AcquisitionResult<()> {
        let serialized = serde_json::to_vec_pretty(document)
            .map_err(|error| AcquisitionError::new(ErrorCode::InternalIo, error.to_string()))?;
        let parent = self.path.parent().ok_or_else(|| {
            AcquisitionError::new(
                ErrorCode::InternalIo,
                "registry path has no parent directory",
            )
        })?;
        // Same-filesystem temp file so the final rename is atomic.
        let temp_path = parent.join(format!(
            "registry.json.tmp-{}",
            uuid::Uuid::new_v4().simple()
        ));
        write_and_sync(&temp_path, &serialized)?;
        fs::rename(&temp_path, &self.path).map_err(|error| {
            let _ = fs::remove_file(&temp_path);
            AcquisitionError::new(
                ErrorCode::InternalIo,
                format!("unable to publish registry: {error}"),
            )
        })
    }
}

fn write_and_sync(path: &Path, bytes: &[u8]) -> AcquisitionResult<()> {
    use std::io::Write;
    let mut file = fs::File::create(path).map_err(|error| {
        AcquisitionError::new(
            ErrorCode::InternalIo,
            format!("unable to create '{}': {error}", path.display()),
        )
    })?;
    file.write_all(bytes)
        .map_err(|error| AcquisitionError::new(ErrorCode::InternalIo, error.to_string()))?;
    file.sync_all()
        .map_err(|error| AcquisitionError::new(ErrorCode::InternalIo, error.to_string()))
}

/// Deletes any stale `registry.json.tmp-*` write-ahead files left behind by
/// a crash mid-write (Decision 0057 §"Atomicity and crash safety" /
/// §"Registry recovery"). Safe to call unconditionally at startup: a
/// temp file is never referenced by anything until it is renamed over
/// `registry.json`, so an orphaned one can never be a valid published
/// state.
pub fn clean_stale_registry_temp_files(registry_dir: &Path) -> AcquisitionResult<u64> {
    let mut removed = 0;
    let entries = match fs::read_dir(registry_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(AcquisitionError::new(
                ErrorCode::InternalIo,
                error.to_string(),
            ))
        }
    };
    for entry in entries {
        let entry = entry
            .map_err(|error| AcquisitionError::new(ErrorCode::InternalIo, error.to_string()))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("registry.json.tmp-") {
            if fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record(id: &str) -> WorkspaceRecord {
        WorkspaceRecord {
            workspace_id: id.to_owned(),
            display_name: "Example".to_owned(),
            acquisition_kind: AcquisitionKind::SafDirectory,
            source_reference: "opaque-tree-ref".to_owned(),
            git_state: GitState::NonGit,
            lifecycle_state: LifecycleState::Ready,
            created_at: "2026-09-14T00:00:00Z".to_owned(),
            imported_at: Some("2026-09-14T00:00:05Z".to_owned()),
            last_export_state: ExportState::NeverExported,
            source_fingerprint: None,
        }
    }

    #[test]
    fn round_trips_a_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let registry = WorkspaceRegistry::open(&path).unwrap();
        registry.upsert(sample_record("ws-1")).unwrap();
        assert!(path.is_file());

        let reopened = WorkspaceRegistry::open(&path).unwrap();
        assert_eq!(reopened.list().len(), 1);
        assert_eq!(reopened.get("ws-1").unwrap().display_name, "Example");
    }

    #[test]
    fn remove_deletes_the_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let registry = WorkspaceRegistry::open(&path).unwrap();
        registry.upsert(sample_record("ws-1")).unwrap();
        registry.remove("ws-1").unwrap();
        assert!(registry.list().is_empty());
        let reopened = WorkspaceRegistry::open(&path).unwrap();
        assert!(reopened.list().is_empty());
    }

    #[test]
    fn corrupt_registry_fails_loudly_rather_than_resetting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("registry.json");
        fs::write(&path, b"{ this is not valid json").unwrap();
        let err = WorkspaceRegistry::open(&path).unwrap_err();
        assert_eq!(err.code, ErrorCode::InternalIo);
        // The corrupt file must be left untouched, not overwritten.
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{ this is not valid json"
        );
    }

    #[test]
    fn no_secret_field_exists_on_the_serialized_record() {
        let json = serde_json::to_string(&sample_record("ws-1")).unwrap();
        for banned in ["password", "token", "secret", "credential", "private_key"] {
            assert!(
                !json.to_lowercase().contains(banned),
                "serialized workspace record must never contain a '{banned}' field"
            );
        }
    }

    #[test]
    fn cleans_stale_temp_files_without_touching_the_published_registry() {
        let dir = tempfile::tempdir().unwrap();
        let registry_path = dir.path().join("registry.json");
        let registry = WorkspaceRegistry::open(&registry_path).unwrap();
        registry.upsert(sample_record("ws-1")).unwrap();

        fs::write(dir.path().join("registry.json.tmp-deadbeef"), b"stale").unwrap();
        let removed = clean_stale_registry_temp_files(dir.path()).unwrap();
        assert_eq!(removed, 1);
        assert!(registry_path.is_file());
        let reopened = WorkspaceRegistry::open(&registry_path).unwrap();
        assert_eq!(reopened.list().len(), 1);
    }
}
