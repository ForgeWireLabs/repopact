//! Shared Tauri application body (WI060): both the desktop binary
//! (`src/main.rs`) and the Android/iOS mobile runtime load this crate and
//! call [`run`]. Command registration lives here exactly once so desktop and
//! mobile never diverge on which operations are exposed.

#[cfg(all(
    target_os = "android",
    debug_assertions,
    feature = "android-debug-validation"
))]
mod android_validation;
#[cfg(not(target_os = "android"))]
mod github_app_registration;

// Not `#[cfg(target_os = "android")]`-gated as a whole: `MobileAcquisitionCoordinator`,
// its DTOs, and the id-only `mobile_workspace_open`/`mobile_operation_*`/
// `mobile_git_capabilities` commands only depend on the platform-neutral
// `repopact-mobile-acquisition` crate, so they compile and are host-tested
// on every platform (WI065 Checkpoint B §35). Only `mobile_import_directory`/
// `mobile_import_archive` (which call the real Android SAF bridge) are
// individually gated inside the module, and only the Android build of this
// app actually registers any of these commands (see `run()` below).
mod mobile_acquisition;

// WI067 Checkpoint B: desktop-only for now (item 8/9 record Android
// credential storage as a distinct, tracked gate rather than forcing a
// substantial new mobile plugin into this checkpoint).
#[cfg(not(target_os = "android"))]
mod remote_provider;

use std::thread;
use std::time::Duration;

use repopact_analysis::AnalysisQuery;
use repopact_desktop_api::{
    AnalysisView, DecisionSummaryView, DesktopError, DesktopService, EvidenceSummaryView,
    GraphQueryRequest, GraphStatusView, GraphView, MutationApplyView, MutationIntent,
    MutationPlanView, RecordDetailView, RepositoryChangedEvent, RepositoryOverview, ValidationView,
    WorkItemDetailView, WorkItemSummaryView,
};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};
#[cfg(not(target_os = "android"))]
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeRequest {
    #[serde(default)]
    candidate_id: Option<String>,
}

/// WI060 AND-006: repository selection is platform-gated. Tauri's Android
/// dialog implementation does not support folder selection, and an Android
/// content:// URI (from the Storage Access Framework) is not equivalent to a
/// recursive filesystem repository root that `DesktopService` can open. A
/// normal Android build therefore returns an explicit, typed error rather
/// than pretending selection works, requesting broad storage permissions,
/// hard-coding a path, or faking a path from a content URI. The one
/// exception is the debug-only, feature-gated app-private validation
/// repository (see `android_validation`), which exists solely to exercise
/// the rest of the Workbench on Android and is unreachable from a normal
/// production build.
#[tauri::command]
fn select_repository(
    app: AppHandle,
    service: State<'_, DesktopService>,
) -> Result<Option<RepositoryOverview>, DesktopError> {
    #[cfg(not(target_os = "android"))]
    {
        let selected = app.dialog().file().blocking_pick_folder();
        let Some(selected) = selected else {
            return Ok(None);
        };
        let Some(path) = selected.into_path().ok() else {
            return Err(DesktopError {
                code: "repository.invalid-selection".to_owned(),
                message: "the selected location is not a local directory".to_owned(),
            });
        };
        service.open_repository(path).map(Some)
    }
    #[cfg(target_os = "android")]
    {
        #[cfg(all(debug_assertions, feature = "android-debug-validation"))]
        if let Some(path) = android_validation::debug_validation_repository_path(&app) {
            return service.open_repository(path).map(Some);
        }
        let _ = app;
        Err(DesktopError {
            code: "repository.mobile-selection-unavailable".to_owned(),
            message: "directory acquisition is not implemented on Android; production repository selection is unresolved pending a follow-up work item".to_owned(),
        })
    }
}

#[tauri::command]
fn close_repository(service: State<'_, DesktopService>) -> Result<(), DesktopError> {
    service.close_repository()
}

// WI067 Checkpoint E (GH-004): a bounded, debug-only native test/control
// surface for the Android Keystore-backed credential store -- see
// `android_validation`'s own module doc comment for the full rationale.
// These four commands are always registered on an Android build (so the
// generated handler list stays static across debug/release), but their
// bodies are only real inside a `debug_assertions` build with
// `--features android-debug-validation`; a normal Android build (debug
// or release) without that feature returns a fixed "debug validation is
// not enabled in this build" error and touches no credential storage at
// all. None of these commands ever returns a stored secret's plaintext
// value to the frontend. Android-only at the function level (not merely
// internally branched) -- these do not exist at all in a desktop build.
#[cfg(target_os = "android")]
#[tauri::command]
fn debug_credential_put(
    #[allow(unused_variables)] app: AppHandle,
    #[allow(unused_variables)] connection_id: String,
    #[allow(unused_variables)] kind: String,
    #[allow(unused_variables)] secret: String,
) -> Result<(), repopact_remote_provider::error::RemoteProviderError> {
    #[cfg(all(debug_assertions, feature = "android-debug-validation"))]
    return android_validation::debug_credential_put(&app, &connection_id, &kind, &secret);
    #[cfg(not(all(debug_assertions, feature = "android-debug-validation")))]
    Err(debug_validation_disabled())
}

#[cfg(target_os = "android")]
#[tauri::command]
fn debug_credential_get_matches(
    #[allow(unused_variables)] app: AppHandle,
    #[allow(unused_variables)] connection_id: String,
    #[allow(unused_variables)] kind: String,
    #[allow(unused_variables)] expected: String,
) -> Result<bool, repopact_remote_provider::error::RemoteProviderError> {
    #[cfg(all(debug_assertions, feature = "android-debug-validation"))]
    return android_validation::debug_credential_get_matches(
        &app,
        &connection_id,
        &kind,
        &expected,
    );
    #[cfg(not(all(debug_assertions, feature = "android-debug-validation")))]
    Err(debug_validation_disabled())
}

#[cfg(target_os = "android")]
#[tauri::command]
fn debug_credential_present(
    #[allow(unused_variables)] app: AppHandle,
    #[allow(unused_variables)] connection_id: String,
    #[allow(unused_variables)] kind: String,
) -> Result<bool, repopact_remote_provider::error::RemoteProviderError> {
    #[cfg(all(debug_assertions, feature = "android-debug-validation"))]
    return android_validation::debug_credential_present(&app, &connection_id, &kind);
    #[cfg(not(all(debug_assertions, feature = "android-debug-validation")))]
    Err(debug_validation_disabled())
}

#[cfg(target_os = "android")]
#[tauri::command]
fn debug_credential_delete(
    #[allow(unused_variables)] app: AppHandle,
    #[allow(unused_variables)] connection_id: String,
    #[allow(unused_variables)] kind: String,
) -> Result<(), repopact_remote_provider::error::RemoteProviderError> {
    #[cfg(all(debug_assertions, feature = "android-debug-validation"))]
    return android_validation::debug_credential_delete(&app, &connection_id, &kind);
    #[cfg(not(all(debug_assertions, feature = "android-debug-validation")))]
    Err(debug_validation_disabled())
}

#[cfg(target_os = "android")]
#[tauri::command]
fn debug_credential_key_info(
    #[allow(unused_variables)] app: AppHandle,
) -> Result<(bool, Option<bool>), repopact_remote_provider::error::RemoteProviderError> {
    #[cfg(all(debug_assertions, feature = "android-debug-validation"))]
    return android_validation::debug_credential_key_info(&app);
    #[cfg(not(all(debug_assertions, feature = "android-debug-validation")))]
    Err(debug_validation_disabled())
}

#[cfg(all(
    target_os = "android",
    not(all(debug_assertions, feature = "android-debug-validation"))
))]
fn debug_validation_disabled() -> repopact_remote_provider::error::RemoteProviderError {
    repopact_remote_provider::error::RemoteProviderError::new(
        repopact_remote_provider::error::ErrorCode::CredentialUnavailable,
        "debug credential validation is not enabled in this build",
    )
}

#[tauri::command]
fn repository_overview(
    service: State<'_, DesktopService>,
) -> Result<RepositoryOverview, DesktopError> {
    service.repository_overview()
}

#[tauri::command]
fn refresh_repository(
    service: State<'_, DesktopService>,
) -> Result<RepositoryOverview, DesktopError> {
    service.refresh_repository()
}

#[tauri::command]
fn validate_repository(service: State<'_, DesktopService>) -> Result<ValidationView, DesktopError> {
    service.validate_repository()
}

#[tauri::command]
fn list_work_items(
    query: Option<String>,
    service: State<'_, DesktopService>,
) -> Result<Vec<WorkItemSummaryView>, DesktopError> {
    service.list_work_items(query)
}

#[tauri::command]
fn get_work_item(
    id: String,
    service: State<'_, DesktopService>,
) -> Result<WorkItemDetailView, DesktopError> {
    service.get_work_item(&id)
}

#[tauri::command]
fn list_decisions(
    service: State<'_, DesktopService>,
) -> Result<Vec<DecisionSummaryView>, DesktopError> {
    service.list_decisions()
}

#[tauri::command]
fn get_decision(
    id: String,
    service: State<'_, DesktopService>,
) -> Result<RecordDetailView, DesktopError> {
    service.get_decision(&id)
}

#[tauri::command]
fn list_evidence(
    service: State<'_, DesktopService>,
) -> Result<Vec<EvidenceSummaryView>, DesktopError> {
    service.list_evidence()
}

#[tauri::command]
fn get_evidence(
    id: String,
    service: State<'_, DesktopService>,
) -> Result<RecordDetailView, DesktopError> {
    service.get_evidence(&id)
}

#[tauri::command]
fn relationship_graph(service: State<'_, DesktopService>) -> Result<GraphView, DesktopError> {
    service.graph()
}

/// The one typed Workbench operator-map query boundary (ROG-027, Decision
/// 0052 section 2): a tagged `GraphQueryRequest` in, a structured
/// `QueryEnvelope<...>` JSON value out -- never a presentation string to
/// re-parse in React. Delegates entirely to `DesktopSession::graph_query`,
/// which runs the canonical `GraphQueryEngine` against this session's
/// `SessionGraphState::effective_graph()`, so dirty working-tree state is
/// always reflected and the result always discloses `status.basis`.
#[tauri::command]
fn graph_query(
    request: GraphQueryRequest,
    service: State<'_, DesktopService>,
) -> Result<Value, DesktopError> {
    service.graph_query(request)
}

/// The authorized Workbench Verify control (ROG-027): read-only by
/// construction. Reports the canonical graph verification result; never
/// builds/updates/enables/disables/repairs as a side effect.
#[tauri::command]
fn graph_verify(service: State<'_, DesktopService>) -> Result<GraphStatusView, DesktopError> {
    service.graph_verify()
}

/// Read-only durable graph status, used by the operator map's freshness/
/// coverage/capability disclosure (ROG-027, Decision 0052 section 2).
#[tauri::command]
fn graph_status(service: State<'_, DesktopService>) -> Result<GraphStatusView, DesktopError> {
    service.graph_status()
}

/// The authorized Workbench Build/Rebuild control (ROG-027): a deliberate
/// durable write, reached only through this Rust command -- never by
/// shelling out to the CLI from JavaScript. On success, the session's
/// `RepositoryOverview`/`SessionGraphState` are refreshed so a subsequent
/// query reflects the freshly built graph; on failure, the session is
/// left untouched.
#[tauri::command]
fn graph_build(service: State<'_, DesktopService>) -> Result<RepositoryOverview, DesktopError> {
    service.graph_build()
}

#[tauri::command]
fn analyze_work_item(
    request: Option<AnalyzeRequest>,
    service: State<'_, DesktopService>,
) -> Result<AnalysisView, DesktopError> {
    let query = request
        .and_then(|request| request.candidate_id)
        .map(AnalysisQuery::for_work_item)
        .unwrap_or_default();
    service.analyze(query)
}

#[tauri::command]
fn plan_mutation(
    intent: MutationIntent,
    service: State<'_, DesktopService>,
) -> Result<MutationPlanView, DesktopError> {
    service.plan_mutation(intent)
}

#[tauri::command]
#[allow(non_snake_case)]
fn apply_mutation_plan(
    sessionId: String,
    planHandle: String,
    service: State<'_, DesktopService>,
) -> Result<MutationApplyView, DesktopError> {
    service.apply_mutation_plan(&sessionId, &planHandle)
}

#[tauri::command]
#[allow(non_snake_case)]
fn discard_mutation_plan(
    sessionId: String,
    planHandle: String,
    service: State<'_, DesktopService>,
) -> Result<(), DesktopError> {
    service.discard_mutation_plan(&sessionId, &planHandle)
}

#[tauri::command]
fn poll_repository_events(
    service: State<'_, DesktopService>,
) -> Result<Vec<RepositoryChangedEvent>, DesktopError> {
    service.poll_repository_events()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let service = DesktopService::new();
    // WI065 Checkpoint B (§24): select_repository and its native-picker/
    // PathBuf path stay desktop-only and completely unchanged. Android
    // gets a separate typed command surface (mobile_*) rather than one
    // command contorted to cover incompatible desktop/mobile semantics.
    // Command registration is therefore fully platform-branched here, not
    // merely the plugin list, so the Android build never even references
    // desktop-only types and vice versa.
    #[cfg(not(target_os = "android"))]
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(service.clone())
        .invoke_handler(tauri::generate_handler![
            select_repository,
            close_repository,
            repository_overview,
            refresh_repository,
            validate_repository,
            list_work_items,
            get_work_item,
            list_decisions,
            get_decision,
            list_evidence,
            get_evidence,
            relationship_graph,
            graph_query,
            graph_verify,
            graph_status,
            graph_build,
            analyze_work_item,
            plan_mutation,
            apply_mutation_plan,
            discard_mutation_plan,
            poll_repository_events,
            remote_provider::remote_provider_capabilities,
            remote_provider::remote_connections,
            remote_provider::remote_connect_start,
            remote_provider::remote_connect_status,
            remote_provider::remote_connect_cancel,
            remote_provider::remote_open_installation_page,
            remote_provider::remote_disconnect,
            remote_provider::remote_accounts,
            remote_provider::remote_repositories,
            remote_provider::remote_repository_refs,
            remote_provider::remote_resolve_ref,
            remote_provider::remote_import_snapshot,
            remote_provider::remote_import_cancel
        ]);
    #[cfg(target_os = "android")]
    let builder = tauri::Builder::default()
        .plugin(repopact_mobile_saf::init_plugin())
        // WI067 Checkpoint E (GH-004): the real, production Android
        // Keystore-backed credential plugin, registered unconditionally
        // (not debug-only) so a release build carries the same protected
        // credential backend a debug build does. This plugin exposes no
        // frontend-invokable commands of its own -- see
        // `repopact-mobile-credential`'s `lib.rs` module doc comment --
        // so registering it here grants the frontend no new capability.
        .plugin(repopact_mobile_credential::init_plugin())
        .manage(service.clone())
        .invoke_handler(tauri::generate_handler![
            select_repository,
            close_repository,
            repository_overview,
            refresh_repository,
            validate_repository,
            list_work_items,
            get_work_item,
            list_decisions,
            get_decision,
            list_evidence,
            get_evidence,
            relationship_graph,
            graph_query,
            graph_verify,
            graph_status,
            graph_build,
            analyze_work_item,
            plan_mutation,
            apply_mutation_plan,
            discard_mutation_plan,
            poll_repository_events,
            mobile_acquisition::mobile_workspace_list,
            mobile_acquisition::mobile_import_directory,
            mobile_acquisition::mobile_import_archive,
            mobile_acquisition::mobile_operation_status,
            mobile_acquisition::mobile_operation_cancel,
            mobile_acquisition::mobile_workspace_open,
            mobile_acquisition::mobile_git_capabilities,
            mobile_acquisition::mobile_export_workspace_directory,
            mobile_acquisition::mobile_export_workspace_archive,
            mobile_acquisition::mobile_workspace_source_status,
            mobile_acquisition::mobile_workspace_remove,
            debug_credential_put,
            debug_credential_get_matches,
            debug_credential_present,
            debug_credential_delete,
            debug_credential_key_info
        ]);
    builder
        .setup(|app| {
            let handle = app.handle().clone();
            let service = app.state::<DesktopService>().inner().clone();
            thread::spawn(move || loop {
                thread::sleep(Duration::from_millis(200));
                let Ok(events) = service.poll_repository_events() else {
                    continue;
                };
                for event in events {
                    let _ = handle.emit("repository-changed", event);
                }
            });
            // WI067 Checkpoint B (§15): the production remote-provider
            // service is initialized exactly once here, from the real
            // app-private data directory, and managed for the app's whole
            // lifetime. Restores a prior GitHub connection from the real
            // OS credential store synchronously if one exists; a missing
            // GitHub App client ID (operator has not yet registered one,
            // see docs/guides/github-app-setup.md) is not a startup
            // failure -- only `remote_connect_start` fails typed
            // (`ProviderNotConfigured`) if actually invoked.
            #[cfg(not(target_os = "android"))]
            {
                let root = app
                    .path()
                    .app_data_dir()
                    .map_err(|error| format!("unable to resolve app_data_dir: {error}"))?;
                let service =
                    remote_provider::RemoteProviderService::open(root).map_err(|error| {
                        format!("unable to initialize the remote provider service: {error}")
                    })?;
                app.manage(std::sync::Arc::new(service));
            }
            // WI065 Checkpoint B (§15/§16): the production mobile workspace
            // registry/importer is initialized exactly once here, from the
            // real app-private data directory (Decision 0057 -- never
            // WI060's debug validation root), and managed for the app's
            // whole lifetime. WorkspaceManager::open already performs
            // Checkpoint A's crash/restart recovery (orphaned staging
            // cleanup, stale registry temp-file cleanup) synchronously
            // before returning; a corrupt registry surfaces as a loud
            // startup error here rather than a silently empty one.
            #[cfg(target_os = "android")]
            {
                let root = app
                    .path()
                    .app_data_dir()
                    .map_err(|error| format!("unable to resolve app_data_dir: {error}"))?
                    .join("repositories");
                let coordinator = mobile_acquisition::MobileAcquisitionCoordinator::open(root)
                    .map_err(|error| {
                        format!("unable to initialize the mobile acquisition coordinator: {error}")
                    })?;
                app.manage(std::sync::Arc::new(coordinator));
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running RepoPact Workbench");
}
