//! WI065 Checkpoint B: the production mobile acquisition coordinator.
//!
//! On a non-Android build, nothing in `run()`'s desktop branch references
//! anything here (only the Android branch's `invoke_handler` does), which
//! is by design (see below) but makes every item in this module look
//! unused to a plain desktop `cargo check`/`cargo test`; `#[cfg(test)]`
//! exercises it directly instead (WI065 Checkpoint B §35), and the Android
//! build's own `invoke_handler` list is every item's real caller.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]
//!
//! Owns the app-lifecycle-scoped `WorkspaceManager` (Decision 0057, rooted
//! at `app_data_dir()/repositories` -- never WI060's debug validation
//! path) and the operation-tracking state the typed `mobile_*` commands
//! below expose to the frontend. `MobileAcquisitionCoordinator` and every
//! command except `mobile_import_directory`/`mobile_import_archive` depend
//! only on the platform-neutral `repopact-mobile-acquisition` crate and
//! compile/host-test on every platform (WI065 Checkpoint B §35); only the
//! two import commands are `#[cfg(target_os = "android")]`-gated, since
//! only they call the real Android SAF bridge. Regardless, only the
//! Android build of `run()` actually registers any `mobile_*` command in
//! its `invoke_handler` -- desktop's `select_repository` and its native-
//! picker/`PathBuf` path are completely untouched by anything here
//! (Decision 0056/0057; WI065 Checkpoint B §24/§31).
//!
//! Every command here is a typed operation (Decision 0056 §"Export
//! authority"/§19 of the WI061 drafting brief): no arbitrary path, no raw
//! URI, no generic filesystem or shell capability is ever accepted from or
//! returned to the frontend. A `content://` URI never leaves this module
//! (it lives only inside `repopact-mobile-saf`'s `PickedTree`/
//! `PickedDocument`/`AndroidSafSource`, all native-owned) -- `DesktopService`
//! only ever sees the app-private `PathBuf`
//! `WorkspaceManager::repository_path` resolves from a workspace *id*.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
#[cfg(target_os = "android")]
use std::thread;

use repopact_desktop_api::{DesktopError, DesktopService, RepositoryOverview};
#[cfg(target_os = "android")]
use repopact_mobile_acquisition::bounds::{ArchiveBounds, ImportBounds};
use repopact_mobile_acquisition::operation::{CancellationToken, OperationProgress};
use repopact_mobile_acquisition::{
    AcquisitionError, AcquisitionResult, ErrorCode, WorkspaceManager, WorkspaceRecord,
};
#[cfg(target_os = "android")]
use repopact_mobile_saf::SafAcquisitionExt;
use serde::Serialize;
#[cfg(target_os = "android")]
use tauri::AppHandle;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub workspace_id: String,
    pub display_name: String,
    pub acquisition_kind: repopact_mobile_acquisition::AcquisitionKind,
    pub git_state: repopact_mobile_acquisition::GitState,
    pub lifecycle_state: repopact_mobile_acquisition::LifecycleState,
    pub last_export_state: repopact_mobile_acquisition::ExportState,
    pub created_at: String,
}

impl From<WorkspaceRecord> for WorkspaceSummary {
    fn from(record: WorkspaceRecord) -> Self {
        Self {
            workspace_id: record.workspace_id,
            display_name: record.display_name,
            acquisition_kind: record.acquisition_kind,
            git_state: record.git_state,
            lifecycle_state: record.lifecycle_state,
            last_export_state: record.last_export_state,
            created_at: record.created_at,
        }
    }
}

/// The Tauri-facing typed error model every `mobile_*` command returns on
/// failure -- a deterministic mapping from Checkpoint A's
/// `AcquisitionError` (itself already the result of this crate's own
/// Android-picker/provider-failure mapping in `repopact-mobile-saf`;
/// WI065 Checkpoint B §20). The frontend switches on `code`, never parses
/// `message` prose.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileAcquisitionError {
    pub code: ErrorCode,
    pub message: String,
}

impl From<AcquisitionError> for MobileAcquisitionError {
    fn from(error: AcquisitionError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }
}

impl MobileAcquisitionError {
    fn not_found(operation_id: &str) -> Self {
        Self {
            code: ErrorCode::WorkspaceNotFound,
            message: format!("no mobile acquisition operation with id '{operation_id}'"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AcquisitionOperation {
    Running { progress: OperationProgress },
    Succeeded { workspace: WorkspaceSummary },
    Failed { error: MobileAcquisitionError },
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage2Status {
    UnsupportedStage2,
}

/// WI065 Checkpoint B §18: Stage 2's future clone/pull/push are represented
/// honestly as unavailable rather than faked as no-op successes.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileCapabilityStatus {
    pub clone_repository: Stage2Status,
    pub pull_repository: Stage2Status,
    pub push_repository: Stage2Status,
}

struct OperationEntry {
    cancel: CancellationToken,
    state: AcquisitionOperation,
}

/// Managed once via `app.manage(Arc::new(MobileAcquisitionCoordinator::open(...)))`
/// in `run()`'s `setup` closure (WI065 Checkpoint B §15) -- never
/// reconstructed per command invocation, so its `WorkspaceManager` (and the
/// crash/restart recovery `WorkspaceManager::open` already performs;
/// Checkpoint A) runs exactly once per app process lifetime.
pub struct MobileAcquisitionCoordinator {
    manager: WorkspaceManager,
    operations: Mutex<HashMap<String, OperationEntry>>,
}

impl MobileAcquisitionCoordinator {
    pub fn open(root: PathBuf) -> AcquisitionResult<Self> {
        Ok(Self {
            manager: WorkspaceManager::open(root)?,
            operations: Mutex::new(HashMap::new()),
        })
    }

    pub fn manager(&self) -> &WorkspaceManager {
        &self.manager
    }

    pub fn list_workspaces(&self) -> Vec<WorkspaceSummary> {
        self.manager
            .list_workspaces()
            .into_iter()
            .map(Into::into)
            .collect()
    }

    fn begin_operation(&self) -> (String, CancellationToken) {
        let operation_id = uuid::Uuid::new_v4().to_string();
        let cancel = CancellationToken::new();
        let initial_progress = OperationProgress {
            operation_id: operation_id.clone(),
            phase: repopact_mobile_acquisition::operation::OperationPhase::Importing,
            entries_processed: 0,
            bytes_processed: 0,
            total_entries: None,
            current_relative_path: None,
        };
        self.operations
            .lock()
            .expect("operations map poisoned")
            .insert(
                operation_id.clone(),
                OperationEntry {
                    cancel: cancel.clone(),
                    state: AcquisitionOperation::Running {
                        progress: initial_progress,
                    },
                },
            );
        (operation_id, cancel)
    }

    fn update_progress(&self, operation_id: &str, progress: OperationProgress) {
        if let Some(entry) = self
            .operations
            .lock()
            .expect("operations map poisoned")
            .get_mut(operation_id)
        {
            entry.state = AcquisitionOperation::Running { progress };
        }
    }

    fn finish(&self, operation_id: &str, result: AcquisitionResult<WorkspaceRecord>) {
        let state = match result {
            Ok(record) => AcquisitionOperation::Succeeded {
                workspace: record.into(),
            },
            Err(error) if error.code == ErrorCode::OperationCancelled => {
                AcquisitionOperation::Cancelled
            }
            Err(error) => AcquisitionOperation::Failed {
                error: error.into(),
            },
        };
        if let Some(entry) = self
            .operations
            .lock()
            .expect("operations map poisoned")
            .get_mut(operation_id)
        {
            entry.state = state;
        }
    }

    pub fn status(
        &self,
        operation_id: &str,
    ) -> Result<AcquisitionOperation, MobileAcquisitionError> {
        self.operations
            .lock()
            .expect("operations map poisoned")
            .get(operation_id)
            .map(|entry| entry.state.clone())
            .ok_or_else(|| MobileAcquisitionError::not_found(operation_id))
    }

    pub fn cancel(&self, operation_id: &str) -> Result<(), MobileAcquisitionError> {
        let guard = self.operations.lock().expect("operations map poisoned");
        match guard.get(operation_id) {
            Some(entry) => {
                entry.cancel.cancel();
                Ok(())
            }
            None => Err(MobileAcquisitionError::not_found(operation_id)),
        }
    }
}

#[tauri::command]
pub fn mobile_workspace_list(
    coordinator: State<'_, Arc<MobileAcquisitionCoordinator>>,
) -> Vec<WorkspaceSummary> {
    coordinator.list_workspaces()
}

#[tauri::command]
pub fn mobile_git_capabilities() -> MobileCapabilityStatus {
    MobileCapabilityStatus {
        clone_repository: Stage2Status::UnsupportedStage2,
        pull_repository: Stage2Status::UnsupportedStage2,
        push_repository: Stage2Status::UnsupportedStage2,
    }
}

#[tauri::command]
pub fn mobile_operation_status(
    operation_id: String,
    coordinator: State<'_, Arc<MobileAcquisitionCoordinator>>,
) -> Result<AcquisitionOperation, MobileAcquisitionError> {
    coordinator.status(&operation_id)
}

#[tauri::command]
pub fn mobile_operation_cancel(
    operation_id: String,
    coordinator: State<'_, Arc<MobileAcquisitionCoordinator>>,
) -> Result<(), MobileAcquisitionError> {
    coordinator.cancel(&operation_id)
}

#[tauri::command]
pub fn mobile_workspace_open(
    workspace_id: String,
    coordinator: State<'_, Arc<MobileAcquisitionCoordinator>>,
    service: State<'_, DesktopService>,
) -> Result<RepositoryOverview, DesktopError> {
    // Never accepts a caller-provided filesystem path (WI065 Checkpoint B
    // §25): the only input is an opaque workspace id, resolved through the
    // native registry into an app-private PathBuf DesktopService already
    // knows how to open, unchanged.
    let path = coordinator
        .manager()
        .repository_path(&workspace_id)
        .map_err(|error| DesktopError {
            code: format!("{:?}", error.code),
            message: error.message,
        })?;
    service.open_repository(path)
}

/// Picks a SAF directory tree (blocking on the user's picker interaction,
/// exactly like desktop's `select_repository` already blocks on its native
/// folder dialog) and, if one was selected, starts a bounded import on a
/// background thread and returns immediately with an operation id the
/// frontend polls via `mobile_operation_status`. `Ok(None)` means the user
/// cancelled the picker -- not an error (WI065 Checkpoint B §4).
#[cfg(target_os = "android")]
#[tauri::command]
pub fn mobile_import_directory(
    app: AppHandle,
    coordinator: State<'_, Arc<MobileAcquisitionCoordinator>>,
) -> Result<Option<String>, MobileAcquisitionError> {
    let picked = app
        .saf_acquisition()
        .pick_directory_tree()
        .map_err(MobileAcquisitionError::from)?;
    let Some(picked) = picked else {
        return Ok(None);
    };

    let (operation_id, cancel) = coordinator.begin_operation();
    let coordinator = coordinator.inner().clone();
    let thread_operation_id = operation_id.clone();
    thread::spawn(move || {
        let mut source = match app
            .saf_acquisition()
            .open_directory_source(picked.tree_uri.clone())
        {
            Ok(source) => source,
            Err(error) => {
                coordinator.finish(&thread_operation_id, Err(error));
                return;
            }
        };
        let progress_operation_id = thread_operation_id.clone();
        let progress_coordinator = coordinator.clone();
        let result = coordinator.manager().import_directory(
            &mut source,
            picked.display_name.clone(),
            picked.tree_uri.clone(),
            &ImportBounds::default(),
            &cancel,
            move |progress| progress_coordinator.update_progress(&progress_operation_id, progress),
        );
        coordinator.finish(&thread_operation_id, result);
    });

    Ok(Some(operation_id))
}
/// Same shape as [`mobile_import_directory`], for a picked SAF archive
/// document. The picked document is copied into app-private staging once
/// (via `open_archive_document`) and handed to Checkpoint A's existing ZIP
/// importer unchanged (WI065 Checkpoint B §12/§28) -- no ZIP parsing exists
/// in this module or in the Kotlin plugin.
#[cfg(target_os = "android")]
#[tauri::command]
pub fn mobile_import_archive(
    app: AppHandle,
    coordinator: State<'_, Arc<MobileAcquisitionCoordinator>>,
) -> Result<Option<String>, MobileAcquisitionError> {
    let picked = app
        .saf_acquisition()
        .pick_archive_document()
        .map_err(MobileAcquisitionError::from)?;
    let Some(picked) = picked else {
        return Ok(None);
    };

    let (operation_id, cancel) = coordinator.begin_operation();
    let coordinator = coordinator.inner().clone();
    let thread_operation_id = operation_id.clone();
    thread::spawn(move || {
        let staged = match app
            .saf_acquisition()
            .open_archive_document(&picked.document_uri)
        {
            Ok(staged) => staged,
            Err(error) => {
                coordinator.finish(&thread_operation_id, Err(error));
                return;
            }
        };
        let progress_operation_id = thread_operation_id.clone();
        let progress_coordinator = coordinator.clone();
        let result = coordinator.manager().import_archive(
            staged,
            picked.display_name.clone(),
            picked.document_uri.clone(),
            &ArchiveBounds::default(),
            &cancel,
            move |progress| progress_coordinator.update_progress(&progress_operation_id, progress),
        );
        coordinator.finish(&thread_operation_id, result);
    });

    Ok(Some(operation_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use repopact_mobile_acquisition::operation::OperationPhase;
    use std::fs;

    fn open_coordinator() -> (tempfile::TempDir, MobileAcquisitionCoordinator) {
        let dir = tempfile::tempdir().unwrap();
        let coordinator =
            MobileAcquisitionCoordinator::open(dir.path().join("repositories")).unwrap();
        (dir, coordinator)
    }

    // WI065 Checkpoint B §35: registry startup initialization -- opening the
    // coordinator from a fresh app-data root creates the Decision 0057
    // layout without requiring any prior state.
    #[test]
    fn coordinator_open_initializes_the_production_layout() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repositories");
        let coordinator = MobileAcquisitionCoordinator::open(root.clone()).unwrap();
        assert!(root.join("workspaces").is_dir());
        assert!(root.join("staging").is_dir());
        assert!(coordinator.list_workspaces().is_empty());
    }

    // §35: stale staging recovery through the app-owned manager -- proves
    // the coordinator wrapper doesn't bypass Checkpoint A's own recovery
    // (already unit-tested in `repopact-mobile-acquisition` directly; this
    // re-proves it through the exact wrapper `run()`'s `setup` closure
    // actually constructs).
    #[test]
    fn coordinator_open_recovers_stale_staging() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repositories");
        fs::create_dir_all(root.join("staging").join("orphaned-op")).unwrap();
        fs::write(
            root.join("staging").join("orphaned-op").join("partial.txt"),
            b"x",
        )
        .unwrap();

        let _coordinator = MobileAcquisitionCoordinator::open(root.clone()).unwrap();
        let staging_entries: Vec<_> = fs::read_dir(root.join("staging")).unwrap().collect();
        assert!(
            staging_entries.is_empty(),
            "stale staging must be cleaned on coordinator startup"
        );
    }

    // §35: a corrupt registry must surface visibly, never silently reset
    // (Decision 0057), even when reached through the app-owned coordinator.
    #[test]
    fn coordinator_open_fails_loudly_on_corrupt_registry() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repositories");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("registry.json"), b"{ not valid json").unwrap();
        let result = MobileAcquisitionCoordinator::open(root);
        let Err(err) = result else {
            panic!("expected MobileAcquisitionCoordinator::open to fail on a corrupt registry");
        };
        assert_eq!(err.code, repopact_mobile_acquisition::ErrorCode::InternalIo);
    }

    // §35: operation lifecycle -- begin -> Running -> progress update ->
    // terminal state (Succeeded/Failed/Cancelled), matching what
    // `mobile_operation_status` actually returns to the frontend.
    #[test]
    fn operation_lifecycle_running_then_succeeded() {
        let (_dir, coordinator) = open_coordinator();
        let (operation_id, _cancel) = coordinator.begin_operation();

        match coordinator.status(&operation_id).unwrap() {
            AcquisitionOperation::Running { progress } => {
                assert_eq!(progress.operation_id, operation_id);
                assert_eq!(progress.entries_processed, 0);
            }
            other => panic!("expected Running, got {other:?}"),
        }

        coordinator.update_progress(
            &operation_id,
            OperationProgress {
                operation_id: operation_id.clone(),
                phase: OperationPhase::Importing,
                entries_processed: 7,
                bytes_processed: 128,
                total_entries: Some(10),
                current_relative_path: Some("a/b.txt".to_owned()),
            },
        );
        match coordinator.status(&operation_id).unwrap() {
            AcquisitionOperation::Running { progress } => assert_eq!(progress.entries_processed, 7),
            other => panic!("expected Running, got {other:?}"),
        }

        let record = repopact_mobile_acquisition::WorkspaceRecord {
            workspace_id: "ws-1".to_owned(),
            display_name: "Example".to_owned(),
            acquisition_kind: repopact_mobile_acquisition::AcquisitionKind::SafDirectory,
            source_reference: "ref".to_owned(),
            git_state: repopact_mobile_acquisition::GitState::NonGit,
            lifecycle_state: repopact_mobile_acquisition::LifecycleState::Ready,
            created_at: "2026-09-15T00:00:00Z".to_owned(),
            imported_at: Some("2026-09-15T00:00:01Z".to_owned()),
            last_export_state: repopact_mobile_acquisition::ExportState::NeverExported,
            source_fingerprint: None,
        };
        coordinator.finish(&operation_id, Ok(record));
        match coordinator.status(&operation_id).unwrap() {
            AcquisitionOperation::Succeeded { workspace } => {
                assert_eq!(workspace.workspace_id, "ws-1");
            }
            other => panic!("expected Succeeded, got {other:?}"),
        }
    }

    #[test]
    fn operation_lifecycle_cancelled_is_distinct_from_failed() {
        let (_dir, coordinator) = open_coordinator();
        let (operation_id, _cancel) = coordinator.begin_operation();
        coordinator.finish(
            &operation_id,
            Err(repopact_mobile_acquisition::AcquisitionError::new(
                repopact_mobile_acquisition::ErrorCode::OperationCancelled,
                "operation was cancelled",
            )),
        );
        match coordinator.status(&operation_id).unwrap() {
            AcquisitionOperation::Cancelled => {}
            other => panic!("expected Cancelled, got {other:?}"),
        }
    }

    #[test]
    fn operation_lifecycle_other_errors_are_failed() {
        let (_dir, coordinator) = open_coordinator();
        let (operation_id, _cancel) = coordinator.begin_operation();
        coordinator.finish(
            &operation_id,
            Err(repopact_mobile_acquisition::AcquisitionError::new(
                repopact_mobile_acquisition::ErrorCode::ResourceLimit,
                "too many entries",
            )),
        );
        match coordinator.status(&operation_id).unwrap() {
            AcquisitionOperation::Failed { error } => {
                assert_eq!(
                    error.code,
                    repopact_mobile_acquisition::ErrorCode::ResourceLimit
                );
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    // §35: the cancellation bridge -- `mobile_operation_cancel` flips the
    // exact `CancellationToken` a background import thread checks.
    #[test]
    fn cancel_bridges_to_the_stored_cancellation_token() {
        let (_dir, coordinator) = open_coordinator();
        let (operation_id, cancel) = coordinator.begin_operation();
        assert!(cancel.check().is_ok());
        coordinator.cancel(&operation_id).unwrap();
        assert_eq!(
            cancel.check().unwrap_err().code,
            repopact_mobile_acquisition::ErrorCode::OperationCancelled
        );
    }

    #[test]
    fn cancel_unknown_operation_is_a_typed_not_found_error() {
        let (_dir, coordinator) = open_coordinator();
        let err = coordinator.cancel("does-not-exist").unwrap_err();
        assert_eq!(
            err.code,
            repopact_mobile_acquisition::ErrorCode::WorkspaceNotFound
        );
    }

    #[test]
    fn status_unknown_operation_is_a_typed_not_found_error() {
        let (_dir, coordinator) = open_coordinator();
        let err = coordinator.status("does-not-exist").unwrap_err();
        assert_eq!(
            err.code,
            repopact_mobile_acquisition::ErrorCode::WorkspaceNotFound
        );
    }

    // §35: `mobile_workspace_open` accepts only an opaque workspace id --
    // there is no parameter through which a caller-provided filesystem path
    // could reach `DesktopService::open_repository`. An unregistered id is
    // a typed error, never a fallback to some caller-supplied path.
    #[test]
    fn workspace_open_by_unknown_id_is_typed_not_found_never_a_path() {
        let (_dir, coordinator) = open_coordinator();
        let err = coordinator
            .manager()
            .repository_path("unregistered-id")
            .unwrap_err();
        assert_eq!(
            err.code,
            repopact_mobile_acquisition::ErrorCode::WorkspaceNotFound
        );
    }

    // §35: Stage-2 capability truthfulness -- clone/pull/push are reported
    // unavailable, never faked as successful no-ops (Decision 0056/0057).
    #[test]
    fn git_capabilities_report_stage2_unavailable() {
        let status = mobile_git_capabilities();
        assert!(matches!(
            status.clone_repository,
            Stage2Status::UnsupportedStage2
        ));
        assert!(matches!(
            status.pull_repository,
            Stage2Status::UnsupportedStage2
        ));
        assert!(matches!(
            status.push_repository,
            Stage2Status::UnsupportedStage2
        ));
    }

    // §35: typed DTO serialization shapes -- the frontend switches on these
    // fields, so their exact JSON shape is worth pinning down.
    #[test]
    fn mobile_acquisition_error_serializes_with_typed_code() {
        let error =
            MobileAcquisitionError::from(repopact_mobile_acquisition::AcquisitionError::new(
                repopact_mobile_acquisition::ErrorCode::PermissionDenied,
                "denied",
            ));
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["code"], "permission_denied");
        assert_eq!(json["message"], "denied");
    }

    #[test]
    fn acquisition_operation_serializes_with_a_typed_state_tag() {
        let running = AcquisitionOperation::Running {
            progress: OperationProgress {
                operation_id: "op-1".to_owned(),
                phase: OperationPhase::Importing,
                entries_processed: 1,
                bytes_processed: 2,
                total_entries: None,
                current_relative_path: None,
            },
        };
        let json = serde_json::to_value(&running).unwrap();
        assert_eq!(json["state"], "running");

        let cancelled = AcquisitionOperation::Cancelled;
        let json = serde_json::to_value(&cancelled).unwrap();
        assert_eq!(json["state"], "cancelled");
    }

    #[test]
    fn workspace_summary_serializes_camel_case_typed_fields() {
        let record = repopact_mobile_acquisition::WorkspaceRecord {
            workspace_id: "ws-1".to_owned(),
            display_name: "Example".to_owned(),
            acquisition_kind: repopact_mobile_acquisition::AcquisitionKind::SafArchive,
            source_reference: "ref".to_owned(),
            git_state: repopact_mobile_acquisition::GitState::GitMetadataPresent,
            lifecycle_state: repopact_mobile_acquisition::LifecycleState::Ready,
            created_at: "2026-09-15T00:00:00Z".to_owned(),
            imported_at: None,
            last_export_state: repopact_mobile_acquisition::ExportState::Exported,
            source_fingerprint: None,
        };
        let summary: WorkspaceSummary = record.into();
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["workspaceId"], "ws-1");
        assert_eq!(json["acquisitionKind"], "saf_archive");
        assert_eq!(json["gitState"], "git_metadata_present");
        assert_eq!(json["lifecycleState"], "ready");
        assert_eq!(json["lastExportState"], "exported");
    }

    #[test]
    fn mobile_capability_status_serializes_camel_case() {
        let json = serde_json::to_value(mobile_git_capabilities()).unwrap();
        assert_eq!(json["cloneRepository"], "unsupported_stage2");
        assert_eq!(json["pullRepository"], "unsupported_stage2");
        assert_eq!(json["pushRepository"], "unsupported_stage2");
    }
}
