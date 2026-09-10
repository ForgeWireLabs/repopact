#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::thread;
use std::time::Duration;

use repopact_analysis::AnalysisQuery;
use repopact_desktop_api::{
    AnalysisView, DesktopError, DesktopService, GraphView, MutationApplyView, MutationIntent,
    MutationPlanView, RecordDetailView, RecordSummaryView, RepositoryChangedEvent,
    RepositoryOverview, ValidationView, WorkItemDetailView, WorkItemSummaryView,
};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeRequest {
    #[serde(default)]
    candidate_id: Option<String>,
}

#[tauri::command]
fn select_repository(
    app: AppHandle,
    service: State<'_, DesktopService>,
) -> Result<Option<RepositoryOverview>, DesktopError> {
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
) -> Result<Vec<RecordSummaryView>, DesktopError> {
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
) -> Result<Vec<RecordSummaryView>, DesktopError> {
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

fn main() {
    let service = DesktopService::new();
    tauri::Builder::default()
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
            analyze_work_item,
            plan_mutation,
            apply_mutation_plan,
            discard_mutation_plan,
            poll_repository_events
        ])
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running RepoPact Workbench");
}
