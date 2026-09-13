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

use crate::{GraphEdge, GraphLayer, GraphNode, RepositoryGraph};

pub const ROG_DIR_NAME: &str = "rog";
/// The schema major version `repopact graph build` always writes (Decision
/// 0045 section 2): once semantic vocabulary exists in the binary, the
/// canonical builder always uses it, rather than sometimes writing v1 and
/// sometimes v2 depending on whether a given build happened to populate
/// semantic content.
pub const CURRENT_GRAPH_SCHEMA_VERSION: u32 = 2;
/// Every major version this implementation can read/validate. Anything
/// outside this set fails closed (Decision 0044 section 4) -- callers
/// must not interpret any other value optimistically. Version 1
/// (physical-only, Decision 0044) remains permanently valid; version 2
/// (Decision 0045) adds semantic vocabulary additively.
pub const SUPPORTED_GRAPH_SCHEMA_VERSIONS: [u32; 2] = [1, 2];
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
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|error| DurableError::new("graph.serialize", error.to_string()))?;
    fs::write(staging.join("manifest.json"), manifest_json)
        .map_err(|error| DurableError::new("graph.io", error.to_string()))?;

    let final_path = rog_root(repository_root);
    if final_path.exists() {
        fs::remove_dir_all(&final_path)
            .map_err(|error| DurableError::new("graph.io", error.to_string()))?;
    }
    fs::rename(&staging, &final_path)
        .map_err(|error| DurableError::new("graph.io", error.to_string()))?;

    Ok(manifest)
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
}
