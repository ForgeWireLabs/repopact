//! Incremental ROG convergence (WI063 incremental-equivalence checkpoint,
//! ROG-012, Decision 0046). The full semantic build
//! ([`crate::RepositoryGraph::build_with_fingerprint`]) is the
//! correctness oracle; everything here is an optimization that must
//! converge to the exact same canonical output, never a second source of
//! truth. Governance and physical topology are always rebuilt globally on
//! every call (Decision 0046 -- both are deterministic and cheap enough
//! that fine-grained incremental tracking would add risk without a real
//! benefit); only semantic extraction is contribution-incremental.

use std::collections::BTreeMap;

use repopact_repository::RepositorySnapshot;
use serde::{Deserialize, Serialize};

use crate::durable::{self, DurableError, SUPPORTED_GRAPH_SCHEMA_VERSIONS};
use crate::projection::SourceProjection;
use crate::semantic::{self, FileCoverageEntry, ResourcePolicy};
use crate::status::{self, Freshness};
use crate::validate;
use crate::RepositoryGraph;

/// Semantic-pipeline version: bump only when adapter output for the same
/// bytes would legitimately change (a new symbol/relation category, a
/// changed stable-ID rule, a changed AST-walk policy). Unrelated crate
/// refactors do not need to bump this.
pub const CURRENT_SEMANTIC_PIPELINE_VERSION: &str = "semantic-pipeline-1";
/// Resource-policy version: bump when [`ResourcePolicy::default`]'s
/// constants change in a way that could change which files are skipped
/// vs. parsed for the same bytes.
pub const CURRENT_RESOURCE_POLICY_VERSION: &str = "resource-policy-1";

/// What must match, bit-for-bit, between a durable graph's baseline and
/// the current implementation before any semantic contribution may be
/// reused unparsed (WI063 incremental-equivalence checkpoint, step 4;
/// Decision 0046). Git diff, mtime, watcher events, and parser-internal
/// caches are never part of this identity and never substitute for it --
/// correctness rests only on repository-relative path + content digest
/// (already covered by [`SourceProjection`]/`FileCoverageEntry`) plus
/// this pipeline-compatibility identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticCompatibility {
    pub graph_schema_major: u32,
    pub pipeline_version: String,
    pub adapter_versions: BTreeMap<String, String>,
    pub resource_policy_version: String,
}

/// The compatibility identity this build of the engine actually produces.
/// An incremental update may reuse a prior file's semantic contribution
/// only when the durable baseline's recorded identity equals this one
/// exactly.
pub fn current_semantic_compatibility() -> SemanticCompatibility {
    let (adapter_versions, _relations_supported) = semantic::adapter_metadata();
    SemanticCompatibility {
        graph_schema_major: durable::CURRENT_GRAPH_SCHEMA_VERSION,
        pipeline_version: CURRENT_SEMANTIC_PIPELINE_VERSION.to_owned(),
        adapter_versions,
        resource_policy_version: CURRENT_RESOURCE_POLICY_VERSION.to_owned(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateMode {
    /// The durable graph was already an exact match for the current
    /// source; nothing was reparsed and `rog/` was not rewritten.
    NoOp,
    /// Semantic contributions were reused for unchanged files and
    /// regenerated only for added/modified files; governance and physical
    /// topology were rebuilt globally as always.
    Incremental,
    /// A full rebuild ran instead of an incremental update, for one of
    /// the reasons named in `fallback_reason`. Never hidden behind the
    /// `incremental` label (WI063 incremental-equivalence checkpoint,
    /// step 8).
    FullFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    pub mode: UpdateMode,
    pub baseline_fingerprint: Option<String>,
    pub current_fingerprint: String,
    pub files_added: usize,
    pub files_modified: usize,
    pub files_deleted: usize,
    pub files_unchanged: usize,
    pub semantic_reparsed: usize,
    pub semantic_reused: usize,
    pub semantic_skipped: usize,
    pub fallback_reason: Option<String>,
    pub final_node_count: usize,
    pub final_edge_count: usize,
    pub freshness: Freshness,
}

/// Classification of one repository-relative path between a durable
/// baseline's per-file inventory and the current `SourceProjection`
/// (WI063 incremental-equivalence checkpoint, step 7). A rename/move is
/// deliberately not a distinct class here: it is a delete at the old path
/// plus an add at the new path, with no fabricated persistent object
/// identity carried across the two (Decision 0046) -- Git rename
/// detection is never consulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileDelta {
    Unchanged,
    Added,
    Modified,
    Deleted,
}

fn compute_delta(
    previous_per_file: &[FileCoverageEntry],
    current_files: &SourceProjection,
) -> BTreeMap<String, FileDelta> {
    let mut previous_digest: BTreeMap<&str, &str> = BTreeMap::new();
    for entry in previous_per_file {
        if let Some(digest) = entry.source_digest.as_deref() {
            previous_digest.insert(entry.relative_path.as_str(), digest);
        }
    }
    let mut current_digest: BTreeMap<&str, &str> = BTreeMap::new();
    for file in &current_files.files {
        current_digest.insert(file.relative_path.as_str(), file.digest.as_str());
    }

    let mut delta = BTreeMap::new();
    for (path, digest) in &current_digest {
        match previous_digest.get(path) {
            None => {
                delta.insert((*path).to_owned(), FileDelta::Added);
            }
            Some(previous) if previous == digest => {
                delta.insert((*path).to_owned(), FileDelta::Unchanged);
            }
            Some(_) => {
                delta.insert((*path).to_owned(), FileDelta::Modified);
            }
        }
    }
    for path in previous_digest.keys() {
        if !current_digest.contains_key(path) {
            delta.insert((*path).to_owned(), FileDelta::Deleted);
        }
    }
    delta
}

/// Update the durable graph at `snapshot.repository().root()`, reusing
/// unchanged semantic contributions where safe and falling back to a full
/// rebuild whenever reuse cannot be trusted (WI063 incremental-equivalence
/// checkpoint). Never treats a structurally corrupt or schema-unsupported
/// baseline as reusable (Decision 0046): an unsupported schema major
/// fails closed; a corrupt-but-supported baseline triggers an explicit
/// full rebuild rather than reusing any of its contents.
pub fn update(snapshot: &RepositorySnapshot) -> Result<UpdateResult, DurableError> {
    let repository = snapshot.repository();
    let root = repository.root();

    let diagnostics = validate::validate_structure(root);
    let is_absent = diagnostics.iter().any(|d| d.code == "graph.absent");
    let is_unsupported = diagnostics
        .iter()
        .any(|d| d.code == "graph.schema-unsupported");
    let is_corrupt = !is_absent && !is_unsupported && !diagnostics.is_empty();

    if is_unsupported {
        return Err(DurableError {
            code: "graph.schema-unsupported".to_owned(),
            message: format!(
                "durable graph declares a schema version outside {SUPPORTED_GRAPH_SCHEMA_VERSIONS:?}; \
                 failing closed rather than updating a version this implementation does not understand"
            ),
        });
    }

    if is_absent {
        return full_rebuild(snapshot, None, "graph_absent");
    }
    if is_corrupt {
        return full_rebuild(snapshot, None, "corrupt_baseline_full_rebuild");
    }

    let previous_manifest = match durable::read_manifest(root)? {
        Some(manifest) => manifest,
        None => return full_rebuild(snapshot, None, "graph_absent"),
    };

    if previous_manifest.graph_schema_version == 1 {
        return full_rebuild(
            snapshot,
            Some(previous_manifest.source_projection_fingerprint.clone()),
            "schema_v1_upgrade",
        );
    }

    let baseline_fingerprint = Some(previous_manifest.source_projection_fingerprint.clone());
    let Some(previous_compatibility) = previous_manifest.semantic_compatibility.clone() else {
        return full_rebuild(
            snapshot,
            baseline_fingerprint,
            "incremental_metadata_absent",
        );
    };
    if previous_compatibility != current_semantic_compatibility() {
        return full_rebuild(
            snapshot,
            baseline_fingerprint,
            "semantic_compatibility_mismatch",
        );
    }

    let topology = snapshot.topology();
    let projection = SourceProjection::build(repository, topology);
    let current_fingerprint = projection.fingerprint();

    if current_fingerprint == previous_manifest.source_projection_fingerprint {
        let coverage = previous_manifest
            .semantic_coverage
            .clone()
            .unwrap_or_default();
        let semantic_skipped =
            coverage.files_skipped_unsupported_language + coverage.files_skipped_policy;
        let freshness = if status::has_semantic_coverage_gap(&previous_manifest) {
            Freshness::Partial
        } else {
            Freshness::Fresh
        };
        return Ok(UpdateResult {
            mode: UpdateMode::NoOp,
            baseline_fingerprint,
            current_fingerprint,
            files_added: 0,
            files_modified: 0,
            files_deleted: 0,
            files_unchanged: coverage.files_considered,
            semantic_reparsed: 0,
            semantic_reused: coverage.files_considered,
            semantic_skipped,
            fallback_reason: None,
            final_node_count: previous_manifest.node_count,
            final_edge_count: previous_manifest.edge_count,
            freshness,
        });
    }

    incremental_update(
        snapshot,
        &projection,
        current_fingerprint,
        baseline_fingerprint,
        previous_manifest,
    )
}

fn full_rebuild(
    snapshot: &RepositorySnapshot,
    baseline_fingerprint: Option<String>,
    reason: &str,
) -> Result<UpdateResult, DurableError> {
    let (graph, fingerprint, semantic_coverage) = RepositoryGraph::build_with_fingerprint(snapshot);
    let manifest = durable::write(
        snapshot.repository().root(),
        &graph,
        &fingerprint,
        semantic_coverage.clone(),
        current_semantic_compatibility(),
    )?;
    let freshness = if status::has_semantic_coverage_gap(&manifest) {
        Freshness::Partial
    } else {
        Freshness::Fresh
    };
    let semantic_reparsed = semantic_coverage.files_complete
        + semantic_coverage.files_partial
        + semantic_coverage.files_failed;
    let semantic_skipped = semantic_coverage.files_skipped_unsupported_language
        + semantic_coverage.files_skipped_policy;
    Ok(UpdateResult {
        mode: UpdateMode::FullFallback,
        baseline_fingerprint,
        current_fingerprint: fingerprint,
        files_added: semantic_coverage.files_considered,
        files_modified: 0,
        files_deleted: 0,
        files_unchanged: 0,
        semantic_reparsed,
        semantic_reused: 0,
        semantic_skipped,
        fallback_reason: Some(reason.to_owned()),
        final_node_count: manifest.node_count,
        final_edge_count: manifest.edge_count,
        freshness,
    })
}

fn incremental_update(
    snapshot: &RepositorySnapshot,
    projection: &SourceProjection,
    current_fingerprint: String,
    baseline_fingerprint: Option<String>,
    previous_manifest: durable::Manifest,
) -> Result<UpdateResult, DurableError> {
    let repository = snapshot.repository();
    let root = repository.root();
    let previous_coverage = previous_manifest
        .semantic_coverage
        .clone()
        .unwrap_or_default();
    let delta = compute_delta(&previous_coverage.per_file, projection);

    // Load the previous durable graph's semantic nodes/edges, grouped by
    // owning file. Ownership is read directly from the existing
    // `source.path` every semantic node/edge already carries (every
    // adapter attributes its output to exactly the file it parsed) --
    // no separate contribution-owner field is needed because current
    // semantic relations (`defines`, `imports`) are file-local by
    // construction (WI063 incremental-equivalence checkpoint, step 6).
    let previous_semantic = read_semantic_contributions_by_file(root, &previous_manifest)?;

    let (mut graph, _fresh_projection) = RepositoryGraph::build_governance_and_physical(snapshot);

    let policy = ResourcePolicy::default();
    let mut per_file = Vec::with_capacity(projection.files.len());
    let mut files_added = 0usize;
    let mut files_modified = 0usize;
    let mut files_unchanged = 0usize;
    let mut semantic_reparsed = 0usize;
    let mut semantic_reused = 0usize;

    for file in &projection.files {
        match delta.get(file.relative_path.as_str()) {
            Some(FileDelta::Unchanged) => {
                files_unchanged += 1;
                semantic_reused += 1;
                if let Some((entry, nodes, edges)) = previous_semantic.get(&file.relative_path) {
                    for node in nodes {
                        graph.node(node.clone());
                    }
                    for edge in edges {
                        graph.edge(edge.clone());
                    }
                    per_file.push(entry.clone());
                } else {
                    // The previous manifest recorded this file but it
                    // carried no semantic node/edge presence (e.g.
                    // unsupported language/skipped/failed with nothing
                    // emitted) -- recover its coverage entry from the
                    // previous per-file record directly rather than
                    // reparsing, since the digest is unchanged and there
                    // is nothing to reuse besides the entry itself.
                    if let Some(previous_entry) = previous_coverage
                        .per_file
                        .iter()
                        .find(|entry| entry.relative_path == file.relative_path)
                    {
                        per_file.push(previous_entry.clone());
                    } else {
                        // Should not happen (delta is computed from this
                        // same previous inventory), but never fabricate a
                        // reused entry -- reparse rather than guess.
                        let contribution = semantic::build_file_contribution(
                            repository,
                            &file.relative_path,
                            &file.digest,
                            &policy,
                        );
                        for node in contribution.nodes {
                            graph.node(node);
                        }
                        for edge in contribution.edges {
                            graph.edge(edge);
                        }
                        per_file.push(contribution.entry);
                        semantic_reused -= 1;
                        semantic_reparsed += 1;
                    }
                }
            }
            Some(FileDelta::Added) => {
                files_added += 1;
                semantic_reparsed += 1;
                let contribution = semantic::build_file_contribution(
                    repository,
                    &file.relative_path,
                    &file.digest,
                    &policy,
                );
                for node in contribution.nodes {
                    graph.node(node);
                }
                for edge in contribution.edges {
                    graph.edge(edge);
                }
                per_file.push(contribution.entry);
            }
            Some(FileDelta::Modified) => {
                files_modified += 1;
                semantic_reparsed += 1;
                // Old contribution for this path is simply never read
                // back in (dropped), never merged with the new one.
                let contribution = semantic::build_file_contribution(
                    repository,
                    &file.relative_path,
                    &file.digest,
                    &policy,
                );
                for node in contribution.nodes {
                    graph.node(node);
                }
                for edge in contribution.edges {
                    graph.edge(edge);
                }
                per_file.push(contribution.entry);
            }
            Some(FileDelta::Deleted) | None => {
                // `None` cannot occur for a file in `projection.files`
                // (delta covers every current file); defensive only.
            }
        }
    }
    let files_deleted = delta.values().filter(|d| **d == FileDelta::Deleted).count();

    graph.edges.sort();
    graph.edges.dedup();

    let semantic_coverage = semantic::aggregate_coverage(per_file);
    let semantic_skipped = semantic_coverage.files_skipped_unsupported_language
        + semantic_coverage.files_skipped_policy;

    let manifest = durable::write(
        root,
        &graph,
        &current_fingerprint,
        semantic_coverage,
        current_semantic_compatibility(),
    )?;
    let freshness = if status::has_semantic_coverage_gap(&manifest) {
        Freshness::Partial
    } else {
        Freshness::Fresh
    };

    Ok(UpdateResult {
        mode: UpdateMode::Incremental,
        baseline_fingerprint,
        current_fingerprint,
        files_added,
        files_modified,
        files_deleted,
        files_unchanged,
        semantic_reparsed,
        semantic_reused,
        semantic_skipped,
        fallback_reason: None,
        final_node_count: manifest.node_count,
        final_edge_count: manifest.edge_count,
        freshness,
    })
}

type SemanticContribution = (
    FileCoverageEntry,
    Vec<crate::GraphNode>,
    Vec<crate::GraphEdge>,
);

/// Read every semantic node/edge out of the previous durable graph's
/// shards, grouped by the repository-relative file each one is attributed
/// to via its existing `source.path`. This is a durable-graph read, not a
/// repository walk or Git call -- it costs disk I/O proportional to the
/// previous graph's size, not to the number of files being updated.
fn read_semantic_contributions_by_file(
    root: &std::path::Path,
    manifest: &durable::Manifest,
) -> Result<BTreeMap<String, SemanticContribution>, DurableError> {
    let mut by_file: BTreeMap<String, (Vec<crate::GraphNode>, Vec<crate::GraphEdge>)> =
        BTreeMap::new();

    for shard in &manifest.node_shards {
        for node in durable::read_node_shard(root, &shard.shard)? {
            if node.layer != crate::GraphLayer::Semantic {
                continue;
            }
            if let Some(source) = &node.source {
                by_file.entry(source.path.clone()).or_default().0.push(node);
            }
        }
    }
    for shard in &manifest.edge_shards {
        for edge in durable::read_edge_shard(root, &shard.shard)? {
            if edge.layer != crate::GraphLayer::Semantic {
                continue;
            }
            by_file
                .entry(edge.source.path.clone())
                .or_default()
                .1
                .push(edge);
        }
    }

    let coverage = manifest.semantic_coverage.clone().unwrap_or_default();
    let mut result = BTreeMap::new();
    for entry in coverage.per_file {
        let (nodes, edges) = by_file.remove(&entry.relative_path).unwrap_or_default();
        result.insert(entry.relative_path.clone(), (entry, nodes, edges));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use repopact_repository::Repository;

    use super::*;
    use crate::test_support::temp_root;

    fn open_snapshot(root: &Path) -> RepositorySnapshot {
        Repository::open(root).session().snapshot()
    }

    fn read_all_shard_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
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

    /// The ROG-012 equivalence assertion: after canonical normalization,
    /// the graph at `incremental_root` (produced by an incremental
    /// update) must be byte-identical to the graph at `clean_root`
    /// (produced by a single clean full build over the same final
    /// source). Comparing manifest fields alone is not enough (matching
    /// counts could hide different content); every shard's bytes are
    /// compared too.
    fn assert_canonically_equal(incremental_root: &Path, clean_root: &Path) {
        let incremental_manifest = durable::read_manifest(incremental_root).unwrap().unwrap();
        let clean_manifest = durable::read_manifest(clean_root).unwrap().unwrap();
        assert_eq!(
            incremental_manifest.graph_schema_version,
            clean_manifest.graph_schema_version
        );
        assert_eq!(
            incremental_manifest.source_projection_fingerprint,
            clean_manifest.source_projection_fingerprint
        );
        assert_eq!(incremental_manifest.node_count, clean_manifest.node_count);
        assert_eq!(incremental_manifest.edge_count, clean_manifest.edge_count);
        assert_eq!(incremental_manifest.coverage, clean_manifest.coverage);
        assert_eq!(
            incremental_manifest.semantic_coverage, clean_manifest.semantic_coverage,
            "semantic coverage (including per-file inventory) must match exactly"
        );
        assert_eq!(
            incremental_manifest.node_shards, clean_manifest.node_shards,
            "node shard hash/count manifest entries must match exactly"
        );
        assert_eq!(
            incremental_manifest.edge_shards, clean_manifest.edge_shards,
            "edge shard hash/count manifest entries must match exactly"
        );
        assert_eq!(
            read_all_shard_bytes(incremental_root),
            read_all_shard_bytes(clean_root),
            "every node/edge shard must be byte-identical"
        );
    }

    /// Build `root` at s0 via `setup`, snapshot+build the durable graph,
    /// apply `mutate` to reach s1, then run `graph.update` in place.
    /// Independently, build a second fixture straight to s1 (`setup` then
    /// `mutate`, never touching s0) and take one clean full build. Assert
    /// the incremental candidate converges to the clean one.
    fn run_equivalence_case(
        name: &str,
        setup: impl Fn(&Path),
        mutate: impl Fn(&Path),
    ) -> UpdateResult {
        let incremental_root = temp_root(&format!("incr-{name}"));
        setup(&incremental_root);
        let snapshot0 = open_snapshot(&incremental_root);
        crate::build_and_write(&snapshot0).expect("baseline s0 build");
        mutate(&incremental_root);
        let snapshot1 = open_snapshot(&incremental_root);
        let result = update(&snapshot1).expect("incremental update to s1");

        let clean_root = temp_root(&format!("clean-{name}"));
        setup(&clean_root);
        mutate(&clean_root);
        let clean_snapshot = open_snapshot(&clean_root);
        crate::build_and_write(&clean_snapshot).expect("clean full build of s1");

        assert_canonically_equal(&incremental_root, &clean_root);

        std::fs::remove_dir_all(&incremental_root).unwrap();
        std::fs::remove_dir_all(&clean_root).unwrap();
        result
    }

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    fn remove(root: &Path, relative: &str) {
        std::fs::remove_file(root.join(relative)).unwrap();
    }

    fn base_fixture(root: &Path) {
        write(root, "Cargo.toml", "[package]\nname = \"fixture\"\n");
        write(root, "src/lib.rs", "pub fn hello() {}\n");
        write(root, "src/util.py", "def helper():\n    pass\n");
        write(root, "src/widget.ts", "export function widget() {}\n");
        write(root, "README.md", "# fixture\n");
    }

    #[test]
    fn addition_is_equivalent_to_full_rebuild() {
        let result = run_equivalence_case("addition", base_fixture, |root| {
            write(root, "src/added.rs", "pub struct Added;\n");
            write(root, "src/added.py", "class Added:\n    pass\n");
            write(root, "src/added.ts", "export interface Added {}\n");
        });
        assert_eq!(result.mode, UpdateMode::Incremental);
        assert_eq!(result.files_added, 3);
        assert_eq!(result.files_modified, 0);
        assert_eq!(result.files_deleted, 0);
    }

    #[test]
    fn declaration_edit_is_equivalent_to_full_rebuild() {
        let result = run_equivalence_case("edit", base_fixture, |root| {
            write(
                root,
                "src/lib.rs",
                "pub fn hello() {}\npub fn goodbye() {}\n",
            );
        });
        assert_eq!(result.mode, UpdateMode::Incremental);
        assert_eq!(result.files_modified, 1);
        assert_eq!(result.semantic_reparsed, 1);
    }

    #[test]
    fn nonsemantic_edit_reparses_but_keeps_stable_symbol_ids() {
        let root = temp_root("incr-nonsemantic");
        base_fixture(&root);
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        let before = durable::read_manifest(&root).unwrap().unwrap();
        let before_symbol_ids: std::collections::BTreeSet<String> = before
            .node_shards
            .iter()
            .flat_map(|shard| durable::read_node_shard(&root, &shard.shard).unwrap())
            .filter(|node| node.kind == crate::GraphNodeKind::Symbol)
            .map(|node| node.id)
            .collect();

        write(
            &root,
            "src/lib.rs",
            "// a leading comment\n\npub fn hello() {}\n",
        );
        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("incremental update");
        assert_eq!(result.mode, UpdateMode::Incremental);
        assert_eq!(
            result.files_modified, 1,
            "the file's digest changed, so it must be classified modified and reparsed"
        );
        assert_eq!(result.semantic_reparsed, 1);

        let after = durable::read_manifest(&root).unwrap().unwrap();
        let after_symbol_ids: std::collections::BTreeSet<String> = after
            .node_shards
            .iter()
            .flat_map(|shard| durable::read_node_shard(&root, &shard.shard).unwrap())
            .filter(|node| node.kind == crate::GraphNodeKind::Symbol)
            .map(|node| node.id)
            .collect();
        assert_eq!(
            before_symbol_ids, after_symbol_ids,
            "a leading comment must not change any symbol's stable identity"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delete_is_equivalent_to_full_rebuild_and_removes_symbols() {
        let result = run_equivalence_case(
            "delete",
            |root| {
                base_fixture(root);
                write(root, "src/doomed.rs", "pub fn doomed() {}\n");
            },
            |root| remove(root, "src/doomed.rs"),
        );
        assert_eq!(result.mode, UpdateMode::Incremental);
        assert_eq!(result.files_deleted, 1);
    }

    #[test]
    fn delete_removes_the_files_symbols_and_edges_from_the_durable_graph() {
        let root = temp_root("incr-delete-content");
        base_fixture(&root);
        write(&root, "src/doomed.rs", "pub fn doomed() {}\n");
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        remove(&root, "src/doomed.rs");
        let snapshot1 = open_snapshot(&root);
        update(&snapshot1).expect("incremental update");

        let manifest = durable::read_manifest(&root).unwrap().unwrap();
        let any_doomed = manifest.node_shards.iter().any(|shard| {
            durable::read_node_shard(&root, &shard.shard)
                .unwrap()
                .iter()
                .any(|node| node.label == "doomed")
        });
        assert!(!any_doomed, "a deleted file's symbols must not remain");
        assert!(manifest
            .semantic_coverage
            .unwrap()
            .per_file
            .iter()
            .all(|entry| entry.relative_path != "src/doomed.rs"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rename_is_delete_plus_add_with_no_fabricated_identity() {
        let result = run_equivalence_case(
            "rename",
            |root| {
                base_fixture(root);
                write(root, "src/old_name.rs", "pub fn renamed_fn() {}\n");
            },
            |root| {
                remove(root, "src/old_name.rs");
                write(root, "src/new_name.rs", "pub fn renamed_fn() {}\n");
            },
        );
        assert_eq!(result.mode, UpdateMode::Incremental);
        assert_eq!(result.files_added, 1);
        assert_eq!(result.files_deleted, 1);
        assert_eq!(
            result.files_modified, 0,
            "a move must be classified as delete+add, never a tracked modification"
        );
    }

    #[test]
    fn manifest_change_converges_through_the_global_physical_rebuild() {
        let result = run_equivalence_case("manifest-change", base_fixture, |root| {
            write(
                root,
                "Cargo.toml",
                "[package]\nname = \"fixture\"\nversion = \"0.2.0\"\n",
            );
            write(root, "pyproject.toml", "[project]\nname = \"fixture\"\n");
            write(root, "package.json", "{\"name\": \"fixture\"}\n");
        });
        assert_eq!(result.mode, UpdateMode::Incremental);
    }

    #[test]
    fn relationship_change_replaces_the_old_import_fact_with_the_new_one() {
        let root = temp_root("incr-relationship");
        write(&root, "src/lib.rs", "use std::fmt;\npub fn hello() {}\n");
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        write(
            &root,
            "src/lib.rs",
            "use std::collections::BTreeMap;\npub fn hello() {}\n",
        );
        let snapshot1 = open_snapshot(&root);
        update(&snapshot1).expect("incremental update");

        let manifest = durable::read_manifest(&root).unwrap().unwrap();
        let edge_targets: Vec<String> = manifest
            .edge_shards
            .iter()
            .flat_map(|shard| durable::read_edge_shard(&root, &shard.shard).unwrap())
            .filter(|edge| edge.kind == crate::GraphEdgeKind::Imports)
            .map(|edge| edge.to)
            .collect();
        assert!(
            edge_targets.iter().any(|id| id.contains("BTreeMap")),
            "the new import fact must appear: {edge_targets:?}"
        );
        assert!(
            !edge_targets
                .iter()
                .any(|id| id.contains("std :: fmt") || id.contains("std::fmt")),
            "the old import fact must not remain: {edge_targets:?}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn governance_change_updates_without_mass_semantic_reparsing() {
        let root = temp_root("incr-governance");
        base_fixture(&root);
        write(
            &root,
            "work/active/001-one/work-item.json",
            r#"{"id":"001","title":"One","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-01"}"#,
        );
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        write(
            &root,
            "work/active/001-one/work-item.json",
            r#"{"id":"001","title":"One (renamed)","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-02"}"#,
        );
        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("incremental update");
        assert_eq!(result.mode, UpdateMode::Incremental);
        assert_eq!(
            result.semantic_reparsed, 1,
            "only the governance JSON file itself should be reprocessed by the semantic \
             pipeline (a cheap unsupported-language classification), not the whole repository"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parser_failure_introduction_isolates_only_the_broken_file() {
        let root = temp_root("incr-failure-intro");
        base_fixture(&root);
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        write(&root, "src/lib.rs", "fn f( { totally not valid rust\n");
        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("incremental update");
        assert_eq!(result.mode, UpdateMode::Incremental);
        assert_eq!(result.files_modified, 1);
        assert_eq!(
            result.semantic_reparsed, 1,
            "only the broken file should be reparsed"
        );
        assert_eq!(
            result.freshness,
            Freshness::Partial,
            "a genuine parse failure must surface as Partial"
        );

        // Unrelated contributions (util.py, widget.ts) must survive.
        let manifest = durable::read_manifest(&root).unwrap().unwrap();
        let coverage = manifest.semantic_coverage.unwrap();
        let python_entry = coverage
            .per_file
            .iter()
            .find(|entry| entry.relative_path == "src/util.py")
            .unwrap();
        assert_eq!(python_entry.coverage, semantic::FileCoverage::Complete);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parser_recovery_clears_partial_and_matches_a_clean_rebuild() {
        let root = temp_root("incr-recovery");
        base_fixture(&root);
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        write(&root, "src/lib.rs", "fn f( { totally not valid rust\n");
        let snapshot1 = open_snapshot(&root);
        update(&snapshot1).expect("incremental update to malformed state");

        write(
            &root,
            "src/lib.rs",
            "pub fn hello() {}\npub fn fixed() {}\n",
        );
        let snapshot2 = open_snapshot(&root);
        let result = update(&snapshot2).expect("incremental update to fixed state");
        assert_eq!(result.mode, UpdateMode::Incremental);
        assert_eq!(result.freshness, Freshness::Fresh);

        let clean_root = temp_root("clean-recovery");
        base_fixture(&clean_root);
        write(
            &clean_root,
            "src/lib.rs",
            "pub fn hello() {}\npub fn fixed() {}\n",
        );
        let clean_snapshot = open_snapshot(&clean_root);
        crate::build_and_write(&clean_snapshot).expect("clean full build");

        assert_canonically_equal(&root, &clean_root);
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(clean_root).unwrap();
    }

    #[test]
    fn policy_transition_normal_to_binary_and_back_leaves_no_stale_contribution() {
        let root = temp_root("incr-policy-binary");
        base_fixture(&root);
        write(&root, "src/toggling.rs", "pub fn toggling() {}\n");
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");

        let mut binary_content = b"pub fn t".to_vec();
        binary_content.extend_from_slice(&[0u8, 1, 2, 3]);
        std::fs::write(root.join("src/toggling.rs"), &binary_content).unwrap();
        let snapshot1 = open_snapshot(&root);
        update(&snapshot1).expect("incremental update to binary");

        let manifest = durable::read_manifest(&root).unwrap().unwrap();
        assert!(!manifest
            .node_shards
            .iter()
            .flat_map(|shard| durable::read_node_shard(&root, &shard.shard).unwrap())
            .any(|node| node.label == "toggling"));

        write(&root, "src/toggling.rs", "pub fn toggling() {}\n");
        let snapshot2 = open_snapshot(&root);
        update(&snapshot2).expect("incremental update back to source");

        let clean_root = temp_root("clean-policy-binary");
        base_fixture(&clean_root);
        write(&clean_root, "src/toggling.rs", "pub fn toggling() {}\n");
        let clean_snapshot = open_snapshot(&clean_root);
        crate::build_and_write(&clean_snapshot).expect("clean full build");
        assert_canonically_equal(&root, &clean_root);

        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(clean_root).unwrap();
    }

    #[test]
    fn unsupported_file_addition_and_deletion_keeps_truthful_coverage() {
        let result = run_equivalence_case("unsupported", base_fixture, |root| {
            write(root, "NOTES.md", "# notes\n");
            remove(root, "README.md");
        });
        assert_eq!(result.mode, UpdateMode::Incremental);
        assert_eq!(result.files_added, 1);
        assert_eq!(result.files_deleted, 1);
    }

    #[test]
    fn rog_only_change_between_calls_causes_no_further_semantic_reparse() {
        let root = temp_root("incr-rog-only");
        base_fixture(&root);
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("first build");
        // Nothing under tracked source changed; the durable rog/ output
        // itself is the only thing that differs from "before any build
        // existed". A second update must be a true no-op.
        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("second update");
        assert_eq!(result.mode, UpdateMode::NoOp);
        assert_eq!(result.semantic_reparsed, 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn excluded_tree_mutation_causes_no_source_delta() {
        let root = temp_root("incr-excluded-tree");
        base_fixture(&root);
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");

        write(&root, "target/debug/build-output.rs", "fn generated() {}\n");
        write(&root, "node_modules/pkg/index.js", "module.exports = {};\n");
        write(&root, ".venv/lib/site.py", "def vendored(): pass\n");

        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("update after excluded-tree writes");
        assert_eq!(
            result.mode,
            UpdateMode::NoOp,
            "excluded directories must never enter the source projection"
        );
        assert_eq!(result.semantic_reparsed, 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn true_no_op_never_rewrites_rog() {
        let root = temp_root("incr-true-noop");
        base_fixture(&root);
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        let manifest_bytes_before = std::fs::read(durable::manifest_path(&root)).unwrap();

        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("no-op update");
        assert_eq!(result.mode, UpdateMode::NoOp);
        assert_eq!(result.semantic_reparsed, 0);
        let manifest_bytes_after = std::fs::read(durable::manifest_path(&root)).unwrap();
        assert_eq!(
            manifest_bytes_before, manifest_bytes_after,
            "a true no-op must not rewrite rog/manifest.json at all"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn observable_reuse_proof_on_a_hundred_file_fixture() {
        let root = temp_root("incr-reuse-proof");
        for index in 0..100 {
            write(
                &root,
                &format!("src/file_{index:03}.rs"),
                &format!("pub fn function_{index:03}() {{}}\n"),
            );
        }
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build of 100 files");

        write(
            &root,
            "src/file_042.rs",
            "pub fn function_042_changed() {}\n",
        );
        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("incremental update touching one file");
        assert_eq!(result.mode, UpdateMode::Incremental);
        assert_eq!(
            result.semantic_reparsed, 1,
            "only the modified file may be reparsed"
        );
        assert_eq!(
            result.semantic_reused, 99,
            "every other file must be reused, not reparsed"
        );

        let clean_root = temp_root("clean-reuse-proof");
        for index in 0..100 {
            let content = if index == 42 {
                "pub fn function_042_changed() {}\n".to_owned()
            } else {
                format!("pub fn function_{index:03}() {{}}\n")
            };
            write(&clean_root, &format!("src/file_{index:03}.rs"), &content);
        }
        let clean_snapshot = open_snapshot(&clean_root);
        crate::build_and_write(&clean_snapshot).expect("clean full build of 100 files");
        assert_canonically_equal(&root, &clean_root);

        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(clean_root).unwrap();
    }

    #[test]
    fn a_true_no_op_reports_zero_reparses_on_a_hundred_file_fixture() {
        let root = temp_root("incr-noop-hundred");
        for index in 0..100 {
            write(
                &root,
                &format!("src/file_{index:03}.rs"),
                &format!("pub fn function_{index:03}() {{}}\n"),
            );
        }
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");
        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("no-op update");
        assert_eq!(result.mode, UpdateMode::NoOp);
        assert_eq!(result.semantic_reparsed, 0);
        assert_eq!(result.semantic_reused, 100);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compatibility_mismatch_forces_a_full_semantic_rebuild() {
        let root = temp_root("incr-compat-mismatch");
        base_fixture(&root);
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");

        // Simulate a changed adapter/pipeline/resource-policy identity by
        // tampering with the recorded compatibility identity directly --
        // exactly the shape a real version bump would produce.
        let manifest_path = durable::manifest_path(&root);
        let text = std::fs::read_to_string(&manifest_path).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        value["semantic_compatibility"]["pipeline_version"] =
            serde_json::Value::String("semantic-pipeline-0-stale".to_owned());
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();

        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("update after compatibility tamper");
        assert_eq!(result.mode, UpdateMode::FullFallback);
        assert_eq!(
            result.fallback_reason.as_deref(),
            Some("semantic_compatibility_mismatch")
        );
        assert_eq!(result.semantic_reused, 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn schema_v1_baseline_forces_a_full_v2_rebuild() {
        let root = temp_root("incr-v1-fallback");
        base_fixture(&root);
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");

        let manifest_path = durable::manifest_path(&root);
        let text = std::fs::read_to_string(&manifest_path).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        value["graph_schema_version"] = serde_json::Value::Number(1.into());
        value.as_object_mut().unwrap().remove("semantic_coverage");
        value
            .as_object_mut()
            .unwrap()
            .remove("semantic_compatibility");
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();

        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("update over a v1 baseline");
        assert_eq!(result.mode, UpdateMode::FullFallback);
        assert_eq!(result.fallback_reason.as_deref(), Some("schema_v1_upgrade"));
        let manifest = durable::read_manifest(&root).unwrap().unwrap();
        assert_eq!(
            manifest.graph_schema_version,
            durable::CURRENT_GRAPH_SCHEMA_VERSION
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn old_v2_baseline_without_incremental_metadata_forces_full_rebuild() {
        let root = temp_root("incr-old-v2-fallback");
        base_fixture(&root);
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");

        let manifest_path = durable::manifest_path(&root);
        let text = std::fs::read_to_string(&manifest_path).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("semantic_compatibility");
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();

        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("update over an old v2 baseline");
        assert_eq!(result.mode, UpdateMode::FullFallback);
        assert_eq!(
            result.fallback_reason.as_deref(),
            Some("incremental_metadata_absent")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_baseline_is_never_reused_and_triggers_a_full_rebuild() {
        let root = temp_root("incr-corrupt");
        base_fixture(&root);
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");

        let manifest = durable::read_manifest(&root).unwrap().unwrap();
        let victim_shard = &manifest.node_shards[0].shard;
        let shard_path = durable::rog_root(&root).join("nodes").join(victim_shard);
        let mut bytes = std::fs::read(&shard_path).unwrap();
        bytes.push(b'\n');
        bytes.extend_from_slice(b"{\"garbage\":true}\n");
        std::fs::write(&shard_path, bytes).unwrap();

        let snapshot1 = open_snapshot(&root);
        let result = update(&snapshot1).expect("update over a corrupt baseline");
        assert_eq!(result.mode, UpdateMode::FullFallback);
        assert_eq!(
            result.fallback_reason.as_deref(),
            Some("corrupt_baseline_full_rebuild")
        );
        // The rebuild must itself be structurally valid -- corruption
        // must not propagate into the freshly written graph.
        assert!(validate::validate_structure(&root).is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unsupported_schema_baseline_fails_closed_without_touching_disk() {
        let root = temp_root("incr-unsupported-schema");
        base_fixture(&root);
        let snapshot0 = open_snapshot(&root);
        crate::build_and_write(&snapshot0).expect("baseline build");

        let manifest_path = durable::manifest_path(&root);
        let text = std::fs::read_to_string(&manifest_path).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        value["graph_schema_version"] = serde_json::Value::Number(999.into());
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();
        let tampered_bytes = std::fs::read(&manifest_path).unwrap();

        let snapshot1 = open_snapshot(&root);
        let error = update(&snapshot1).expect_err("an unsupported schema version must fail closed");
        assert_eq!(error.code, "graph.schema-unsupported");
        let bytes_after = std::fs::read(&manifest_path).unwrap();
        assert_eq!(
            tampered_bytes, bytes_after,
            "a failed-closed update must not have written anything"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_baseline_at_all_is_a_full_fallback_not_an_error() {
        let root = temp_root("incr-absent");
        base_fixture(&root);
        let snapshot = open_snapshot(&root);
        let result = update(&snapshot).expect("update with no prior graph");
        assert_eq!(result.mode, UpdateMode::FullFallback);
        assert_eq!(result.fallback_reason.as_deref(), Some("graph_absent"));
        assert_eq!(result.semantic_reused, 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn update_does_not_exceed_the_wi057_bounded_git_invocation_count() {
        use repopact_repository::CountingGitRunner;

        let root = temp_root("incr-git-bound");
        base_fixture(&root);
        let runner = CountingGitRunner::native();
        let repository = Repository::with_git_runner(&root, runner.clone());
        let snapshot0 = repository.session().snapshot();
        crate::build_and_write(&snapshot0).expect("baseline build");
        let baseline_count = runner.count();

        write(&root, "src/added_for_bound_check.rs", "pub fn added() {}\n");
        let runner2 = CountingGitRunner::native();
        let repository2 = Repository::with_git_runner(&root, runner2.clone());
        let snapshot1 = repository2.session().snapshot();
        update(&snapshot1).expect("incremental update");
        assert!(
            runner2.count() <= 4,
            "an incremental update must not add git invocations beyond the bounded \
             snapshot cost, got {} (baseline snapshot cost was {})",
            runner2.count(),
            baseline_count
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn durable_write_survives_a_failed_final_rename_without_losing_the_old_graph() {
        // Fault-injection proof for `swap_with_rollback` (step 10): the
        // first rename (moving the old graph to a backup path) succeeds,
        // the second (installing the new graph) is forced to fail, and
        // the old graph's manifest must still be readable and byte-exact
        // afterward -- never a half-written or missing graph.
        let root = temp_root("incr-swap-fault");
        base_fixture(&root);
        std::fs::create_dir_all(&root).unwrap();
        let final_path = durable::rog_root(&root);
        let backup_path = root.join("rog.previous-test");
        let staging_path = root.join("rog.building-test");

        std::fs::create_dir_all(&final_path).unwrap();
        std::fs::write(final_path.join("marker.txt"), b"old graph").unwrap();
        std::fs::create_dir_all(&staging_path).unwrap();
        std::fs::write(staging_path.join("marker.txt"), b"new graph").unwrap();

        let mut call_count = 0u32;
        let result = durable::swap_with_rollback(
            move |from: &Path, to: &Path| {
                call_count += 1;
                if call_count == 2 {
                    return Err(std::io::Error::other("simulated install failure"));
                }
                std::fs::rename(from, to)
            },
            &staging_path,
            &final_path,
            &backup_path,
        );
        assert!(result.is_err(), "the simulated failure must propagate");
        assert!(
            final_path.join("marker.txt").is_file(),
            "the old graph must still be present at the final path after a failed install"
        );
        let recovered = std::fs::read(final_path.join("marker.txt")).unwrap();
        assert_eq!(
            recovered, b"old graph",
            "the recovered graph must be byte-identical to the original, not the half-installed one"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
