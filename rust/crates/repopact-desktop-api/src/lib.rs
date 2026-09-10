//! The narrow Rust-owned boundary used by the WI055 desktop workbench.
//!
//! This crate deliberately has no Tauri dependency.  It owns the selected
//! repository session, the in-memory mutation plans, and the native watcher;
//! the Tauri adapter is only a transport for these typed operations.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use repopact_analysis::{AnalysisFinding, AnalysisQuery};
use repopact_core::RepoPactCore;
use repopact_graph::{GraphEdge, GraphNode, RepositoryGraph};
use repopact_mutation::{
    CreateWorkItem, EditWorkItem, GeneratedImpact, MutationDiagnostic, MutationPlan,
    MutationRequest, MutationResult, TransitionWorkItem, WorkItemEdits,
};
use repopact_repository::{IndexedRecord, RecordIndex, RepositorySnapshot, IGNORED_PARTS};
use repopact_types::{
    AcceptanceCriterion, Diagnostic, RecordKind, RecordRef, RepositoryIdentity, Severity, WorkItem,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const PLAN_LIMIT: usize = 32;
const WATCH_DEBOUNCE: Duration = Duration::from_millis(120);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopError {
    pub code: String,
    pub message: String,
}

impl DesktopError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticView {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub path: Option<String>,
    pub record: Option<String>,
    pub field: Option<String>,
    pub related_records: Vec<String>,
    pub suggested_actions: Vec<String>,
}

impl From<&Diagnostic> for DiagnosticView {
    fn from(value: &Diagnostic) -> Self {
        Self {
            code: value.code.clone(),
            severity: value.severity,
            message: value.message.clone(),
            path: value.path.clone(),
            record: value.record.clone(),
            field: value.field.clone(),
            related_records: value.related_records.clone().unwrap_or_default(),
            suggested_actions: value.suggested_actions.clone().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationView {
    pub valid: bool,
    pub diagnostics: Vec<DiagnosticView>,
    pub error_count: usize,
    pub warning_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatcherState {
    Running,
    Unavailable,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatcherStatusView {
    pub state: WatcherState,
    pub recursive: bool,
    pub debounce_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryOverview {
    pub session_id: String,
    pub generation: u64,
    pub identity: RepositoryIdentity,
    pub validation: ValidationView,
    pub work_item_count: usize,
    pub evidence_count: usize,
    pub decision_count: usize,
    pub graph_node_count: usize,
    pub graph_edge_count: usize,
    pub snapshot_token: String,
    pub watcher: WatcherStatusView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItemSummaryView {
    pub id: String,
    pub title: String,
    pub status: String,
    pub owner_scope: String,
    pub affected_scopes: Vec<String>,
    pub depends_on: Vec<String>,
    pub provenance: String,
    pub path: String,
    pub criterion_count: usize,
    pub evidence_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItemDetailView {
    pub summary: WorkItemSummaryView,
    pub work_item: WorkItem,
    pub dependents: Vec<String>,
    pub raw_record: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSummaryView {
    pub reference: RecordRef,
    pub readable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordDetailView {
    pub reference: RecordRef,
    pub value: Option<Value>,
    pub text: Option<String>,
    pub readable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphView {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisFindingView {
    pub kind: repopact_analysis::AnalysisKind,
    pub classification: repopact_analysis::FindingClassification,
    pub code: String,
    pub message: String,
    pub basis: Vec<RecordRef>,
    pub related_records: Vec<String>,
    pub remediation: Option<String>,
}

impl From<&AnalysisFinding> for AnalysisFindingView {
    fn from(value: &AnalysisFinding) -> Self {
        Self {
            kind: value.kind,
            classification: value.classification,
            code: value.code.clone(),
            message: value.message.clone(),
            basis: value.basis.clone(),
            related_records: value.related_records.clone(),
            remediation: value.remediation.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisView {
    pub findings: Vec<AnalysisFindingView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateWorkItemIntent {
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItemEditsIntent {
    pub title: Option<String>,
    pub owner_scope: Option<String>,
    pub affected_scopes: Option<Vec<String>>,
    pub depends_on: Option<Vec<String>>,
    pub provenance: Option<String>,
    pub acceptance_criteria: Option<Vec<AcceptanceCriterion>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditWorkItemIntent {
    pub id: String,
    pub changes: WorkItemEditsIntent,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionWorkItemIntent {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum MutationIntent {
    CreateWorkItem(CreateWorkItemIntent),
    EditWorkItem(EditWorkItemIntent),
    TransitionWorkItem(TransitionWorkItemIntent),
}

impl MutationIntent {
    fn into_request(self) -> MutationRequest {
        match self {
            Self::CreateWorkItem(value) => MutationRequest::CreateWorkItem(CreateWorkItem {
                title: value.title,
                status: value.status,
                date: value.date,
                owner_scope: value.owner_scope,
                affected_scopes: value.affected_scopes,
                depends_on: value.depends_on,
                provenance: value.provenance,
                acceptance_criteria: value.acceptance_criteria,
            }),
            Self::EditWorkItem(value) => MutationRequest::EditWorkItem(EditWorkItem {
                id: value.id,
                changes: WorkItemEdits {
                    title: value.changes.title,
                    owner_scope: value.changes.owner_scope,
                    affected_scopes: value.changes.affected_scopes,
                    depends_on: value.changes.depends_on,
                    provenance: value.changes.provenance,
                    acceptance_criteria: value.changes.acceptance_criteria,
                },
                date: value.date,
            }),
            Self::TransitionWorkItem(value) => {
                MutationRequest::TransitionWorkItem(TransitionWorkItem {
                    id: value.id,
                    status: value.status,
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedImpactView {
    pub path: String,
    pub before_digest: String,
    pub after_digest: String,
    pub preview: String,
    pub reason: String,
}

impl From<&GeneratedImpact> for GeneratedImpactView {
    fn from(value: &GeneratedImpact) -> Self {
        Self {
            path: value.path.clone(),
            before_digest: value.before_digest.clone(),
            after_digest: value.after_digest.clone(),
            preview: value.preview.clone(),
            reason: value.reason.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationDiagnosticView {
    pub code: String,
    pub message: String,
    pub path: Option<String>,
    pub blocking: bool,
    pub related_records: Vec<String>,
}

impl From<&MutationDiagnostic> for MutationDiagnosticView {
    fn from(value: &MutationDiagnostic) -> Self {
        Self {
            code: value.code.clone(),
            message: value.message.clone(),
            path: value.path.clone(),
            blocking: value.blocking,
            related_records: value.related_records.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationPlanView {
    pub session_id: String,
    pub plan_handle: String,
    pub plan_token: String,
    pub intent: MutationIntent,
    pub diagnostics: Vec<MutationDiagnosticView>,
    pub generated_impacts: Vec<GeneratedImpactView>,
    pub graph_impacts: Vec<GraphEdge>,
    pub preview: String,
    pub applicable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationApplyView {
    pub session_id: String,
    pub generation: u64,
    pub plan_token: String,
    pub success: bool,
    pub rolled_back: bool,
    pub stale: bool,
    pub changed_paths: Vec<String>,
    pub diagnostics: Vec<MutationDiagnosticView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeOrigin {
    External,
    SelfApply,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryChangedEvent {
    pub session_id: String,
    pub generation: u64,
    pub snapshot_token: String,
    pub origin: ChangeOrigin,
    pub changed_paths: Vec<String>,
    pub overview: RepositoryOverview,
}

#[derive(Debug, Clone)]
struct StoredPlan {
    plan: MutationPlan,
}

struct ActiveSession {
    id: String,
    generation: u64,
    core: RepoPactCore,
    watcher: Option<RepositoryWatcher>,
    plans: BTreeMap<String, StoredPlan>,
    plan_order: VecDeque<String>,
    pending_self_paths: BTreeSet<String>,
}

#[derive(Default)]
struct DesktopState {
    next_session: u64,
    next_plan: u64,
    active: Option<ActiveSession>,
}

/// Thread-safe, in-memory desktop boundary.  No repository-local state is
/// created: plans and session generations disappear when this value drops.
#[derive(Clone, Default)]
pub struct DesktopService {
    state: Arc<Mutex<DesktopState>>,
}

impl DesktopService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_repository(
        &self,
        root: impl AsRef<Path>,
    ) -> Result<RepositoryOverview, DesktopError> {
        let root = root.as_ref();
        if !root.is_dir() {
            return Err(DesktopError::new(
                "repository.invalid-root",
                format!("repository root is not a directory: {}", root.display()),
            ));
        }
        let mut state = self.lock()?;
        state.next_session += 1;
        let generation = state.next_session;
        let id = format!("session-{generation}");
        let core = RepoPactCore::open(root);
        let watcher = RepositoryWatcher::start(root, WATCH_DEBOUNCE).ok();
        let active = ActiveSession {
            id: id.clone(),
            generation,
            core,
            watcher,
            plans: BTreeMap::new(),
            plan_order: VecDeque::new(),
            pending_self_paths: BTreeSet::new(),
        };
        state.active = Some(active);
        let active = state
            .active
            .as_ref()
            .expect("active session just installed");
        Ok(overview(active))
    }

    pub fn close_repository(&self) -> Result<(), DesktopError> {
        let mut state = self.lock()?;
        state.active = None;
        Ok(())
    }

    pub fn repository_overview(&self) -> Result<RepositoryOverview, DesktopError> {
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        Ok(overview(active))
    }

    pub fn refresh_repository(&self) -> Result<RepositoryOverview, DesktopError> {
        self.repository_overview()
    }

    pub fn validate_repository(&self) -> Result<ValidationView, DesktopError> {
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        Ok(validation_view(&active.core.validate()))
    }

    pub fn list_work_items(
        &self,
        query: Option<String>,
    ) -> Result<Vec<WorkItemSummaryView>, DesktopError> {
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        let snapshot = active.core.snapshot();
        let needle = query.unwrap_or_default().to_lowercase();
        Ok(snapshot
            .index()
            .work_items
            .iter()
            .filter_map(work_summary)
            .filter(|item| {
                needle.is_empty()
                    || item.id.to_lowercase().contains(&needle)
                    || item.title.to_lowercase().contains(&needle)
                    || item.status.to_lowercase().contains(&needle)
            })
            .collect())
    }

    pub fn get_work_item(&self, id: &str) -> Result<WorkItemDetailView, DesktopError> {
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        let snapshot = active.core.snapshot();
        let record = snapshot
            .index()
            .work_items
            .iter()
            .find(|record| typed_work_id(record).as_deref() == Some(id))
            .ok_or_else(|| {
                DesktopError::new("work-item.not-found", format!("unknown work item: {id}"))
            })?;
        let work_item = typed_work(record)?;
        let summary = work_summary(record).ok_or_else(|| {
            DesktopError::new(
                "work-item.invalid",
                format!("unable to read work item: {id}"),
            )
        })?;
        let dependents = snapshot
            .index()
            .work_items
            .iter()
            .filter_map(|record| typed_work(record).ok())
            .filter(|candidate| {
                candidate
                    .depends_on
                    .iter()
                    .any(|dependency| dependency == id)
            })
            .map(|candidate| candidate.id)
            .collect();
        Ok(WorkItemDetailView {
            summary,
            work_item,
            raw_record: record.value.clone().map_err(|error| {
                DesktopError::new("record.invalid-json", format!("{id}: {error}"))
            })?,
            dependents,
        })
    }

    pub fn list_decisions(&self) -> Result<Vec<RecordSummaryView>, DesktopError> {
        self.list_records(RecordKind::Decision)
    }

    pub fn list_evidence(&self) -> Result<Vec<RecordSummaryView>, DesktopError> {
        self.list_records(RecordKind::EvidenceRun)
    }

    pub fn get_decision(&self, id: &str) -> Result<RecordDetailView, DesktopError> {
        self.get_record(RecordKind::Decision, id)
    }

    pub fn get_evidence(&self, id: &str) -> Result<RecordDetailView, DesktopError> {
        self.get_record(RecordKind::EvidenceRun, id)
    }

    pub fn get_indexed_record_raw(
        &self,
        kind: RecordKind,
        id: &str,
    ) -> Result<RecordDetailView, DesktopError> {
        self.get_record(kind, id)
    }

    pub fn graph(&self) -> Result<GraphView, DesktopError> {
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        let graph = active.core.graph();
        Ok(graph_view(graph))
    }

    pub fn analyze(&self, query: AnalysisQuery) -> Result<AnalysisView, DesktopError> {
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        let report = active.core.analyze(&query);
        Ok(AnalysisView {
            findings: report
                .findings
                .iter()
                .map(AnalysisFindingView::from)
                .collect(),
        })
    }

    pub fn plan_mutation(&self, intent: MutationIntent) -> Result<MutationPlanView, DesktopError> {
        let mut state = self.lock()?;
        let next_plan = {
            state.next_plan += 1;
            state.next_plan
        };
        let active = active_mut(&mut state)?;
        let request = intent.clone().into_request();
        let plan = active.core.plan_mutation(request);
        let handle = format!("plan-{}-{next_plan}", active.generation);
        let view = plan_view(&active.id, &handle, &intent, &plan);
        active.plans.insert(handle.clone(), StoredPlan { plan });
        active.plan_order.push_back(handle);
        while active.plan_order.len() > PLAN_LIMIT {
            if let Some(expired) = active.plan_order.pop_front() {
                active.plans.remove(&expired);
            }
        }
        Ok(view)
    }

    pub fn apply_mutation_plan(
        &self,
        session_id: &str,
        plan_handle: &str,
    ) -> Result<MutationApplyView, DesktopError> {
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        if active.id != session_id {
            return Err(DesktopError::new(
                "session.stale",
                "the repository session changed; select the repository again",
            ));
        }
        let stored = active.plans.remove(plan_handle).ok_or_else(|| {
            DesktopError::new(
                "plan.stale",
                "the mutation plan is missing, expired, or already consumed",
            )
        })?;
        active.plan_order.retain(|handle| handle != plan_handle);
        let result = active.core.apply_mutation(&stored.plan);
        let stale = result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.contains("stale"));
        if result.success {
            active
                .pending_self_paths
                .extend(result.changed_paths.iter().cloned());
        }
        Ok(apply_view(active, result, stale))
    }

    pub fn discard_mutation_plan(
        &self,
        session_id: &str,
        plan_handle: &str,
    ) -> Result<(), DesktopError> {
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        if active.id != session_id {
            return Err(DesktopError::new(
                "session.stale",
                "the repository session changed",
            ));
        }
        active.plans.remove(plan_handle).ok_or_else(|| {
            DesktopError::new(
                "plan.stale",
                "the mutation plan is missing, expired, or already consumed",
            )
        })?;
        active.plan_order.retain(|handle| handle != plan_handle);
        Ok(())
    }

    pub fn poll_repository_events(&self) -> Result<Vec<RepositoryChangedEvent>, DesktopError> {
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        let Some(watcher) = active.watcher.as_mut() else {
            return Ok(Vec::new());
        };
        let changes = watcher.drain();
        if changes.is_empty() {
            return Ok(Vec::new());
        }
        let snapshot = active.core.snapshot();
        let overview = overview_from_snapshot(active, &snapshot);
        let snapshot_token = snapshot.token();
        let mut paths = BTreeSet::new();
        for change in changes {
            paths.extend(change);
        }
        let self_apply = paths
            .iter()
            .all(|path| active.pending_self_paths.remove(path));
        let origin = if self_apply {
            ChangeOrigin::SelfApply
        } else {
            active.pending_self_paths.clear();
            ChangeOrigin::External
        };
        Ok(vec![RepositoryChangedEvent {
            session_id: active.id.clone(),
            generation: active.generation,
            snapshot_token,
            origin,
            changed_paths: paths.into_iter().collect(),
            overview,
        }])
    }

    fn list_records(&self, kind: RecordKind) -> Result<Vec<RecordSummaryView>, DesktopError> {
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        let snapshot = active.core.snapshot();
        Ok(records_for_kind(snapshot.index(), kind)
            .into_iter()
            .map(|record| RecordSummaryView {
                reference: record.reference.clone(),
                readable: record.value.is_ok() || record.text.is_some(),
            })
            .collect())
    }

    fn get_record(&self, kind: RecordKind, id: &str) -> Result<RecordDetailView, DesktopError> {
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        let snapshot = active.core.snapshot();
        let record = records_for_kind(snapshot.index(), kind)
            .into_iter()
            .find(|record| record.reference.id == id || record.reference.path == id)
            .ok_or_else(|| {
                DesktopError::new("record.not-found", format!("record not found: {id}"))
            })?;
        Ok(record_detail(record))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, DesktopState>, DesktopError> {
        self.state.lock().map_err(|_| {
            DesktopError::new("desktop.state-poisoned", "desktop state is unavailable")
        })
    }
}

fn default_concrete() -> String {
    "concrete".to_owned()
}

fn active_mut(state: &mut DesktopState) -> Result<&mut ActiveSession, DesktopError> {
    state
        .active
        .as_mut()
        .ok_or_else(|| DesktopError::new("session.not-open", "select a repository first"))
}

fn validation_view(report: &repopact_types::ValidationReport) -> ValidationView {
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
    ValidationView {
        valid: error_count == 0,
        diagnostics: report
            .diagnostics
            .iter()
            .map(DiagnosticView::from)
            .collect(),
        error_count,
        warning_count,
    }
}

fn overview(active: &ActiveSession) -> RepositoryOverview {
    let snapshot = active.core.snapshot();
    overview_from_snapshot(active, &snapshot)
}

fn overview_from_snapshot(
    active: &ActiveSession,
    snapshot: &RepositorySnapshot,
) -> RepositoryOverview {
    let validation = validation_view(&repopact_validation::validate_snapshot(snapshot));
    let graph = repopact_graph::build(snapshot);
    RepositoryOverview {
        session_id: active.id.clone(),
        generation: active.generation,
        identity: snapshot.identity(),
        validation,
        work_item_count: snapshot.index().work_items.len(),
        evidence_count: snapshot.index().evidence.len(),
        decision_count: snapshot.index().decisions.len(),
        graph_node_count: graph.nodes.len(),
        graph_edge_count: graph.edges.len(),
        snapshot_token: snapshot.token(),
        watcher: WatcherStatusView {
            state: if active.watcher.is_some() {
                WatcherState::Running
            } else {
                WatcherState::Unavailable
            },
            recursive: active.watcher.is_some(),
            debounce_ms: WATCH_DEBOUNCE.as_millis() as u64,
        },
    }
}

fn work_summary(record: &IndexedRecord) -> Option<WorkItemSummaryView> {
    let item = typed_work(record).ok()?;
    Some(WorkItemSummaryView {
        id: item.id,
        title: item.title,
        status: item.status,
        owner_scope: item.owner_scope,
        affected_scopes: item.affected_scopes,
        depends_on: item.depends_on,
        provenance: item.provenance,
        path: record.reference.path.clone(),
        criterion_count: item.acceptance_criteria.len(),
        evidence_count: item
            .acceptance_criteria
            .iter()
            .flat_map(|criterion| criterion.evidence.iter())
            .count(),
    })
}

fn typed_work(record: &IndexedRecord) -> Result<WorkItem, DesktopError> {
    record
        .value
        .clone()
        .map_err(|error| DesktopError::new("work-item.invalid-json", error))
        .and_then(|value| {
            serde_json::from_value(value)
                .map_err(|error| DesktopError::new("work-item.invalid-schema", error.to_string()))
        })
}

fn typed_work_id(record: &IndexedRecord) -> Option<String> {
    typed_work(record).ok().map(|item| item.id)
}

fn records_for_kind<'a>(index: &'a RecordIndex, kind: RecordKind) -> Vec<&'a IndexedRecord> {
    match kind {
        RecordKind::WorkItem => index.work_items.iter().collect(),
        RecordKind::EvidenceRun => index.evidence.iter().collect(),
        RecordKind::Decision => index.decisions.iter().collect(),
        RecordKind::Policy => index.policies.iter().collect(),
        RecordKind::Contract => index.contracts.iter().collect(),
        RecordKind::AuditFinding => index.audit_findings.iter().collect(),
        RecordKind::Invariant => index.invariants.iter().collect(),
        RecordKind::FrozenSurface => index.frozen_surface.iter().collect(),
        RecordKind::Role => index.owners.iter().collect(),
        RecordKind::AuditRegistry => index.audit_registry.iter().collect(),
        RecordKind::Dashboard => index.dashboard.iter().collect(),
        _ => Vec::new(),
    }
}

fn record_detail(record: &IndexedRecord) -> RecordDetailView {
    RecordDetailView {
        reference: record.reference.clone(),
        value: record.value.clone().ok(),
        text: record.text.clone(),
        readable: record.value.is_ok() || record.text.is_some(),
    }
}

fn graph_view(graph: RepositoryGraph) -> GraphView {
    GraphView {
        nodes: graph.nodes.into_values().collect(),
        edges: graph.edges,
    }
}

fn plan_view(
    session_id: &str,
    handle: &str,
    intent: &MutationIntent,
    plan: &MutationPlan,
) -> MutationPlanView {
    MutationPlanView {
        session_id: session_id.to_owned(),
        plan_handle: handle.to_owned(),
        plan_token: plan.plan_token.clone(),
        intent: intent.clone(),
        diagnostics: plan
            .diagnostics
            .iter()
            .map(MutationDiagnosticView::from)
            .collect(),
        generated_impacts: plan
            .generated_impacts
            .iter()
            .map(GeneratedImpactView::from)
            .collect(),
        graph_impacts: plan.graph_impacts.clone(),
        preview: plan.preview.clone(),
        applicable: plan.is_applicable(),
    }
}

fn apply_view(active: &ActiveSession, result: MutationResult, stale: bool) -> MutationApplyView {
    MutationApplyView {
        session_id: active.id.clone(),
        generation: active.generation,
        plan_token: result.plan_token.clone(),
        success: result.success,
        rolled_back: result.rolled_back,
        stale,
        changed_paths: result.changed_paths,
        diagnostics: result
            .diagnostics
            .iter()
            .map(MutationDiagnosticView::from)
            .collect(),
    }
}

#[derive(Debug)]
struct RawWatchChange {
    paths: Vec<String>,
    received_at: Instant,
}

/// Native recursive watcher.  It only emits normalized, repository-relative
/// paths and never grants the frontend filesystem access.
pub struct RepositoryWatcher {
    _watcher: RecommendedWatcher,
    receiver: mpsc::Receiver<RawWatchChange>,
    root: PathBuf,
    debounce: Duration,
    pending_paths: BTreeSet<String>,
    last_received: Option<Instant>,
}

impl RepositoryWatcher {
    pub fn start(root: impl AsRef<Path>, debounce: Duration) -> Result<Self, DesktopError> {
        let root = root.as_ref().to_path_buf();
        let (sender, receiver) = mpsc::channel();
        let callback_root = root.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                let Ok(event) = result else { return };
                if !is_relevant_event(&event) {
                    return;
                }
                let paths = event
                    .paths
                    .into_iter()
                    .filter_map(|path| normalize_watch_path(&callback_root, &path))
                    .filter(|path| !is_ignored_path(path))
                    .collect::<Vec<_>>();
                if paths.is_empty() {
                    return;
                }
                let _ = sender.send(RawWatchChange {
                    paths,
                    received_at: Instant::now(),
                });
            },
            Config::default(),
        )
        .map_err(|error| DesktopError::new("watcher.start-failed", error.to_string()))?;
        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|error| DesktopError::new("watcher.watch-failed", error.to_string()))?;
        Ok(Self {
            _watcher: watcher,
            receiver,
            root,
            debounce,
            pending_paths: BTreeSet::new(),
            last_received: None,
        })
    }

    pub fn drain(&mut self) -> Vec<Vec<String>> {
        while let Ok(change) = self.receiver.try_recv() {
            self.pending_paths.extend(change.paths);
            self.last_received = Some(change.received_at);
        }
        if self
            .last_received
            .is_some_and(|received| received.elapsed() < self.debounce)
        {
            return Vec::new();
        }
        if self.pending_paths.is_empty() {
            Vec::new()
        } else {
            self.last_received = None;
            vec![std::mem::take(&mut self.pending_paths)
                .into_iter()
                .collect()]
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn is_relevant_event(event: &Event) -> bool {
    !matches!(event.kind, EventKind::Access(_))
}

fn normalize_watch_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let value = relative.to_string_lossy().replace('\\', "/");
    (!value.is_empty()).then_some(value)
}

fn is_ignored_path(path: &str) -> bool {
    path.split('/').any(|part| IGNORED_PARTS.contains(&part))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn plan_view_does_not_expose_file_operations_and_apply_uses_handle_registry() {
        let dir = tempdir().unwrap();
        let service = DesktopService::new();
        service.open_repository(dir.path()).unwrap();
        let intent = MutationIntent::CreateWorkItem(CreateWorkItemIntent {
            title: "Desktop adapter proof".to_owned(),
            status: "active".to_owned(),
            date: "2026-09-10".to_owned(),
            owner_scope: "governance".to_owned(),
            affected_scopes: Vec::new(),
            depends_on: Vec::new(),
            provenance: "concrete".to_owned(),
            acceptance_criteria: Vec::new(),
        });
        let plan = service.plan_mutation(intent).unwrap();
        assert!(!serde_json::to_value(&plan)
            .unwrap()
            .to_string()
            .contains("file_operations"));
        let apply = service
            .apply_mutation_plan(&plan.session_id, &plan.plan_handle)
            .unwrap();
        assert_eq!(apply.success, plan.applicable);
        assert!(service
            .apply_mutation_plan(&plan.session_id, &plan.plan_handle)
            .is_err());
    }

    #[test]
    fn session_switch_invalidates_old_plan_and_keeps_linked_identity() {
        let first = tempdir().unwrap();
        let second = tempdir().unwrap();
        let service = DesktopService::new();
        service.open_repository(first.path()).unwrap();
        let plan = service
            .plan_mutation(MutationIntent::TransitionWorkItem(
                TransitionWorkItemIntent {
                    id: "999".to_owned(),
                    status: "completed".to_owned(),
                },
            ))
            .unwrap();
        let second_view = service.open_repository(second.path()).unwrap();
        assert_ne!(plan.session_id, second_view.session_id);
        assert!(service
            .apply_mutation_plan(&plan.session_id, &plan.plan_handle)
            .is_err());
        assert!(!second_view.identity.root.is_empty());
    }

    #[test]
    fn watcher_path_classification_is_relative_and_ignores_generated_directories() {
        let root = PathBuf::from(r"C:\repo");
        assert_eq!(
            normalize_watch_path(&root, &root.join("work/active/item/work-item.json")),
            Some("work/active/item/work-item.json".to_owned())
        );
        assert!(is_ignored_path("fixtures/sample/work-item.json"));
        assert!(is_ignored_path("node_modules/pkg/index.js"));
        assert!(!is_ignored_path("work/active/item/work-item.json"));
    }

    #[test]
    fn raw_record_access_is_indexed_identity_only() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("decisions")).unwrap();
        fs::write(
            dir.path().join("decisions/0042-test.md"),
            "---\nid: 0042\n---\n# Test\n",
        )
        .unwrap();
        let service = DesktopService::new();
        service.open_repository(dir.path()).unwrap();
        let detail = service.get_decision("0042").unwrap();
        assert!(detail.readable);
        assert!(detail.text.is_some());
        assert!(service.get_decision("C:/not-indexed.json").is_err());
    }
}
