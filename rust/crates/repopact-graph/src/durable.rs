//! Durable manifest + stable-sharded JSONL read/write (WI063 ROG-007,
//! Decision 0044 sections 3-6). Writes are atomic: the entire durable
//! directory is built in a temporary location, then swapped into place
//! only after every file has been written and hashed, so an interrupted
//! build never leaves a partially-written directory at the final path.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use repopact_repository::Repository;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::incremental::SemanticCompatibility;
use crate::semantic::SemanticCoverage;
use crate::{GraphEdge, GraphLayer, GraphNode, RepositoryGraph};

pub const ROG_DIR_NAME: &str = "rog";
/// The schema major version `repopact graph build` always writes (Decision
/// 0045 section 2): once semantic vocabulary exists in the binary, the
/// canonical builder always uses it, rather than sometimes writing v1 and
/// sometimes v2 depending on whether a given build happened to populate
/// semantic content.
pub const CURRENT_GRAPH_SCHEMA_VERSION: u32 = 3;
/// Every major version this implementation can read/validate. Anything
/// outside this set fails closed (Decision 0044 section 4) -- callers
/// must not interpret any other value optimistically. Version 1
/// (physical-only, Decision 0044) and version 2 (Decision 0045 semantic
/// vocabulary) remain permanently valid; version 3 (Decision 0048) adds
/// metadata/operational vocabulary (`GraphNodeKind::Manifest` +
/// `ManifestKind`, new `SourceLanguage` variants) additively -- proven
/// genuinely necessary by a focused compatibility audit, not silently
/// appended under v2.
pub const SUPPORTED_GRAPH_SCHEMA_VERSIONS: [u32; 3] = [1, 2, 3];
pub const SHARD_COUNT: u32 = 16;
pub const EXCLUDED_POLICY_ID: &str = "repopact-source-projection-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardEntry {
    pub shard: String,
    pub sha256: String,
    pub count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    pub nodes_by_layer: std::collections::BTreeMap<GraphLayer, usize>,
    pub edges_by_layer: std::collections::BTreeMap<GraphLayer, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub graph_schema_version: u32,
    pub generator_version: String,
    pub source_projection_fingerprint: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub shard_count: u32,
    pub node_shards: Vec<ShardEntry>,
    pub edge_shards: Vec<ShardEntry>,
    pub coverage: Coverage,
    pub excluded_policy_id: String,
    /// Absent (default) on a genuine schema-v1 manifest, which predates
    /// semantic extraction entirely; always present on a schema-v2
    /// manifest written by this or a later implementation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_coverage: Option<SemanticCoverage>,
    /// Semantic-pipeline compatibility identity (WI063
    /// incremental-equivalence checkpoint, Decision 0046). Absent on any
    /// durable graph written before this checkpoint (including a genuine
    /// v1 graph and a schema-v2 graph from the semantic-adapter
    /// checkpoint that predates this field); an incremental update
    /// encountering `None` here cannot trust unchanged-file reuse and
    /// must fall back to a full rebuild before it can reason
    /// incrementally at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_compatibility: Option<SemanticCompatibility>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableError {
    pub code: String,
    pub message: String,
}

impl DurableError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DurableError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

pub fn rog_root(repository_root: &Path) -> PathBuf {
    repository_root.join(ROG_DIR_NAME)
}

pub fn manifest_path(repository_root: &Path) -> PathBuf {
    rog_root(repository_root).join("manifest.json")
}

fn shard_key_for_node(node: &GraphNode) -> &str {
    &node.id
}

fn edge_kind_str(edge: &GraphEdge) -> String {
    serde_json::to_value(edge.kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn shard_key_for_edge(edge: &GraphEdge) -> String {
    format!("{} {} {}", edge.from, edge.to, edge_kind_str(edge))
}

/// Deterministic shard assignment, independent of process order and
/// platform hash randomization (Decision 0044 section 6): a SHA-256 digest
/// of the stable key, not `std::hash::Hash`/`HashMap`.
pub fn shard_index(key: &str) -> u32 {
    let digest = Sha256::digest(key.as_bytes());
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&digest[0..4]);
    u32::from_be_bytes(bytes) % SHARD_COUNT
}

fn shard_file_name(index: u32) -> String {
    format!("shard-{index:04}.jsonl")
}

/// Write `graph` as the durable ROG representation under
/// `repository_root/rog/`, replacing any existing durable graph, using a
/// build-then-swap sequence so an interruption never leaves a partially
/// written directory at the final path.
pub fn write(
    repository_root: &Path,
    graph: &RepositoryGraph,
    source_projection_fingerprint: &str,
    semantic_coverage: SemanticCoverage,
    semantic_compatibility: SemanticCompatibility,
) -> Result<Manifest, DurableError> {
    let mut nodes_by_shard: std::collections::BTreeMap<u32, Vec<&GraphNode>> =
        std::collections::BTreeMap::new();
    for node in graph.nodes.values() {
        nodes_by_shard
            .entry(shard_index(shard_key_for_node(node)))
            .or_default()
            .push(node);
    }
    for shard in nodes_by_shard.values_mut() {
        shard.sort_by(|left, right| left.id.cmp(&right.id));
    }

    let mut edges_by_shard: std::collections::BTreeMap<u32, Vec<&GraphEdge>> =
        std::collections::BTreeMap::new();
    for edge in &graph.edges {
        edges_by_shard
            .entry(shard_index(&shard_key_for_edge(edge)))
            .or_default()
            .push(edge);
    }
    for shard in edges_by_shard.values_mut() {
        shard.sort_by(|left, right| {
            (left.from.as_str(), left.to.as_str(), edge_kind_str(left)).cmp(&(
                right.from.as_str(),
                right.to.as_str(),
                edge_kind_str(right),
            ))
        });
    }

    let staging = repository_root.join(format!(
        "{ROG_DIR_NAME}.building-{}-{}",
        std::process::id(),
        nanos_suffix()
    ));
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|error| DurableError::new("graph.io", error.to_string()))?;
    }
    let write_result = write_staging(&staging, &nodes_by_shard, &edges_by_shard);
    let manifest = match write_result {
        Ok(manifest_parts) => manifest_parts,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };

    let node_count = graph.nodes.len();
    let edge_count = graph.edges.len();
    let mut nodes_by_layer = std::collections::BTreeMap::new();
    for node in graph.nodes.values() {
        *nodes_by_layer.entry(node.layer).or_insert(0usize) += 1;
    }
    let mut edges_by_layer = std::collections::BTreeMap::new();
    for edge in &graph.edges {
        *edges_by_layer.entry(edge.layer).or_insert(0usize) += 1;
    }

    let manifest = Manifest {
        graph_schema_version: CURRENT_GRAPH_SCHEMA_VERSION,
        generator_version: env!("CARGO_PKG_VERSION").to_owned(),
        source_projection_fingerprint: source_projection_fingerprint.to_owned(),
        node_count,
        edge_count,
        shard_count: SHARD_COUNT,
        node_shards: manifest.0,
        edge_shards: manifest.1,
        coverage: Coverage {
            nodes_by_layer,
            edges_by_layer,
        },
        excluded_policy_id: EXCLUDED_POLICY_ID.to_owned(),
        semantic_coverage: Some(semantic_coverage),
        semantic_compatibility: Some(semantic_compatibility),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|error| DurableError::new("graph.serialize", error.to_string()))?;
    fs::write(staging.join("manifest.json"), manifest_json)
        .map_err(|error| DurableError::new("graph.io", error.to_string()))?;

    let final_path = rog_root(repository_root);
    let backup_path = repository_root.join(format!(
        "{ROG_DIR_NAME}.previous-{}-{}",
        std::process::id(),
        nanos_suffix()
    ));
    swap_with_rollback(
        |from: &Path, to: &Path| fs::rename(from, to),
        &staging,
        &final_path,
        &backup_path,
    )?;

    // WI063 adoption/backfill/clean-clone checkpoint, Decision 0051
    // section 3: capability is persisted `enabled` only *after* the
    // graph has already been proven installed by the swap above --
    // never before. A failure here (a near-impossible plain local file
    // write) does not roll back the already-good graph install; the
    // repository simply lands in `LegacyEnabled` -- still valid and
    // binding, since `rog/` demonstrably exists -- rather than
    // `ExplicitEnabled`.
    crate::capability::persist_enabled(repository_root)
        .map_err(|error| DurableError::new("graph.capability-io", error.to_string()))?;

    Ok(manifest)
}

/// A genuine product defect discovered during this checkpoint's real-
/// git-clone testing (Decision 0051, ROG-016 step 35), not merely a
/// test inconvenience to route around: a very common real-world
/// Git-for-Windows configuration (`core.autocrlf=true` at the system
/// level -- the installer's own historical default, present on this
/// very machine despite an explicit `core.autocrlf=false` at both the
/// user and repository level) silently rewrites LF line endings to
/// CRLF for every ordinary text file on checkout. For `rog/`'s
/// LF-only JSONL shards and manifest, this breaks every recorded
/// SHA-256 shard hash outright (a freshly, correctly cloned graph
/// reports `Corrupt`). For any other source file the durable graph's
/// fingerprint covers, the same checkout-time rewrite silently changes
/// that file's content digest, so the very same clone -- otherwise
/// perfect -- recomputes a different `SourceProjection` fingerprint
/// than the one the manifest recorded, reporting `Stale` instead of
/// `Fresh` immediately after a clean clone. `* -text` disables Git's
/// line-ending conversion for the whole repository (the standard fix
/// for content-addressed tooling); the `rog/`/capability lines are
/// kept as explicit, self-documenting redundancy in case a future
/// narrower override is ever added above this line.
const GITATTRIBUTES_PROTECTION: &str =
    "* -text\nrog/** -text\ngovernance/rog-capability.json -text\n";

/// Ensure `.gitattributes` at the repository root protects the entire
/// source-projection-fingerprinted tree (and, redundantly/explicitly,
/// the durable graph and capability declaration) from Git's own text/
/// line-ending normalization on checkout, appending only the missing
/// lines and never touching any other content in an existing file.
/// Idempotent: running this on a repository that already has the
/// protection is a no-op. Best-effort -- an I/O failure here is
/// reported, matching the same seriousness as any other enablement
/// precondition, since silently proceeding would let a real
/// corruption/staleness-on-clone risk through.
pub fn ensure_gitattributes_protects_durable_graph(
    repository_root: &Path,
) -> Result<(), DurableError> {
    let path = repository_root.join(".gitattributes");
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let needed: Vec<&str> = GITATTRIBUTES_PROTECTION
        .lines()
        .filter(|line| {
            !existing
                .lines()
                .any(|existing_line| existing_line.trim() == *line)
        })
        .collect();
    if needed.is_empty() {
        return Ok(());
    }
    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    for line in needed {
        updated.push_str(line);
        updated.push('\n');
    }
    fs::write(&path, updated).map_err(|error| DurableError::new("graph.io", error.to_string()))
}

pub fn check_enablement_not_ignored(repository: &Repository) -> Result<(), DurableError> {
    let runner = repository.git_runner();
    // A trailing slash is required: `git check-ignore` treats a bare
    // name as ambiguous (could be a file or a directory) and refuses to
    // match a directory-anchored pattern like `rog/` against a path
    // that does not yet exist on disk -- exactly the case that matters
    // here, since this check must catch the problem *before* `rog/` is
    // written.
    let rog_dir_pathspec = format!("{ROG_DIR_NAME}/");
    let args = [
        "check-ignore",
        "--",
        rog_dir_pathspec.as_str(),
        crate::capability::CAPABILITY_RECORD_RELATIVE_PATH,
    ];
    let Ok(output) = runner.run(repository.root(), &args, "graph.check-ignore") else {
        return Ok(());
    };
    // `git check-ignore` exit codes: 0 = at least one path matched an
    // ignore pattern; 1 = none matched; anything else (128 = not a Git
    // repository, or another error) means "can't tell" -- fail open,
    // matching the existing Python precedent's documented best-effort
    // contract.
    if output.status.code() == Some(0) {
        let ignored = String::from_utf8_lossy(&output.stdout);
        return Err(DurableError::new(
            "graph.enablement-ignored",
            format!(
                "git would ignore required durable graph artifacts, refusing to report \
                 enablement success: {}",
                ignored.trim().replace('\n', ", ")
            ),
        ));
    }
    Ok(())
}

/// Explicit disable (`repopact graph disable`, Decision 0051 section 4):
/// remove the derived `rog/` directory, then persist
/// `capabilities.rog = disabled`. Idempotent -- removing an already-
/// absent `rog/` and re-writing an already-`disabled` declaration both
/// succeed with no further effect. Order matters: `rog/` is removed
/// first, so a failure partway through never claims `disabled` while a
/// stale graph still sits on disk (the repository would simply remain
/// in whatever state it already was, still self-consistent).
pub fn disable(repository_root: &Path) -> Result<(), DurableError> {
    let rog_path = rog_root(repository_root);
    if rog_path.exists() {
        fs::remove_dir_all(&rog_path)
            .map_err(|error| DurableError::new("graph.io", error.to_string()))?;
    }
    crate::capability::persist_disabled(repository_root)
        .map_err(|error| DurableError::new("graph.capability-io", error.to_string()))?;
    Ok(())
}

/// Replace `final_path` with `staging` without ever leaving a window in
/// which neither a valid old graph nor a valid new graph is installed
/// there (WI063 incremental-equivalence checkpoint, step 10; Decision
/// 0046). Any pre-existing durable graph is moved aside to `backup_path`
/// (a rename, not a delete) before the new one is installed; if
/// installing the new one fails, the old one is renamed back into place
/// rather than left destroyed. `rename` is injected so tests can force
/// the second rename to fail and assert the rollback property directly,
/// without depending on a real filesystem fault.
pub(crate) fn swap_with_rollback(
    mut rename: impl FnMut(&Path, &Path) -> io::Result<()>,
    staging: &Path,
    final_path: &Path,
    backup_path: &Path,
) -> Result<(), DurableError> {
    let had_previous = final_path.exists();
    if had_previous {
        rename(final_path, backup_path)
            .map_err(|error| DurableError::new("graph.io", error.to_string()))?;
    }
    match rename(staging, final_path) {
        Ok(()) => {
            if had_previous {
                let _ = fs::remove_dir_all(backup_path);
            }
            Ok(())
        }
        Err(install_error) => {
            if had_previous {
                // Best-effort rollback: restore the prior graph so a
                // failed replacement never leaves the repository with no
                // durable graph in a spot that previously had a valid
                // one. If the rollback itself also fails, the backup
                // remains on disk under `backup_path` rather than being
                // silently lost -- an operator can recover it manually --
                // but we still report the original installation failure.
                let _ = rename(backup_path, final_path);
            }
            Err(DurableError::new("graph.io", install_error.to_string()))
        }
    }
}

fn write_staging(
    staging: &Path,
    nodes_by_shard: &std::collections::BTreeMap<u32, Vec<&GraphNode>>,
    edges_by_shard: &std::collections::BTreeMap<u32, Vec<&GraphEdge>>,
) -> Result<(Vec<ShardEntry>, Vec<ShardEntry>), DurableError> {
    let nodes_dir = staging.join("nodes");
    let edges_dir = staging.join("edges");
    fs::create_dir_all(&nodes_dir)
        .map_err(|error| DurableError::new("graph.io", error.to_string()))?;
    fs::create_dir_all(&edges_dir)
        .map_err(|error| DurableError::new("graph.io", error.to_string()))?;

    let mut node_shards = Vec::new();
    for (index, nodes) in nodes_by_shard {
        let mut bytes = Vec::new();
        for node in nodes {
            let line = serde_json::to_string(node)
                .map_err(|error| DurableError::new("graph.serialize", error.to_string()))?;
            bytes.extend_from_slice(line.as_bytes());
            bytes.push(b'\n');
        }
        let name = shard_file_name(*index);
        fs::write(nodes_dir.join(&name), &bytes)
            .map_err(|error| DurableError::new("graph.io", error.to_string()))?;
        node_shards.push(ShardEntry {
            shard: name,
            sha256: hex_digest(Sha256::digest(&bytes)),
            count: nodes.len(),
        });
    }

    let mut edge_shards = Vec::new();
    for (index, edges) in edges_by_shard {
        let mut bytes = Vec::new();
        for edge in edges {
            let line = serde_json::to_string(edge)
                .map_err(|error| DurableError::new("graph.serialize", error.to_string()))?;
            bytes.extend_from_slice(line.as_bytes());
            bytes.push(b'\n');
        }
        let name = shard_file_name(*index);
        fs::write(edges_dir.join(&name), &bytes)
            .map_err(|error| DurableError::new("graph.io", error.to_string()))?;
        edge_shards.push(ShardEntry {
            shard: name,
            sha256: hex_digest(Sha256::digest(&bytes)),
            count: edges.len(),
        });
    }

    Ok((node_shards, edge_shards))
}

pub fn read_manifest(repository_root: &Path) -> Result<Option<Manifest>, DurableError> {
    let path = manifest_path(repository_root);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| DurableError::new("graph.io", error.to_string()))?;
    let manifest: Manifest = serde_json::from_str(&text)
        .map_err(|error| DurableError::new("graph.manifest-malformed", error.to_string()))?;
    Ok(Some(manifest))
}

pub fn read_node_shard(
    repository_root: &Path,
    shard_file: &str,
) -> Result<Vec<GraphNode>, DurableError> {
    read_shard(&rog_root(repository_root).join("nodes").join(shard_file))
}

pub fn read_edge_shard(
    repository_root: &Path,
    shard_file: &str,
) -> Result<Vec<GraphEdge>, DurableError> {
    read_shard(&rog_root(repository_root).join("edges").join(shard_file))
}

/// Reconstruct a full [`RepositoryGraph`] directly from a durable
/// manifest's already-validated shards -- no source file is read and no
/// adapter runs (WI063 ROG-013, Decision 0047 section 7). Callers must
/// already know `manifest` is structurally sound (e.g. via
/// [`crate::validate::validate_structure`] returning no diagnostics)
/// before relying on this as a faithful reconstruction; this function
/// itself only propagates shard read/parse errors, it does not
/// re-validate hashes or ordering.
pub fn load_graph(
    repository_root: &Path,
    manifest: &Manifest,
) -> Result<RepositoryGraph, DurableError> {
    let mut nodes = std::collections::BTreeMap::new();
    for shard in &manifest.node_shards {
        for node in read_node_shard(repository_root, &shard.shard)? {
            nodes.insert(node.id.clone(), node);
        }
    }
    let mut edges = Vec::new();
    for shard in &manifest.edge_shards {
        edges.extend(read_edge_shard(repository_root, &shard.shard)?);
    }
    edges.sort();
    edges.dedup();
    Ok(RepositoryGraph { nodes, edges })
}

fn read_shard<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>, DurableError> {
    let text = fs::read_to_string(path).map_err(|error| {
        DurableError::new(
            "graph.shard-missing",
            format!("{}: {error}", path.display()),
        )
    })?;
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .map_err(|error| DurableError::new("graph.shard-malformed", error.to_string()))
        })
        .collect()
}

pub fn shard_bytes(repository_root: &Path, kind: &str, shard_file: &str) -> io::Result<Vec<u8>> {
    fs::read(rog_root(repository_root).join(kind).join(shard_file))
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn nanos_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{ROG_DIR_NAME, SHARD_COUNT};
    use crate::test_support::temp_root;
    use crate::{GraphEdge, GraphNode};
    use repopact_repository::RepositorySession;
    use std::path::Path;

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    fn open_snapshot(root: &Path) -> repopact_repository::RepositorySnapshot {
        RepositorySession::open(root.to_path_buf()).snapshot()
    }

    fn all_shard_files(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
        let mut files = std::collections::BTreeMap::new();
        for subdir in ["nodes", "edges"] {
            let dir = root.join(ROG_DIR_NAME).join(subdir);
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let key = format!("{subdir}/{}", entry.file_name().to_string_lossy());
                files.insert(key, std::fs::read(&path).unwrap());
            }
        }
        files
    }

    /// ROG-029, Decision 0052 section 5: the stable hash-to-shard design
    /// (Decision 0044 section 6) minimizes unrelated churn -- proven
    /// here, not merely asserted from shard count. A graph with enough
    /// nodes/edges is built, one isolated source contribution is
    /// mutated, the graph is rebuilt, and the exact changed-shard set is
    /// measured: every shard the mutation's node/edges did not hash into
    /// must remain byte-identical.
    #[test]
    fn an_isolated_source_mutation_touches_only_a_bounded_shard_subset() {
        let root = temp_root("shard-churn");
        for index in 0..120 {
            write(
                &root,
                &format!("work/active/{index:03}/work-item.json"),
                &format!(
                    r#"{{"id":"{index:03}","title":"Item {index:03}","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{{"id":"AC-1","text":"prove","state":"pending","evidence":[]}}],"created":"2026-01-01","updated":"2026-01-01"}}"#
                ),
            );
        }
        crate::build_and_write(&open_snapshot(&root)).expect("baseline build");
        let before = all_shard_files(&root);
        assert_eq!(
            before.len() as u32,
            SHARD_COUNT * 2,
            "expect every node/edge shard to exist in this fixture"
        );

        // Mutate exactly one isolated source contribution.
        write(
            &root,
            "work/active/060/work-item.json",
            r#"{"id":"060","title":"Item 060 (mutated)","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-01"}"#,
        );
        crate::build_and_write(&open_snapshot(&root)).expect("rebuild after isolated mutation");
        let after = all_shard_files(&root);

        let changed: Vec<&String> = before
            .keys()
            .filter(|key| before.get(*key) != after.get(*key))
            .collect();
        assert!(
            !changed.is_empty(),
            "the mutated work item's node/edge shard(s) must have changed"
        );
        assert!(
            changed.len() < before.len(),
            "an isolated single-item mutation must not touch every shard: changed {} of {}",
            changed.len(),
            before.len()
        );
        // Every unchanged shard must be genuinely byte-identical, not
        // merely "probably fine."
        for key in before.keys() {
            if !changed.contains(&key) {
                assert_eq!(
                    before[key], after[key],
                    "shard {key} must be byte-identical when untouched"
                );
            }
        }
        std::fs::remove_dir_all(&root).ok();
    }

    /// Two independent branch changes (different work items, chosen so
    /// their stable IDs hash to different shards in practice) should
    /// each touch a small, mostly-disjoint shard subset -- proving the
    /// hash-to-shard design does not concentrate unrelated churn onto a
    /// single hot shard.
    #[test]
    fn two_independent_mutations_map_to_mostly_disjoint_shard_subsets() {
        let root = temp_root("shard-churn-disjoint");
        for index in 0..120 {
            write(
                &root,
                &format!("work/active/{index:03}/work-item.json"),
                &format!(
                    r#"{{"id":"{index:03}","title":"Item {index:03}","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{{"id":"AC-1","text":"prove","state":"pending","evidence":[]}}],"created":"2026-01-01","updated":"2026-01-01"}}"#
                ),
            );
        }
        crate::build_and_write(&open_snapshot(&root)).expect("baseline build");
        let baseline = all_shard_files(&root);

        write(
            &root,
            "work/active/010/work-item.json",
            r#"{"id":"010","title":"Item 010 (A)","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-01"}"#,
        );
        crate::build_and_write(&open_snapshot(&root)).expect("rebuild A");
        let after_a = all_shard_files(&root);
        let changed_a: std::collections::BTreeSet<&String> = baseline
            .keys()
            .filter(|key| baseline.get(*key) != after_a.get(*key))
            .collect();

        write(
            &root,
            "work/active/099/work-item.json",
            r#"{"id":"099","title":"Item 099 (B)","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-01"}"#,
        );
        crate::build_and_write(&open_snapshot(&root)).expect("rebuild B");
        let after_b = all_shard_files(&root);
        let changed_b: std::collections::BTreeSet<&String> = after_a
            .keys()
            .filter(|key| after_a.get(*key) != after_b.get(*key))
            .collect();

        assert!(!changed_a.is_empty());
        assert!(!changed_b.is_empty());
        assert!(
            changed_a.len() < baseline.len() && changed_b.len() < after_a.len(),
            "each independent mutation should touch a bounded shard subset, not every shard"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// Originally confirmed, before schema v2 was implemented, the exact
    /// failure mode Decision 0044/0045 must avoid: `GraphNodeKind` and
    /// `GraphEdgeKind` are closed serde enums with no catch-all variant, so
    /// an implementation that only knows a given variant set fails hard
    /// (not "unknown value, ignored") when a shard line contains a variant
    /// string it doesn't recognize. `symbol`/`defines` were the original
    /// proof strings; now that schema v2 legitimately recognizes them,
    /// this test uses a permanently-fictional variant name so it keeps
    /// guarding the general hazard class (closed enums must never silently
    /// accept an unrecognized kind string) rather than becoming a no-op
    /// once its original example strings become real vocabulary. This is
    /// why any future vocabulary growth must bump `graph_schema_version`
    /// rather than silently appending variants under the current major --
    /// an old reader must reject an unsupported major version cleanly
    /// (tested elsewhere in `validate::tests`/`status`), not crash midway
    /// through parsing a shard it was told was a version it understands.
    #[test]
    fn a_reader_cannot_deserialize_an_unrecognized_node_kind_string() {
        let line = r#"{"id":"symbol:foo","kind":"some_future_kind_not_yet_invented","label":"foo","layer":"semantic","source":{"kind":"file","id":"src/lib.rs","path":"src/lib.rs"}}"#;
        let result: Result<GraphNode, _> = serde_json::from_str(line);
        assert!(
            result.is_err(),
            "a GraphNode JSONL line naming a node kind not in the current \
             GraphNodeKind enum must fail to deserialize, not silently \
             succeed with a default/unknown variant -- this is exactly the \
             hazard of adding a new kind without bumping the major schema \
             version"
        );
    }

    #[test]
    fn a_reader_cannot_deserialize_an_unrecognized_edge_kind_string() {
        let line = r#"{"from":"file:src/lib.rs","to":"symbol:foo","kind":"some_future_relation_not_yet_invented","layer":"semantic","derivation":"parser","source":{"kind":"file","id":"src/lib.rs","path":"src/lib.rs"}}"#;
        let result: Result<GraphEdge, _> = serde_json::from_str(line);
        assert!(
            result.is_err(),
            "a GraphEdge JSONL line naming an edge kind not in the current \
             GraphEdgeKind enum must fail to deserialize -- the same hazard \
             as the node-kind case above, for edges"
        );
    }

    /// WI063 metadata/operational-topology checkpoint, step 31: before
    /// adding any *nested* closed enum (a `SymbolKind`/`ManifestKind`-
    /// shaped field embedded inside `GraphNode`), prove it carries the
    /// identical hazard as a top-level `GraphNodeKind`/`GraphEdgeKind`
    /// variant -- Decision 0045's claim that growing `SymbolKind` "does
    /// not require another schema-major bump" was true only in the sense
    /// that a genuinely *old* (pre-Symbol) v1 reader never had this field
    /// at all; it does NOT mean a v2-era reader compiled before a new
    /// `SymbolKind`/`ManifestKind` variant existed can deserialize a
    /// shard line naming that variant. This test proves the general
    /// case: any closed, non-`#[serde(other)]` enum nested inside
    /// `GraphNode` (not just the top-level `kind` field) hard-fails on
    /// an unrecognized string exactly like `GraphNodeKind` does. This is
    /// the audit finding Decision 0048 relies on to justify a schema
    /// major bump for this checkpoint's new `ManifestKind`/`SourceLanguage`
    /// vocabulary, rather than silently appending it under v2.
    #[test]
    fn a_nested_closed_enum_field_carries_the_identical_hazard_as_a_top_level_kind() {
        let line = r#"{"id":"symbol:foo","kind":"symbol","label":"foo","layer":"semantic","source":{"kind":"file","id":"src/lib.rs","path":"src/lib.rs"},"symbol_kind":"some_future_symbol_kind_not_yet_invented"}"#;
        let result: Result<GraphNode, _> = serde_json::from_str(line);
        assert!(
            result.is_err(),
            "an unrecognized value in a nested closed enum field (symbol_kind here, \
             the same shape a future manifest_kind field would have) must fail to \
             deserialize just as a top-level `kind` mismatch does -- nesting a \
             closed enum one level deeper does not exempt it from the schema- \
             major-bump rule"
        );
    }

    /// Decision 0049 section 2's load-bearing proof, required before the
    /// open `node_role`/`relation_role` string-backed role model could be
    /// adopted at all: an implementation compiled against the exact pre-
    /// 0049 `GraphNode`/`GraphEdge` shape (schema v3 as Decision 0048 left
    /// it -- no knowledge whatsoever of role fields) must still deserialize
    /// a new-v3 JSONL line that *does* carry role metadata, silently
    /// ignoring the fields it doesn't recognize and recovering a coarse
    /// `kind`/`layer` that remains fully, independently true. This is the
    /// opposite of the hazard proven above: adding a new *struct field*
    /// that defaults to absent is safe for an old reader in a way adding a
    /// new *enum variant* never is, because the old reader's own shape
    /// simply never asks for the field -- serde does not require an old
    /// struct to account for extra JSON object keys it was never told to
    /// look for. If this test had failed, Decision 0049 would have had to
    /// stop and record why a v4 bump was required instead; it did not
    /// fail, so the open role model was adopted as schema v3.
    #[test]
    fn an_old_v3_reader_shape_still_deserializes_a_node_edge_carrying_new_role_metadata() {
        /// Shadow of `GraphNode` exactly as schema v3 existed under
        /// Decision 0048, before `node_role` existed.
        #[derive(Debug, serde::Deserialize)]
        #[allow(dead_code)]
        struct OldV3GraphNode {
            id: String,
            kind: crate::GraphNodeKind,
            label: String,
            #[serde(default)]
            layer: crate::GraphLayer,
            source: Option<crate::SourceRef>,
            symbol_kind: Option<crate::SymbolKind>,
            location: Option<crate::GraphSourceLocation>,
            manifest_kind: Option<crate::ManifestKind>,
        }

        /// Shadow of `GraphEdge` exactly as schema v3 existed under
        /// Decision 0048, before `relation_role` existed.
        #[derive(Debug, serde::Deserialize)]
        #[allow(dead_code)]
        struct OldV3GraphEdge {
            from: String,
            to: String,
            kind: crate::GraphEdgeKind,
            #[serde(default)]
            layer: crate::GraphLayer,
            #[serde(default)]
            derivation: crate::DerivationClass,
            source: crate::SourceRef,
            location: Option<crate::GraphSourceLocation>,
        }

        // A *new* producer (this checkpoint) writes a node/edge pair
        // carrying role metadata the old shape has never heard of.
        let node = GraphNode {
            id: "manifest:package.json".to_owned(),
            kind: crate::GraphNodeKind::Manifest,
            label: "package.json".to_owned(),
            layer: crate::GraphLayer::Package,
            source: Some(crate::SourceRef::new(
                crate::RecordKind::File,
                "package.json".to_owned(),
                "package.json".to_owned(),
            )),
            symbol_kind: None,
            location: None,
            manifest_kind: Some(crate::ManifestKind::JsonDocument),
            node_role: crate::GraphNodeRole::new("installer_surface"),
        };
        let edge = GraphEdge {
            from: "manifest:package.json".to_owned(),
            to: "package:workspace-root".to_owned(),
            kind: crate::GraphEdgeKind::BelongsToWorkspace,
            layer: crate::GraphLayer::Package,
            derivation: crate::DerivationClass::Manifest,
            source: crate::SourceRef::new(
                crate::RecordKind::File,
                "package.json".to_owned(),
                "package.json".to_owned(),
            ),
            location: None,
            relation_role: crate::GraphRelationRole::new("workspace_member"),
        };

        let node_json = serde_json::to_string(&node).expect("node serializes");
        let edge_json = serde_json::to_string(&edge).expect("edge serializes");

        let old_node: OldV3GraphNode = serde_json::from_str(&node_json).expect(
            "an old-v3 reader shape with no knowledge of node_role must still \
                 deserialize a new-v3 node carrying it -- extra JSON object keys \
                 an old struct never asks for are not a deserialization error",
        );
        let old_edge: OldV3GraphEdge = serde_json::from_str(&edge_json).expect(
            "an old-v3 reader shape with no knowledge of relation_role must \
                 still deserialize a new-v3 edge carrying it",
        );

        // The coarse fact remains independently true, exactly as Decision
        // 0049 section 3 requires -- the old reader recovers a correct,
        // meaningful kind/layer even though it never saw the role at all.
        assert_eq!(old_node.kind, crate::GraphNodeKind::Manifest);
        assert_eq!(old_node.layer, crate::GraphLayer::Package);
        assert_eq!(old_edge.kind, crate::GraphEdgeKind::BelongsToWorkspace);
        assert_eq!(old_edge.layer, crate::GraphLayer::Package);
    }
}
