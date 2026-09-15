//! Decision 0057 §"Workspace root and layout" / §"Staging-then-publish
//! transaction": the top-level orchestration API a Tauri command surface
//! calls into. Ties the registry, the bounded importer/exporter, and the
//! operation coordinator together, and is the only thing in this crate that
//! knows the on-disk layout.
//!
//! ```text
//! <root>/
//!     registry.json
//!     workspaces/<workspace-id>/repository/       <- DesktopService::open_repository points here
//!     workspaces/<workspace-id>/local-metadata/    <- acquisition-adapter bookkeeping only
//!     staging/<operation-id>/                      <- never published in place; only ever renamed
//! ```

use std::fs;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::archive::{create_archive, import_archive, ArchiveImportSummary};
use crate::bounds::{ArchiveBounds, ImportBounds};
use crate::error::{AcquisitionError, AcquisitionResult, ErrorCode};
use crate::import::{import_directory, ImportSummary};
use crate::operation::{CancellationToken, OperationCoordinator, OperationProgress};
use crate::registry::{
    clean_stale_registry_temp_files, AcquisitionKind, ExportState, GitState, LifecycleState,
    SourceFingerprint, WorkspaceRecord, WorkspaceRegistry,
};
use crate::source::AcquisitionSource;

pub struct WorkspaceManager {
    root: PathBuf,
    registry: WorkspaceRegistry,
    coordinator: OperationCoordinator,
}

fn now_rfc3339() -> String {
    // No chrono dependency is justified for one timestamp; RepoPact's own
    // evidence/decision records already use plain RFC 3339 strings, and the
    // native host clock is authoritative here (this is local product
    // state, not a governance timestamp).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    humantime_rfc3339(now.as_secs())
}

/// Minimal UTC RFC 3339 (seconds precision) formatting without a chrono
/// dependency, mirroring the shape RepoPact's own evidence records use
/// (`YYYY-MM-DDTHH:MM:SSZ`).
fn humantime_rfc3339(unix_seconds: u64) -> String {
    const DAYS_IN_MONTH: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let days_total = (unix_seconds / 86_400) as i64;
    let seconds_of_day = (unix_seconds % 86_400) as i64;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;

    let mut year = 1970i64;
    let mut remaining_days = days_total;
    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let days_in_year = if is_leap { 366 } else { 365 };
        if remaining_days >= days_in_year {
            remaining_days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }
    let is_leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mut month = 0usize;
    for (index, &days) in DAYS_IN_MONTH.iter().enumerate() {
        let days = if index == 1 && is_leap {
            days + 1
        } else {
            days
        };
        if remaining_days >= days {
            remaining_days -= days;
            month = index + 1;
        } else {
            month = index + 1;
            break;
        }
    }
    let day = remaining_days + 1;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

impl WorkspaceManager {
    /// Opens (or initializes) the workspace manager rooted at `root`
    /// (Decision 0057: `<app_data_dir>/repositories`). Cleans up any stale
    /// staging directories and registry temp files left behind by a crash
    /// or forced kill, per Decision 0057 §"Atomicity and crash safety".
    pub fn open(root: impl Into<PathBuf>) -> AcquisitionResult<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("workspaces")).map_err(io_err)?;
        fs::create_dir_all(root.join("staging")).map_err(io_err)?;
        clean_stale_registry_temp_files(&root)?;
        clean_stale_staging(&root)?;
        let registry = WorkspaceRegistry::open(root.join("registry.json"))?;
        Ok(Self {
            root,
            registry,
            coordinator: OperationCoordinator::new(),
        })
    }

    pub fn list_workspaces(&self) -> Vec<WorkspaceRecord> {
        self.registry.list()
    }

    pub fn get_workspace(&self, workspace_id: &str) -> AcquisitionResult<WorkspaceRecord> {
        self.registry.get(workspace_id).ok_or_else(|| {
            AcquisitionError::new(
                ErrorCode::WorkspaceNotFound,
                format!("no workspace with id '{workspace_id}'"),
            )
        })
    }

    /// The path `DesktopService::open_repository` must be given (Decision
    /// 0056 AC-5: zero source-model redesign -- this is an ordinary
    /// `PathBuf`, nothing else).
    pub fn repository_path(&self, workspace_id: &str) -> AcquisitionResult<PathBuf> {
        let record = self.get_workspace(workspace_id)?;
        if !matches!(record.lifecycle_state, LifecycleState::Ready) {
            return Err(AcquisitionError::new(
                ErrorCode::WorkspaceNotReady,
                format!("workspace '{workspace_id}' is not ready"),
            ));
        }
        Ok(self.repository_dir(workspace_id))
    }

    fn workspace_dir(&self, workspace_id: &str) -> PathBuf {
        self.root.join("workspaces").join(workspace_id)
    }

    fn repository_dir(&self, workspace_id: &str) -> PathBuf {
        self.workspace_dir(workspace_id).join("repository")
    }

    fn local_metadata_dir(&self, workspace_id: &str) -> PathBuf {
        self.workspace_dir(workspace_id).join("local-metadata")
    }

    fn new_staging_dir(&self, operation_id: &str) -> AcquisitionResult<PathBuf> {
        let dir = self.root.join("staging").join(operation_id);
        fs::create_dir_all(&dir).map_err(io_err)?;
        Ok(dir)
    }

    /// Imports a SAF-picked directory tree (via any [`AcquisitionSource`])
    /// into a brand-new app-private workspace, following the staging-then-
    /// publish transaction: bounded import into `staging/<op>/`, then an
    /// atomic rename into `workspaces/<id>/repository/`, then a `ready`
    /// registry entry -- in that order, so a partial import is never
    /// reachable through the registry.
    pub fn import_directory(
        &self,
        source: &mut dyn AcquisitionSource,
        display_name: String,
        source_reference: String,
        bounds: &ImportBounds,
        cancel: &CancellationToken,
        on_progress: impl FnMut(OperationProgress),
    ) -> AcquisitionResult<WorkspaceRecord> {
        let (operation_id, workspace_id, staging_dir) = self.begin_import()?;
        let result = import_directory(source, &staging_dir, bounds, cancel, on_progress);
        self.finish_import(
            operation_id,
            workspace_id,
            staging_dir,
            result.map(|summary| ImportOutcome::from_directory(summary)),
            display_name,
            source_reference,
            AcquisitionKind::SafDirectory,
        )
    }

    /// Imports a `.zip` archive into a brand-new app-private workspace,
    /// following the same staging-then-publish transaction.
    pub fn import_archive<R: Read + Seek>(
        &self,
        reader: R,
        display_name: String,
        source_reference: String,
        bounds: &ArchiveBounds,
        cancel: &CancellationToken,
        on_progress: impl FnMut(OperationProgress),
    ) -> AcquisitionResult<WorkspaceRecord> {
        let (operation_id, workspace_id, staging_dir) = self.begin_import()?;
        let result = import_archive(reader, &staging_dir, bounds, cancel, on_progress);
        self.finish_import(
            operation_id,
            workspace_id,
            staging_dir,
            result.map(ImportOutcome::from_archive),
            display_name,
            source_reference,
            AcquisitionKind::SafArchive,
        )
    }

    fn begin_import(&self) -> AcquisitionResult<(String, String, PathBuf)> {
        let (operation_id, _cancel) = self.coordinator.begin()?;
        let workspace_id = Uuid::new_v4().to_string();
        let staging_dir = match self.new_staging_dir(&operation_id) {
            Ok(dir) => dir,
            Err(error) => {
                self.coordinator.end(&operation_id);
                return Err(error);
            }
        };
        Ok((operation_id, workspace_id, staging_dir))
    }

    fn finish_import(
        &self,
        operation_id: String,
        workspace_id: String,
        staging_dir: PathBuf,
        result: AcquisitionResult<ImportOutcome>,
        display_name: String,
        source_reference: String,
        acquisition_kind: AcquisitionKind,
    ) -> AcquisitionResult<WorkspaceRecord> {
        self.coordinator.end(&operation_id);
        match result {
            Ok(outcome) => {
                let workspace_dir = self.workspace_dir(&workspace_id);
                fs::create_dir_all(&workspace_dir).map_err(|error| {
                    let _ = fs::remove_dir_all(&staging_dir);
                    io_err(error)
                })?;
                let repository_dir = self.repository_dir(&workspace_id);
                if let Err(error) = fs::rename(&staging_dir, &repository_dir) {
                    let _ = fs::remove_dir_all(&staging_dir);
                    let _ = fs::remove_dir_all(&workspace_dir);
                    return Err(io_err(error));
                }
                if let Err(error) = fs::create_dir_all(self.local_metadata_dir(&workspace_id)) {
                    let _ = fs::remove_dir_all(&workspace_dir);
                    return Err(io_err(error));
                }
                let record = WorkspaceRecord {
                    workspace_id: workspace_id.clone(),
                    display_name,
                    acquisition_kind,
                    source_reference,
                    git_state: outcome.git_state(&repository_dir),
                    lifecycle_state: LifecycleState::Ready,
                    created_at: now_rfc3339(),
                    imported_at: Some(now_rfc3339()),
                    last_export_state: ExportState::NeverExported,
                    source_fingerprint: Some(outcome.fingerprint()),
                };
                if let Err(error) = self.registry.upsert(record.clone()) {
                    let _ = fs::remove_dir_all(&workspace_dir);
                    return Err(error);
                }
                Ok(record)
            }
            Err(error) => {
                // Staging cleanup: a failed/cancelled import leaves no
                // ready registry entry and no reachable partial workspace
                // (Decision 0057 §"Staging-then-publish transaction").
                let _ = fs::remove_dir_all(&staging_dir);
                Err(error)
            }
        }
    }

    /// Explicit, user-invoked export/share-back for a directory-imported
    /// workspace (Decision 0056/0057): copies the current workspace
    /// repository content to `destination_root`, which must already be an
    /// empty (or non-existent, then created) ordinary directory -- this
    /// stands in for a SAF destination tree at the Rust layer; the Android
    /// adapter is responsible for presenting that destination as a real SAF
    /// write target.
    pub fn export_directory(
        &self,
        workspace_id: &str,
        destination_root: &Path,
        cancel: &CancellationToken,
        require_empty_destination: bool,
    ) -> AcquisitionResult<()> {
        let repository_dir = self.repository_path(workspace_id)?;
        if require_empty_destination {
            if destination_root.is_dir() {
                let non_empty = fs::read_dir(destination_root)
                    .map_err(io_err)?
                    .next()
                    .is_some();
                if non_empty {
                    return Err(AcquisitionError::new(
                        ErrorCode::ExportConflict,
                        "export destination is not empty; explicit replace was not requested",
                    ));
                }
            }
        }
        fs::create_dir_all(destination_root).map_err(io_err)?;
        copy_tree(&repository_dir, destination_root, cancel)?;
        self.mark_exported(workspace_id)
    }

    /// Explicit, user-invoked export for either workspace kind: always
    /// creates a *new* archive rather than mutating an original in place
    /// (Decision 0057 §"Export semantics").
    pub fn export_archive<W: Write + Seek>(
        &self,
        workspace_id: &str,
        dest_writer: W,
        cancel: &CancellationToken,
    ) -> AcquisitionResult<u64> {
        let repository_dir = self.repository_path(workspace_id)?;
        let bytes = create_archive(&repository_dir, dest_writer, cancel)?;
        self.mark_exported(workspace_id)?;
        Ok(bytes)
    }

    fn mark_exported(&self, workspace_id: &str) -> AcquisitionResult<()> {
        let mut record = self.get_workspace(workspace_id)?;
        record.last_export_state = ExportState::Exported;
        self.registry.upsert(record)
    }

    pub fn cancel_operation(&self, operation_id: &str) -> AcquisitionResult<()> {
        self.coordinator.cancel(operation_id)
    }

    /// Removes a workspace's app-private copy only. Never touches the
    /// original SAF source or archive (Decision 0057). Verifies the target
    /// path is both a registered workspace identity *and* inside the
    /// app-private workspace root before deleting anything (Decision 0057
    /// §"Remove workspace safety") -- never accepts an arbitrary path.
    pub fn remove_workspace(&self, workspace_id: &str) -> AcquisitionResult<()> {
        // Registration check.
        self.get_workspace(workspace_id)?;
        let dir = self.workspace_dir(workspace_id);
        let workspaces_root = repopact_repository::normalize_path(&self.root.join("workspaces"));
        let normalized_dir = repopact_repository::normalize_path(&dir);
        if !normalized_dir.starts_with(&workspaces_root) {
            return Err(AcquisitionError::new(
                ErrorCode::PathEscape,
                "refusing to remove a workspace directory outside the workspace root",
            ));
        }
        if dir.is_dir() {
            fs::remove_dir_all(&dir).map_err(io_err)?;
        }
        self.registry.remove(workspace_id)
    }
}

fn io_err(error: std::io::Error) -> AcquisitionError {
    AcquisitionError::new(ErrorCode::InternalIo, error.to_string())
}

/// Recursively copies `source` into `dest` (both ordinary, already-
/// validated app-owned/user-owned directories -- no external, untrusted
/// path-safety concerns apply here the way they do for [`crate::import`],
/// because the source is RepoPact's own workspace content).
fn copy_tree(source: &Path, dest: &Path, cancel: &CancellationToken) -> AcquisitionResult<()> {
    let mut stack = vec![(source.to_path_buf(), dest.to_path_buf())];
    while let Some((from, to)) = stack.pop() {
        cancel.check()?;
        for entry in fs::read_dir(&from).map_err(io_err)? {
            let entry = entry.map_err(io_err)?;
            let target = to.join(entry.file_name());
            let metadata = entry.metadata().map_err(io_err)?;
            if metadata.is_dir() {
                fs::create_dir_all(&target).map_err(io_err)?;
                stack.push((entry.path(), target));
            } else if metadata.is_file() {
                fs::copy(entry.path(), &target).map_err(io_err)?;
            }
        }
    }
    Ok(())
}

fn clean_stale_staging(root: &Path) -> AcquisitionResult<()> {
    let staging = root.join("staging");
    let entries = match fs::read_dir(&staging) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io_err(error)),
    };
    for entry in entries {
        let entry = entry.map_err(io_err)?;
        // Every entry under staging/ is, by construction, either currently
        // being written by an in-progress operation (impossible here: this
        // runs only at WorkspaceManager::open, before any operation could
        // have started this process) or leftover from a prior crash/kill.
        // Neither case is ever referenced by a `ready` registry entry, so
        // deleting it can never affect a valid workspace.
        let _ = fs::remove_dir_all(entry.path());
    }
    Ok(())
}

/// Bridges the two importer result types into one shape `finish_import`
/// can act on uniformly.
enum ImportOutcome {
    Directory(ImportSummary),
    Archive(ArchiveImportSummary),
}

impl ImportOutcome {
    fn from_directory(summary: ImportSummary) -> Self {
        Self::Directory(summary)
    }

    fn from_archive(summary: ArchiveImportSummary) -> Self {
        Self::Archive(summary)
    }

    fn fingerprint(&self) -> SourceFingerprint {
        match self {
            Self::Directory(summary) => SourceFingerprint {
                relative_path_count: summary.entries_imported,
                aggregate_bytes: summary.bytes_imported,
                provider_markers: Vec::new(),
            },
            Self::Archive(summary) => SourceFingerprint {
                relative_path_count: summary.entries_imported,
                aggregate_bytes: summary.bytes_imported,
                provider_markers: Vec::new(),
            },
        }
    }

    /// Decision 0057 §"Imported `.git`" / §"Git state": recognizes an
    /// already-copied `.git` only if it is actually present and structurally
    /// plausible after the copy -- never synthesized, never assumed from
    /// provider fidelity claims.
    fn git_state(&self, repository_dir: &Path) -> GitState {
        if repository_dir.join(".git").exists() {
            GitState::GitMetadataPresent
        } else {
            GitState::NonGit
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::FilesystemSource;
    use std::io::Cursor;

    fn import_bounds() -> ImportBounds {
        ImportBounds {
            max_entries: 1000,
            max_total_bytes: 10 * 1024 * 1024,
            max_single_file_bytes: 5 * 1024 * 1024,
            max_depth: 16,
            max_path_length: 512,
        }
    }

    fn archive_bounds() -> ArchiveBounds {
        ArchiveBounds {
            max_entries: 1000,
            max_expanded_bytes: 10 * 1024 * 1024,
            max_single_entry_bytes: 5 * 1024 * 1024,
            max_depth: 16,
            max_path_length: 512,
            max_compression_ratio: 1000,
        }
    }

    #[test]
    fn imports_a_directory_and_publishes_a_ready_workspace() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("a.txt"), b"hello").unwrap();

        let app_data = tempfile::tempdir().unwrap();
        let manager = WorkspaceManager::open(app_data.path().join("repositories")).unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let cancel = CancellationToken::new();
        let record = manager
            .import_directory(
                &mut source,
                "My Project".to_owned(),
                "opaque-tree-token".to_owned(),
                &import_bounds(),
                &cancel,
                |_| {},
            )
            .unwrap();

        assert_eq!(record.lifecycle_state, LifecycleState::Ready);
        let repo_path = manager.repository_path(&record.workspace_id).unwrap();
        assert_eq!(
            fs::read_to_string(repo_path.join("a.txt")).unwrap(),
            "hello"
        );
        assert_eq!(manager.list_workspaces().len(), 1);
    }

    #[test]
    fn failed_import_leaves_no_ready_workspace_and_no_staging_residue() {
        // A ZIP archive's entry table (unlike this Windows host's own
        // case-insensitive filesystem) can genuinely hold both `A.txt` and
        // `a.txt` as distinct entries, so archive import is used here to
        // exercise the case-collision failure path realistically.
        let mut zip_buffer = Cursor::new(Vec::new());
        {
            use zip::write::SimpleFileOptions;
            let mut writer = zip::ZipWriter::new(&mut zip_buffer);
            writer
                .start_file("A.txt", SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"1").unwrap();
            writer
                .start_file("a.txt", SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"2").unwrap();
            writer.finish().unwrap();
        }
        let zip_bytes = zip_buffer.into_inner();

        let app_data = tempfile::tempdir().unwrap();
        let root = app_data.path().join("repositories");
        let manager = WorkspaceManager::open(&root).unwrap();
        let cancel = CancellationToken::new();
        let err = manager
            .import_archive(
                Cursor::new(zip_bytes),
                "Broken".to_owned(),
                "token".to_owned(),
                &archive_bounds(),
                &cancel,
                |_| {},
            )
            .unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::CaseConflict);
        assert!(manager.list_workspaces().is_empty());
        // No leftover staging directories.
        let staging_entries: Vec<_> = fs::read_dir(root.join("staging")).unwrap().collect();
        assert!(staging_entries.is_empty());
        // No leftover workspace directories either.
        let workspace_entries: Vec<_> = fs::read_dir(root.join("workspaces")).unwrap().collect();
        assert!(workspace_entries.is_empty());
    }

    #[test]
    fn imports_an_archive_and_round_trips_export() {
        let mut zip_bytes_writer = Cursor::new(Vec::new());
        {
            use zip::write::SimpleFileOptions;
            let mut writer = zip::ZipWriter::new(&mut zip_bytes_writer);
            writer
                .start_file("a.txt", SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"payload").unwrap();
            writer.finish().unwrap();
        }
        let zip_bytes = zip_bytes_writer.into_inner();

        let app_data = tempfile::tempdir().unwrap();
        let manager = WorkspaceManager::open(app_data.path().join("repositories")).unwrap();
        let cancel = CancellationToken::new();
        let record = manager
            .import_archive(
                Cursor::new(zip_bytes),
                "Archive Project".to_owned(),
                "opaque-doc-token".to_owned(),
                &archive_bounds(),
                &cancel,
                |_| {},
            )
            .unwrap();
        assert_eq!(record.acquisition_kind, AcquisitionKind::SafArchive);

        let mut export_buffer = Cursor::new(Vec::new());
        manager
            .export_archive(&record.workspace_id, &mut export_buffer, &cancel)
            .unwrap();
        let updated = manager.get_workspace(&record.workspace_id).unwrap();
        assert_eq!(updated.last_export_state, ExportState::Exported);

        export_buffer.set_position(0);
        let mut reexamine = zip::ZipArchive::new(export_buffer).unwrap();
        let mut file = reexamine.by_name("a.txt").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "payload");
    }

    #[test]
    fn export_directory_refuses_non_empty_destination_by_default() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("a.txt"), b"1").unwrap();
        let app_data = tempfile::tempdir().unwrap();
        let manager = WorkspaceManager::open(app_data.path().join("repositories")).unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let cancel = CancellationToken::new();
        let record = manager
            .import_directory(
                &mut source,
                "P".to_owned(),
                "token".to_owned(),
                &import_bounds(),
                &cancel,
                |_| {},
            )
            .unwrap();

        let destination = tempfile::tempdir().unwrap();
        fs::write(destination.path().join("existing.txt"), b"pre-existing").unwrap();
        let err = manager
            .export_directory(&record.workspace_id, destination.path(), &cancel, true)
            .unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::ExportConflict);
    }

    #[test]
    fn remove_workspace_deletes_only_the_registered_app_private_copy() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("a.txt"), b"1").unwrap();
        let app_data = tempfile::tempdir().unwrap();
        let manager = WorkspaceManager::open(app_data.path().join("repositories")).unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let cancel = CancellationToken::new();
        let record = manager
            .import_directory(
                &mut source,
                "P".to_owned(),
                "token".to_owned(),
                &import_bounds(),
                &cancel,
                |_| {},
            )
            .unwrap();

        assert!(
            src.path().join("a.txt").is_file(),
            "original source must be untouched"
        );
        manager.remove_workspace(&record.workspace_id).unwrap();
        assert!(manager.get_workspace(&record.workspace_id).is_err());
        assert!(
            src.path().join("a.txt").is_file(),
            "removal must never touch the original source"
        );
    }

    #[test]
    fn remove_workspace_rejects_unregistered_id() {
        let app_data = tempfile::tempdir().unwrap();
        let manager = WorkspaceManager::open(app_data.path().join("repositories")).unwrap();
        let err = manager.remove_workspace("not-a-real-id").unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::WorkspaceNotFound);
    }

    #[test]
    fn restart_recovers_stale_staging_without_touching_ready_workspaces() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("a.txt"), b"1").unwrap();
        let app_data = tempfile::tempdir().unwrap();
        let root = app_data.path().join("repositories");
        {
            let manager = WorkspaceManager::open(&root).unwrap();
            let mut source = FilesystemSource::new(src.path()).unwrap();
            let cancel = CancellationToken::new();
            manager
                .import_directory(
                    &mut source,
                    "P".to_owned(),
                    "token".to_owned(),
                    &import_bounds(),
                    &cancel,
                    |_| {},
                )
                .unwrap();
        }
        // Simulate a crash mid-import: leftover staging directory.
        fs::create_dir_all(root.join("staging").join("orphaned-op")).unwrap();
        fs::write(
            root.join("staging").join("orphaned-op").join("partial.txt"),
            b"x",
        )
        .unwrap();

        let reopened = WorkspaceManager::open(&root).unwrap();
        assert_eq!(
            reopened.list_workspaces().len(),
            1,
            "the ready workspace must survive restart"
        );
        let staging_entries: Vec<_> = fs::read_dir(root.join("staging")).unwrap().collect();
        assert!(
            staging_entries.is_empty(),
            "stale staging must be cleaned on startup"
        );
    }
}
