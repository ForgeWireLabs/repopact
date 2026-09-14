//! Session-scoped working-tree ROG overlay (WI063 ROG-013, Decision 0047).
//!
//! Conceptual model:
//!
//! ```text
//!                    durable ROG baseline
//!                           Gd
//!                           |
//!                           v
//!                  session graph state
//!                           |
//!         +-----------------+------------------+
//!         |                                    |
//!         v                                    v
//! RepositorySnapshot                    watcher change set
//! current governance                    changed paths
//! current topology                           |
//!         |                                   |
//!         +----------------+------------------+
//!                          |
//!                          v
//!                in-memory reconciliation
//!                          |
//!                          v
//!                     working overlay
//!                          Gw
//! ```
//!
//! No durable ROG files are changed by ordinary overlay reconciliation --
//! only an explicit `graph.update`/`graph.build` (Decision 0046) writes
//! `rog/`. This module owns the canonical, Rust-core overlay engine so
//! that `repopact-desktop-api` and any other embedder orchestrate it
//! rather than reimplementing graph semantics; the correctness machinery
//! itself (per-file contribution generation, delta reconciliation) is
//! reused directly from [`crate::semantic`] and [`crate::incremental`],
//! never duplicated.

use std::collections::BTreeMap;

use repopact_repository::RepositorySnapshot;
use repopact_types::PathState;
use serde::{Deserialize, Serialize};

use crate::durable;
use crate::incremental::{self, SemanticContribution};
use crate::projection::{ProjectedFile, SourceProjection};
use crate::semantic::{
    self, FileCoverage, FileCoverageEntry, ResourcePolicy, SkipReason, SourceLanguage,
};
use crate::validate;
use crate::RepositoryGraph;

/// Whether the currently effective in-memory graph is exactly what the
/// durable baseline on disk holds, or reflects working-tree state the
/// durable baseline does not yet have (Decision 0047 section 2). This is
/// deliberately orthogonal to [`GraphCoverageState`] (a working overlay
/// can itself have partial semantic coverage) and to [`DurableFreshness`]
/// (the durable baseline's own staleness, independent of what the
/// session currently holds in memory).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphBasis {
    Durable,
    WorkingOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphCoverageState {
    Complete,
    Partial,
}

/// The durable baseline's own freshness, independent of what the session
/// currently holds effective in memory. Deliberately a separate type from
/// [`crate::status::Freshness`] -- that type remains the durable CLI's
/// own model (`repopact graph status/build/verify`), unchanged by this
/// checkpoint; `repopact graph status` may legitimately report `stale`
/// while a session's effective basis is `working_overlay` at the same
/// moment. Both are true and neither contradicts the other (see Decision
/// 0047 section 5 / step 19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DurableFreshness {
    Absent,
    Fresh,
    Stale,
    Unsupported,
    Corrupt,
}

/// Typed disclosure of the session's effective graph state (WI063
/// ROG-010/013, step 17). No host absolute paths and no source bodies
/// ever appear here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveGraphStatus {
    pub basis: GraphBasis,
    pub durable_freshness: DurableFreshness,
    pub coverage: GraphCoverageState,
    pub baseline_fingerprint: Option<String>,
    pub effective_fingerprint: String,
    pub changed_path_count: usize,
    pub overlay_generation: u64,
}

/// Conservative constants governing when watcher-reported changes are
/// too uncertain to reconcile with a targeted per-path patch and must
/// instead trigger a full in-memory [`SessionGraphState::refresh`]
/// (Decision 0047 section on watcher uncertainty; WI063 step 12).
/// Deliberately centralized, not scattered per call site, and easy to
/// revisit once real measurement evidence (beyond this checkpoint's
/// engineering timings) suggests a different value.
pub const LARGE_BURST_FALLBACK_THRESHOLD: usize = 64;

/// Outcome of one reconciliation call ([`SessionGraphState::reconcile`]
/// or [`SessionGraphState::refresh`]), reporting exactly what changed
/// this call -- distinct from [`EffectiveGraphStatus`], which describes
/// the resulting steady state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReconcileOutcome {
    pub files_added: usize,
    pub files_modified: usize,
    pub files_deleted: usize,
    pub semantic_reparsed: usize,
    pub semantic_reused: usize,
    /// True only if anything in the effective graph actually changed
    /// (paths added/modified/deleted with a genuinely different digest).
    /// A duplicate/no-op watcher event, or a self-applied change already
    /// reflected in the overlay, reports `false` and does not bump
    /// `overlay_generation`.
    pub changed: bool,
    /// Set when this call fell back to a full in-memory reconcile because
    /// the reported changes were too ambiguous for a targeted patch
    /// (directory-level events, a burst above
    /// [`LARGE_BURST_FALLBACK_THRESHOLD`]).
    pub fell_back_to_full_reconcile: bool,
}

/// Canonical, Rust-core session working-tree overlay (Decision 0047).
/// Owns durable-baseline loading, the in-memory source inventory, cached
/// semantic contributions, the effective graph, and dirty/overlay state.
/// Never serialized into `rog/`; never a repository-local database --
/// this value disappears when the owning session/process ends (WI063
/// step 8).
pub struct SessionGraphState {
    baseline_manifest_fingerprint: Option<String>,
    baseline_inventory: BTreeMap<String, String>,
    durable_freshness: DurableFreshness,
    current_inventory: BTreeMap<String, String>,
    semantic_by_file: BTreeMap<String, SemanticContribution>,
    effective_graph: RepositoryGraph,
    effective_fingerprint: String,
    overlay_generation: u64,
}

fn projection_from_inventory(inventory: &BTreeMap<String, String>) -> SourceProjection {
    SourceProjection {
        files: inventory
            .iter()
            .map(|(path, digest)| ProjectedFile {
                relative_path: path.clone(),
                digest: digest.clone(),
            })
            .collect(),
    }
}

fn inventory_from_projection(projection: &SourceProjection) -> BTreeMap<String, String> {
    projection
        .files
        .iter()
        .map(|file| (file.relative_path.clone(), file.digest.clone()))
        .collect()
}

fn has_coverage_gap(coverage: &semantic::SemanticCoverage) -> bool {
    coverage.files_partial > 0 || coverage.files_failed > 0
}

fn coverage_state(coverage: &semantic::SemanticCoverage) -> GraphCoverageState {
    if has_coverage_gap(coverage) {
        GraphCoverageState::Partial
    } else {
        GraphCoverageState::Complete
    }
}

impl SessionGraphState {
    /// Open a session overlay for `snapshot`. Prefers loading the durable
    /// graph verbatim (no reparsing) when it structurally validates and
    /// its recorded fingerprint already matches current source; otherwise
    /// reconciles once, in memory, against whatever durable baseline
    /// exists (or performs a full in-memory build if none does). This
    /// pays for one full source-projection walk -- a one-time,
    /// session-open cost, not a per-watcher-event cost (WI063 step 7).
    pub fn open(snapshot: &RepositorySnapshot) -> Self {
        let repository = snapshot.repository();
        let root = repository.root();
        let topology = snapshot.topology();
        let projection = SourceProjection::build(repository, topology);
        let current_fingerprint = projection.fingerprint();
        let inventory = inventory_from_projection(&projection);

        let diagnostics = validate::validate_structure(root);
        let is_absent = diagnostics.iter().any(|d| d.code == "graph.absent");
        let is_unsupported = diagnostics
            .iter()
            .any(|d| d.code == "graph.schema-unsupported");
        let is_corrupt = !is_absent && !is_unsupported && !diagnostics.is_empty();

        if is_absent {
            return Self::full_build_in_memory(snapshot, inventory, DurableFreshness::Absent, None);
        }
        if is_unsupported {
            return Self::full_build_in_memory(
                snapshot,
                inventory,
                DurableFreshness::Unsupported,
                None,
            );
        }
        if is_corrupt {
            return Self::full_build_in_memory(
                snapshot,
                inventory,
                DurableFreshness::Corrupt,
                None,
            );
        }

        let manifest = match durable::read_manifest(root) {
            Ok(Some(manifest)) => manifest,
            _ => {
                return Self::full_build_in_memory(
                    snapshot,
                    inventory,
                    DurableFreshness::Absent,
                    None,
                )
            }
        };

        if manifest.source_projection_fingerprint == current_fingerprint {
            if let Ok(graph) = durable::load_graph(root, &manifest) {
                let semantic_by_file =
                    incremental::read_semantic_contributions_by_file(root, &manifest)
                        .unwrap_or_default();
                return Self {
                    baseline_manifest_fingerprint: Some(current_fingerprint.clone()),
                    baseline_inventory: inventory.clone(),
                    durable_freshness: DurableFreshness::Fresh,
                    current_inventory: inventory,
                    semantic_by_file,
                    effective_graph: graph,
                    effective_fingerprint: current_fingerprint,
                    overlay_generation: 0,
                };
            }
            // A structurally-valid manifest whose shards still fail to
            // load is a genuine corruption this checkpoint did not
            // predict; reconcile in memory rather than trust it further.
        }

        let previous_coverage = manifest.semantic_coverage.clone().unwrap_or_default();
        let known_semantic =
            incremental::read_semantic_contributions_by_file(root, &manifest).unwrap_or_default();
        let plan = incremental::plan_reconciliation(
            snapshot,
            &projection,
            &previous_coverage.per_file,
            &known_semantic,
        );
        Self {
            baseline_manifest_fingerprint: Some(manifest.source_projection_fingerprint.clone()),
            baseline_inventory: inventory.clone(),
            durable_freshness: DurableFreshness::Stale,
            current_inventory: inventory,
            semantic_by_file: plan.semantic_by_file,
            effective_graph: plan.graph,
            effective_fingerprint: current_fingerprint,
            overlay_generation: 1,
        }
    }

    fn full_build_in_memory(
        snapshot: &RepositorySnapshot,
        inventory: BTreeMap<String, String>,
        durable_freshness: DurableFreshness,
        baseline_manifest_fingerprint: Option<String>,
    ) -> Self {
        let projection = projection_from_inventory(&inventory);
        let plan = incremental::plan_reconciliation(snapshot, &projection, &[], &BTreeMap::new());
        let fingerprint = projection.fingerprint();
        Self {
            baseline_manifest_fingerprint,
            baseline_inventory: inventory.clone(),
            durable_freshness,
            current_inventory: inventory,
            semantic_by_file: plan.semantic_by_file,
            effective_graph: plan.graph,
            effective_fingerprint: fingerprint,
            overlay_generation: 1,
        }
    }

    /// Targeted, watcher-driven reconciliation (WI063 step 10/11): for
    /// each changed repository-relative path, re-stat/hash only that one
    /// path, update the in-memory inventory, and regenerate only that
    /// file's semantic contribution -- every other cached contribution is
    /// reused untouched. Never performs a full source-projection walk.
    /// Falls back to [`Self::refresh`] when the reported changes are too
    /// ambiguous to patch safely (a directory-shaped path, a symlink, or
    /// a burst above [`LARGE_BURST_FALLBACK_THRESHOLD`]) -- per Decision
    /// 0047, watcher events are hints, never infallible authority.
    pub fn reconcile(
        &mut self,
        snapshot: &RepositorySnapshot,
        changed_paths: &[String],
    ) -> ReconcileOutcome {
        if changed_paths.is_empty() {
            return ReconcileOutcome::default();
        }
        if changed_paths.len() > LARGE_BURST_FALLBACK_THRESHOLD {
            let mut outcome = self.refresh(snapshot);
            outcome.fell_back_to_full_reconcile = true;
            return outcome;
        }

        let repository = snapshot.repository();
        let root = repository.root();
        let mut deduped: Vec<&String> = changed_paths.iter().collect();
        deduped.sort();
        deduped.dedup();

        let policy = ResourcePolicy::default();
        let mut outcome = ReconcileOutcome::default();
        for path in deduped {
            let absolute = root.join(path);
            match std::fs::symlink_metadata(&absolute) {
                Err(_) => {
                    // Absent: a deletion, if we knew about it; otherwise
                    // nothing to do (never tracked, or already removed).
                    if self.current_inventory.remove(path).is_some() {
                        self.semantic_by_file.remove(path);
                        outcome.files_deleted += 1;
                        outcome.changed = true;
                    }
                }
                Ok(metadata) if metadata.is_dir() || metadata.file_type().is_symlink() => {
                    // Ambiguous: a directory-level event (create, remove,
                    // or rename of a whole subtree) or a symlink cannot be
                    // safely reduced to "reparse this one file." Fall
                    // back to a full in-memory reconcile rather than
                    // guess (WI063 step 12).
                    let mut fallback_outcome = self.refresh(snapshot);
                    fallback_outcome.fell_back_to_full_reconcile = true;
                    return fallback_outcome;
                }
                Ok(_) => {
                    let digest = match repository.path_state(&absolute) {
                        PathState::Present { digest, .. } => digest,
                        PathState::Absent => continue,
                    };
                    if self.current_inventory.get(path) == Some(&digest) {
                        // Duplicate/no-op watcher event for this path --
                        // do not reparse, do not touch overlay_generation.
                        continue;
                    }
                    let was_tracked = self.current_inventory.contains_key(path);
                    self.current_inventory.insert(path.clone(), digest.clone());
                    let contribution = if SourceLanguage::from_extension(path).is_some() {
                        semantic::build_file_contribution(repository, path, &digest, &policy)
                    } else {
                        semantic::FileContribution {
                            entry: FileCoverageEntry {
                                relative_path: path.clone(),
                                language: None,
                                coverage: FileCoverage::Skipped {
                                    reason: SkipReason::UnsupportedLanguage,
                                },
                                nodes_emitted: 0,
                                edges_emitted: 0,
                                source_digest: Some(digest.clone()),
                            },
                            nodes: Vec::new(),
                            edges: Vec::new(),
                        }
                    };
                    self.semantic_by_file.insert(
                        path.clone(),
                        (contribution.entry, contribution.nodes, contribution.edges),
                    );
                    if was_tracked {
                        outcome.files_modified += 1;
                    } else {
                        outcome.files_added += 1;
                    }
                    outcome.semantic_reparsed += 1;
                    outcome.changed = true;
                }
            }
        }

        if outcome.changed {
            self.rebuild_effective_graph(snapshot);
            self.overlay_generation += 1;
        }
        outcome.semantic_reused = self
            .semantic_by_file
            .len()
            .saturating_sub(outcome.semantic_reparsed);
        outcome
    }

    /// Explicit, correctness-recovery reconciliation (WI063 step 13): pays
    /// for one full source-projection walk (like [`Self::open`]) and
    /// diffs it against the overlay's own current in-memory inventory
    /// (not necessarily the durable baseline, which is untouched here),
    /// regenerating contributions only for paths whose digest actually
    /// changed. After this returns, the effective session graph truthfully
    /// represents the current filesystem state even if the durable
    /// baseline remains stale.
    pub fn refresh(&mut self, snapshot: &RepositorySnapshot) -> ReconcileOutcome {
        let repository = snapshot.repository();
        let topology = snapshot.topology();
        let projection = SourceProjection::build(repository, topology);

        let previous_per_file: Vec<FileCoverageEntry> = self
            .semantic_by_file
            .values()
            .map(|(entry, _, _)| entry.clone())
            .collect();
        let plan = incremental::plan_reconciliation(
            snapshot,
            &projection,
            &previous_per_file,
            &self.semantic_by_file,
        );
        let outcome = ReconcileOutcome {
            files_added: plan.counts.files_added,
            files_modified: plan.counts.files_modified,
            files_deleted: plan.counts.files_deleted,
            semantic_reparsed: plan.counts.semantic_reparsed,
            semantic_reused: plan.counts.semantic_reused,
            changed: plan.counts.files_added > 0
                || plan.counts.files_modified > 0
                || plan.counts.files_deleted > 0,
            fell_back_to_full_reconcile: false,
        };

        self.current_inventory = inventory_from_projection(&projection);
        self.semantic_by_file = plan.semantic_by_file;
        self.effective_graph = plan.graph;
        self.effective_fingerprint = projection.fingerprint();
        if outcome.changed {
            self.overlay_generation += 1;
        }
        outcome
    }

    fn rebuild_effective_graph(&mut self, snapshot: &RepositorySnapshot) {
        let projection = projection_from_inventory(&self.current_inventory);
        let mut graph =
            RepositoryGraph::build_governance_and_physical_with_projection(snapshot, &projection);
        for (nodes, edges) in self.semantic_by_file.values().map(|(_, n, e)| (n, e)) {
            for node in nodes {
                graph.node(node.clone());
            }
            for edge in edges {
                graph.edge(edge.clone());
            }
        }
        graph.edges.sort();
        graph.edges.dedup();
        self.effective_graph = graph;
        self.effective_fingerprint = projection.fingerprint();
    }

    pub fn effective_graph(&self) -> &RepositoryGraph {
        &self.effective_graph
    }

    /// Current aggregate semantic coverage, recomputed deterministically
    /// from the same per-file inventory every status/reconcile call reads
    /// (never incrementally bookkept -- Decision 0046 section 9 applies
    /// here too).
    pub fn coverage(&self) -> semantic::SemanticCoverage {
        let per_file: Vec<FileCoverageEntry> = self
            .current_inventory
            .keys()
            .filter_map(|path| {
                self.semantic_by_file
                    .get(path)
                    .map(|(entry, _, _)| entry.clone())
            })
            .collect();
        semantic::aggregate_coverage(per_file)
    }

    pub fn status(&self) -> EffectiveGraphStatus {
        let basis =
            if Some(&self.effective_fingerprint) == self.baseline_manifest_fingerprint.as_ref() {
                GraphBasis::Durable
            } else {
                GraphBasis::WorkingOverlay
            };
        let changed_path_count =
            symmetric_difference_count(&self.baseline_inventory, &self.current_inventory);
        EffectiveGraphStatus {
            basis,
            durable_freshness: self.durable_freshness,
            coverage: coverage_state(&self.coverage()),
            baseline_fingerprint: self.baseline_manifest_fingerprint.clone(),
            effective_fingerprint: self.effective_fingerprint.clone(),
            changed_path_count,
            overlay_generation: self.overlay_generation,
        }
    }

    /// Assemble the [`crate::query::GraphQueryContext`] the query kernel
    /// needs to disclose graph state for a query run against this
    /// session's effective graph (WI063 bounded-query checkpoint,
    /// Decision 0050 section 8). The in-memory effective graph is always
    /// built by the current build code -- never loaded from an old durable
    /// shard -- so it is always reported at the current schema major.
    pub fn query_context(
        &self,
        repository_root: &std::path::Path,
    ) -> crate::query::GraphQueryContext {
        let capability_state = crate::capability::current_state(repository_root)
            .unwrap_or(crate::capability::CapabilityState::LegacyAbsent);
        crate::query::GraphQueryContext {
            graph_schema_version: crate::durable::CURRENT_GRAPH_SCHEMA_VERSION,
            graph_fingerprint: self.effective_fingerprint.clone(),
            status: self.status(),
            semantic_coverage: self.coverage(),
            capability_state,
        }
    }
}

fn symmetric_difference_count(a: &BTreeMap<String, String>, b: &BTreeMap<String, String>) -> usize {
    let mut count = 0usize;
    for (path, digest) in a {
        if b.get(path) != Some(digest) {
            count += 1;
        }
    }
    for path in b.keys() {
        if !a.contains_key(path) {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use repopact_repository::Repository;

    use super::*;
    use crate::test_support::temp_root;

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    fn base_fixture(root: &Path) {
        write(root, "src/lib.rs", "pub fn hello() {}\n");
        write(root, "src/util.py", "def helper():\n    pass\n");
        write(root, "src/widget.ts", "export function widget() {}\n");
        write(root, "README.md", "# fixture\n");
    }

    fn snapshot_of(root: &Path) -> repopact_repository::RepositorySnapshot {
        Repository::open(root).session().snapshot()
    }

    fn all_shard_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
        let manifest = durable::read_manifest(root).unwrap().unwrap();
        let mut map = BTreeMap::new();
        for shard in &manifest.node_shards {
            map.insert(
                format!("nodes/{}", shard.shard),
                durable::shard_bytes(root, "nodes", &shard.shard).unwrap(),
            );
        }
        for shard in &manifest.edge_shards {
            map.insert(
                format!("edges/{}", shard.shard),
                durable::shard_bytes(root, "edges", &shard.shard).unwrap(),
            );
        }
        map
    }

    #[test]
    fn single_semantic_edit_reparses_one_file_and_becomes_working_overlay() {
        let root = temp_root("overlay-single-edit");
        base_fixture(&root);
        let snapshot0 = snapshot_of(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        let mut overlay = SessionGraphState::open(&snapshot0);

        write(
            &root,
            "src/lib.rs",
            "pub fn hello() {}\npub fn goodbye() {}\n",
        );
        let snapshot = snapshot_of(&root);
        let outcome = overlay.reconcile(&snapshot, &["src/lib.rs".to_owned()]);

        assert!(outcome.changed);
        assert_eq!(outcome.semantic_reparsed, 1);
        assert!(outcome.fell_back_to_full_reconcile.then_some(()).is_none());

        let status = overlay.status();
        assert_eq!(status.basis, GraphBasis::WorkingOverlay);
        assert_eq!(status.durable_freshness, DurableFreshness::Fresh);
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "goodbye"));
        // Unrelated contributions from other files must still be present.
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "helper"));
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "widget"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn overlay_operations_never_write_the_durable_graph() {
        let root = temp_root("overlay-no-durable-write");
        base_fixture(&root);
        crate::build_and_write(&snapshot_of(&root)).expect("baseline build");
        let before = all_shard_bytes(&root);
        let manifest_before = std::fs::read(durable::manifest_path(&root)).unwrap();

        let snapshot = snapshot_of(&root);
        let mut overlay = SessionGraphState::open(&snapshot);
        // A burst of many working-tree edits through the overlay only.
        for round in 0..5 {
            write(
                &root,
                "src/lib.rs",
                &format!("pub fn hello() {{}}\npub fn round_{round}() {{}}\n"),
            );
            overlay.reconcile(&snapshot, &["src/lib.rs".to_owned()]);
        }
        write(&root, "src/added.rs", "pub fn added() {}\n");
        overlay.reconcile(&snapshot, &["src/added.rs".to_owned()]);
        write(&root, "src/util.py", "");
        std::fs::remove_file(root.join("src/util.py")).unwrap();
        overlay.reconcile(&snapshot, &["src/util.py".to_owned()]);

        let after = all_shard_bytes(&root);
        let manifest_after = std::fs::read(durable::manifest_path(&root)).unwrap();
        assert_eq!(
            before, after,
            "no overlay operation may mutate any durable node/edge shard"
        );
        assert_eq!(
            manifest_before, manifest_after,
            "no overlay operation may mutate rog/manifest.json"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn only_touched_files_reparse_in_a_multi_file_burst() {
        let root = temp_root("overlay-multi-file-burst");
        base_fixture(&root);
        let snapshot0 = snapshot_of(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        let mut overlay = SessionGraphState::open(&snapshot0);

        write(
            &root,
            "src/lib.rs",
            "pub fn hello() {}\npub fn extra() {}\n",
        );
        write(
            &root,
            "src/util.py",
            "def helper():\n    pass\n\n\ndef extra():\n    pass\n",
        );
        let snapshot = snapshot_of(&root);
        let outcome = overlay.reconcile(
            &snapshot,
            &["src/lib.rs".to_owned(), "src/util.py".to_owned()],
        );
        assert_eq!(outcome.semantic_reparsed, 2);
        assert_eq!(outcome.files_modified, 2);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn addition_appears_and_deletion_disappears_from_the_overlay() {
        let root = temp_root("overlay-add-delete");
        base_fixture(&root);
        let snapshot0 = snapshot_of(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        let mut overlay = SessionGraphState::open(&snapshot0);

        write(&root, "src/added.rs", "pub fn added() {}\n");
        std::fs::remove_file(root.join("src/widget.ts")).unwrap();
        let snapshot = snapshot_of(&root);
        let outcome = overlay.reconcile(
            &snapshot,
            &["src/added.rs".to_owned(), "src/widget.ts".to_owned()],
        );
        assert_eq!(outcome.files_added, 1);
        assert_eq!(outcome.files_deleted, 1);
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "added"));
        assert!(!overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "widget"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rename_is_delete_plus_add_in_the_overlay() {
        let root = temp_root("overlay-rename");
        base_fixture(&root);
        let snapshot0 = snapshot_of(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        let mut overlay = SessionGraphState::open(&snapshot0);

        std::fs::rename(
            root.join("src/widget.ts"),
            root.join("src/renamed_widget.ts"),
        )
        .unwrap();
        let snapshot = snapshot_of(&root);
        let outcome = overlay.reconcile(
            &snapshot,
            &[
                "src/widget.ts".to_owned(),
                "src/renamed_widget.ts".to_owned(),
            ],
        );
        assert_eq!(outcome.files_added, 1);
        assert_eq!(outcome.files_deleted, 1);
        assert_eq!(outcome.files_modified, 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unsupported_file_change_updates_inventory_without_fabricating_semantics() {
        // `.md` is now a recognized metadata language -- use a genuinely
        // unrecognized extension for this fixture instead.
        let root = temp_root("overlay-unsupported");
        base_fixture(&root);
        let snapshot0 = snapshot_of(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        let mut overlay = SessionGraphState::open(&snapshot0);

        write(&root, "notes.xyz", "# fixture\nmore notes\n");
        let snapshot = snapshot_of(&root);
        let outcome = overlay.reconcile(&snapshot, &["notes.xyz".to_owned()]);
        assert!(outcome.changed);
        assert_eq!(
            outcome.semantic_reparsed, 1,
            "the file is reprocessed (classified) but never sent to an adapter"
        );
        let coverage = overlay.coverage();
        assert_eq!(
            coverage.files_skipped_unsupported_language, 2,
            "notes.xyz plus the .gitattributes the baseline build's enablement wrote \
             (Decision 0051 step 35), both unrecognized-extension skips"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parser_failure_becomes_partial_without_destroying_unrelated_contributions() {
        let root = temp_root("overlay-parser-failure");
        base_fixture(&root);
        crate::build_and_write(&snapshot_of(&root)).expect("baseline build");

        write(&root, "src/lib.rs", "fn f( { totally not valid rust\n");
        let snapshot = snapshot_of(&root);
        let mut overlay = SessionGraphState::open(&snapshot);
        overlay.reconcile(&snapshot, &["src/lib.rs".to_owned()]);

        let status = overlay.status();
        assert_eq!(status.basis, GraphBasis::WorkingOverlay);
        assert_eq!(status.coverage, GraphCoverageState::Partial);
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "helper"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovery_clears_partial_coverage() {
        let root = temp_root("overlay-recovery");
        base_fixture(&root);
        crate::build_and_write(&snapshot_of(&root)).expect("baseline build");

        write(&root, "src/lib.rs", "fn f( { totally not valid rust\n");
        let snapshot = snapshot_of(&root);
        let mut overlay = SessionGraphState::open(&snapshot);
        overlay.reconcile(&snapshot, &["src/lib.rs".to_owned()]);
        assert_eq!(overlay.status().coverage, GraphCoverageState::Partial);

        write(
            &root,
            "src/lib.rs",
            "pub fn hello() {}\npub fn fixed() {}\n",
        );
        overlay.reconcile(&snapshot, &["src/lib.rs".to_owned()]);
        assert_eq!(overlay.status().coverage, GraphCoverageState::Complete);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pre_existing_dirty_state_at_session_open_is_detected_not_silently_fresh() {
        let root = temp_root("overlay-preexisting-dirty");
        base_fixture(&root);
        crate::build_and_write(&snapshot_of(&root)).expect("baseline build");

        // Modify source with no watcher ever having observed it, then open
        // a brand new session (no reconcile() call happens at all).
        write(
            &root,
            "src/lib.rs",
            "pub fn hello() {}\npub fn added_offline() {}\n",
        );
        let snapshot = snapshot_of(&root);
        let overlay = SessionGraphState::open(&snapshot);

        let status = overlay.status();
        assert_eq!(status.durable_freshness, DurableFreshness::Stale);
        assert_eq!(status.basis, GraphBasis::WorkingOverlay);
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "added_offline"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn directory_level_watcher_event_falls_back_to_full_reconcile() {
        let root = temp_root("overlay-directory-event");
        base_fixture(&root);
        crate::build_and_write(&snapshot_of(&root)).expect("baseline build");

        std::fs::create_dir_all(root.join("src/newdir")).unwrap();
        write(&root, "src/newdir/inner.rs", "pub fn inner() {}\n");
        let snapshot = snapshot_of(&root);
        let mut overlay = SessionGraphState::open(&snapshot);
        // The watcher reports the directory path itself (ambiguous),
        // not the file inside it.
        let outcome = overlay.reconcile(&snapshot, &["src/newdir".to_owned()]);
        assert!(outcome.fell_back_to_full_reconcile);
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "inner"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_burst_above_the_threshold_falls_back_to_full_reconcile() {
        let root = temp_root("overlay-large-burst");
        base_fixture(&root);
        crate::build_and_write(&snapshot_of(&root)).expect("baseline build");
        write(
            &root,
            "src/lib.rs",
            "pub fn hello() {}\npub fn changed() {}\n",
        );

        let snapshot = snapshot_of(&root);
        let mut overlay = SessionGraphState::open(&snapshot);
        let noisy: Vec<String> = (0..(LARGE_BURST_FALLBACK_THRESHOLD + 1))
            .map(|index| format!("noise-{index}.txt"))
            .collect();
        let outcome = overlay.reconcile(&snapshot, &noisy);
        assert!(outcome.fell_back_to_full_reconcile);
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "changed"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_watcher_event_does_not_reparse_or_bump_generation() {
        let root = temp_root("overlay-duplicate-event");
        base_fixture(&root);
        crate::build_and_write(&snapshot_of(&root)).expect("baseline build");
        let snapshot = snapshot_of(&root);
        let mut overlay = SessionGraphState::open(&snapshot);
        let generation_before = overlay.status().overlay_generation;

        // No actual filesystem change; the watcher reports the path
        // anyway (a spurious/duplicate event).
        let outcome = overlay.reconcile(&snapshot, &["src/lib.rs".to_owned()]);
        assert!(!outcome.changed);
        assert_eq!(outcome.semantic_reparsed, 0);
        assert_eq!(overlay.status().overlay_generation, generation_before);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_refresh_reconciles_paths_the_watcher_never_reported() {
        let root = temp_root("overlay-explicit-refresh");
        base_fixture(&root);
        crate::build_and_write(&snapshot_of(&root)).expect("baseline build");
        let snapshot = snapshot_of(&root);
        let mut overlay = SessionGraphState::open(&snapshot);

        // Simulate a missed/uncertain watcher window: source changes but
        // reconcile() is never called for it.
        write(
            &root,
            "src/lib.rs",
            "pub fn hello() {}\npub fn missed() {}\n",
        );
        let outcome = overlay.refresh(&snapshot);
        assert!(outcome.changed);
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "missed"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hundred_file_overlay_burst_reparses_only_the_changed_file() {
        let root = temp_root("overlay-hundred-files");
        for index in 0..100 {
            write(
                &root,
                &format!("src/file_{index:03}.rs"),
                &format!("pub fn function_{index:03}() {{}}\n"),
            );
        }
        let snapshot0 = snapshot_of(&root);
        crate::build_and_write(&snapshot0).expect("baseline build of 100 files");
        let mut overlay = SessionGraphState::open(&snapshot0);

        write(
            &root,
            "src/file_042.rs",
            "pub fn function_042_changed() {}\n",
        );
        let snapshot = snapshot_of(&root);
        let outcome = overlay.reconcile(&snapshot, &["src/file_042.rs".to_owned()]);
        assert_eq!(outcome.semantic_reparsed, 1);
        assert_eq!(
            outcome.semantic_reused, 100,
            "99 unchanged fixture files plus the .gitattributes the baseline build's \
             enablement wrote (Decision 0051 step 35)"
        );

        // A second, no-op reconcile: zero reparses, zero durable writes
        // (implicitly -- overlay never writes durable at all).
        let noop = overlay.reconcile(&snapshot, &["src/file_042.rs".to_owned()]);
        assert!(!noop.changed);
        assert_eq!(noop.semantic_reparsed, 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn metadata_mutation_updates_the_graph_and_reuses_unrelated_source_contributions() {
        // WI063 metadata/operational-topology checkpoint, step 34: a
        // manifest edit through the session overlay must update the
        // in-memory graph, regenerate only that manifest's own
        // contribution, reuse unrelated semantic source contributions
        // untouched, leave rog/ unwritten, and disclose working-overlay
        // basis -- exactly the same properties already proven for a
        // source-language edit.
        let root = temp_root("overlay-metadata-mutation");
        base_fixture(&root);
        write(&root, "Cargo.toml", "[package]\nname = \"fixture\"\n");
        let snapshot0 = snapshot_of(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        let mut overlay = SessionGraphState::open(&snapshot0);
        let before_shards = all_shard_bytes(&root);

        write(
            &root,
            "Cargo.toml",
            "[package]\nname = \"fixture\"\n\n[dependencies]\nserde = \"1.0\"\n",
        );
        let snapshot = snapshot_of(&root);
        let outcome = overlay.reconcile(&snapshot, &["Cargo.toml".to_owned()]);

        assert!(outcome.changed);
        assert_eq!(
            outcome.semantic_reparsed, 1,
            "only Cargo.toml's own contribution should be regenerated"
        );
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "serde"));
        // Unrelated source contributions must still be present.
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "helper"));
        assert!(overlay
            .effective_graph()
            .nodes
            .values()
            .any(|node| node.label == "widget"));

        let status = overlay.status();
        assert_eq!(status.basis, GraphBasis::WorkingOverlay);

        let after_shards = all_shard_bytes(&root);
        assert_eq!(
            before_shards, after_shards,
            "an overlay reconcile over a metadata file must never write rog/"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// WI063 operational-surface-completion checkpoint (ROG-013 for the
    /// new operational mutation classes): `refresh` runs a full
    /// `plan_reconciliation`, which now includes
    /// `semantic::operational::apply` -- an operational role fact added
    /// on disk must become visible in the effective (working-overlay)
    /// graph via `refresh`, never requiring a durable `rog/` write.
    #[test]
    fn refresh_surfaces_a_new_operational_role_fact_without_a_durable_write() {
        let root = temp_root("overlay-operational-refresh");
        base_fixture(&root);
        write(&root, "Cargo.toml", "[package]\nname = \"fixture\"\n");
        write(&root, "src/main.rs", "fn main() {}\n");
        let snapshot0 = snapshot_of(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        let mut overlay = SessionGraphState::open(&snapshot0);
        let before_shards = all_shard_bytes(&root);

        // src/main.rs already existed at open() time; the implicit-binary
        // pass should already have tagged it. Now genuinely mutate: add a
        // sibling test target under tests/, an operational fact that
        // requires the orchestrator's cross-file pass, not a per-file
        // adapter alone.
        write(&root, "tests/integration.rs", "#[test]\nfn it_works() {}\n");
        let snapshot = snapshot_of(&root);
        let outcome = overlay.refresh(&snapshot);

        assert!(outcome.changed);
        let test_node = overlay
            .effective_graph()
            .nodes
            .get("file:tests/integration.rs")
            .expect("tests/integration.rs should be a physical node after refresh");
        assert_eq!(
            test_node
                .node_role
                .as_ref()
                .map(crate::GraphNodeRole::as_str),
            Some(crate::roles::TEST_TARGET),
            "refresh must run the orchestrator's Cargo integration-test tagging pass"
        );

        let after_shards = all_shard_bytes(&root);
        assert_eq!(
            before_shards, after_shards,
            "refresh must never write the durable rog/ graph"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Documents the disclosed, bounded scope of the targeted watcher fast
    /// path (`reconcile`): unlike `refresh`/`open`, it never re-runs
    /// `semantic::operational::apply` (module-level doc comment on
    /// `semantic::operational`) -- an operational cross-file fact from a
    /// single-path reconcile becomes visible on the next full reconcile,
    /// not immediately. This is a deliberate, bounded limitation, not a
    /// silent gap: this test locks in that exact boundary so a future
    /// change to `reconcile`'s scope must consciously update it.
    #[test]
    fn targeted_reconcile_does_not_yet_surface_a_new_operational_role_fact() {
        let root = temp_root("overlay-operational-reconcile-boundary");
        base_fixture(&root);
        write(&root, "Cargo.toml", "[package]\nname = \"fixture\"\n");
        let snapshot0 = snapshot_of(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        let mut overlay = SessionGraphState::open(&snapshot0);

        write(&root, "tests/integration.rs", "#[test]\nfn it_works() {}\n");
        let snapshot = snapshot_of(&root);
        let outcome = overlay.reconcile(&snapshot, &["tests/integration.rs".to_owned()]);

        assert!(outcome.changed);
        let test_node = overlay
            .effective_graph()
            .nodes
            .get("file:tests/integration.rs")
            .expect("the file node itself must still exist");
        assert_eq!(
            test_node.node_role, None,
            "a targeted reconcile does not re-run the orchestrator's \
             cross-file operational passes -- this is the disclosed \
             boundary, not a regression"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
