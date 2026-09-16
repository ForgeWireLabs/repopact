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
use crate::bounds::{ArchiveBounds, ExportBounds, ImportBounds};
use crate::divergence::{compare_fingerprints, SourceStatus};
use crate::error::{AcquisitionError, AcquisitionResult, ErrorCode};
use crate::export::{export_tree, scan_source_fingerprint, ExportSummary};
use crate::import::{import_directory, ImportSummary};
use crate::operation::{CancellationToken, OperationCoordinator, OperationProgress};
use crate::registry::{
    clean_stale_registry_temp_files, AcquisitionKind, ExportState, GitState, LifecycleState,
    RemoteSnapshotProvenance, SourceFingerprint, WorkspaceRecord, WorkspaceRegistry,
};
use crate::sink::{ExportSink, FilesystemSink};
use crate::source::{AcquisitionSource, FilesystemSource};

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

    /// The manager's own root directory (`<app_data_dir>/repositories`).
    /// Exposed so a caller (the app's mobile-acquisition coordinator) can
    /// allocate its own scratch space alongside `staging/`/`workspaces/`
    /// for concerns this crate does not itself need to know about (e.g. a
    /// transient local ZIP file en route to a real SAF upload).
    pub fn root(&self) -> &Path {
        &self.root
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
            None,
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
            None,
        )
    }

    /// WI067 Checkpoint C: imports a provider-supplied bounded snapshot
    /// archive (already downloaded to local app-private staging by the
    /// caller -- this method never performs network I/O) into a brand-new
    /// app-private workspace, through the *same* staging-then-publish
    /// transaction and the *same* bounded archive extractor as
    /// [`Self::import_archive`]. The only difference is the acquisition
    /// kind (`RemoteSnapshot`, never `SafArchive` or `RemoteGit` -- see
    /// `registry::AcquisitionKind`'s own doc comments for why those two are
    /// wrong here) and the attached, credential-free provenance record.
    ///
    /// If `strip_single_root_directory` is set (Decision 0061: GitHub's
    /// zipball archives always wrap their contents in one synthetic
    /// `owner-repo-shortsha/` directory), the extracted tree is verified to
    /// contain exactly one top-level directory entry and its contents are
    /// promoted up one level via ordinary same-filesystem renames -- a
    /// purely local filesystem step that runs strictly *after* the
    /// extractor's own zip-slip/symlink/bounds validation already accepted
    /// every path, so it cannot itself escape the workspace root or bypass
    /// any archive-security check.
    #[allow(clippy::too_many_arguments)]
    pub fn import_remote_snapshot<R: Read + Seek>(
        &self,
        reader: R,
        display_name: String,
        source_reference: String,
        bounds: &ArchiveBounds,
        cancel: &CancellationToken,
        on_progress: impl FnMut(OperationProgress),
        provenance: RemoteSnapshotProvenance,
        strip_single_root_directory: bool,
    ) -> AcquisitionResult<WorkspaceRecord> {
        let (operation_id, workspace_id, staging_dir) = self.begin_import()?;
        let result =
            import_archive(reader, &staging_dir, bounds, cancel, on_progress).and_then(|summary| {
                if strip_single_root_directory {
                    normalize_single_root_directory(&staging_dir)?;
                }
                Ok(summary)
            });
        self.finish_import(
            operation_id,
            workspace_id,
            staging_dir,
            result.map(ImportOutcome::from_archive),
            display_name,
            source_reference,
            AcquisitionKind::RemoteSnapshot,
            Some(provenance),
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
        remote_snapshot_provenance: Option<RemoteSnapshotProvenance>,
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
                    remote_snapshot_provenance,
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
    /// workspace (Decision 0056/0057) against an ordinary filesystem
    /// destination -- used by desktop's own export path and by this crate's
    /// host-side tests. `destination_root` must already be an empty (or
    /// non-existent, then created) ordinary directory; a non-empty
    /// destination is a typed `ExportConflict` (Decision 0057 §"Export
    /// semantics": v1 requires a new/empty destination by default, never a
    /// silent merge).
    pub fn export_directory(
        &self,
        workspace_id: &str,
        destination_root: &Path,
        cancel: &CancellationToken,
        require_empty_destination: bool,
    ) -> AcquisitionResult<ExportSummary> {
        if require_empty_destination && destination_root.is_dir() {
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
        fs::create_dir_all(destination_root).map_err(io_err)?;
        let mut sink = FilesystemSink::new(destination_root);
        self.export_directory_via_sink(
            workspace_id,
            &mut sink,
            &ExportBounds::default(),
            cancel,
            |_| {},
        )
    }

    /// The platform-neutral entry point every export destination (a real
    /// Android SAF export root, or [`FilesystemSink`] above) goes through.
    /// Rust owns the entire traversal/bounds/cancellation/progress
    /// discipline (Decision 0057 §12/§AC-6/§AC-7); the sink implementation
    /// owns only how a directory/file actually gets created at its
    /// destination. The sink's destination root (an already-created,
    /// collision-checked location) is the caller's responsibility to
    /// establish before calling this -- see
    /// `mobile_acquisition::mobile_export_workspace_directory` for the real
    /// Android flow (pick parent -> create one new export root -> call this).
    pub fn export_directory_via_sink(
        &self,
        workspace_id: &str,
        sink: &mut dyn ExportSink,
        bounds: &ExportBounds,
        cancel: &CancellationToken,
        on_progress: impl FnMut(OperationProgress),
    ) -> AcquisitionResult<ExportSummary> {
        let repository_dir = self.repository_path(workspace_id)?;
        let mut source = FilesystemSource::new(&repository_dir)?;
        let summary = export_tree(&mut source, sink, bounds, cancel, on_progress)?;
        self.mark_exported(workspace_id)?;
        Ok(summary)
    }

    /// Builds a complete archive snapshot of the workspace's current
    /// repository content without marking the workspace exported (Decision
    /// 0057 §18: on Android, the workspace must not be marked exported
    /// until the completed archive has actually been copied to its real SAF
    /// destination -- a step this crate cannot perform itself). Desktop's
    /// own [`Self::export_archive`] wraps this with an immediate
    /// [`Self::mark_exported`], since there `dest_writer` already *is* the
    /// final destination.
    pub fn build_archive_snapshot<W: Write + Seek>(
        &self,
        workspace_id: &str,
        dest_writer: W,
        bounds: &ExportBounds,
        cancel: &CancellationToken,
        on_progress: impl FnMut(OperationProgress),
    ) -> AcquisitionResult<u64> {
        let repository_dir = self.repository_path(workspace_id)?;
        create_archive(&repository_dir, dest_writer, bounds, cancel, on_progress)
    }

    /// Explicit, user-invoked export for either workspace kind: always
    /// creates a *new* archive rather than mutating an original in place
    /// (Decision 0057 §"Export semantics").
    pub fn export_archive<W: Write + Seek>(
        &self,
        workspace_id: &str,
        dest_writer: W,
        bounds: &ExportBounds,
        cancel: &CancellationToken,
        on_progress: impl FnMut(OperationProgress),
    ) -> AcquisitionResult<u64> {
        let bytes =
            self.build_archive_snapshot(workspace_id, dest_writer, bounds, cancel, on_progress)?;
        self.mark_exported(workspace_id)?;
        Ok(bytes)
    }

    /// Marks a workspace exported after its content has genuinely reached
    /// an external destination. Public so a caller that had to split
    /// "build the snapshot" from "the destination write actually succeeded"
    /// across an external boundary (WI065 Checkpoint D's Android archive
    /// export: build locally, then upload through the SAF bridge) can
    /// record success only once that upload is confirmed.
    pub fn mark_workspace_exported(&self, workspace_id: &str) -> AcquisitionResult<()> {
        self.mark_exported(workspace_id)
    }

    fn mark_exported(&self, workspace_id: &str) -> AcquisitionResult<()> {
        let mut record = self.get_workspace(workspace_id)?;
        record.last_export_state = ExportState::Exported;
        self.registry.upsert(record)?;
        // WI065 Checkpoint D §27: record a local, product-owned digest of
        // the just-exported app-private repository content so a later
        // mutation can be honestly detected as `changed_since_export`
        // without ever inspecting the external destination (export is a
        // one-way copy-out; this crate never reads back what it wrote).
        if let Ok(repository_dir) = self.repository_path(workspace_id) {
            if let Ok(mut scan_source) = FilesystemSource::new(&repository_dir) {
                let cancel = CancellationToken::new();
                if let Ok(fingerprint) = scan_source_fingerprint(&mut scan_source, &cancel) {
                    let _ = self.write_export_fingerprint(workspace_id, &fingerprint);
                }
            }
        }
        Ok(())
    }

    fn export_fingerprint_path(&self, workspace_id: &str) -> PathBuf {
        self.local_metadata_dir(workspace_id)
            .join("export-fingerprint.json")
    }

    fn write_export_fingerprint(
        &self,
        workspace_id: &str,
        fingerprint: &SourceFingerprint,
    ) -> AcquisitionResult<()> {
        let path = self.export_fingerprint_path(workspace_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_err)?;
        }
        let serialized = serde_json::to_vec_pretty(fingerprint)
            .map_err(|error| AcquisitionError::new(ErrorCode::InternalIo, error.to_string()))?;
        fs::write(&path, serialized).map_err(io_err)
    }

    fn read_export_fingerprint(&self, workspace_id: &str) -> Option<SourceFingerprint> {
        let path = self.export_fingerprint_path(workspace_id);
        let raw = fs::read(&path).ok()?;
        serde_json::from_slice(&raw).ok()
    }

    /// WI065 Checkpoint D §27: re-derives whether a previously-exported
    /// workspace has been mutated locally since that export, using only the
    /// same bounded entry-count/aggregate-bytes comparison Decision 0057
    /// already accepts for source fingerprints -- never a claim of
    /// cryptographic proof, and never conflated with the *external source*
    /// divergence check ([`Self::check_source_status`]). Updates and
    /// returns the registry's `last_export_state` when it can honestly do
    /// so; leaves `NeverExported`/`DivergenceUnknown`/`Diverged` alone (this
    /// only ever transitions between `Exported` and `ChangedSinceExport`).
    pub fn refresh_export_freshness(&self, workspace_id: &str) -> AcquisitionResult<ExportState> {
        let record = self.get_workspace(workspace_id)?;
        if !matches!(
            record.last_export_state,
            ExportState::Exported | ExportState::ChangedSinceExport
        ) {
            return Ok(record.last_export_state);
        }
        let Some(exported_fingerprint) = self.read_export_fingerprint(workspace_id) else {
            return Ok(record.last_export_state);
        };
        let repository_dir = self.repository_path(workspace_id)?;
        let mut source = FilesystemSource::new(&repository_dir)?;
        let cancel = CancellationToken::new();
        let current_fingerprint = scan_source_fingerprint(&mut source, &cancel)?;
        let status = compare_fingerprints(&exported_fingerprint, &current_fingerprint);
        let next_state = match status {
            SourceStatus::Unchanged => ExportState::Exported,
            _ => ExportState::ChangedSinceExport,
        };
        if next_state != record.last_export_state {
            let mut updated = record;
            updated.last_export_state = next_state;
            self.registry.upsert(updated)?;
        }
        Ok(next_state)
    }

    /// WI065 Checkpoint D §8/§9: an honest, bounded check of whether the
    /// *external* SAF source a directory workspace was imported from has
    /// obviously changed, using a fresh, read-only, non-writing re-listing
    /// (`source` walks the same tree `AndroidSafSource`/`FilesystemSource`
    /// walked at import time) compared against the persisted
    /// `source_fingerprint`. Never mutates the workspace or its registry
    /// entry -- this is a query, not a write-back decision.
    pub fn check_source_status(
        &self,
        workspace_id: &str,
        source: &mut dyn AcquisitionSource,
        cancel: &CancellationToken,
    ) -> SourceStatus {
        let Ok(record) = self.get_workspace(workspace_id) else {
            return SourceStatus::Unknown;
        };
        let Some(persisted) = record.source_fingerprint else {
            return SourceStatus::Unknown;
        };
        match scan_source_fingerprint(source, cancel) {
            Ok(current) => compare_fingerprints(&persisted, &current),
            Err(error) => crate::divergence::status_from_scan_error(error.code),
        }
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

/// WI067 Checkpoint C, Phase 7/9: normalizes a `SnapshotLayout::
/// SingleRootDirectory`-shaped extracted tree (e.g. GitHub's zipball
/// archives, which always wrap their contents in one synthetic
/// `owner-repo-shortsha/` directory) by promoting that one directory's
/// contents up to `staging_dir` and removing the now-empty wrapper.
///
/// Fails closed rather than guessing: exactly one top-level entry, and it
/// must be a directory, or this returns `ArchiveInvalid` and the caller's
/// existing staging-cleanup path removes the whole tree -- never a partial
/// or silently-wrong promotion. Runs strictly after `import_archive`'s own
/// zip-slip/symlink/bounds validation already accepted every path in the
/// tree, so every rename here moves an already-validated path within the
/// same staging root; it cannot itself escape the workspace or bypass any
/// archive-security check.
fn normalize_single_root_directory(staging_dir: &Path) -> AcquisitionResult<()> {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(staging_dir)
        .map_err(io_err)?
        .collect::<Result<_, _>>()
        .map_err(io_err)?;
    if entries.len() != 1 {
        return Err(AcquisitionError::new(
            ErrorCode::ArchiveInvalid,
            format!(
                "expected exactly one top-level entry for a single-root-directory snapshot, found {}",
                entries.len()
            ),
        ));
    }
    let wrapper = entries.remove(0);
    let wrapper_path = wrapper.path();
    let wrapper_type = wrapper.file_type().map_err(io_err)?;
    if !wrapper_type.is_dir() {
        return Err(AcquisitionError::new(
            ErrorCode::ArchiveInvalid,
            "expected the single top-level entry of a single-root-directory snapshot to be a directory",
        ));
    }
    for child in fs::read_dir(&wrapper_path).map_err(io_err)? {
        let child = child.map_err(io_err)?;
        let destination = staging_dir.join(child.file_name());
        fs::rename(child.path(), destination).map_err(io_err)?;
    }
    fs::remove_dir(&wrapper_path).map_err(io_err)?;
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

    fn sample_provenance() -> RemoteSnapshotProvenance {
        RemoteSnapshotProvenance {
            provider: "github".to_owned(),
            provider_repository_id: "1".to_owned(),
            owner_label: "octocat".to_owned(),
            repository_name: "Hello-World".to_owned(),
            selected_ref: "master".to_owned(),
            ref_kind: "branch".to_owned(),
            resolved_commit_sha: "a".repeat(40),
            acquired_at: "2026-09-16T00:00:00Z".to_owned(),
            snapshot_semantics: "immutable_snapshot".to_owned(),
        }
    }

    // WI067 Checkpoint C, Phase 14: proof that the remote-snapshot import
    // route genuinely passes through WI065's *existing*, unmodified
    // archive-security machinery -- not a second, weaker copy of it. Does
    // not re-derive every adversarial case archive.rs's own test suite
    // already covers (zip-slip, symlink escape, absolute paths, etc.); it
    // proves the pass-through itself, via two cases exercised through
    // `import_remote_snapshot` specifically.

    #[test]
    fn remote_snapshot_import_rejects_a_case_colliding_archive_and_leaves_no_ready_workspace() {
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
            .import_remote_snapshot(
                Cursor::new(zip_bytes),
                "octocat/Hello-World".to_owned(),
                "github:octocat/Hello-World@aaaa".to_owned(),
                &archive_bounds(),
                &cancel,
                |_| {},
                sample_provenance(),
                false,
            )
            .unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::CaseConflict);
        assert!(manager.list_workspaces().is_empty());
        assert!(fs::read_dir(root.join("staging")).unwrap().next().is_none());
        assert!(fs::read_dir(root.join("workspaces"))
            .unwrap()
            .next()
            .is_none());
    }

    #[test]
    fn remote_snapshot_import_rejects_an_archive_exceeding_the_entry_bound() {
        let mut zip_buffer = Cursor::new(Vec::new());
        {
            use zip::write::SimpleFileOptions;
            let mut writer = zip::ZipWriter::new(&mut zip_buffer);
            for index in 0..10 {
                writer
                    .start_file(format!("file-{index}.txt"), SimpleFileOptions::default())
                    .unwrap();
                std::io::Write::write_all(&mut writer, b"x").unwrap();
            }
            writer.finish().unwrap();
        }
        let zip_bytes = zip_buffer.into_inner();

        let app_data = tempfile::tempdir().unwrap();
        let root = app_data.path().join("repositories");
        let manager = WorkspaceManager::open(&root).unwrap();
        let mut bounds = archive_bounds();
        bounds.max_entries = 5; // fewer than the 10 entries above
        let cancel = CancellationToken::new();
        let err = manager
            .import_remote_snapshot(
                Cursor::new(zip_bytes),
                "octocat/Hello-World".to_owned(),
                "github:octocat/Hello-World@aaaa".to_owned(),
                &bounds,
                &cancel,
                |_| {},
                sample_provenance(),
                false,
            )
            .unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::ResourceLimit);
        assert!(manager.list_workspaces().is_empty());
        assert!(fs::read_dir(root.join("staging")).unwrap().next().is_none());
    }

    #[test]
    fn remote_snapshot_import_rejects_a_malformed_archive() {
        let app_data = tempfile::tempdir().unwrap();
        let root = app_data.path().join("repositories");
        let manager = WorkspaceManager::open(&root).unwrap();
        let cancel = CancellationToken::new();
        let err = manager
            .import_remote_snapshot(
                Cursor::new(b"this is not a zip file".to_vec()),
                "octocat/Hello-World".to_owned(),
                "github:octocat/Hello-World@aaaa".to_owned(),
                &archive_bounds(),
                &cancel,
                |_| {},
                sample_provenance(),
                false,
            )
            .unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::ArchiveInvalid);
        assert!(manager.list_workspaces().is_empty());
    }

    #[test]
    fn remote_snapshot_import_publishes_with_credential_free_provenance_and_snapshot_kind() {
        let mut zip_buffer = Cursor::new(Vec::new());
        {
            use zip::write::SimpleFileOptions;
            let mut writer = zip::ZipWriter::new(&mut zip_buffer);
            writer
                .start_file("README.md", SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"hi").unwrap();
            writer.finish().unwrap();
        }
        let zip_bytes = zip_buffer.into_inner();

        let app_data = tempfile::tempdir().unwrap();
        let manager = WorkspaceManager::open(app_data.path().join("repositories")).unwrap();
        let cancel = CancellationToken::new();
        let record = manager
            .import_remote_snapshot(
                Cursor::new(zip_bytes),
                "octocat/Hello-World".to_owned(),
                "github:octocat/Hello-World@aaaa".to_owned(),
                &archive_bounds(),
                &cancel,
                |_| {},
                sample_provenance(),
                false,
            )
            .unwrap();
        assert_eq!(record.acquisition_kind, AcquisitionKind::RemoteSnapshot);
        let provenance = record.remote_snapshot_provenance.unwrap();
        assert_eq!(provenance.resolved_commit_sha, "a".repeat(40));
        assert_eq!(provenance.snapshot_semantics, "immutable_snapshot");
        let published = manager.repository_path(&record.workspace_id).unwrap();
        assert!(published.join("README.md").exists());
    }

    #[test]
    fn remote_snapshot_import_strips_a_single_root_wrapper_directory_when_requested() {
        let mut zip_buffer = Cursor::new(Vec::new());
        {
            use zip::write::SimpleFileOptions;
            let mut writer = zip::ZipWriter::new(&mut zip_buffer);
            // GitHub's real zipball shape: everything nested one level
            // under a single synthetic `owner-repo-sha/` directory.
            writer
                .start_file(
                    "octocat-Hello-World-aaaaaaa/README.md",
                    SimpleFileOptions::default(),
                )
                .unwrap();
            std::io::Write::write_all(&mut writer, b"hi").unwrap();
            writer
                .start_file(
                    "octocat-Hello-World-aaaaaaa/src/main.rs",
                    SimpleFileOptions::default(),
                )
                .unwrap();
            std::io::Write::write_all(&mut writer, b"fn main() {}").unwrap();
            writer.finish().unwrap();
        }
        let zip_bytes = zip_buffer.into_inner();

        let app_data = tempfile::tempdir().unwrap();
        let manager = WorkspaceManager::open(app_data.path().join("repositories")).unwrap();
        let cancel = CancellationToken::new();
        let record = manager
            .import_remote_snapshot(
                Cursor::new(zip_bytes),
                "octocat/Hello-World".to_owned(),
                "github:octocat/Hello-World@aaaa".to_owned(),
                &archive_bounds(),
                &cancel,
                |_| {},
                sample_provenance(),
                true,
            )
            .unwrap();
        let published = manager.repository_path(&record.workspace_id).unwrap();
        // The wrapper directory itself must not survive.
        assert!(!published.join("octocat-Hello-World-aaaaaaa").exists());
        assert!(published.join("README.md").exists());
        assert!(published.join("src/main.rs").exists());
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
            .export_archive(
                &record.workspace_id,
                &mut export_buffer,
                &ExportBounds::default(),
                &cancel,
                |_| {},
            )
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

    // WI065 Checkpoint D: proves the Checkpoint-C mutation pattern end to
    // end through export -- import, mutate the app-private file directly
    // (standing in for a real plan/apply cycle, already proven separately
    // in Checkpoint C), export, and confirm the exported copy carries the
    // mutation while the original import source is untouched.
    #[test]
    fn export_carries_a_local_mutation_while_source_stays_untouched() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("a.txt"), b"original").unwrap();
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

        let repo_path = manager.repository_path(&record.workspace_id).unwrap();
        fs::write(repo_path.join("a.txt"), b"mutated").unwrap();

        let destination = tempfile::tempdir().unwrap();
        let summary = manager
            .export_directory(&record.workspace_id, destination.path(), &cancel, true)
            .unwrap();
        assert_eq!(summary.files_exported, 1);
        assert_eq!(
            fs::read_to_string(destination.path().join("a.txt")).unwrap(),
            "mutated"
        );
        assert_eq!(
            fs::read_to_string(src.path().join("a.txt")).unwrap(),
            "original",
            "the original import source must never be silently written back to"
        );
    }

    #[test]
    fn export_directory_via_sink_matches_the_filesystem_path() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("a.txt"), b"hello").unwrap();
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
        let mut sink = FilesystemSink::new(destination.path());
        let summary = manager
            .export_directory_via_sink(
                &record.workspace_id,
                &mut sink,
                &ExportBounds::default(),
                &cancel,
                |_| {},
            )
            .unwrap();
        assert_eq!(summary.files_exported, 1);
        assert_eq!(
            fs::read_to_string(destination.path().join("a.txt")).unwrap(),
            "hello"
        );
    }

    /// A sink that fails partway through, used to prove the exporter
    /// surfaces a real error (never a false success) when a destination
    /// write fails mid-export -- a synthetic stand-in for a real
    /// DocumentsProvider failure (WI065 Checkpoint D §33/§34).
    struct FailingAfterNSink {
        inner: FilesystemSink,
        remaining_successes: usize,
    }

    impl ExportSink for FailingAfterNSink {
        fn create_directory(&mut self, relative_path: &str) -> AcquisitionResult<()> {
            self.inner.create_directory(relative_path)
        }

        fn create_file(
            &mut self,
            relative_path: &str,
        ) -> AcquisitionResult<Box<dyn crate::sink::ExportFileWriter + '_>> {
            if self.remaining_successes == 0 {
                return Err(AcquisitionError::new(
                    ErrorCode::InternalIo,
                    "synthetic provider failure",
                ));
            }
            self.remaining_successes -= 1;
            self.inner.create_file(relative_path)
        }
    }

    #[test]
    fn export_via_sink_surfaces_a_mid_export_failure_rather_than_a_false_success() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("a.txt"), b"1").unwrap();
        fs::write(src.path().join("b.txt"), b"2").unwrap();
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
        let mut sink = FailingAfterNSink {
            inner: FilesystemSink::new(destination.path()),
            remaining_successes: 0,
        };
        let err = manager
            .export_directory_via_sink(
                &record.workspace_id,
                &mut sink,
                &ExportBounds::default(),
                &cancel,
                |_| {},
            )
            .unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InternalIo);
        // The workspace must not be marked exported on a failed export.
        let after = manager.get_workspace(&record.workspace_id).unwrap();
        assert_eq!(after.last_export_state, ExportState::NeverExported);
    }

    #[test]
    fn refresh_export_freshness_detects_a_local_mutation_after_export() {
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
        manager
            .export_directory(&record.workspace_id, destination.path(), &cancel, true)
            .unwrap();
        assert_eq!(
            manager
                .refresh_export_freshness(&record.workspace_id)
                .unwrap(),
            ExportState::Exported
        );

        // Local mutation after export (standing in for a real Checkpoint-C
        // mutation apply): add a new file to the app-private repository.
        let repo_path = manager.repository_path(&record.workspace_id).unwrap();
        fs::write(repo_path.join("new-file.txt"), b"added after export").unwrap();
        assert_eq!(
            manager
                .refresh_export_freshness(&record.workspace_id)
                .unwrap(),
            ExportState::ChangedSinceExport
        );
        let updated = manager.get_workspace(&record.workspace_id).unwrap();
        assert_eq!(updated.last_export_state, ExportState::ChangedSinceExport);
    }

    #[test]
    fn check_source_status_reports_unchanged_for_an_untouched_source() {
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

        let mut rescan = FilesystemSource::new(src.path()).unwrap();
        let status = manager.check_source_status(&record.workspace_id, &mut rescan, &cancel);
        assert_eq!(status, crate::divergence::SourceStatus::Unchanged);
    }

    #[test]
    fn check_source_status_detects_an_obviously_changed_source() {
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

        // The external source changes after import (a new file added
        // outside RepoPact's knowledge).
        fs::write(src.path().join("b.txt"), b"new").unwrap();
        let mut rescan = FilesystemSource::new(src.path()).unwrap();
        let status = manager.check_source_status(&record.workspace_id, &mut rescan, &cancel);
        assert_eq!(status, crate::divergence::SourceStatus::ObviouslyChanged);
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
