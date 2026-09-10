use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use repopact_analysis::{AnalysisQuery, AnalysisReport};
use repopact_core::RepoPactCore;
use repopact_mutation::{
    CreateWorkItem, EditWorkItem, MutationRequest, MutationResult, WorkItemEdits,
};
use repopact_protocol::{
    capabilities, EngineRequest, EngineResponse, ProtocolDiagnostic, PROTOCOL, PROTOCOL_VERSION,
};
use repopact_types::{Diagnostic, Severity, WorkItem};
use serde::Deserialize;
use serde_json::{json, Value};

const ENGINE_VERSION: &str = env!("REPOPACT_ENGINE_VERSION");

#[derive(Debug, Deserialize)]
struct CreateParams {
    title: String,
    date: String,
    #[serde(default = "default_active")]
    status: String,
}

#[derive(Debug, Deserialize)]
struct AmendProposalParams {
    id: String,
    title: String,
    date: String,
}

fn default_active() -> String {
    "active".to_owned()
}

fn main() -> ExitCode {
    let mut input = String::new();
    if let Err(error) = io::stdin().read_to_string(&mut input) {
        return emit(EngineResponse::failure(
            "",
            ENGINE_VERSION,
            "protocol.stdin",
            format!("unable to read request: {error}"),
        ));
    }
    let request = match serde_json::from_str::<EngineRequest>(&input) {
        Ok(request) => request,
        Err(error) => {
            return emit(EngineResponse::failure(
                "",
                ENGINE_VERSION,
                "protocol.malformed-request",
                format!("request must be one JSON object: {error}"),
            ));
        }
    };
    emit(handle(request))
}

fn emit(response: EngineResponse) -> ExitCode {
    match serde_json::to_string(&response) {
        Ok(encoded) => {
            println!("{encoded}");
            if response.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("unable to encode engine response: {error}");
            ExitCode::from(1)
        }
    }
}

fn handle(request: EngineRequest) -> EngineResponse {
    if request.protocol != PROTOCOL {
        return EngineResponse::failure(
            request.request_id,
            ENGINE_VERSION,
            "protocol.name-mismatch",
            format!(
                "unsupported protocol '{}'; expected '{PROTOCOL}'",
                request.protocol
            ),
        );
    }
    if request.protocol_version != PROTOCOL_VERSION {
        return EngineResponse::failure(
            request.request_id,
            ENGINE_VERSION,
            "protocol.version-mismatch",
            format!(
                "unsupported protocol major {}; expected {}",
                request.protocol_version, PROTOCOL_VERSION
            ),
        );
    }
    match request.operation.as_str() {
        "handshake" | "capabilities" => handshake(&request),
        "validate" => validate(&request),
        "dashboard.write" => dashboard_write(&request),
        "work.create" | "work.propose" => create_work(&request),
        "work.amend_proposal" => amend_proposal(&request),
        "graph" => graph(&request),
        "analyze" => analyze(&request),
        operation => EngineResponse::failure(
            request.request_id.clone(),
            ENGINE_VERSION,
            "operation.unsupported",
            format!("unsupported engine operation '{operation}'"),
        ),
    }
}

fn handshake(request: &EngineRequest) -> EngineResponse {
    let mut response = EngineResponse::success(
        request.request_id.clone(),
        ENGINE_VERSION,
        json!({"status": "ready"}),
    );
    response.capabilities = Some(capabilities());
    response
}

fn require_root(request: &EngineRequest) -> Result<PathBuf, EngineResponse> {
    let Some(root) = request.root.as_deref() else {
        return Err(EngineResponse::failure(
            request.request_id.clone(),
            ENGINE_VERSION,
            "repository.root-required",
            "this engine operation requires a repository root",
        ));
    };
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err(EngineResponse::failure(
            request.request_id.clone(),
            ENGINE_VERSION,
            "repository.invalid-root",
            format!("repository root is not a directory: {}", root.display()),
        ));
    }
    Ok(root)
}

fn validate(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let report = RepoPactCore::open(root).validate();
    validation_response(request, report)
}

fn validation_response(
    request: &EngineRequest,
    report: repopact_types::ValidationReport,
) -> EngineResponse {
    let diagnostics = report
        .diagnostics
        .iter()
        .map(protocol_diagnostic)
        .collect::<Vec<_>>();
    let error_count = report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    let warning_count = report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .count();
    let mut response = EngineResponse::success(
        request.request_id.clone(),
        ENGINE_VERSION,
        json!({
            "valid": error_count == 0,
            "error_count": error_count,
            "warning_count": warning_count,
            "diagnostics": diagnostics,
        }),
    );
    response.diagnostics = diagnostics;
    response
}

fn dashboard_write(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let core = RepoPactCore::open(&root);
    let snapshot = core.snapshot();
    let content = match repopact_validation::render_dashboard_snapshot(&snapshot) {
        Ok(content) => content,
        Err(error) => return semantic_failure(request, "dashboard.render", error),
    };
    let path = root.join("audits/reports/dashboard.md");
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            return EngineResponse::failure(
                request.request_id.clone(),
                ENGINE_VERSION,
                "dashboard.write-failed",
                format!("unable to create dashboard directory: {error}"),
            );
        }
    }
    if let Err(error) = fs::write(&path, content) {
        return EngineResponse::failure(
            request.request_id.clone(),
            ENGINE_VERSION,
            "dashboard.write-failed",
            format!("unable to write dashboard: {error}"),
        );
    }
    EngineResponse::success(
        request.request_id.clone(),
        ENGINE_VERSION,
        json!({"path": "audits/reports/dashboard.md"}),
    )
}

fn create_work(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params = match serde_json::from_value::<CreateParams>(request.params.clone()) {
        Ok(params) => params,
        Err(error) => {
            return semantic_failure(request, "work.invalid-parameters", error.to_string())
        }
    };
    let status = if request.operation == "work.propose" {
        "proposed".to_owned()
    } else {
        params.status
    };
    let core = RepoPactCore::open(&root);
    let snapshot = core.snapshot();
    let intent = CreateWorkItem::new(params.title, params.date).with_status(status);
    let plan = core.plan_mutation_snapshot(&snapshot, MutationRequest::create_work_item(intent));
    let result = plan.apply();
    mutation_response(request, result)
}

fn amend_proposal(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params = match serde_json::from_value::<AmendProposalParams>(request.params.clone()) {
        Ok(params) => params,
        Err(error) => {
            return semantic_failure(request, "work.invalid-parameters", error.to_string())
        }
    };
    let core = RepoPactCore::open(&root);
    let snapshot = core.snapshot();
    let Some(record) = snapshot.index().work_item(&params.id) else {
        return semantic_failure(
            request,
            "mutation.work-not-found",
            format!(
                "unknown work item '{}'; only proposed work may be amended",
                params.id
            ),
        );
    };
    let is_proposed = record
        .value
        .as_ref()
        .ok()
        .and_then(|value| serde_json::from_value::<WorkItem>(value.clone()).ok())
        .is_some_and(|item| item.status == "proposed");
    if !is_proposed {
        return semantic_failure(
            request,
            "mutation.proposal-only",
            "Only proposed work may be amended".to_owned(),
        );
    }
    let intent = EditWorkItem::new(params.id, WorkItemEdits::title(params.title), params.date);
    let plan = core.plan_mutation_snapshot(&snapshot, MutationRequest::edit_work_item(intent));
    let result = plan.apply();
    mutation_response(request, result)
}

fn graph(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let core = RepoPactCore::open(root);
    let snapshot = core.snapshot();
    EngineResponse::success(
        request.request_id.clone(),
        ENGINE_VERSION,
        serde_json::to_value(core.graph_snapshot(&snapshot)).unwrap_or(Value::Null),
    )
}

fn analyze(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let query = match serde_json::from_value::<AnalysisQuery>(request.params.clone()) {
        Ok(query) => query,
        Err(error) => {
            return semantic_failure(request, "analysis.invalid-parameters", error.to_string())
        }
    };
    let core = RepoPactCore::open(root);
    let snapshot = core.snapshot();
    let report: AnalysisReport = core.analyze_snapshot(&snapshot, &query);
    EngineResponse::success(
        request.request_id.clone(),
        ENGINE_VERSION,
        serde_json::to_value(report).unwrap_or(Value::Null),
    )
}

fn mutation_response(request: &EngineRequest, result: MutationResult) -> EngineResponse {
    let diagnostics = result
        .diagnostics
        .iter()
        .map(mutation_diagnostic)
        .collect::<Vec<_>>();
    let value = json!({
        "success": result.success,
        "rolled_back": result.rolled_back,
        "plan_token": result.plan_token,
        "changed_paths": result.changed_paths,
        "diagnostics": result.diagnostics,
    });
    let mut response = EngineResponse::success(request.request_id.clone(), ENGINE_VERSION, value);
    response.diagnostics = diagnostics;
    response
}

fn semantic_failure(
    request: &EngineRequest,
    code: impl Into<String>,
    message: impl Into<String>,
) -> EngineResponse {
    let diagnostic = ProtocolDiagnostic {
        code: code.into(),
        severity: "error".to_owned(),
        message: message.into(),
        path: None,
        record: None,
        field: None,
        related_records: Vec::new(),
        suggested_actions: Vec::new(),
    };
    let mut response = EngineResponse::success(
        request.request_id.clone(),
        ENGINE_VERSION,
        json!({"success": false, "diagnostics": [diagnostic.clone()]}),
    );
    response.diagnostics.push(diagnostic);
    response
}

fn protocol_diagnostic(value: &Diagnostic) -> ProtocolDiagnostic {
    ProtocolDiagnostic {
        code: value.code.clone(),
        severity: match value.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
        .to_owned(),
        message: value.message.clone(),
        path: value.path.clone(),
        record: value.record.clone(),
        field: value.field.clone(),
        related_records: value.related_records.clone().unwrap_or_default(),
        suggested_actions: value.suggested_actions.clone().unwrap_or_default(),
    }
}

fn mutation_diagnostic(value: &repopact_mutation::MutationDiagnostic) -> ProtocolDiagnostic {
    ProtocolDiagnostic {
        code: value.code.clone(),
        severity: "error".to_owned(),
        message: value.message.clone(),
        path: value.path.clone(),
        record: None,
        field: None,
        related_records: value.related_records.clone(),
        suggested_actions: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(0);

    fn request(operation: &str) -> EngineRequest {
        EngineRequest {
            protocol: PROTOCOL.to_owned(),
            protocol_version: PROTOCOL_VERSION,
            request_id: format!("test-{}", NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed)),
            operation: operation.to_owned(),
            root: None,
            params: Value::Object(Default::default()),
        }
    }

    #[test]
    fn handshake_is_structured_and_echoes_request_identity() {
        let request = request("handshake");
        let request_id = request.request_id.clone();
        let response = handle(request);
        assert!(response.ok);
        assert_eq!(response.request_id, request_id);
        assert_eq!(response.engine_version, ENGINE_VERSION);
        assert!(response
            .capabilities
            .unwrap()
            .operations
            .contains(&"validate".to_owned()));
    }

    #[test]
    fn protocol_major_mismatch_is_transport_failure() {
        let mut request = request("handshake");
        request.protocol_version += 1;
        let response = handle(request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "protocol.version-mismatch");
    }

    #[test]
    fn unsupported_operation_is_explicit() {
        let response = handle(request("filesystem.write"));
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "operation.unsupported");
    }

    #[test]
    fn malformed_request_is_rejected_before_semantic_dispatch() {
        let error = serde_json::from_str::<EngineRequest>("not-json").unwrap_err();
        assert!(error.is_syntax());
    }

    #[test]
    fn invalid_repository_is_a_semantic_result() {
        let root = std::env::temp_dir().join(format!(
            "repopact-engine-test-{}-{}",
            std::process::id(),
            NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let mut request = request("validate");
        request.root = Some(root.to_string_lossy().into_owned());
        let response = handle(request);
        assert!(response.ok);
        assert_eq!(response.result.unwrap()["valid"], false);
        fs::remove_dir_all(root).unwrap();
    }
}
