use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use repopact_graph::{GraphEdge, RepositoryGraph};
use repopact_repository::{path_string, Repository, RepositorySnapshot};
use repopact_types::{
    hex_digest, AcceptanceCriterion, LifecycleStatus, PathState, ReadFact, ReadSet,
    RepositoryIdentity, WorkItem,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateWorkItem {
    pub title: String,
    pub status: String,
    pub date: String,
    pub owner_scope: String,
    #[serde(default)]
    pub affected_scopes: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default = "default_concrete")]
    pub provenance: String,
    #[serde(default)]
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
}

impl CreateWorkItem {
    pub fn new(title: impl Into<String>, date: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            status: "active".to_owned(),
            date: date.into(),
            owner_scope: "governance".to_owned(),
            affected_scopes: Vec::new(),
            depends_on: Vec::new(),
            provenance: "concrete".to_owned(),
            acceptance_criteria: Vec::new(),
        }
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }
    pub fn with_owner_scope(mut self, owner_scope: impl Into<String>) -> Self {
        self.owner_scope = owner_scope.into();
        self
    }
    pub fn with_affected_scopes(mut self, scopes: Vec<String>) -> Self {
        self.affected_scopes = scopes;
        self
    }
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.depends_on = dependencies;
        self
    }
    pub fn with_acceptance_criteria(mut self, criteria: Vec<AcceptanceCriterion>) -> Self {
        self.acceptance_criteria = criteria;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItemEdits {
    pub title: Option<String>,
    pub owner_scope: Option<String>,
    pub affected_scopes: Option<Vec<String>>,
    pub depends_on: Option<Vec<String>>,
    pub provenance: Option<String>,
    pub acceptance_criteria: Option<Vec<AcceptanceCriterion>>,
}

impl WorkItemEdits {
    pub fn title(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            ..Self::default()
        }
    }
    pub fn owner_scope(owner_scope: impl Into<String>) -> Self {
        Self {
            owner_scope: Some(owner_scope.into()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditWorkItem {
    pub id: String,
    pub changes: WorkItemEdits,
    pub date: String,
}

impl EditWorkItem {
    pub fn new(id: impl Into<String>, changes: WorkItemEdits, date: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            changes,
            date: date.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionWorkItem {
    pub id: String,
    pub status: String,
}

impl TransitionWorkItem {
    pub fn new(id: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: status.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationRequest {
    CreateWorkItem(CreateWorkItem),
    EditWorkItem(EditWorkItem),
    TransitionWorkItem(TransitionWorkItem),
}

impl From<CreateWorkItem> for MutationRequest {
    fn from(request: CreateWorkItem) -> Self {
        Self::CreateWorkItem(request)
    }
}

impl From<EditWorkItem> for MutationRequest {
    fn from(request: EditWorkItem) -> Self {
        Self::EditWorkItem(request)
    }
}

impl From<TransitionWorkItem> for MutationRequest {
    fn from(request: TransitionWorkItem) -> Self {
        Self::TransitionWorkItem(request)
    }
}

impl MutationRequest {
    pub fn create_work_item(request: CreateWorkItem) -> Self {
        Self::CreateWorkItem(request)
    }
    pub fn edit_work_item(request: EditWorkItem) -> Self {
        Self::EditWorkItem(request)
    }
    pub fn transition_work_item(request: TransitionWorkItem) -> Self {
        Self::TransitionWorkItem(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathOperation {
    Write { path: String, content: String },
    Move { from: String, to: String },
}

pub type FileOperation = PathOperation;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedImpact {
    pub path: String,
    pub before_digest: String,
    pub after_digest: String,
    pub preview: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationDiagnostic {
    pub code: String,
    pub message: String,
    pub path: Option<String>,
    pub blocking: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_records: Vec<String>,
}

impl MutationDiagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            path: None,
            blocking: true,
            related_records: Vec::new(),
        }
    }

    pub fn at(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationPlan {
    pub repository_identity: RepositoryIdentity,
    pub read_set: ReadSet,
    pub plan_token: String,
    pub request: MutationRequest,
    pub file_operations: Vec<PathOperation>,
    pub generated_impacts: Vec<GeneratedImpact>,
    pub graph_impacts: Vec<GraphEdge>,
    pub diagnostics: Vec<MutationDiagnostic>,
    pub preview: String,
}

impl MutationPlan {
    pub fn is_applicable(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.blocking)
    }
    pub fn has_errors(&self) -> bool {
        !self.is_applicable()
    }

    pub fn apply(&self) -> MutationResult {
        apply(self, &ApplyOptions::default())
    }

    pub fn apply_with_options(&self, options: &ApplyOptions) -> MutationResult {
        apply(self, options)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyOptions {
    pub fail_before_operation: Option<usize>,
    pub fail_after_operation: Option<usize>,
    pub external_drift_at: Option<usize>,
    pub external_drift_path: Option<String>,
}

impl ApplyOptions {
    pub fn fail_after(operation: usize) -> Self {
        Self {
            fail_after_operation: Some(operation),
            ..Self::default()
        }
    }
    pub fn fail_before(operation: usize) -> Self {
        Self {
            fail_before_operation: Some(operation),
            ..Self::default()
        }
    }
    pub fn external_drift(operation: usize, path: impl Into<String>) -> Self {
        Self {
            external_drift_at: Some(operation),
            external_drift_path: Some(path.into()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationResult {
    pub success: bool,
    pub rolled_back: bool,
    pub plan_token: String,
    pub changed_paths: Vec<String>,
    pub diagnostics: Vec<MutationDiagnostic>,
}

pub fn plan(snapshot: &RepositorySnapshot, request: MutationRequest) -> MutationPlan {
    let repository = snapshot.repository();
    let identity = snapshot.identity();
    let mut diagnostics = Vec::new();
    let mut operations = Vec::new();
    let mut extra_absent = Vec::new();
    let request_for_graph = request.clone();

    if has_wi050_surface(repository.root()) {
        diagnostics.push(MutationDiagnostic::error(
            "mutation.unsupported-wi050",
            "WI050 admission/guard/enforcement semantics are outside the WI054 native mutation boundary",
        ));
        return empty_plan(snapshot, request, diagnostics);
    }

    match &request {
        MutationRequest::CreateWorkItem(request) => {
            let Some(status) = LifecycleStatus::parse(&request.status) else {
                diagnostics.push(MutationDiagnostic::error(
                    "mutation.status-invalid",
                    format!("unknown work-item status '{}'", request.status),
                ));
                return empty_plan(snapshot, request.clone(), diagnostics);
            };
            let id = format!("{:03}", next_work_id(snapshot.index()));
            let slug = slugify(&request.title);
            let directory = format!("work/{}/{id}-{slug}", status.as_str());
            let manifest_path = format!("{directory}/work-item.json");
            let readme_path = format!("{directory}/README.md");
            let (manifest, _) = create_manifest(request, &id, status.as_str(), repository);
            operations.push(PathOperation::Write {
                path: manifest_path.clone(),
                content: manifest,
            });
            operations.push(PathOperation::Write {
                path: readme_path.clone(),
                content: create_readme(repository, &id, &request.title),
            });
            extra_absent.extend([directory, manifest_path, readme_path]);
        }
        MutationRequest::EditWorkItem(request) => {
            let Some(record) = snapshot.index().work_item(&request.id) else {
                diagnostics.push(MutationDiagnostic::error(
                    "mutation.work-not-found",
                    format!("unknown work item '{}'", request.id),
                ));
                return empty_plan(snapshot, request.clone(), diagnostics);
            };
            match edited_manifest(record, &request.changes, &request.date) {
                Ok(content) => operations.push(PathOperation::Write {
                    path: repository.relative_path(&record.path),
                    content,
                }),
                Err(error) => diagnostics.push(
                    MutationDiagnostic::error("mutation.edit-invalid", error)
                        .at(repository.relative_path(&record.path)),
                ),
            }
        }
        MutationRequest::TransitionWorkItem(request) => {
            let Some(status) = LifecycleStatus::parse(&request.status) else {
                diagnostics.push(MutationDiagnostic::error(
                    "mutation.status-invalid",
                    format!("unknown work-item status '{}'", request.status),
                ));
                return empty_plan(snapshot, request.clone(), diagnostics);
            };
            let Some(record) = snapshot.index().work_item(&request.id) else {
                diagnostics.push(MutationDiagnostic::error(
                    "mutation.work-not-found",
                    format!("unknown work item '{}'", request.id),
                ));
                return empty_plan(snapshot, request.clone(), diagnostics);
            };
            let Some(item) = record
                .value
                .clone()
                .ok()
                .and_then(|value| serde_json::from_value::<WorkItem>(value).ok())
            else {
                diagnostics.push(
                    MutationDiagnostic::error(
                        "mutation.work-invalid",
                        "cannot transition a work item whose JSON is not valid".to_owned(),
                    )
                    .at(repository.relative_path(&record.path)),
                );
                return empty_plan(snapshot, request.clone(), diagnostics);
            };
            let Some(source_directory) = record.path.parent() else {
                return empty_plan(snapshot, request.clone(), diagnostics);
            };
            let Some(source_status) = source_directory
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
            else {
                return empty_plan(snapshot, request.clone(), diagnostics);
            };
            let Some(name) = source_directory.file_name().and_then(|name| name.to_str()) else {
                return empty_plan(snapshot, request.clone(), diagnostics);
            };
            let destination_directory = repository
                .root()
                .join("work")
                .join(status.as_str())
                .join(name);
            let source_relative = repository.relative_path(source_directory);
            let destination_relative = repository.relative_path(&destination_directory);
            if source_status == status.as_str() {
                let mut value = record.value.clone().unwrap_or(Value::Null);
                if let Some(object) = value.as_object_mut() {
                    object.insert(
                        "status".to_owned(),
                        Value::String(status.as_str().to_owned()),
                    );
                }
                operations.push(PathOperation::Write {
                    path: repository.relative_path(&record.path),
                    content: canonical_json(&value),
                });
            } else {
                let mut value = record.value.clone().unwrap_or(Value::Null);
                if let Some(object) = value.as_object_mut() {
                    object.insert(
                        "status".to_owned(),
                        Value::String(status.as_str().to_owned()),
                    );
                }
                operations.push(PathOperation::Move {
                    from: source_relative.clone(),
                    to: destination_relative.clone(),
                });
                operations.push(PathOperation::Write {
                    path: format!("{destination_relative}/work-item.json"),
                    content: canonical_json(&value),
                });
                extra_absent.push(destination_relative);
            }
            let _ = item;
        }
    }

    if diagnostics.iter().any(|diagnostic| diagnostic.blocking) {
        return empty_plan(snapshot, request.clone(), diagnostics);
    }

    let dashboard = dashboard_for_candidate(repository.root(), &operations);
    match dashboard {
        Ok(content) => operations.push(PathOperation::Write {
            path: "audits/reports/dashboard.md".to_owned(),
            content: platform_text(&content),
        }),
        Err(error) => diagnostics.push(MutationDiagnostic::error(
            "mutation.dashboard-render",
            error,
        )),
    }
    let mut read_facts = snapshot.read_set().facts.clone();
    for relative in extra_absent {
        let path = repository.root().join(&relative);
        read_facts.push(ReadFact {
            path: relative,
            expected: repository.path_state(&path),
        });
    }
    let read_set = ReadSet::new(read_facts);
    let graph = RepositoryGraph::build(snapshot);
    let mut plan = MutationPlan {
        repository_identity: identity.clone(),
        plan_token: read_set.token(&identity),
        read_set,
        request,
        generated_impacts: impacts(repository, &operations),
        preview: preview(&operations),
        file_operations: operations,
        graph_impacts: graph_impacts(&graph, &request_for_graph),
        diagnostics,
    };
    add_frozen_diagnostics(snapshot, &mut plan);
    plan.diagnostics.sort_by(|left, right| {
        (
            left.code.as_str(),
            left.path.as_deref(),
            left.message.as_str(),
        )
            .cmp(&(
                right.code.as_str(),
                right.path.as_deref(),
                right.message.as_str(),
            ))
    });
    plan
}

pub fn plan_mutation(snapshot: &RepositorySnapshot, request: MutationRequest) -> MutationPlan {
    plan(snapshot, request)
}

fn empty_plan(
    snapshot: &RepositorySnapshot,
    request: impl Into<MutationRequest>,
    diagnostics: Vec<MutationDiagnostic>,
) -> MutationPlan {
    let request = request.into();
    let identity = snapshot.identity();
    let read_set = snapshot.read_set().clone();
    MutationPlan {
        repository_identity: identity.clone(),
        read_set: read_set.clone(),
        plan_token: read_set.token(&identity),
        request,
        file_operations: Vec::new(),
        generated_impacts: Vec::new(),
        graph_impacts: Vec::new(),
        diagnostics,
        preview: String::new(),
    }
}

fn create_manifest(
    request: &CreateWorkItem,
    id: &str,
    status: &str,
    repository: &Repository,
) -> (String, WorkItem) {
    let criteria = if request.acceptance_criteria.is_empty() {
        vec![CanonicalCriterion {
            id: "AC-1".to_owned(),
            text: "TODO: observable outcome".to_owned(),
            state: "pending".to_owned(),
            evidence: Vec::new(),
            provenance: None,
        }]
    } else {
        request
            .acceptance_criteria
            .iter()
            .map(|criterion| CanonicalCriterion::from(criterion))
            .collect()
    };
    let marker = repopact_types::PreflightMarker {
        created_before_work_started: true,
        created_at: format!("{}T00:00:00Z", request.date),
        note: "Stamped by `repopact new`; the work item was recorded before implementation began."
            .to_owned(),
    };
    let manifest = CanonicalWorkItem {
        schema: if repository.root().join("repopact/schemas").is_dir() {
            "../../../repopact/schemas/work-item.schema.json".to_owned()
        } else {
            "../../../schemas/work-item.schema.json".to_owned()
        },
        id: id.to_owned(),
        title: request.title.clone(),
        status: status.to_owned(),
        owner_scope: request.owner_scope.clone(),
        affected_scopes: request.affected_scopes.clone(),
        depends_on: request.depends_on.clone(),
        provenance: request.provenance.clone(),
        preflight: marker,
        acceptance_criteria: criteria,
        created: request.date.clone(),
        updated: request.date.clone(),
    };
    let value = serde_json::to_value(&manifest).unwrap_or(Value::Null);
    let typed = serde_json::from_value(value.clone()).unwrap_or_else(|_| WorkItem {
        id: id.to_owned(),
        title: request.title.clone(),
        status: status.to_owned(),
        owner_scope: request.owner_scope.clone(),
        affected_scopes: request.affected_scopes.clone(),
        depends_on: request.depends_on.clone(),
        provenance: request.provenance.clone(),
        preflight: Some(manifest.preflight.clone()),
        acceptance_criteria: request.acceptance_criteria.clone(),
        created: request.date.clone(),
        updated: request.date.clone(),
    });
    (canonical_json(&value), typed)
}

#[derive(Serialize)]
struct CanonicalWorkItem {
    #[serde(rename = "$schema")]
    schema: String,
    id: String,
    title: String,
    status: String,
    owner_scope: String,
    affected_scopes: Vec<String>,
    depends_on: Vec<String>,
    provenance: String,
    preflight: repopact_types::PreflightMarker,
    acceptance_criteria: Vec<CanonicalCriterion>,
    created: String,
    updated: String,
}

#[derive(Serialize)]
struct CanonicalCriterion {
    id: String,
    text: String,
    state: String,
    evidence: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provenance: Option<String>,
}

impl From<&AcceptanceCriterion> for CanonicalCriterion {
    fn from(criterion: &AcceptanceCriterion) -> Self {
        Self {
            id: criterion.id.clone(),
            text: criterion.text.clone(),
            state: criterion.state.clone(),
            evidence: criterion.evidence.clone(),
            provenance: (criterion.provenance != "concrete").then(|| criterion.provenance.clone()),
        }
    }
}

fn edited_manifest(
    record: &repopact_repository::IndexedRecord,
    changes: &WorkItemEdits,
    date: &str,
) -> Result<String, String> {
    let mut value = record
        .value
        .clone()
        .map_err(|error| format!("cannot edit malformed work-item JSON: {error}"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "work-item JSON must be an object".to_owned())?;
    if let Some(value) = &changes.title {
        object.insert("title".to_owned(), Value::String(value.clone()));
    }
    if let Some(value) = &changes.owner_scope {
        object.insert("owner_scope".to_owned(), Value::String(value.clone()));
    }
    if let Some(value) = &changes.affected_scopes {
        object.insert(
            "affected_scopes".to_owned(),
            serde_json::to_value(value).map_err(|error| error.to_string())?,
        );
    }
    if let Some(value) = &changes.depends_on {
        object.insert(
            "depends_on".to_owned(),
            serde_json::to_value(value).map_err(|error| error.to_string())?,
        );
    }
    if let Some(value) = &changes.provenance {
        object.insert("provenance".to_owned(), Value::String(value.clone()));
    }
    if let Some(value) = &changes.acceptance_criteria {
        object.insert(
            "acceptance_criteria".to_owned(),
            serde_json::to_value(value).map_err(|error| error.to_string())?,
        );
    }
    object.insert("updated".to_owned(), Value::String(date.to_owned()));
    Ok(canonical_json(&value))
}

fn create_readme(repository: &Repository, id: &str, title: &str) -> String {
    let template = [
        repository.root().join("templates/work-item.README.md"),
        repository
            .root()
            .join("repopact/templates/work-item.README.md"),
    ]
    .into_iter()
    .find_map(|path| fs::read_to_string(path).ok())
    .unwrap_or_else(|| {
        include_str!("../../../../repopact/templates/work-item.README.md").to_owned()
    });
    platform_text(
        &template
            .replace("NNN", id)
            .replace("Title Of The Work", title),
    )
}

fn dashboard_for_candidate(root: &Path, operations: &[PathOperation]) -> Result<String, String> {
    let temporary = std::env::temp_dir().join(format!(
        "repopact-mutation-plan-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&temporary).map_err(|error| error.to_string())?;
    let result = (|| {
        copy_tree(root, &temporary)?;
        for operation in operations {
            apply_operation(&temporary, operation)?;
        }
        repopact_validation::render_dashboard(&temporary)
    })();
    let _ = fs::remove_dir_all(&temporary);
    result
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| path_string(&left.path()).cmp(&path_string(&right.path())));
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if matches!(
            name.as_str(),
            ".git"
                | "target"
                | "__pycache__"
                | ".venv"
                | ".pytest_cache"
                | "node_modules"
                | "build"
                | "dist"
                | "worktrees"
                | "fixtures"
        ) {
            continue;
        }
        let from = entry.path();
        let to = destination.join(&name);
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            copy_tree(&from, &to)?;
        } else if kind.is_file() {
            fs::copy(&from, &to).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn apply_operation(root: &Path, operation: &PathOperation) -> Result<(), String> {
    match operation {
        PathOperation::Write { path, content } => {
            let path = root.join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(path, content).map_err(|error| error.to_string())
        }
        PathOperation::Move { from, to } => {
            let from = root.join(from);
            let to = root.join(to);
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::rename(from, to).map_err(|error| error.to_string())
        }
    }
}

fn impacts(repository: &Repository, operations: &[PathOperation]) -> Vec<GeneratedImpact> {
    operations
        .iter()
        .filter_map(|operation| match operation {
            PathOperation::Write { path, content } => {
                let absolute = repository.root().join(path);
                let before = digest_state(&repository.path_state(&absolute));
                let after = hex_digest(Sha256::digest(content.as_bytes()));
                Some(GeneratedImpact {
                    path: path.clone(),
                    before_digest: before,
                    after_digest: after,
                    preview: format!("--- {path}\n+++ {path}\n{content}"),
                    reason: if path == "audits/reports/dashboard.md" {
                        "owned generated dashboard projection".to_owned()
                    } else {
                        "durable work-item record".to_owned()
                    },
                })
            }
            PathOperation::Move { from, to } => Some(GeneratedImpact {
                path: format!("{from} -> {to}"),
                before_digest: digest_state(&repository.path_state(&repository.root().join(from))),
                after_digest: digest_state(&repository.path_state(&repository.root().join(from))),
                preview: format!("rename {from} -> {to}"),
                reason: "complete work-item directory lifecycle transition".to_owned(),
            }),
        })
        .collect()
}

fn preview(operations: &[PathOperation]) -> String {
    operations
        .iter()
        .map(|operation| match operation {
            PathOperation::Write { path, content } => format!("--- {path}\n+++ {path}\n{content}"),
            PathOperation::Move { from, to } => format!("rename {from} -> {to}"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn graph_impacts(graph: &RepositoryGraph, request: &MutationRequest) -> Vec<GraphEdge> {
    let id = match request {
        MutationRequest::CreateWorkItem(_) => None,
        MutationRequest::EditWorkItem(request) => Some(request.id.as_str()),
        MutationRequest::TransitionWorkItem(request) => Some(request.id.as_str()),
    };
    id.map(|id| graph.edges_from(&format!("work:{id}")))
        .unwrap_or_default()
}

fn add_frozen_diagnostics(snapshot: &RepositorySnapshot, plan: &mut MutationPlan) {
    let Some(frozen) = &snapshot.index().frozen_surface else {
        return;
    };
    let Some(value) = frozen.value.as_ref().ok() else {
        return;
    };
    for operation in &plan.file_operations {
        let paths = match operation {
            PathOperation::Write { path, .. } => vec![path.as_str()],
            PathOperation::Move { from, to } => vec![from.as_str(), to.as_str()],
        };
        for path in paths {
            for entry in value
                .get("protected")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let glob = entry.get("glob").and_then(Value::as_str).unwrap_or("");
                if wildcard_match(path, glob) {
                    let reason = entry
                        .get("reason")
                        .and_then(Value::as_str)
                        .unwrap_or("protected frozen-surface path");
                    plan.diagnostics.push(
                        MutationDiagnostic::error(
                            "mutation.frozen-surface",
                            format!("native apply is closed for protected path '{path}': {reason}"),
                        )
                        .at(path),
                    );
                }
            }
        }
    }
}

pub fn apply(plan: &MutationPlan, options: &ApplyOptions) -> MutationResult {
    let mut result = MutationResult {
        success: false,
        rolled_back: false,
        plan_token: plan.plan_token.clone(),
        changed_paths: Vec::new(),
        diagnostics: plan.diagnostics.clone(),
    };
    if !plan.is_applicable() {
        return result;
    }
    let repository = Repository::open(&plan.repository_identity.root);
    if repository.identity() != plan.repository_identity {
        result.diagnostics.push(MutationDiagnostic::error(
            "mutation.repository-identity",
            "repository identity changed since planning",
        ));
        return result;
    }
    if let Some(stale) = stale_read_fact(&repository, &plan.read_set) {
        result.diagnostics.push(
            MutationDiagnostic::error(
                "mutation.plan-stale",
                format!(
                    "plan read fact for '{}' no longer matches; re-plan before apply",
                    stale.path
                ),
            )
            .at(stale.path),
        );
        return result;
    }

    let roots = operation_roots(plan, &repository);
    let preimages = capture_preimages(&repository, &roots);
    let mut expected = roots
        .iter()
        .map(|path| {
            (
                path.clone(),
                repository.path_state(&repository.root().join(path)),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (index, operation) in plan.file_operations.iter().enumerate() {
        if options.external_drift_at == Some(index) {
            if let Some(path) = options.external_drift_path.as_deref() {
                let absolute = repository.root().join(path);
                if let Some(parent) = absolute.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Err(error) = fs::write(&absolute, b"external drift injected during apply") {
                    return failure_with_rollback(
                        &repository,
                        &roots,
                        &preimages,
                        result,
                        format!("unable to inject external drift: {error}"),
                    );
                }
            }
        }
        if options.fail_before_operation == Some(index) {
            return failure_with_rollback(
                &repository,
                &roots,
                &preimages,
                result,
                format!("failure injected before operation {index}"),
            );
        }
        if let Some(stale_path) = operation_drift(operation, &repository, &expected) {
            return failure_with_rollback(&repository, &roots, &preimages, result, format!("target '{stale_path}' changed during apply; refusing to merge external content"));
        }
        if let Err(error) = apply_operation(repository.root(), operation) {
            return failure_with_rollback(
                &repository,
                &roots,
                &preimages,
                result,
                format!("apply failed: {error}"),
            );
        }
        refresh_expected_paths(&repository, &mut expected, operation_paths(operation));
        if options.fail_after_operation == Some(index) {
            return failure_with_rollback(
                &repository,
                &roots,
                &preimages,
                result,
                format!("failure injected after operation {index}"),
            );
        }
    }
    let post = repopact_validation::validate(repository.root());
    if post.has_errors() {
        let messages = post
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        return failure_with_rollback(
            &repository,
            &roots,
            &preimages,
            result,
            format!("post-state validation failed: {messages}"),
        );
    }
    result.success = true;
    result.changed_paths = plan
        .file_operations
        .iter()
        .flat_map(operation_paths)
        .map(|path| path_string(&path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    result
}

fn failure_with_rollback(
    repository: &Repository,
    roots: &[PathBuf],
    preimages: &Preimages,
    mut result: MutationResult,
    message: String,
) -> MutationResult {
    result
        .diagnostics
        .push(MutationDiagnostic::error("mutation.apply-failed", message));
    match restore_preimages(repository, roots, preimages) {
        Ok(()) => result.rolled_back = true,
        Err(error) => result
            .diagnostics
            .push(MutationDiagnostic::error("mutation.rollback-failed", error)),
    }
    result
}

fn stale_read_fact(repository: &Repository, read_set: &ReadSet) -> Option<ReadFact> {
    read_set
        .facts
        .iter()
        .find(|fact| repository.path_state(&repository.root().join(&fact.path)) != fact.expected)
        .cloned()
}

fn operation_roots(plan: &MutationPlan, repository: &Repository) -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    for operation in &plan.file_operations {
        match operation {
            PathOperation::Write { path, .. } => {
                let file = PathBuf::from(path);
                roots.insert(file.clone());
                if let Some(parent) = file
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                {
                    let absolute = repository.root().join(parent);
                    if repository.path_state(&absolute) == PathState::Absent {
                        roots.insert(parent.to_path_buf());
                    }
                }
            }
            PathOperation::Move { from, to } => {
                roots.insert(PathBuf::from(from));
                roots.insert(PathBuf::from(to));
            }
        }
    }
    roots.into_iter().collect()
}

fn operation_paths(operation: &PathOperation) -> Vec<PathBuf> {
    match operation {
        PathOperation::Write { path, .. } => vec![PathBuf::from(path)],
        PathOperation::Move { from, to } => vec![PathBuf::from(from), PathBuf::from(to)],
    }
}

fn operation_drift(
    operation: &PathOperation,
    repository: &Repository,
    expected: &BTreeMap<PathBuf, PathState>,
) -> Option<String> {
    operation_paths(operation)
        .into_iter()
        .find(|path| {
            repository.path_state(&repository.root().join(path))
                != expected.get(path).cloned().unwrap_or(PathState::Absent)
        })
        .map(|path| path_string(&path))
}

fn refresh_expected_paths(
    repository: &Repository,
    expected: &mut BTreeMap<PathBuf, PathState>,
    paths: Vec<PathBuf>,
) {
    for path in paths {
        let absolute = repository.root().join(&path);
        expected.insert(path, repository.path_state(&absolute));
        if absolute.is_dir() {
            for file in repository.files_under(&absolute) {
                expected.insert(
                    PathBuf::from(repository.relative_path(&file)),
                    repository.path_state(&file),
                );
            }
        }
    }
}

#[derive(Debug, Default)]
struct Preimages {
    files: BTreeMap<PathBuf, Vec<u8>>,
    directories: BTreeSet<PathBuf>,
}

fn capture_preimages(repository: &Repository, roots: &[PathBuf]) -> Preimages {
    let mut preimages = Preimages::default();
    for root in roots {
        let absolute = repository.root().join(root);
        if absolute.is_file() {
            if let Ok(bytes) = fs::read(&absolute) {
                preimages.files.insert(root.clone(), bytes);
            }
        } else if absolute.is_dir() {
            collect_preimage_tree(repository.root(), &absolute, &mut preimages);
        }
    }
    preimages
}

fn collect_preimage_tree(root: &Path, current: &Path, preimages: &mut Preimages) {
    let relative = current.strip_prefix(root).unwrap_or(current).to_path_buf();
    preimages.directories.insert(relative);
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            collect_preimage_tree(root, &path, preimages);
        } else if entry
            .file_type()
            .map(|kind| kind.is_file())
            .unwrap_or(false)
        {
            if let Ok(bytes) = fs::read(&path) {
                preimages.files.insert(
                    path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                    bytes,
                );
            }
        }
    }
}

fn restore_preimages(
    repository: &Repository,
    roots: &[PathBuf],
    preimages: &Preimages,
) -> Result<(), String> {
    let mut removal = roots
        .iter()
        .map(|root| repository.root().join(root))
        .collect::<Vec<_>>();
    removal.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    removal.dedup();
    for path in removal {
        remove_path(&path)?;
    }
    let mut directories = preimages
        .directories
        .iter()
        .map(|path| repository.root().join(path))
        .collect::<Vec<_>>();
    directories.sort_by_key(|path| path.components().count());
    for path in directories {
        fs::create_dir_all(path).map_err(|error| error.to_string())?;
    }
    for (relative, bytes) in &preimages.files {
        let path = repository.root().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(path, bytes).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|error| error.to_string())
    } else if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())
    } else {
        Ok(())
    }
}

fn digest_state(state: &PathState) -> String {
    match state {
        PathState::Absent => "absent".to_owned(),
        PathState::Present { digest, .. } => digest.clone(),
    }
}

fn canonical_json(value: &Value) -> String {
    platform_text(&(serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_owned()) + "\n"))
}

fn platform_text(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n");
    if cfg!(windows) {
        normalized.replace('\n', "\r\n")
    } else {
        normalized
    }
}
fn default_concrete() -> String {
    "concrete".to_owned()
}

fn next_work_id(index: &repopact_repository::RecordIndex) -> u64 {
    index
        .work_items
        .iter()
        .filter_map(|record| {
            record
                .reference
                .path
                .rsplit('/')
                .nth(1)
                .and_then(|directory| directory.split('-').next())
                .and_then(|id| id.parse::<u64>().ok())
                .or_else(|| record.reference.id.parse::<u64>().ok())
        })
        .max()
        .unwrap_or(0)
        + 1
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut dash = false;
    for character in title.to_lowercase().chars() {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            slug.push(character);
            dash = false;
        } else if !dash {
            slug.push('-');
            dash = true;
        }
    }
    slug.trim_matches('-').to_owned()
}

fn wildcard_match(value: &str, pattern: &str) -> bool {
    let value = value.as_bytes();
    let pattern = pattern.as_bytes();
    let mut previous = vec![false; value.len() + 1];
    previous[0] = true;
    for character in pattern {
        let mut current = vec![false; value.len() + 1];
        match character {
            b'*' => {
                current[0] = previous[0];
                for index in 1..=value.len() {
                    current[index] = previous[index] || current[index - 1];
                }
            }
            b'?' => {
                for index in 1..=value.len() {
                    current[index] = previous[index - 1];
                }
            }
            literal => {
                for index in 1..=value.len() {
                    current[index] = previous[index - 1] && value[index - 1] == *literal;
                }
            }
        }
        previous = current;
    }
    previous[value.len()]
}

fn has_wi050_surface(root: &Path) -> bool {
    [
        "governance/admission-policy.json",
        "governance/operator-authority.json",
        "governance/repository-registration.json",
    ]
    .into_iter()
    .any(|relative| root.join(relative).is_file())
        || root.join("evidence/admission").is_dir()
        || root.join("governance/adapters").is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use repopact_repository::RepositorySession;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ROOT_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn fixture() -> (PathBuf, RepositorySnapshot) {
        let root = std::env::temp_dir().join(format!(
            "repopact-mutation-test-{}-{}",
            std::process::id(),
            TEST_ROOT_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        let valid =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../conformance/fixtures/valid");
        copy_tree(&valid, &root).unwrap();
        fs::remove_dir_all(root.join("work/completed/001-seed")).unwrap();
        fs::create_dir_all(root.join("work/active/001-one")).unwrap();
        fs::write(root.join("work/active/001-one/work-item.json"), r#"{"id":"001","title":"One","status":"active","owner_scope":"governance","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-01"}"#).unwrap();
        fs::write(
            root.join("work/active/001-one/README.md"),
            "# One\n\nIntent\n",
        )
        .unwrap();
        fs::write(root.join("work/active/001-one/extra.md"), "history\n").unwrap();
        fs::write(root.join("audits/reports/dashboard.md"), "old\n").unwrap();
        let snapshot = RepositorySession::open(&root).snapshot();
        (root, snapshot)
    }

    #[test]
    fn planning_is_deterministic_and_does_not_write() {
        let (root, snapshot) = fixture();
        let before = fs::read(root.join("work/active/001-one/work-item.json")).unwrap();
        let request =
            MutationRequest::create_work_item(CreateWorkItem::new("A New Thing", "2026-01-02"));
        let first = plan(&snapshot, request.clone());
        let second = plan(&snapshot, request);
        assert_eq!(first.plan_token, second.plan_token);
        assert_eq!(first.file_operations, second.file_operations);
        assert_eq!(
            before,
            fs::read(root.join("work/active/001-one/work-item.json")).unwrap()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn expected_absent_target_makes_plan_stale() {
        let (root, snapshot) = fixture();
        let plan = plan(
            &snapshot,
            MutationRequest::create_work_item(CreateWorkItem::new("A New Thing", "2026-01-02")),
        );
        fs::create_dir_all(root.join("work/active/002-a-new-thing")).unwrap();
        let result = plan.apply();
        assert!(!result.success);
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "mutation.plan-stale"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn injected_failure_restores_complete_transition_directory() {
        let (root, snapshot) = fixture();
        let request =
            MutationRequest::transition_work_item(TransitionWorkItem::new("001", "completed"));
        let plan = plan(&snapshot, request);
        let result = plan.apply_with_options(&ApplyOptions::fail_after(0));
        assert!(!result.success);
        assert!(result.rolled_back);
        assert!(root.join("work/active/001-one/extra.md").is_file());
        assert!(!root.join("work/completed/001-one").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pending_completion_fails_post_validation_and_rolls_back() {
        let (root, snapshot) = fixture();
        let plan = plan(
            &snapshot,
            MutationRequest::transition_work_item(TransitionWorkItem::new("001", "completed")),
        );
        let result = plan.apply();
        assert!(!result.success);
        assert!(result.rolled_back);
        assert!(root.join("work/active/001-one/work-item.json").is_file());
        assert!(!root.join("work/completed/001-one").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn typed_edit_updates_only_allowed_fields_and_validates() {
        let (root, snapshot) = fixture();
        let changes = WorkItemEdits {
            title: Some("Edited title".to_owned()),
            ..WorkItemEdits::default()
        };
        let plan = plan(
            &snapshot,
            MutationRequest::edit_work_item(EditWorkItem::new("001", changes, "2026-01-03")),
        );
        let result = plan.apply();
        assert!(result.success, "{result:?}");
        let value: Value = serde_json::from_slice(
            &fs::read(root.join("work/active/001-one/work-item.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(value["title"], "Edited title");
        assert_eq!(value["updated"], "2026-01-03");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unrelated_file_not_in_read_set_does_not_stale_plan() {
        let (root, snapshot) = fixture();
        let plan = plan(
            &snapshot,
            MutationRequest::create_work_item(CreateWorkItem::new("A New Thing", "2026-01-02")),
        );
        fs::write(root.join("unrelated.txt"), "outside the governed read set").unwrap();
        let result = plan.apply();
        assert!(result.success, "{result:?}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn drift_during_apply_is_rejected_and_rolled_back() {
        let (root, snapshot) = fixture();
        let plan = plan(
            &snapshot,
            MutationRequest::create_work_item(CreateWorkItem::new("A New Thing", "2026-01-02")),
        );
        let result = plan.apply_with_options(&ApplyOptions::external_drift(
            0,
            "work/active/002-a-new-thing/work-item.json",
        ));
        assert!(!result.success);
        assert!(result.rolled_back);
        assert!(!root.join("work/active/002-a-new-thing").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn consumed_work_record_drift_is_stale() {
        let (root, snapshot) = fixture();
        let plan = plan(
            &snapshot,
            MutationRequest::create_work_item(CreateWorkItem::new("A New Thing", "2026-01-02")),
        );
        fs::write(
            root.join("work/active/001-one/work-item.json"),
            br#"{"id":"001","title":"Changed","status":"active","owner_scope":"governance","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-01"}"#,
        )
        .unwrap();
        let result = plan.apply();
        assert!(!result.success);
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "mutation.plan-stale"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn consumed_owner_and_dashboard_inputs_are_stale() {
        for relative in ["governance/owners.json", "audits/reports/dashboard.md"] {
            let (root, snapshot) = fixture();
            let plan = plan(
                &snapshot,
                MutationRequest::create_work_item(CreateWorkItem::new("A New Thing", "2026-01-02")),
            );
            fs::write(root.join(relative), b"changed input\n").unwrap();
            let result = plan.apply();
            assert!(!result.success, "{relative}: {result:?}");
            assert!(result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "mutation.plan-stale"));
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn canonical_edit_fixture_matches_exact_after_state() {
        let before: Value =
            serde_json::from_str(include_str!("../tests/fixtures/edit/before/work-item.json"))
                .unwrap();
        let record = repopact_repository::IndexedRecord {
            reference: repopact_types::RecordRef::new(
                repopact_types::RecordKind::WorkItem,
                "001",
                "work-item.json",
            ),
            path: PathBuf::from("work-item.json"),
            value: Ok(before),
            text: None,
            front_matter: Err("not Markdown".to_owned()),
        };
        let actual =
            edited_manifest(&record, &WorkItemEdits::title("Edited title"), "2026-01-03").unwrap();
        let expected = include_str!("../tests/fixtures/edit/after/work-item.json");
        assert_eq!(actual.replace("\r\n", "\n"), expected.replace("\r\n", "\n"));
    }

    #[test]
    fn canonical_transition_fixture_matches_exact_after_state() {
        let mut value: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/transition/before/work/completed/001-reopen/work-item.json"
        ))
        .unwrap();
        value["status"] = Value::String("active".to_owned());
        let actual = canonical_json(&value);
        let expected = include_str!(
            "../tests/fixtures/transition/after/work/active/001-reopen/work-item.json"
        );
        assert_eq!(actual.replace("\r\n", "\n"), expected.replace("\r\n", "\n"));
    }
}
