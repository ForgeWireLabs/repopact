use std::path::Path;

use repopact_analysis::{analyze, AnalysisQuery, AnalysisReport};
use repopact_graph::{build, RepositoryGraph};
use repopact_mutation::{apply, plan, ApplyOptions, MutationPlan, MutationRequest, MutationResult};
use repopact_repository::{Repository, RepositorySession, RepositorySnapshot};
use repopact_types::{RepositoryIdentity, ValidationReport};

/// Reusable, non-Tauri RepoPact façade. Graph, analysis, and mutation all start
/// from an immutable snapshot produced by the same repository session.
pub struct RepoPactCore {
    session: RepositorySession,
}

impl RepoPactCore {
    pub fn open(root: impl AsRef<Path>) -> Self {
        Self {
            session: Repository::open(root).session(),
        }
    }

    pub fn identity(&self) -> RepositoryIdentity {
        self.session.repository().identity()
    }

    pub fn snapshot(&self) -> RepositorySnapshot {
        self.session.snapshot()
    }

    pub fn validate(&self) -> ValidationReport {
        let snapshot = self.snapshot();
        repopact_validation::validate_snapshot(&snapshot)
    }

    pub fn graph(&self) -> RepositoryGraph {
        build(&self.snapshot())
    }

    pub fn analyze(&self, query: &AnalysisQuery) -> AnalysisReport {
        let snapshot = self.snapshot();
        analyze(&snapshot, query)
    }

    pub fn plan_mutation(&self, request: MutationRequest) -> MutationPlan {
        let snapshot = self.snapshot();
        plan(&snapshot, request)
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
