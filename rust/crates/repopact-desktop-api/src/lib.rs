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
use repopact_graph::overlay::{EffectiveGraphStatus, SessionGraphState};
use repopact_graph::{GraphEdge, GraphNode};
use repopact_mutation::{
    CreateWorkItem, EditWorkItem, GeneratedImpact, MutationDiagnostic, MutationPlan,
    MutationRequest, MutationResult, TransitionWorkItem, WorkItemEdits,
};
use repopact_repository::{
    GitRunner, IndexedRecord, RecordIndex, Repository, RepositorySnapshot, IGNORED_PARTS,
};
use repopact_types::{
    AcceptanceCriterion, Diagnostic, RecordKind, RecordRef, RepositoryIdentity, Severity, WorkItem,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const PLAN_LIMIT: usize = 32;
const WATCH_DEBOUNCE: Duration = Duration::from_millis(120);
const WATCH_IGNORED_PARTS: [&str; 15] = [
    ".git",
    "target",
    "node_modules",
    ".venv",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    "build",
    "dist",
    "fixtures",
    "worktrees",
    ".cache",
    "coverage",
    "out",
];

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
pub struct DecisionSummaryView {
    pub reference: RecordRef,
    pub readable: bool,
    pub title: Option<String>,
    pub status: Option<String>,
    pub date: Option<String>,
    pub supersedes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSummaryView {
    pub reference: RecordRef,
    pub readable: bool,
    pub timestamp: Option<String>,
    pub work_item: Option<String>,
    pub result: Option<String>,
    pub provenance: Option<String>,
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
    /// WI063 ROG-010/013 typed disclosure: whether this graph reflects
    /// the durable baseline exactly or working-tree state not yet
    /// written to `rog/`, the durable baseline's own freshness, and
    /// whether semantic coverage is complete or partial -- three
    /// orthogonal facts a client must not flatten into "current
    /// complete" (Decision 0047).
    pub status: EffectiveGraphStatus,
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
    snapshot: Arc<RepositorySnapshot>,
    /// WI063 ROG-013 session working-tree overlay (Decision 0047). Owns
    /// the effective graph; never serialized, never written to `rog/` by
    /// ordinary session activity.
    overlay: SessionGraphState,
    overview: RepositoryOverview,
    watcher: Option<RepositoryWatcher>,
    plans: BTreeMap<String, StoredPlan>,
    plan_order: VecDeque<String>,
    pending_self_paths: BTreeSet<String>,
    refresh_in_flight: bool,
}

#[derive(Default)]
struct DesktopState {
    next_open_request: u64,
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
        self.open_repository_with_runner(root, None)
    }

    pub fn open_repository_with_git_runner(
        &self,
        root: impl AsRef<Path>,
        git_runner: Arc<dyn GitRunner>,
    ) -> Result<RepositoryOverview, DesktopError> {
        self.open_repository_with_runner(root, Some(git_runner))
    }

    fn open_repository_with_runner(
        &self,
        root: impl AsRef<Path>,
        git_runner: Option<Arc<dyn GitRunner>>,
    ) -> Result<RepositoryOverview, DesktopError> {
        let root = root.as_ref();
        if !root.is_dir() {
            return Err(DesktopError::new(
                "repository.invalid-root",
                format!("repository root is not a directory: {}", root.display()),
            ));
        }
        let open_request = {
            let mut state = self.lock()?;
            state.next_open_request += 1;
            state.next_open_request
        };
        let repository = git_runner
            .map(|runner| Repository::with_git_runner(root, runner))
            .unwrap_or_else(|| Repository::open(root));
        let core = RepoPactCore::open_repository(repository);
        let snapshot = Arc::new(core.snapshot());
        let watcher = RepositoryWatcher::start(root, WATCH_DEBOUNCE).ok();
        let generation = {
            let mut state = self.lock()?;
            state.next_session += 1;
            state.next_session
        };
        let id = format!("session-{generation}");
        let overlay = SessionGraphState::open(&snapshot);
        let overview = overview_from_snapshot(
            &id,
            generation,
            watcher.is_some(),
            &snapshot,
            overlay.effective_graph().nodes.len(),
            overlay.effective_graph().edges.len(),
        );
        let active = ActiveSession {
            id: id.clone(),
            generation,
            core,
            snapshot,
            overlay,
            overview: overview.clone(),
            watcher,
            plans: BTreeMap::new(),
            plan_order: VecDeque::new(),
            pending_self_paths: BTreeSet::new(),
            refresh_in_flight: false,
        };
        let mut state = self.lock()?;
        if open_request != state.next_open_request {
            return Err(DesktopError::new(
                "session.stale",
                "the repository selection was superseded by a newer selection",
            ));
        }
        state.active = Some(active);
        Ok(overview)
    }

    pub fn close_repository(&self) -> Result<(), DesktopError> {
        let mut state = self.lock()?;
        state.active = None;
        Ok(())
    }

    pub fn repository_overview(&self) -> Result<RepositoryOverview, DesktopError> {
        let state = self.lock()?;
        Ok(state
            .active
            .as_ref()
            .ok_or_else(|| DesktopError::new("session.not-open", "select a repository first"))?
            .overview
            .clone())
    }

    pub fn refresh_repository(&self) -> Result<RepositoryOverview, DesktopError> {
        let (id, generation, core, watcher_running) = {
            let mut state = self.lock()?;
            let active = active_mut(&mut state)?;
            if active.refresh_in_flight {
                return Ok(active.overview.clone());
            }
            active.refresh_in_flight = true;
            (
                active.id.clone(),
                active.generation,
                RepoPactCore::open_repository(active.core.repository().clone()),
                active.watcher.is_some(),
            )
        };
        let snapshot = Arc::new(core.snapshot());
        let next_generation = generation + 1;
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        if active.id != id || active.generation != generation {
            return Err(DesktopError::new(
                "session.stale",
                "the repository session changed during refresh",
            ));
        }
        // Explicit refresh is correctness recovery (Decision 0047 section
        // 9): one full source-projection walk, diffed against the
        // overlay's own current in-memory inventory, never a full
        // governance+physical+semantic rebuild from scratch.
        active.overlay.refresh(&snapshot);
        let overview = overview_from_snapshot(
            &id,
            next_generation,
            watcher_running,
            &snapshot,
            active.overlay.effective_graph().nodes.len(),
            active.overlay.effective_graph().edges.len(),
        );
        active.generation = next_generation;
        active.snapshot = snapshot;
        active.overview = overview.clone();
        active.refresh_in_flight = false;
        active.pending_self_paths.clear();
        Ok(overview)
    }

    pub fn validate_repository(&self) -> Result<ValidationView, DesktopError> {
        let state = self.lock()?;
        Ok(state
            .active
            .as_ref()
            .ok_or_else(|| DesktopError::new("session.not-open", "select a repository first"))?
            .overview
            .validation
            .clone())
    }

    pub fn list_work_items(
        &self,
        query: Option<String>,
    ) -> Result<Vec<WorkItemSummaryView>, DesktopError> {
        let snapshot = self.snapshot()?;
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
        let snapshot = self.snapshot()?;
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

    pub fn list_decisions(&self) -> Result<Vec<DecisionSummaryView>, DesktopError> {
        let snapshot = self.snapshot()?;
        Ok(snapshot
            .index()
            .decisions
            .iter()
            .map(decision_summary)
            .collect())
    }

    pub fn list_evidence(&self) -> Result<Vec<EvidenceSummaryView>, DesktopError> {
        let snapshot = self.snapshot()?;
        Ok(snapshot
            .index()
            .evidence
            .iter()
            .map(evidence_summary)
            .collect())
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
        let state = self.lock()?;
        let active = state
            .active
            .as_ref()
            .ok_or_else(|| DesktopError::new("session.not-open", "select a repository first"))?;
        Ok(graph_view(&active.overlay))
    }

    pub fn analyze(&self, query: AnalysisQuery) -> Result<AnalysisView, DesktopError> {
        let snapshot = self.snapshot()?;
        let report = repopact_analysis::analyze(&snapshot, &query);
        Ok(AnalysisView {
            findings: report
                .findings
                .iter()
                .map(AnalysisFindingView::from)
                .collect(),
        })
    }

    pub fn plan_mutation(&self, intent: MutationIntent) -> Result<MutationPlanView, DesktopError> {
        let (snapshot, session_id, generation, next_plan) = {
            let mut state = self.lock()?;
            let next_plan = {
                state.next_plan += 1;
                state.next_plan
            };
            let active = active_mut(&mut state)?;
            (
                active.snapshot.clone(),
                active.id.clone(),
                active.generation,
                next_plan,
            )
        };
        let request = intent.clone().into_request();
        let plan = repopact_mutation::plan(&snapshot, request);
        let handle = format!("plan-{generation}-{next_plan}");
        let view = plan_view(&session_id, &handle, &intent, &plan);
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        if active.id != session_id || active.generation != generation {
            return Err(DesktopError::new(
                "session.stale",
                "the repository session changed while planning",
            ));
        }
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
        let (repository, plan, generation) = {
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
            active.refresh_in_flight = true;
            (
                active.core.repository().clone(),
                stored.plan,
                active.generation,
            )
        };
        let result = RepoPactCore::open_repository(repository.clone()).apply_mutation(&plan);
        let stale = result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.contains("stale"));
        if !result.success {
            if stale {
                // A failed read-set check proves that the cached generation is
                // no longer current. Refresh it before allowing a re-plan,
                // while keeping the failed plan's mutation authority consumed.
                let core = RepoPactCore::open_repository(repository.clone());
                let snapshot = Arc::new(core.snapshot());
                let next_generation = generation + 1;
                let watcher_running = {
                    let state = self.lock()?;
                    state
                        .active
                        .as_ref()
                        .is_some_and(|active| active.id == session_id && active.watcher.is_some())
                };
                let mut state = self.lock()?;
                let active = active_mut(&mut state)?;
                if active.id != session_id || active.generation != generation {
                    return Err(DesktopError::new(
                        "session.stale",
                        "the repository session changed after stale-plan refresh",
                    ));
                }
                // The mutation failed on a stale read-set, not a
                // successful self-apply -- reconcile fully, like an
                // explicit refresh, rather than trusting any particular
                // changed-path set.
                active.overlay.refresh(&snapshot);
                let overview = overview_from_snapshot(
                    session_id,
                    next_generation,
                    watcher_running,
                    &snapshot,
                    active.overlay.effective_graph().nodes.len(),
                    active.overlay.effective_graph().edges.len(),
                );
                active.generation = next_generation;
                active.snapshot = snapshot;
                active.overview = overview;
                active.refresh_in_flight = false;
                return Ok(apply_view(session_id, next_generation, result, stale));
            }
            let mut state = self.lock()?;
            if let Some(active) = state.active.as_mut() {
                if active.id == session_id && active.generation == generation {
                    active.refresh_in_flight = false;
                }
            }
            return Ok(apply_view(session_id, generation, result, stale));
        }
        let changed_paths = result.changed_paths.clone();
        let core = RepoPactCore::open_repository(repository);
        let snapshot = Arc::new(core.snapshot());
        let next_generation = generation + 1;
        let watcher_running = {
            let state = self.lock()?;
            state
                .active
                .as_ref()
                .is_some_and(|active| active.id == session_id && active.watcher.is_some())
        };
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        if active.id != session_id || active.generation != generation {
            return Err(DesktopError::new(
                "session.stale",
                "the repository session changed after applying the plan",
            ));
        }
        // Self-applied mutations update the overlay immediately from the
        // exact changed paths the mutation itself produced (WI063 step
        // 15) -- no waiting for the OS watcher to observe the same edit,
        // and no full rebuild.
        active.overlay.reconcile(&snapshot, &changed_paths);
        let overview = overview_from_snapshot(
            session_id,
            next_generation,
            watcher_running,
            &snapshot,
            active.overlay.effective_graph().nodes.len(),
            active.overlay.effective_graph().edges.len(),
        );
        active.generation = next_generation;
        active.snapshot = snapshot;
        active.overview = overview;
        active.pending_self_paths.extend(changed_paths);
        active.refresh_in_flight = false;
        Ok(apply_view(session_id, next_generation, result, stale))
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
        let (session_id, generation, repository, changes, old_token, watcher_running) = {
            let mut state = self.lock()?;
            let active = active_mut(&mut state)?;
            let Some(watcher) = active.watcher.as_mut() else {
                return Ok(Vec::new());
            };
            if active.refresh_in_flight {
                return Ok(Vec::new());
            }
            let changes = watcher.drain();
            if changes.is_empty() {
                return Ok(Vec::new());
            }
            active.refresh_in_flight = true;
            (
                active.id.clone(),
                active.generation,
                active.core.repository().clone(),
                changes,
                active.snapshot.token(),
                active.watcher.is_some(),
            )
        };
        let core = RepoPactCore::open_repository(repository);
        let snapshot = Arc::new(core.snapshot());
        let snapshot_token = snapshot.token();
        let mut paths = BTreeSet::new();
        for change in changes {
            paths.extend(change);
        }
        let paths_vec: Vec<String> = paths.iter().cloned().collect();
        let next_generation = generation + 1;
        let mut state = self.lock()?;
        let active = active_mut(&mut state)?;
        if active.id != session_id || active.generation != generation {
            return Ok(Vec::new());
        }
        active.refresh_in_flight = false;

        // Reconcile the overlay against the watcher's own changed paths
        // regardless of whether the governance snapshot token moved --
        // `snapshot_token` only reflects governance-record content, so an
        // ordinary source-file edit (the exact case ROG-013 targets)
        // never changes it, but must still update the effective graph
        // (WI063 step 10). This never performs a full source-projection
        // walk or writes `rog/` -- see `SessionGraphState::reconcile`.
        let reconcile_outcome = active.overlay.reconcile(&snapshot, &paths_vec);

        if snapshot_token == old_token && !reconcile_outcome.changed {
            for path in &paths {
                active.pending_self_paths.remove(path);
            }
            return Ok(Vec::new());
        }

        let overview = overview_from_snapshot(
            &session_id,
            next_generation,
            watcher_running,
            &snapshot,
            active.overlay.effective_graph().nodes.len(),
            active.overlay.effective_graph().edges.len(),
        );
        let self_apply = paths
            .iter()
            .all(|path| active.pending_self_paths.remove(path));
        let origin = if self_apply {
            ChangeOrigin::SelfApply
        } else {
            active.pending_self_paths.clear();
            ChangeOrigin::External
        };
        active.generation = next_generation;
        active.snapshot = snapshot;
        active.overview = overview.clone();
        Ok(vec![RepositoryChangedEvent {
            session_id,
            generation: next_generation,
            snapshot_token,
            origin,
            changed_paths: paths.into_iter().collect(),
            overview,
        }])
    }

    fn get_record(&self, kind: RecordKind, id: &str) -> Result<RecordDetailView, DesktopError> {
        let snapshot = self.snapshot()?;
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

    fn snapshot(&self) -> Result<Arc<RepositorySnapshot>, DesktopError> {
        let state = self.lock()?;
        Ok(state
            .active
            .as_ref()
            .ok_or_else(|| DesktopError::new("session.not-open", "select a repository first"))?
            .snapshot
            .clone())
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

fn overview_from_snapshot(
    session_id: &str,
    generation: u64,
    watcher_running: bool,
    snapshot: &RepositorySnapshot,
    graph_node_count: usize,
    graph_edge_count: usize,
) -> RepositoryOverview {
    let validation = validation_view(&repopact_validation::validate_snapshot(snapshot));
    RepositoryOverview {
        session_id: session_id.to_owned(),
        generation,
        identity: snapshot.identity(),
        validation,
        work_item_count: snapshot.index().work_items.len(),
        evidence_count: snapshot.index().evidence.len(),
        decision_count: snapshot.index().decisions.len(),
        graph_node_count,
        graph_edge_count,
        snapshot_token: snapshot.token(),
        watcher: WatcherStatusView {
            state: if watcher_running {
                WatcherState::Running
            } else {
                WatcherState::Unavailable
            },
            recursive: watcher_running,
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

fn decision_summary(record: &IndexedRecord) -> DecisionSummaryView {
    let fields = record.front_matter.as_ref().ok();
    DecisionSummaryView {
        reference: record.reference.clone(),
        readable: record.value.is_ok() || record.text.is_some(),
        title: front_matter_string(fields, "title"),
        status: front_matter_string(fields, "status"),
        date: front_matter_string(fields, "date"),
        supersedes: front_matter_strings(fields, "supersedes"),
    }
}

fn evidence_summary(record: &IndexedRecord) -> EvidenceSummaryView {
    let object = record.value.as_ref().ok().and_then(Value::as_object);
    EvidenceSummaryView {
        reference: record.reference.clone(),
        readable: record.value.is_ok() || record.text.is_some(),
        timestamp: object
            .and_then(|value| value.get("timestamp"))
            .and_then(value_string),
        work_item: object
            .and_then(|value| value.get("work_item"))
            .and_then(value_string),
        result: object
            .and_then(|value| value.get("result"))
            .and_then(value_string),
        provenance: object
            .and_then(|value| value.get("provenance"))
            .and_then(value_string),
    }
}

fn front_matter_string(fields: Option<&BTreeMap<String, Value>>, key: &str) -> Option<String> {
    fields
        .and_then(|fields| fields.get(key))
        .and_then(value_string)
}

fn front_matter_strings(fields: Option<&BTreeMap<String, Value>>, key: &str) -> Vec<String> {
    fields
        .and_then(|fields| fields.get(key))
        .map(value_strings)
        .unwrap_or_default()
}

fn value_string(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn value_strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|values| values.iter().filter_map(value_string).collect())
        .or_else(|| value_string(value).map(|value| vec![value]))
        .unwrap_or_default()
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

fn graph_view(overlay: &SessionGraphState) -> GraphView {
    let graph = overlay.effective_graph();
    GraphView {
        nodes: graph.nodes.values().cloned().collect(),
        edges: graph.edges.clone(),
        status: overlay.status(),
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

fn apply_view(
    session_id: &str,
    generation: u64,
    result: MutationResult,
    stale: bool,
) -> MutationApplyView {
    MutationApplyView {
        session_id: session_id.to_owned(),
        generation,
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
    #[cfg(test)]
    test_sender: mpsc::Sender<RawWatchChange>,
    root: PathBuf,
    debounce: Duration,
    pending_paths: BTreeSet<String>,
    last_received: Option<Instant>,
}

impl RepositoryWatcher {
    pub fn start(root: impl AsRef<Path>, debounce: Duration) -> Result<Self, DesktopError> {
        let root = root.as_ref().to_path_buf();
        let (sender, receiver) = mpsc::channel();
        let callback_sender = sender.clone();
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
                let _ = callback_sender.send(RawWatchChange {
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
            #[cfg(test)]
            test_sender: sender,
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

    #[cfg(test)]
    fn inject_for_test(&self, paths: &[&str]) {
        let _ = self.test_sender.send(RawWatchChange {
            paths: paths.iter().map(|path| (*path).to_owned()).collect(),
            received_at: Instant::now(),
        });
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
    path.split('/')
        .any(|part| IGNORED_PARTS.contains(&part) || WATCH_IGNORED_PARTS.contains(&part))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Condvar, Mutex};
    use std::thread;
    use tempfile::tempdir;

    #[derive(Debug, Default)]
    struct GateState {
        armed: bool,
        entered: bool,
        released: bool,
        entries: usize,
    }

    #[derive(Debug)]
    struct GitGate {
        delegate: Arc<dyn GitRunner>,
        state: Arc<(Mutex<GateState>, Condvar)>,
    }

    impl GitGate {
        fn new(delegate: Arc<dyn GitRunner>) -> Arc<Self> {
            Arc::new(Self {
                delegate,
                state: Arc::new((Mutex::new(GateState::default()), Condvar::new())),
            })
        }

        fn arm(&self) {
            let (lock, _) = &*self.state;
            let mut state = lock.lock().unwrap();
            state.armed = true;
            state.entered = false;
            state.released = false;
        }

        fn wait_entered(&self) {
            let (lock, condition) = &*self.state;
            let state = lock.lock().unwrap();
            let (state, _) = condition
                .wait_timeout_while(state, Duration::from_secs(5), |state| !state.entered)
                .unwrap();
            assert!(state.entered, "the gated Git query did not begin");
        }

        fn release(&self) {
            let (lock, condition) = &*self.state;
            let mut state = lock.lock().unwrap();
            state.released = true;
            condition.notify_all();
        }

        fn entries(&self) -> usize {
            let (lock, _) = &*self.state;
            lock.lock().unwrap().entries
        }
    }

    impl GitRunner for GitGate {
        fn run(
            &self,
            root: &Path,
            args: &[&str],
            label: &str,
        ) -> Result<repopact_repository::GitOutput, repopact_repository::GitError> {
            let should_wait = {
                let (lock, condition) = &*self.state;
                let mut state = lock.lock().unwrap();
                if state.armed && !state.entered {
                    state.entered = true;
                    state.entries += 1;
                    condition.notify_all();
                    true
                } else {
                    false
                }
            };
            if should_wait {
                let (lock, condition) = &*self.state;
                let mut state = lock.lock().unwrap();
                while !state.released {
                    state = condition.wait(state).unwrap();
                }
            }
            self.delegate.run(root, args, label)
        }
    }

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
        assert!(is_ignored_path("target/debug/repopact-cli.exe"));
        assert!(is_ignored_path(".cache/generated.json"));
        assert!(!is_ignored_path("work/active/item/work-item.json"));
    }

    #[test]
    fn desktop_reads_reuse_one_snapshot_generation_without_git_fanout() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let runner = repopact_repository::CountingGitRunner::native();
        let service = DesktopService::new();
        let opened = service
            .open_repository_with_git_runner(dir.path(), runner.clone())
            .unwrap();
        let construction_count = runner.count();
        assert!(construction_count <= 4);

        assert_eq!(
            opened.snapshot_token,
            service.repository_overview().unwrap().snapshot_token
        );
        assert!(service.list_work_items(None).unwrap().is_empty());
        assert!(service.list_decisions().unwrap().is_empty());
        assert!(service.list_evidence().unwrap().is_empty());
        assert!(service.graph().unwrap().nodes.len() >= 1);
        assert!(!service
            .analyze(AnalysisQuery::default())
            .unwrap()
            .findings
            .is_empty());
        let _ = service.validate_repository().unwrap();
        assert_eq!(construction_count, runner.count());
        assert!(runner.max_concurrency() <= 1);
        eprintln!(
            "desktop read fanout git_invocations={} max_concurrency={}",
            runner.count(),
            runner.max_concurrency()
        );
    }

    #[test]
    fn refresh_replaces_generation_and_suppresses_unchanged_watcher_tokens() {
        let dir = tempdir().unwrap();
        let runner = repopact_repository::CountingGitRunner::native();
        let service = DesktopService::new();
        let first = service
            .open_repository_with_git_runner(dir.path(), runner.clone())
            .unwrap();
        let refreshed = service.refresh_repository().unwrap();
        assert!(refreshed.generation > first.generation);
        assert!(runner.count() <= 8);
        assert!(service.poll_repository_events().unwrap().is_empty());
    }

    #[test]
    fn stale_refresh_cannot_clear_new_session_single_flight_guard() {
        let first = tempdir().unwrap();
        let second = tempdir().unwrap();
        fs::create_dir_all(first.path().join(".git")).unwrap();
        fs::create_dir_all(second.path().join(".git")).unwrap();
        let first_delegate = repopact_repository::CountingGitRunner::native();
        let second_delegate = repopact_repository::CountingGitRunner::native();
        let first_gate = GitGate::new(first_delegate);
        let second_gate = GitGate::new(second_delegate);
        let service = DesktopService::new();
        service
            .open_repository_with_git_runner(first.path(), first_gate.clone())
            .unwrap();
        first_gate.arm();

        let stale_service = service.clone();
        let stale_refresh = thread::spawn(move || stale_service.refresh_repository());
        first_gate.wait_entered();

        let second_view = service
            .open_repository_with_git_runner(second.path(), second_gate.clone())
            .unwrap();
        second_gate.arm();
        let current_service = service.clone();
        let current_refresh = thread::spawn(move || current_service.refresh_repository());
        second_gate.wait_entered();

        first_gate.release();
        let stale_result = stale_refresh.join().unwrap();
        assert_eq!(stale_result.unwrap_err().code, "session.stale");

        let entries_before_second_attempt = second_gate.entries();
        let second_attempt = service.refresh_repository().unwrap();
        assert_eq!(second_attempt.generation, second_view.generation);
        assert_eq!(second_gate.entries(), entries_before_second_attempt);

        second_gate.release();
        let completed = current_refresh.join().unwrap().unwrap();
        assert!(completed.generation > second_view.generation);
    }

    #[test]
    fn slower_older_open_cannot_overwrite_newer_selection() {
        let first = tempdir().unwrap();
        let second = tempdir().unwrap();
        fs::create_dir_all(first.path().join(".git")).unwrap();
        let first_delegate = repopact_repository::CountingGitRunner::native();
        let first_gate = GitGate::new(first_delegate);
        let service = DesktopService::new();
        first_gate.arm();
        let old_service = service.clone();
        let old_root = first.path().to_path_buf();
        let old_gate = first_gate.clone();
        let old_open =
            thread::spawn(move || old_service.open_repository_with_git_runner(old_root, old_gate));
        first_gate.wait_entered();

        let new_view = service.open_repository(second.path()).unwrap();
        first_gate.release();
        let old_result = old_open.join().unwrap();
        assert_eq!(old_result.unwrap_err().code, "session.stale");
        let current = service.repository_overview().unwrap();
        assert_eq!(current.session_id, new_view.session_id);
        assert_eq!(current.identity.root, new_view.identity.root);
    }

    #[test]
    fn watcher_burst_has_one_bounded_refresh_and_ignores_build_churn() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let item = dir.path().join("work/active/001-burst");
        fs::create_dir_all(&item).unwrap();
        fs::write(
            item.join("work-item.json"),
            r#"{"id":"001","title":"initial"}"#,
        )
        .unwrap();
        let runner = repopact_repository::CountingGitRunner::native();
        let service = DesktopService::new();
        service
            .open_repository_with_git_runner(dir.path(), runner.clone())
            .unwrap();
        let baseline = runner.count();
        for title in ["first", "second", "third", "final"] {
            fs::write(
                item.join("work-item.json"),
                format!(r#"{{"id":"001","title":"{title}"}}"#),
            )
            .unwrap();
        }
        fs::create_dir_all(dir.path().join("target/debug")).unwrap();
        for number in 0..8 {
            fs::write(
                dir.path()
                    .join(format!("target/debug/generated-{number}.txt")),
                "ignored",
            )
            .unwrap();
        }
        for _ in 0..8 {
            let state = service.lock().unwrap();
            state
                .active
                .as_ref()
                .and_then(|active| active.watcher.as_ref())
                .expect("test repository watcher should be available")
                .inject_for_test(&["work/active/001-burst/work-item.json"]);
        }
        let mut events = Vec::new();
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(80));
            events.extend(service.poll_repository_events().unwrap());
            if !events.is_empty() {
                break;
            }
        }
        assert_eq!(events.len(), 1, "one governed burst should publish once");
        assert!(runner.count() - baseline <= 4);
        assert!(runner.max_concurrency() <= 1);
        std::thread::sleep(Duration::from_millis(180));
        assert!(service.poll_repository_events().unwrap().is_empty());
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

    #[test]
    fn typed_decision_and_evidence_summaries_use_the_cached_snapshot() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        fs::create_dir_all(dir.path().join("decisions")).unwrap();
        fs::create_dir_all(dir.path().join("evidence/runs")).unwrap();
        fs::write(
            dir.path().join("decisions/0042-test.md"),
            "---\nid: 0042\ntitle: Canonical Rust engine\nstatus: accepted\ndate: 2026-09-01\nsupersedes: [0041]\n---\n# Test\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("evidence/runs/run-1.json"),
            r#"{"id":"run-1","timestamp":"2026-09-10T10:00:00Z","work_item":"058","result":"passed","provenance":"test"}"#,
        )
        .unwrap();

        let runner = repopact_repository::CountingGitRunner::native();
        let service = DesktopService::new();
        service
            .open_repository_with_git_runner(dir.path(), runner.clone())
            .unwrap();
        let construction_count = runner.count();
        let decision = service.list_decisions().unwrap().pop().unwrap();
        let evidence = service.list_evidence().unwrap().pop().unwrap();
        assert_eq!(decision.title.as_deref(), Some("Canonical Rust engine"));
        assert_eq!(decision.status.as_deref(), Some("accepted"));
        assert_eq!(decision.supersedes, vec!["0041"]);
        assert_eq!(evidence.result.as_deref(), Some("passed"));
        assert_eq!(evidence.work_item.as_deref(), Some("058"));
        assert_eq!(construction_count, runner.count());
    }

    #[test]
    fn desktop_boundary_has_no_generic_process_authority() {
        let source = include_str!("lib.rs").replace("\r\n", "\n");
        let production = source
            .split("#[cfg(test)]\nmod tests")
            .next()
            .expect("desktop test module marker should be present");
        assert!(!production.contains("std::process"));
        assert!(!production.contains("Command::new"));
        assert!(!production.contains("shell = true"));
    }

    fn all_durable_shard_bytes(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
        use repopact_graph::durable;
        let mut map = std::collections::BTreeMap::new();
        let Ok(Some(manifest)) = durable::read_manifest(root) else {
            return map;
        };
        for shard in &manifest.node_shards {
            if let Ok(bytes) = durable::shard_bytes(root, "nodes", &shard.shard) {
                map.insert(format!("nodes/{}", shard.shard), bytes);
            }
        }
        for shard in &manifest.edge_shards {
            if let Ok(bytes) = durable::shard_bytes(root, "edges", &shard.shard) {
                map.insert(format!("edges/{}", shard.shard), bytes);
            }
        }
        map
    }

    #[test]
    fn watcher_reported_source_edit_updates_the_graph_even_when_governance_token_is_unchanged() {
        // Editing an ordinary source file never changes RepositorySnapshot::token()
        // (governance-record content only), which is exactly the case ROG-013's
        // watcher-driven overlay reconciliation must still handle -- previously
        // this class of edit never updated the cached graph at all.
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "pub fn hello() {}\n").unwrap();
        let service = DesktopService::new();
        service.open_repository(dir.path()).unwrap();
        let before = service.graph().unwrap();
        assert!(before.nodes.iter().any(|node| node.label == "hello"));

        fs::write(
            dir.path().join("src/lib.rs"),
            "pub fn hello() {}\npub fn goodbye() {}\n",
        )
        .unwrap();
        {
            let state = service.lock().unwrap();
            state
                .active
                .as_ref()
                .and_then(|active| active.watcher.as_ref())
                .expect("watcher should be available")
                .inject_for_test(&["src/lib.rs"]);
        }
        let mut events = Vec::new();
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(80));
            events.extend(service.poll_repository_events().unwrap());
            if !events.is_empty() {
                break;
            }
        }
        assert_eq!(events.len(), 1);

        let after = service.graph().unwrap();
        assert!(after.nodes.iter().any(|node| node.label == "goodbye"));
        assert_eq!(
            after.status.basis,
            repopact_graph::overlay::GraphBasis::WorkingOverlay
        );
    }

    #[test]
    fn self_applied_mutation_updates_the_graph_immediately() {
        let dir = tempdir().unwrap();
        // Minimal governed-repository scaffold so a real mutation actually
        // succeeds end to end (unrelated to overlay behavior, but required
        // for `apply_mutation_plan` to succeed rather than merely plan).
        fs::create_dir_all(dir.path().join("governance")).unwrap();
        fs::create_dir_all(dir.path().join("audits")).unwrap();
        fs::write(dir.path().join("VERSION"), "0.0.0\n").unwrap();
        fs::write(dir.path().join("AGENTS.md"), "# Agents\n").unwrap();
        fs::write(
            dir.path().join("governance/invariants.json"),
            r#"{"version":1,"invariants":[{"id":"INV-1","statement":"placeholder","rationale":"placeholder","escalation":"placeholder","enforced_by":null}]}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("governance/frozen-surface.json"),
            r#"{"version":1,"protected":[]}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("governance/owners.json"),
            r#"{"version":2,"enforce_tracked_path_ownership":false,"scopes":[{"id":"governance","paths":["**"],"owner":"governance-owner"}]}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("audits/registry.json"),
            r#"{"version":1,"scopes":[]}"#,
        )
        .unwrap();
        let service = DesktopService::new();
        service.open_repository(dir.path()).unwrap();
        let before_nodes = service.graph().unwrap().nodes.len();

        let intent = MutationIntent::CreateWorkItem(CreateWorkItemIntent {
            title: "Overlay self-apply proof".to_owned(),
            status: "active".to_owned(),
            date: "2026-09-13".to_owned(),
            owner_scope: "governance".to_owned(),
            affected_scopes: Vec::new(),
            depends_on: Vec::new(),
            provenance: "concrete".to_owned(),
            acceptance_criteria: Vec::new(),
        });
        let plan = service.plan_mutation(intent).unwrap();
        let apply = service
            .apply_mutation_plan(&plan.session_id, &plan.plan_handle)
            .unwrap();
        assert!(
            apply.success,
            "mutation apply should succeed against a minimal but valid governed repository: {:?}",
            apply.diagnostics
        );

        // Immediately after apply, no watcher poll has happened yet --
        // the graph must already reflect the new work item.
        let after_nodes = service.graph().unwrap().nodes.len();
        assert!(after_nodes > before_nodes);

        // The watcher later reporting the same self-applied paths must
        // not duplicate work or manufacture a second visible change.
        {
            let state = service.lock().unwrap();
            state
                .active
                .as_ref()
                .and_then(|active| active.watcher.as_ref())
                .expect("watcher should be available")
                .inject_for_test(
                    &apply
                        .changed_paths
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                );
        }
        std::thread::sleep(Duration::from_millis(200));
        let events = service.poll_repository_events().unwrap();
        assert!(
            events.is_empty() || events[0].origin == ChangeOrigin::SelfApply,
            "a self-applied change reported later by the watcher must be recognized as such, not External"
        );
        assert_eq!(service.graph().unwrap().nodes.len(), after_nodes);
    }

    #[test]
    fn desktop_session_activity_never_writes_the_durable_graph() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "pub fn hello() {}\n").unwrap();
        // No durable rog/ exists yet -- session activity below must never
        // create or otherwise write one; only an explicit `graph build`/
        // `graph update` (never called here) may do that.
        let service = DesktopService::new();
        service.open_repository(dir.path()).unwrap();

        fs::write(
            dir.path().join("src/lib.rs"),
            "pub fn hello() {}\npub fn added() {}\n",
        )
        .unwrap();
        {
            let state = service.lock().unwrap();
            state
                .active
                .as_ref()
                .and_then(|active| active.watcher.as_ref())
                .expect("watcher should be available")
                .inject_for_test(&["src/lib.rs"]);
        }
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(60));
            if !service.poll_repository_events().unwrap().is_empty() {
                break;
            }
        }
        service.refresh_repository().unwrap();

        assert!(
            !dir.path().join("rog").exists(),
            "ordinary desktop session activity (open/watcher/refresh) must never create rog/"
        );
        assert!(all_durable_shard_bytes(dir.path()).is_empty());
    }
}
