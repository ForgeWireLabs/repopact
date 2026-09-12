use std::path::Path;

use repopact_analysis::{analyze, AnalysisQuery, AnalysisReport};
use repopact_graph::{build, RepositoryGraph};
use repopact_mutation::{apply, plan, ApplyOptions, MutationPlan, MutationRequest, MutationResult};
use repopact_repository::{Repository, RepositorySession, RepositorySnapshot};
use repopact_types::{RepositoryIdentity, ValidationReport};

/// Reusable, non-Tauri RepoPact façade. Graph, analysis, mutation, and
/// validation all start from an immutable snapshot produced by the same
/// repository session.
pub struct RepoPactCore {
    session: RepositorySession,
}

impl RepoPactCore {
    pub fn open(root: impl AsRef<Path>) -> Self {
        Self::open_repository(Repository::open(root))
    }

    pub fn open_repository(repository: Repository) -> Self {
        Self {
            session: repository.session(),
        }
    }

    pub fn repository(&self) -> &Repository {
        self.session.repository()
    }

    pub fn identity(&self) -> RepositoryIdentity {
        self.session.repository().identity()
    }

    /// Build one fresh immutable generation. Desktop/session consumers should
    /// retain this snapshot and use the `*_snapshot` projections below.
    pub fn snapshot(&self) -> RepositorySnapshot {
        self.session.snapshot()
    }

    /// Fresh-snapshot convenience for isolated callers.
    pub fn validate(&self) -> ValidationReport {
        let snapshot = self.snapshot();
        self.validate_snapshot(&snapshot)
    }

    pub fn validate_snapshot(&self, snapshot: &RepositorySnapshot) -> ValidationReport {
        repopact_validation::validate_snapshot(snapshot)
    }

    /// Fresh-snapshot convenience for isolated callers.
    pub fn graph(&self) -> RepositoryGraph {
        self.graph_snapshot(&self.snapshot())
    }

    pub fn graph_snapshot(&self, snapshot: &RepositorySnapshot) -> RepositoryGraph {
        build(snapshot)
    }

    /// Fresh-snapshot convenience for isolated callers.
    pub fn analyze(&self, query: &AnalysisQuery) -> AnalysisReport {
        let snapshot = self.snapshot();
        self.analyze_snapshot(&snapshot, query)
    }

    pub fn analyze_snapshot(
        &self,
        snapshot: &RepositorySnapshot,
        query: &AnalysisQuery,
    ) -> AnalysisReport {
        analyze(snapshot, query)
    }

    /// Fresh-snapshot convenience for isolated callers.
    pub fn plan_mutation(&self, request: MutationRequest) -> MutationPlan {
        let snapshot = self.snapshot();
        self.plan_mutation_snapshot(&snapshot, request)
    }

    pub fn plan_mutation_snapshot(
        &self,
        snapshot: &RepositorySnapshot,
        request: MutationRequest,
    ) -> MutationPlan {
        plan(snapshot, request)
    }

    pub fn apply_mutation(&self, mutation: &MutationPlan) -> MutationResult {
        apply(mutation, &ApplyOptions::default())
    }

    pub fn apply_mutation_with_options(
        &self,
        mutation: &MutationPlan,
        options: &ApplyOptions,
    ) -> MutationResult {
        apply(mutation, options)
    }
}

pub fn validate(root: impl AsRef<Path>) -> ValidationReport {
    RepoPactCore::open(root).validate()
}
