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

// Not `#[cfg(target_os = "android")]`-gated as a whole: `MobileAcquisitionCoordinator`,
// its DTOs, and the id-only `mobile_workspace_open`/`mobile_operation_*`/
// `mobile_git_capabilities` commands only depend on the platform-neutral
// `repopact-mobile-acquisition` crate, so they compile and are host-tested
// on every platform (WI065 Checkpoint B §35). Only `mobile_import_directory`/
// `mobile_import_archive` (which call the real Android SAF bridge) are
// individually gated inside the module, and only the Android build of this
// app actually registers any of these commands (see `run()` below).
mod mobile_acquisition;

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
            poll_repository_events
        ]);
    #[cfg(target_os = "android")]
    let builder = tauri::Builder::default()
        .plugin(repopact_mobile_saf::init_plugin())
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
            mobile_acquisition::mobile_git_capabilities
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
