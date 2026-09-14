//! Durable manifest + stable-sharded JSONL read/write (WI063 ROG-007,
//! Decision 0044 sections 3-6). Writes are atomic: the entire durable
//! directory is built in a temporary location, then swapped into place
//! only after every file has been written and hashed, so an interrupted
//! build never leaves a partially-written directory at the final path.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

    Ok(manifest)
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
    use crate::{GraphEdge, GraphNode};

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

        let old_node: OldV3GraphNode =
            serde_json::from_str(&node_json).expect(
                "an old-v3 reader shape with no knowledge of node_role must still \
                 deserialize a new-v3 node carrying it -- extra JSON object keys \
                 an old struct never asks for are not a deserialization error",
            );
        let old_edge: OldV3GraphEdge =
            serde_json::from_str(&edge_json).expect(
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
