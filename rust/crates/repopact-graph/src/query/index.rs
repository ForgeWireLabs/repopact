//! Disposable, in-memory, deterministic query-time index (Decision 0050
//! section 4 / "disposable in-memory query index only"). Rebuilt fresh
//! for every [`super::GraphQueryEngine::new`] call from an
//! already-materialized [`crate::RepositoryGraph`] -- never persisted,
//! never `rog/`-adjacent, never authoritative. `BTreeMap`/sorted-vector
//! structures throughout so iteration order is always deterministic,
//! never a `HashMap`'s process-order-dependent iteration.

use std::collections::BTreeMap;

use crate::{GraphEdgeKind, GraphNodeKind, RepositoryGraph};

pub(super) struct GraphQueryIndex {
    /// `node_id -> sorted edge indices (into `graph.edges`) where that
    /// node is the `from` side.
    pub(super) outgoing: BTreeMap<String, Vec<usize>>,
    /// `node_id -> sorted edge indices where that node is the `to` side.
    pub(super) incoming: BTreeMap<String, Vec<usize>>,
    /// Repository-relative source path -> sorted node IDs whose `source
    /// .path` equals that path.
    pub(super) nodes_by_source_path: BTreeMap<String, Vec<String>>,
    /// Open role token -> sorted node IDs carrying that `node_role`.
    /// Not yet consumed by any operation in this checkpoint (every
    /// current role lookup goes through a resolved node's direct
    /// relations instead) -- retained as designed index surface for a
    /// future "list all X-role nodes" style operation.
    #[allow(dead_code)]
    pub(super) nodes_by_role: BTreeMap<String, Vec<String>>,
    /// Node kind -> sorted node IDs of that kind.
    pub(super) nodes_by_kind: BTreeMap<GraphNodeKind, Vec<String>>,
    /// Edge kind -> sorted edge indices of that kind. Not yet consumed
    /// by any operation in this checkpoint (all current relation-kind
    /// lookups start from a specific node, not from the whole graph) --
    /// retained as designed index surface for the same reason as
    /// `nodes_by_role`.
    #[allow(dead_code)]
    pub(super) edges_by_kind: BTreeMap<GraphEdgeKind, Vec<usize>>,
}

impl GraphQueryIndex {
    pub(super) fn build(graph: &RepositoryGraph) -> Self {
        let mut outgoing: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut incoming: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut edges_by_kind: BTreeMap<GraphEdgeKind, Vec<usize>> = BTreeMap::new();
        for (index, edge) in graph.edges.iter().enumerate() {
            outgoing.entry(edge.from.clone()).or_default().push(index);
            incoming.entry(edge.to.clone()).or_default().push(index);
            edges_by_kind.entry(edge.kind).or_default().push(index);
        }

        let mut nodes_by_source_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut nodes_by_role: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut nodes_by_kind: BTreeMap<GraphNodeKind, Vec<String>> = BTreeMap::new();
        for node in graph.nodes.values() {
            if let Some(source) = &node.source {
                nodes_by_source_path
                    .entry(source.path.clone())
                    .or_default()
                    .push(node.id.clone());
            }
            if let Some(role) = &node.node_role {
                nodes_by_role
                    .entry(role.as_str().to_owned())
                    .or_default()
                    .push(node.id.clone());
            }
            nodes_by_kind
                .entry(node.kind)
                .or_default()
                .push(node.id.clone());
        }
        // `graph.nodes` is already a `BTreeMap<String, GraphNode>` keyed
        // by ID, so pushing in key order keeps every bucket sorted
        // without a separate sort pass.

        Self {
            outgoing,
            incoming,
            nodes_by_source_path,
            nodes_by_role,
            nodes_by_kind,
            edges_by_kind,
        }
    }

    pub(super) fn outgoing_edges(&self, node_id: &str) -> &[usize] {
        self.outgoing.get(node_id).map_or(&[], Vec::as_slice)
    }

    pub(super) fn incoming_edges(&self, node_id: &str) -> &[usize] {
        self.incoming.get(node_id).map_or(&[], Vec::as_slice)
    }

    pub(super) fn nodes_by_path(&self, path: &str) -> &[String] {
        self.nodes_by_source_path
            .get(path)
            .map_or(&[], Vec::as_slice)
    }

    #[allow(dead_code)]
    pub(super) fn nodes_with_role(&self, role: &str) -> &[String] {
        self.nodes_by_role.get(role).map_or(&[], Vec::as_slice)
    }

    pub(super) fn nodes_of_kind(&self, kind: GraphNodeKind) -> &[String] {
        self.nodes_by_kind.get(&kind).map_or(&[], Vec::as_slice)
    }
}
