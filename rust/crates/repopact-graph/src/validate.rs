//! Structural validation of an on-disk durable ROG directory (WI063
//! ROG-031). Freshness (comparing the manifest's recorded fingerprint
//! against a freshly recomputed one) is a separate concern handled by
//! [`crate::status`]; this module only checks internal structural
//! integrity of whatever is currently on disk.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::durable::{self, DurableError, SUPPORTED_GRAPH_SCHEMA_VERSIONS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphDiagnostic {
    pub code: String,
    pub message: String,
}

fn diagnostic(code: &str, message: impl Into<String>) -> GraphDiagnostic {
    GraphDiagnostic {
        code: code.to_owned(),
        message: message.into(),
    }
}

/// Validate the durable graph at `repository_root/rog/`. Returns an empty
/// list only if the graph is structurally sound. A missing manifest is
/// reported as a single `graph.absent` diagnostic distinguishable from
/// genuine corruption -- callers that want "disabled repositories are
/// valid" semantics should treat `graph.absent` specially rather than as
/// an error (see [`crate::status`]).
pub fn validate_structure(repository_root: &Path) -> Vec<GraphDiagnostic> {
    let manifest = match durable::read_manifest(repository_root) {
        Ok(Some(manifest)) => manifest,
        Ok(None) => return vec![diagnostic("graph.absent", "no rog/manifest.json present")],
        Err(error) => return vec![manifest_error_to_diagnostic(error)],
    };

    let mut diagnostics = Vec::new();

    if !SUPPORTED_GRAPH_SCHEMA_VERSIONS.contains(&manifest.graph_schema_version) {
        diagnostics.push(diagnostic(
            "graph.schema-unsupported",
            format!(
                "durable graph declares schema version {} but this implementation supports only {SUPPORTED_GRAPH_SCHEMA_VERSIONS:?}",
                manifest.graph_schema_version
            ),
        ));
        // An unsupported major version must not be interpreted
        // optimistically -- stop here rather than attempt to read shards
        // whose shape this version does not promise to understand.
        return diagnostics;
    }

    let mut seen_node_ids = std::collections::BTreeSet::new();
    let mut node_count = 0usize;
    for entry in &manifest.node_shards {
        let bytes = match durable::shard_bytes(repository_root, "nodes", &entry.shard) {
            Ok(bytes) => bytes,
            Err(error) => {
                diagnostics.push(diagnostic(
                    "graph.shard-missing",
                    format!("nodes/{}: {error}", entry.shard),
                ));
                continue;
            }
        };
        let actual_hash = hex_digest(Sha256::digest(&bytes));
        if actual_hash != entry.sha256 {
            diagnostics.push(diagnostic(
                "graph.shard-hash-mismatch",
                format!(
                    "nodes/{}: manifest sha256 {} does not match on-disk sha256 {actual_hash}",
                    entry.shard, entry.sha256
                ),
            ));
            continue;
        }
        let nodes = match durable::read_node_shard(repository_root, &entry.shard) {
            Ok(nodes) => nodes,
            Err(error) => {
                diagnostics.push(diagnostic(
                    "graph.shard-malformed",
                    format!("nodes/{}: {error}", entry.shard),
                ));
                continue;
            }
        };
        if nodes.len() != entry.count {
            diagnostics.push(diagnostic(
                "graph.shard-count-mismatch",
                format!(
                    "nodes/{}: manifest declares {} entries, found {}",
                    entry.shard,
                    entry.count,
                    nodes.len()
                ),
            ));
        }
        let mut previous: Option<&str> = None;
        for node in &nodes {
            if let Some(previous_id) = previous {
                if previous_id > node.id.as_str() {
                    diagnostics.push(diagnostic(
                        "graph.non-canonical-order",
                        format!(
                            "nodes/{}: {node_id} is out of canonical order after {previous_id}",
                            entry.shard,
                            node_id = node.id
                        ),
                    ));
                }
            }
            previous = Some(&node.id);
            if !seen_node_ids.insert(node.id.clone()) {
                diagnostics.push(diagnostic(
                    "graph.duplicate-node",
                    format!("duplicate node id: {}", node.id),
                ));
            }
            if let Some(source) = &node.source {
                if let Some(issue) = path_safety_issue(&source.path) {
                    diagnostics.push(diagnostic(
                        "graph.path-unsafe",
                        format!("node {}: {issue}", node.id),
                    ));
                }
            }
        }
        node_count += nodes.len();
    }
    if node_count != manifest.node_count {
        diagnostics.push(diagnostic(
            "graph.manifest-count-mismatch",
            format!(
                "manifest declares node_count {} but shards contain {node_count}",
                manifest.node_count
            ),
        ));
    }

    let mut edge_count = 0usize;
    for entry in &manifest.edge_shards {
        let bytes = match durable::shard_bytes(repository_root, "edges", &entry.shard) {
            Ok(bytes) => bytes,
            Err(error) => {
                diagnostics.push(diagnostic(
                    "graph.shard-missing",
                    format!("edges/{}: {error}", entry.shard),
                ));
                continue;
            }
        };
        let actual_hash = hex_digest(Sha256::digest(&bytes));
        if actual_hash != entry.sha256 {
            diagnostics.push(diagnostic(
                "graph.shard-hash-mismatch",
                format!(
                    "edges/{}: manifest sha256 {} does not match on-disk sha256 {actual_hash}",
                    entry.shard, entry.sha256
                ),
            ));
            continue;
        }
        let edges = match durable::read_edge_shard(repository_root, &entry.shard) {
            Ok(edges) => edges,
            Err(error) => {
                diagnostics.push(diagnostic(
                    "graph.shard-malformed",
                    format!("edges/{}: {error}", entry.shard),
                ));
                continue;
            }
        };
        if edges.len() != entry.count {
            diagnostics.push(diagnostic(
                "graph.shard-count-mismatch",
                format!(
                    "edges/{}: manifest declares {} entries, found {}",
                    entry.shard,
                    entry.count,
                    edges.len()
                ),
            ));
        }
        for edge in &edges {
            if !seen_node_ids.contains(&edge.from) {
                diagnostics.push(diagnostic(
                    "graph.dangling-edge",
                    format!(
                        "edge from {} -> {} references missing node {}",
                        edge.from, edge.to, edge.from
                    ),
                ));
            }
            if !seen_node_ids.contains(&edge.to) {
                diagnostics.push(diagnostic(
                    "graph.dangling-edge",
                    format!(
                        "edge from {} -> {} references missing node {}",
                        edge.from, edge.to, edge.to
                    ),
                ));
            }
            if let Some(issue) = path_safety_issue(&edge.source.path) {
                diagnostics.push(diagnostic(
                    "graph.path-unsafe",
                    format!("edge {} -> {}: {issue}", edge.from, edge.to),
                ));
            }
        }
        edge_count += edges.len();
    }
    if edge_count != manifest.edge_count {
        diagnostics.push(diagnostic(
            "graph.manifest-count-mismatch",
            format!(
                "manifest declares edge_count {} but shards contain {edge_count}",
                manifest.edge_count
            ),
        ));
    }

    diagnostics
}

fn manifest_error_to_diagnostic(error: DurableError) -> GraphDiagnostic {
    diagnostic(&error.code, error.message)
}

fn path_safety_issue(path: &str) -> Option<String> {
    if path == "<root>" {
        return None;
    }
    if path.starts_with('/') || (path.len() > 1 && path.as_bytes()[1] == b':') {
        return Some(format!("absolute path recorded in durable graph: {path}"));
    }
    if path.split('/').any(|part| part == "..") {
        return Some(format!("path escape recorded in durable graph: {path}"));
    }
    None
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
