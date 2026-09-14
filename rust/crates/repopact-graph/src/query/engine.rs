//! The canonical Rust query kernel (WI063 ROG-023/024/025/026, Decision
//! 0050). One [`GraphQueryEngine`] implementation is reused identically
//! by the engine binary, the desktop API's session-overlay queries, and
//! (through the engine) the Python CLI -- no traversal logic is
//! duplicated in any adapter layer.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::{GraphEdge, GraphEdgeKind, GraphLayer, GraphNodeKind, RepositoryGraph};

use super::bounds::QueryBounds;
use super::cursor::{self, CursorError};
use super::dto::{
    ContextResult, DependenciesResult, DependentsResult, Direction, FactRef, GovernanceResult,
    HintReason, ImpactResult, NavigationHint, NeighborsResult, OrientOutcome, OrientResult,
    PathOutcome, PathResult, QueryEnvelope, RelationFact, RelationProvenance, ResolutionOutcome,
    TestsResult,
};
use super::index::GraphQueryIndex;
use super::open::GraphQueryContext;
use super::resolve;
use super::selector::NodeSelector;
use super::QUERY_CONTRACT_VERSION;

/// The one, source-backed disclosure this checkpoint's directive names
/// explicitly (ROG-019 remains pending on exactly this gap): RepoPact's
/// own `fixtures/`-named directories are excluded from the entire graph
/// by the pre-existing `IGNORED_PARTS` policy (Decision 0044), proven by
/// ROG-021. No included RepoPact metadata currently names an
/// excluded-fixture-boundary fact.
const FIXTURE_COVERAGE_WARNING: &str = "test fixture topology unavailable: fixture directories are excluded by repository projection policy (Decision 0044/ROG-021); absence of a fixture fact here does not mean no fixtures exist";

const GOVERNANCE_EDGE_KINDS: &[GraphEdgeKind] = &[
    GraphEdgeKind::SupportedBy,
    GraphEdgeKind::SupportsWorkItem,
    GraphEdgeKind::OwnedBy,
    GraphEdgeKind::Affects,
    GraphEdgeKind::Supersedes,
    GraphEdgeKind::Concerns,
    GraphEdgeKind::ConstrainedBy,
    GraphEdgeKind::Intersects,
    GraphEdgeKind::AppliesTo,
    GraphEdgeKind::Allows,
];

pub struct GraphQueryEngine<'a> {
    graph: &'a RepositoryGraph,
    context: GraphQueryContext,
    index: GraphQueryIndex,
}

struct Page<'g> {
    items: Vec<&'g GraphEdge>,
    truncated: bool,
    next_cursor: Option<String>,
    warnings: Vec<String>,
}

impl<'a> GraphQueryEngine<'a> {
    pub fn new(graph: &'a RepositoryGraph, context: GraphQueryContext) -> Self {
        let index = GraphQueryIndex::build(graph);
        Self {
            graph,
            context,
            index,
        }
    }

    fn envelope<T>(
        &self,
        result: T,
        mut warnings: Vec<String>,
        truncated: bool,
        returned_nodes: usize,
        returned_edges: usize,
        next_cursor: Option<String>,
    ) -> QueryEnvelope<T> {
        if self.context.status.durable_freshness == crate::overlay::DurableFreshness::Stale {
            warnings.push("stale durable graph".to_owned());
        }
        if self.context.status.coverage == crate::overlay::GraphCoverageState::Partial {
            warnings.push(
                "partial semantic coverage: some supported files were not fully processed"
                    .to_owned(),
            );
        }
        if self.context.status.basis == crate::overlay::GraphBasis::WorkingOverlay {
            warnings.push("basis is working_overlay: uncommitted working-tree changes contribute to this answer".to_owned());
        }
        QueryEnvelope {
            query_contract_version: QUERY_CONTRACT_VERSION,
            graph_schema_version: self.context.graph_schema_version,
            graph_fingerprint: self.context.graph_fingerprint.clone(),
            status: self.context.status.clone(),
            warnings,
            truncated,
            returned_nodes,
            returned_edges,
            next_cursor,
            result,
        }
    }

    fn not_found_envelope<T>(&self, message: String) -> QueryEnvelope<Option<T>> {
        self.envelope(None, vec![message], false, 0, 0, None)
    }

    fn invalid_cursor_envelope<T>(&self, error: CursorError) -> QueryEnvelope<Option<T>> {
        self.envelope(
            None,
            vec![format!("invalid cursor: {error}")],
            false,
            0,
            0,
            None,
        )
    }

    /// Bound, hash, and paginate a candidate edge list (shared by
    /// `neighbors`/`dependencies`/`dependents`). `candidates` must
    /// already be in deterministic (ascending edge-index) order.
    fn page_edges<'g>(
        &self,
        operation: &str,
        request_key: &impl Serialize,
        mut candidates: Vec<&'g GraphEdge>,
        bounds: &QueryBounds,
    ) -> Result<Page<'g>, CursorError> {
        let mut warnings = Vec::new();
        if candidates.len() > bounds.max_edges {
            warnings.push(format!(
                "relation count exceeds max_edges bound ({}); only the first {} in stable order are considered",
                bounds.max_edges, bounds.max_edges
            ));
            candidates.truncate(bounds.max_edges);
        }
        let request_hash = cursor::request_identity(request_key);
        let start = match &bounds.cursor {
            Some(raw) => cursor::decode_and_validate_cursor(
                raw,
                QUERY_CONTRACT_VERSION,
                &self.context.graph_fingerprint,
                operation,
                &request_hash,
            )? as usize,
            None => 0,
        };
        let page_size = bounds.page_size.max(1);
        let end = (start + page_size).min(candidates.len());
        let items = if start < candidates.len() {
            candidates[start..end].to_vec()
        } else {
            Vec::new()
        };
        let has_more = end < candidates.len();
        let next_cursor = has_more.then(|| {
            cursor::encode_cursor(
                QUERY_CONTRACT_VERSION,
                &self.context.graph_fingerprint,
                operation,
                &request_hash,
                end as u64,
            )
        });
        Ok(Page {
            items,
            truncated: has_more,
            next_cursor,
            warnings,
        })
    }

    fn fact(&self, node_id: &str) -> Option<FactRef> {
        self.graph.nodes.get(node_id).map(FactRef::from_node)
    }

    // ---- graph.resolve ----------------------------------------------

    pub fn resolve(
        &self,
        selector: &NodeSelector,
        bounds: &QueryBounds,
    ) -> QueryEnvelope<ResolutionOutcome> {
        let outcome = resolve::resolve(self.graph, &self.index, selector);
        let (outcome, warnings) = match outcome {
            ResolutionOutcome::Ambiguous { candidates } => {
                let mut warnings = Vec::new();
                let capped = if candidates.len() > bounds.max_nodes {
                    warnings.push(format!(
                        "ambiguous candidate list exceeds max_nodes bound ({}); showing the first {} in stable order",
                        bounds.max_nodes, bounds.max_nodes
                    ));
                    candidates.into_iter().take(bounds.max_nodes).collect()
                } else {
                    candidates
                };
                (
                    ResolutionOutcome::Ambiguous { candidates: capped },
                    warnings,
                )
            }
            other => (other, Vec::new()),
        };
        let returned_nodes = match &outcome {
            ResolutionOutcome::Exact { .. } => 1,
            ResolutionOutcome::Ambiguous { candidates } => candidates.len(),
            ResolutionOutcome::NotFound => 0,
        };
        self.envelope(outcome, warnings, false, returned_nodes, 0, None)
    }

    fn resolve_exact(&self, selector: &NodeSelector) -> Result<FactRef, ResolutionOutcome> {
        match resolve::resolve(self.graph, &self.index, selector) {
            ResolutionOutcome::Exact { fact } => Ok(fact),
            other => Err(other),
        }
    }

    // ---- graph.neighbors ----------------------------------------------

    pub fn neighbors(
        &self,
        node_id: &str,
        direction: Direction,
        bounds: &QueryBounds,
    ) -> QueryEnvelope<Option<NeighborsResult>> {
        let Some(node_fact) = self.fact(node_id) else {
            return self.not_found_envelope(format!("node not found: {node_id}"));
        };

        let mut indices: Vec<usize> = Vec::new();
        if matches!(direction, Direction::Outgoing | Direction::Both) {
            indices.extend(self.index.outgoing_edges(node_id));
        }
        if matches!(direction, Direction::Incoming | Direction::Both) {
            indices.extend(self.index.incoming_edges(node_id));
        }
        indices.sort_unstable();
        indices.dedup();

        let candidates: Vec<&GraphEdge> = indices
            .into_iter()
            .map(|i| &self.graph.edges[i])
            .filter(|edge| {
                bounds.allows_layer(edge.layer) && bounds.allows_relation_kind(edge.kind)
            })
            .collect();

        let request_key = (node_id, direction, &bounds.layers, &bounds.relation_kinds);
        let page = match self.page_edges("graph.neighbors", &request_key, candidates, bounds) {
            Ok(page) => page,
            Err(error) => return self.invalid_cursor_envelope(error),
        };

        let relations: Vec<RelationFact> = page
            .items
            .iter()
            .map(|edge| RelationFact::from_edge(edge, RelationProvenance::Persisted))
            .collect();
        let mut neighbor_ids: Vec<&str> = Vec::new();
        for edge in &page.items {
            let other = if edge.from == node_id {
                edge.to.as_str()
            } else {
                edge.from.as_str()
            };
            if !neighbor_ids.contains(&other) {
                neighbor_ids.push(other);
            }
        }
        let neighbors: Vec<FactRef> = neighbor_ids.iter().filter_map(|id| self.fact(id)).collect();

        let returned_edges = relations.len();
        let result = NeighborsResult {
            node: node_fact,
            relations,
            neighbors,
        };
        self.envelope(
            Some(result),
            page.warnings,
            page.truncated,
            1,
            returned_edges,
            page.next_cursor,
        )
    }

    // ---- graph.dependencies ----------------------------------------

    pub fn dependencies(
        &self,
        node_id: &str,
        transitive: bool,
        bounds: &QueryBounds,
    ) -> QueryEnvelope<Option<DependenciesResult>> {
        let Some(node_fact) = self.fact(node_id) else {
            return self.not_found_envelope(format!("node not found: {node_id}"));
        };

        let mut warnings = Vec::new();
        let (edge_indices, node_ids, depth_truncated) = if transitive {
            self.bfs_forward(node_id, GraphEdgeKind::DependsOn, bounds)
        } else {
            let indices: Vec<usize> = self
                .index
                .outgoing_edges(node_id)
                .iter()
                .copied()
                .filter(|&i| self.graph.edges[i].kind == GraphEdgeKind::DependsOn)
                .collect();
            let nodes: Vec<String> = indices
                .iter()
                .map(|&i| self.graph.edges[i].to.clone())
                .collect();
            (indices, nodes, false)
        };
        if depth_truncated {
            warnings.push(format!(
                "transitive dependency search stopped at max_depth ({}) or max_nodes ({}) before exhausting the graph",
                bounds.max_depth, bounds.max_nodes
            ));
        }

        let candidates: Vec<&GraphEdge> = edge_indices
            .into_iter()
            .map(|i| &self.graph.edges[i])
            .collect();
        let request_key = (
            node_id,
            transitive,
            &bounds.relation_kinds,
            bounds.max_depth,
        );
        let page = match self.page_edges("graph.dependencies", &request_key, candidates, bounds) {
            Ok(page) => page,
            Err(error) => return self.invalid_cursor_envelope(error),
        };
        warnings.extend(page.warnings);

        let dependencies: Vec<RelationFact> = page
            .items
            .iter()
            .map(|edge| RelationFact::from_edge(edge, RelationProvenance::Persisted))
            .collect();
        let mut seen: Vec<String> = Vec::new();
        for id in &node_ids {
            if !seen.contains(id) {
                seen.push(id.clone());
            }
        }
        let nodes: Vec<FactRef> = seen
            .iter()
            .take(bounds.max_nodes)
            .filter_map(|id| self.fact(id))
            .collect();

        let returned_edges = dependencies.len();
        let returned_nodes = nodes.len();
        let result = DependenciesResult {
            node: node_fact,
            dependencies,
            nodes,
            transitive,
        };
        self.envelope(
            Some(result),
            warnings,
            page.truncated,
            returned_nodes,
            returned_edges,
            page.next_cursor,
        )
    }

    /// Bounded BFS over outgoing edges of `kind` from `start`, returning
    /// `(visited edge indices, visited node ids in BFS order, whether the
    /// search stopped due to a bound rather than exhausting the graph)`.
    fn bfs_forward(
        &self,
        start: &str,
        kind: GraphEdgeKind,
        bounds: &QueryBounds,
    ) -> (Vec<usize>, Vec<String>, bool) {
        let mut visited_nodes: Vec<String> = Vec::new();
        let mut visited_edges: Vec<usize> = Vec::new();
        let mut frontier: Vec<(String, usize)> = vec![(start.to_owned(), 0)];
        let mut seen_nodes: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        seen_nodes.insert(start.to_owned());
        let mut truncated = false;

        while let Some((current, depth)) = frontier.pop() {
            if depth >= bounds.max_depth {
                if !self.index.outgoing_edges(&current).is_empty() {
                    truncated = true;
                }
                continue;
            }
            let mut edges: Vec<usize> = self
                .index
                .outgoing_edges(&current)
                .iter()
                .copied()
                .filter(|&i| self.graph.edges[i].kind == kind)
                .collect();
            edges.sort_unstable();
            for index in edges {
                if visited_edges.len() >= bounds.max_edges
                    || visited_nodes.len() >= bounds.max_nodes
                {
                    truncated = true;
                    break;
                }
                let edge = &self.graph.edges[index];
                visited_edges.push(index);
                if seen_nodes.insert(edge.to.clone()) {
                    visited_nodes.push(edge.to.clone());
                    frontier.push((edge.to.clone(), depth + 1));
                }
            }
        }
        (visited_edges, visited_nodes, truncated)
    }

    // ---- graph.dependents -------------------------------------------

    /// Direct dependents of `node_id`, normalized regardless of whether
    /// the underlying fact is a persisted `ReverseDependency` edge
    /// (governance work items, Decision 0048/0049) or the query-derived
    /// inverse of an incoming `DependsOn` edge (packages). Every returned
    /// [`RelationFact`] uses `from = dependent`, `to = node_id` -- the
    /// natural reading direction for a dependents list -- regardless of
    /// which stored shape produced it.
    fn dependent_facts(&self, node_id: &str) -> Vec<RelationFact> {
        let mut by_dependent: BTreeMap<String, RelationFact> = BTreeMap::new();
        for &i in self.index.incoming_edges(node_id) {
            let edge = &self.graph.edges[i];
            if edge.kind == GraphEdgeKind::DependsOn {
                by_dependent.insert(
                    edge.from.clone(),
                    normalized_relation(
                        edge,
                        edge.from.clone(),
                        node_id.to_owned(),
                        RelationProvenance::QueryDerivedInverse,
                    ),
                );
            }
        }
        for &i in self.index.outgoing_edges(node_id) {
            let edge = &self.graph.edges[i];
            if edge.kind == GraphEdgeKind::ReverseDependency {
                by_dependent.insert(
                    edge.to.clone(),
                    normalized_relation(
                        edge,
                        edge.to.clone(),
                        node_id.to_owned(),
                        RelationProvenance::Persisted,
                    ),
                );
            }
        }
        by_dependent.into_values().collect()
    }

    pub fn dependents(
        &self,
        node_id: &str,
        bounds: &QueryBounds,
    ) -> QueryEnvelope<Option<DependentsResult>> {
        let Some(node_fact) = self.fact(node_id) else {
            return self.not_found_envelope(format!("node not found: {node_id}"));
        };
        let facts = self.dependent_facts(node_id);
        let mut warnings = Vec::new();
        let capped_len = facts.len().min(bounds.max_edges);
        if facts.len() > bounds.max_edges {
            warnings.push(format!(
                "dependent count exceeds max_edges bound ({}); only the first {} in stable order are considered",
                bounds.max_edges, bounds.max_edges
            ));
        }
        let request_key = (node_id, "dependents");
        let request_hash = cursor::request_identity(&request_key);
        let start = match &bounds.cursor {
            Some(raw) => match cursor::decode_and_validate_cursor(
                raw,
                QUERY_CONTRACT_VERSION,
                &self.context.graph_fingerprint,
                "graph.dependents",
                &request_hash,
            ) {
                Ok(pos) => pos as usize,
                Err(error) => return self.invalid_cursor_envelope(error),
            },
            None => 0,
        };
        let page_size = bounds.page_size.max(1);
        let end = (start + page_size).min(capped_len);
        let page: Vec<RelationFact> = if start < capped_len {
            facts[start..end].to_vec()
        } else {
            Vec::new()
        };
        let has_more = end < capped_len;
        let next_cursor = has_more.then(|| {
            cursor::encode_cursor(
                QUERY_CONTRACT_VERSION,
                &self.context.graph_fingerprint,
                "graph.dependents",
                &request_hash,
                end as u64,
            )
        });

        let nodes: Vec<FactRef> = page
            .iter()
            .filter_map(|fact| self.fact(&fact.from))
            .collect();
        let returned_edges = page.len();
        let returned_nodes = nodes.len();
        let result = DependentsResult {
            node: node_fact,
            dependents: page,
            nodes,
        };
        self.envelope(
            Some(result),
            warnings,
            has_more,
            returned_nodes,
            returned_edges,
            next_cursor,
        )
    }

    // ---- graph.path ---------------------------------------------------

    pub fn path(
        &self,
        from: &str,
        to: &str,
        bounds: &QueryBounds,
    ) -> QueryEnvelope<Option<PathResult>> {
        let Some(from_fact) = self.fact(from) else {
            return self.not_found_envelope(format!("node not found: {from}"));
        };
        let Some(to_fact) = self.fact(to) else {
            return self.not_found_envelope(format!("node not found: {to}"));
        };
        if from == to {
            let result = PathResult {
                from: from_fact.clone(),
                to: to_fact,
                path: PathOutcome::Found {
                    nodes: vec![from_fact],
                    edges: Vec::new(),
                },
            };
            return self.envelope(Some(result), Vec::new(), false, 1, 0, None);
        }

        // Deterministic bounded BFS: a plain FIFO queue with node IDs
        // visited in ascending order at each depth level, so the first
        // path found is always the same shortest path with a stable
        // tie-break (Decision 0050 section 3).
        let mut visited: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        visited.insert(from.to_owned());
        let mut queue: std::collections::VecDeque<(String, usize)> =
            std::collections::VecDeque::new();
        queue.push_back((from.to_owned(), 0));
        let mut predecessor: BTreeMap<String, (String, usize)> = BTreeMap::new();
        let mut nodes_visited = 1usize;
        let mut edges_visited = 0usize;
        let mut truncated = false;
        let mut found = false;

        'search: while let Some((current, depth)) = queue.pop_front() {
            if depth >= bounds.max_depth {
                if !self.index.outgoing_edges(&current).is_empty() {
                    truncated = true;
                }
                continue;
            }
            let mut edges: Vec<usize> = self.index.outgoing_edges(&current).to_vec();
            edges.sort_unstable();
            for index in edges {
                let edge = &self.graph.edges[index];
                if !bounds.allows_layer(edge.layer) || !bounds.allows_relation_kind(edge.kind) {
                    continue;
                }
                if edges_visited >= bounds.max_edges || nodes_visited >= bounds.max_nodes {
                    truncated = true;
                    break 'search;
                }
                edges_visited += 1;
                if visited.insert(edge.to.clone()) {
                    nodes_visited += 1;
                    predecessor.insert(edge.to.clone(), (current.clone(), index));
                    if edge.to == to {
                        found = true;
                        break 'search;
                    }
                    queue.push_back((edge.to.clone(), depth + 1));
                }
            }
        }

        let outcome = if found {
            let mut node_chain = vec![to.to_owned()];
            let mut edge_chain = Vec::new();
            let mut cursor_node = to.to_owned();
            while let Some((previous, edge_index)) = predecessor.get(&cursor_node) {
                edge_chain.push(*edge_index);
                node_chain.push(previous.clone());
                cursor_node = previous.clone();
                if cursor_node == from {
                    break;
                }
            }
            node_chain.reverse();
            edge_chain.reverse();
            let nodes: Vec<FactRef> = node_chain.iter().filter_map(|id| self.fact(id)).collect();
            let edges: Vec<RelationFact> = edge_chain
                .iter()
                .map(|&i| {
                    RelationFact::from_edge(&self.graph.edges[i], RelationProvenance::Persisted)
                })
                .collect();
            PathOutcome::Found { nodes, edges }
        } else if truncated {
            PathOutcome::SearchTruncatedBeforeProof
        } else {
            PathOutcome::NoPath
        };

        let warnings = if matches!(outcome, PathOutcome::SearchTruncatedBeforeProof) {
            vec![format!(
                "path search stopped at max_depth ({}) / max_nodes ({}) / max_edges ({}) before proving connectivity either way",
                bounds.max_depth, bounds.max_nodes, bounds.max_edges
            )]
        } else {
            Vec::new()
        };
        let (returned_nodes, returned_edges) = match &outcome {
            PathOutcome::Found { nodes, edges } => (nodes.len(), edges.len()),
            _ => (0, 0),
        };
        let result = PathResult {
            from: from_fact,
            to: to_fact,
            path: outcome,
        };
        self.envelope(
            Some(result),
            warnings,
            false,
            returned_nodes,
            returned_edges,
            None,
        )
    }

    // ---- graph.context --------------------------------------------

    pub fn context(
        &self,
        node_id: &str,
        bounds: &QueryBounds,
    ) -> QueryEnvelope<Option<ContextResult>> {
        let Some(identity) = self.fact(node_id) else {
            return self.not_found_envelope(format!("node not found: {node_id}"));
        };
        let mut indices: Vec<usize> = self.index.outgoing_edges(node_id).to_vec();
        indices.extend(self.index.incoming_edges(node_id));
        indices.sort_unstable();
        indices.dedup();

        let mut containment = Vec::new();
        let mut direct_relations = Vec::new();
        for &i in &indices {
            let edge = &self.graph.edges[i];
            if !bounds.allows_layer(edge.layer) || !bounds.allows_relation_kind(edge.kind) {
                continue;
            }
            let fact = RelationFact::from_edge(edge, RelationProvenance::Persisted);
            if edge.kind == GraphEdgeKind::Contains {
                containment.push(fact);
            } else {
                direct_relations.push(fact);
            }
        }
        let mut warnings = Vec::new();
        if containment.len() > bounds.max_edges {
            warnings.push(format!(
                "containment facts exceed max_edges bound ({})",
                bounds.max_edges
            ));
            containment.truncate(bounds.max_edges);
        }
        if direct_relations.len() > bounds.max_edges {
            warnings.push(format!(
                "direct relation facts exceed max_edges bound ({})",
                bounds.max_edges
            ));
            direct_relations.truncate(bounds.max_edges);
        }

        let mut related_ids: Vec<&str> = Vec::new();
        for fact in containment.iter().chain(direct_relations.iter()) {
            let other = if fact.from == node_id {
                fact.to.as_str()
            } else {
                fact.from.as_str()
            };
            if !related_ids.contains(&other) {
                related_ids.push(other);
            }
        }
        let related_nodes: Vec<FactRef> = related_ids
            .into_iter()
            .take(bounds.max_nodes)
            .filter_map(|id| self.fact(id))
            .collect();

        let returned_edges = containment.len() + direct_relations.len();
        let returned_nodes = 1 + related_nodes.len();
        let truncated = !warnings.is_empty();
        let result = ContextResult {
            identity,
            containment,
            direct_relations,
            related_nodes,
        };
        self.envelope(
            Some(result),
            warnings,
            truncated,
            returned_nodes,
            returned_edges,
            None,
        )
    }

    // ---- graph.tests --------------------------------------------------

    pub fn tests(&self, node_id: &str, bounds: &QueryBounds) -> QueryEnvelope<Option<TestsResult>> {
        let Some(node_fact) = self.fact(node_id) else {
            return self.not_found_envelope(format!("node not found: {node_id}"));
        };
        let relations = self.role_relations(node_id, crate::roles::TESTS, bounds);
        let mut test_target_ids: Vec<&str> = Vec::new();
        for fact in &relations {
            let other = if fact.from == node_id {
                fact.to.as_str()
            } else {
                fact.from.as_str()
            };
            if !test_target_ids.contains(&other) {
                test_target_ids.push(other);
            }
        }
        let test_targets: Vec<FactRef> = test_target_ids
            .iter()
            .filter_map(|id| self.fact(id))
            .collect();

        let mut warnings = vec![FIXTURE_COVERAGE_WARNING.to_owned()];
        if test_targets.is_empty() {
            warnings.push(
                "no direct test relation found for this target at the current extraction tier -- \
                 absence here does not prove no tests exist"
                    .to_owned(),
            );
        }
        let returned_edges = relations.len();
        let returned_nodes = 1 + test_targets.len();
        let result = TestsResult {
            node: node_fact,
            test_targets,
            relations,
        };
        self.envelope(
            Some(result),
            warnings,
            false,
            returned_nodes,
            returned_edges,
            None,
        )
    }

    fn role_relations(&self, node_id: &str, role: &str, bounds: &QueryBounds) -> Vec<RelationFact> {
        let mut indices: Vec<usize> = self.index.outgoing_edges(node_id).to_vec();
        indices.extend(self.index.incoming_edges(node_id));
        indices.sort_unstable();
        indices.dedup();
        let mut facts: Vec<RelationFact> = indices
            .into_iter()
            .map(|i| &self.graph.edges[i])
            .filter(|edge| edge.relation_role.as_ref().map(|r| r.as_str()) == Some(role))
            .map(|edge| RelationFact::from_edge(edge, RelationProvenance::Persisted))
            .collect();
        facts.truncate(bounds.max_edges);
        facts
    }

    // ---- graph.governance -----------------------------------------

    pub fn governance(
        &self,
        node_id: &str,
        bounds: &QueryBounds,
    ) -> QueryEnvelope<Option<GovernanceResult>> {
        let Some(node_fact) = self.fact(node_id) else {
            return self.not_found_envelope(format!("node not found: {node_id}"));
        };
        let mut indices: Vec<usize> = self.index.outgoing_edges(node_id).to_vec();
        indices.extend(self.index.incoming_edges(node_id));
        indices.sort_unstable();
        indices.dedup();
        let mut relations: Vec<RelationFact> = indices
            .into_iter()
            .map(|i| &self.graph.edges[i])
            .filter(|edge| GOVERNANCE_EDGE_KINDS.contains(&edge.kind))
            .map(|edge| RelationFact::from_edge(edge, RelationProvenance::Persisted))
            .collect();
        let mut warnings = Vec::new();
        if relations.len() > bounds.max_edges {
            warnings.push(format!(
                "governance relations exceed max_edges bound ({})",
                bounds.max_edges
            ));
            relations.truncate(bounds.max_edges);
        }
        let mut fact_ids: Vec<&str> = Vec::new();
        for relation in &relations {
            let other = if relation.from == node_id {
                relation.to.as_str()
            } else {
                relation.from.as_str()
            };
            if !fact_ids.contains(&other) {
                fact_ids.push(other);
            }
        }
        let facts: Vec<FactRef> = fact_ids
            .into_iter()
            .filter_map(|id| self.fact(id))
            .collect();
        let returned_edges = relations.len();
        let returned_nodes = 1 + facts.len();
        let result = GovernanceResult {
            node: node_fact,
            relations,
            facts,
        };
        self.envelope(
            Some(result),
            warnings,
            false,
            returned_nodes,
            returned_edges,
            None,
        )
    }

    // ---- graph.impact ---------------------------------------------

    pub fn impact(
        &self,
        node_id: &str,
        bounds: &QueryBounds,
    ) -> QueryEnvelope<Option<ImpactResult>> {
        let Some(target) = self.fact(node_id) else {
            return self.not_found_envelope(format!("node not found: {node_id}"));
        };
        let dependents: Vec<FactRef> = self
            .dependent_facts(node_id)
            .iter()
            .take(bounds.max_nodes)
            .filter_map(|fact| self.fact(&fact.from))
            .collect();
        let test_relations = self.role_relations(node_id, crate::roles::TESTS, bounds);
        let mut test_ids: Vec<&str> = Vec::new();
        for fact in &test_relations {
            let other = if fact.from == node_id {
                fact.to.as_str()
            } else {
                fact.from.as_str()
            };
            if !test_ids.contains(&other) {
                test_ids.push(other);
            }
        }
        let test_targets: Vec<FactRef> = test_ids.iter().filter_map(|id| self.fact(id)).collect();

        let mut surface_indices: Vec<usize> = self.index.outgoing_edges(node_id).to_vec();
        surface_indices.extend(self.index.incoming_edges(node_id));
        surface_indices.sort_unstable();
        surface_indices.dedup();
        let mut surface_ids: Vec<&str> = Vec::new();
        for &i in &surface_indices {
            let edge = &self.graph.edges[i];
            if matches!(
                edge.layer,
                GraphLayer::Build | GraphLayer::Package | GraphLayer::Runtime
            ) {
                let other = if edge.from == node_id {
                    edge.to.as_str()
                } else {
                    edge.from.as_str()
                };
                if !surface_ids.contains(&other) {
                    surface_ids.push(other);
                }
            }
        }
        let build_package_runtime_surfaces: Vec<FactRef> = surface_ids
            .into_iter()
            .take(bounds.max_nodes)
            .filter_map(|id| self.fact(id))
            .collect();

        let governance_relations: Vec<RelationFact> = surface_indices
            .iter()
            .map(|&i| &self.graph.edges[i])
            .filter(|edge| GOVERNANCE_EDGE_KINDS.contains(&edge.kind))
            .map(|edge| RelationFact::from_edge(edge, RelationProvenance::Persisted))
            .collect();
        let mut governance_ids: Vec<&str> = Vec::new();
        for relation in &governance_relations {
            let other = if relation.from == node_id {
                relation.to.as_str()
            } else {
                relation.from.as_str()
            };
            if !governance_ids.contains(&other) {
                governance_ids.push(other);
            }
        }
        let applicable_governance: Vec<FactRef> = governance_ids
            .into_iter()
            .take(bounds.max_nodes)
            .filter_map(|id| self.fact(id))
            .collect();

        let mut warnings = vec![
            "impact_semantics = structural_only: this reflects graph structure, not proven runtime behavior or test outcome".to_owned(),
            FIXTURE_COVERAGE_WARNING.to_owned(),
        ];
        if test_targets.is_empty() {
            warnings.push(
                "no direct test relation found for this target at the current extraction tier"
                    .to_owned(),
            );
        }
        let returned_nodes = 1
            + dependents.len()
            + test_targets.len()
            + build_package_runtime_surfaces.len()
            + applicable_governance.len();
        let result = ImpactResult {
            target,
            impact_semantics: "structural_only",
            dependents,
            test_targets,
            build_package_runtime_surfaces,
            applicable_governance,
        };
        self.envelope(Some(result), warnings, false, returned_nodes, 0, None)
    }

    // ---- graph.orient -----------------------------------------------

    pub fn orient(
        &self,
        selector: &NodeSelector,
        bounds: &QueryBounds,
    ) -> QueryEnvelope<OrientOutcome> {
        let identity = match self.resolve_exact(selector) {
            Ok(fact) => fact,
            Err(ResolutionOutcome::Ambiguous { candidates }) => {
                return self.envelope(
                    OrientOutcome::Ambiguous {
                        candidates: candidates.clone(),
                    },
                    Vec::new(),
                    false,
                    candidates.len(),
                    0,
                    None,
                );
            }
            Err(_) => return self.envelope(OrientOutcome::NotFound, Vec::new(), false, 0, 0, None),
        };
        let node_id = identity.id.clone();

        let context = self.context(&node_id, bounds);
        let containment = context
            .result
            .as_ref()
            .map(|c| c.containment.clone())
            .unwrap_or_default();

        let dependencies = self.dependencies(&node_id, false, bounds);
        let direct_dependencies = dependencies
            .result
            .as_ref()
            .map(|d| d.dependencies.clone())
            .unwrap_or_default();

        let dependents_result = self.dependents(&node_id, bounds);
        let direct_dependents = dependents_result
            .result
            .as_ref()
            .map(|d| d.dependents.clone())
            .unwrap_or_default();

        let tests_result = self.tests(&node_id, bounds);
        let relevant_tests = tests_result
            .result
            .as_ref()
            .map(|t| t.test_targets.clone())
            .unwrap_or_default();

        let mut runtime_entrypoints: Vec<FactRef> = Vec::new();
        let mut build_surfaces: Vec<FactRef> = Vec::new();
        let mut package_surfaces: Vec<FactRef> = Vec::new();
        let mut surface_indices: Vec<usize> = self.index.outgoing_edges(&node_id).to_vec();
        surface_indices.extend(self.index.incoming_edges(&node_id));
        surface_indices.sort_unstable();
        surface_indices.dedup();
        for &i in &surface_indices {
            let edge = &self.graph.edges[i];
            let other = if edge.from == node_id {
                edge.to.as_str()
            } else {
                edge.from.as_str()
            };
            let Some(other_fact) = self.fact(other) else {
                continue;
            };
            match edge.layer {
                GraphLayer::Runtime => {
                    if !runtime_entrypoints.iter().any(|f| f.id == other_fact.id) {
                        runtime_entrypoints.push(other_fact);
                    }
                }
                GraphLayer::Build => {
                    if !build_surfaces.iter().any(|f| f.id == other_fact.id) {
                        build_surfaces.push(other_fact);
                    }
                }
                GraphLayer::Package => {
                    if !package_surfaces.iter().any(|f| f.id == other_fact.id) {
                        package_surfaces.push(other_fact);
                    }
                }
                _ => {}
            }
        }
        runtime_entrypoints.truncate(bounds.max_nodes);
        build_surfaces.truncate(bounds.max_nodes);
        package_surfaces.truncate(bounds.max_nodes);

        let governance_result = self.governance(&node_id, bounds);
        let applicable_governance = governance_result
            .result
            .as_ref()
            .map(|g| g.facts.clone())
            .unwrap_or_default();

        let mut hints: Vec<NavigationHint> = Vec::new();
        if identity.source.is_some() {
            hints.push(NavigationHint {
                subject: identity.clone(),
                reason: HintReason::TargetSource,
                rank: 0,
            });
        }
        for relation in &containment {
            if relation.to == node_id {
                if let Some(fact) = self.fact(&relation.from) {
                    if fact.kind == GraphNodeKind::Manifest {
                        hints.push(NavigationHint {
                            subject: fact,
                            reason: HintReason::OwningManifest,
                            rank: 0,
                        });
                    }
                }
            }
        }
        for relation in &direct_dependencies {
            if let Some(fact) = self.fact(&relation.to) {
                hints.push(NavigationHint {
                    subject: fact,
                    reason: HintReason::DirectDependency,
                    rank: 0,
                });
            }
        }
        for relation in &direct_dependents {
            if let Some(fact) = self.fact(&relation.from) {
                hints.push(NavigationHint {
                    subject: fact,
                    reason: HintReason::DirectDependent,
                    rank: 0,
                });
            }
        }
        for fact in &relevant_tests {
            hints.push(NavigationHint {
                subject: fact.clone(),
                reason: HintReason::RelevantTest,
                rank: 0,
            });
        }
        for fact in &runtime_entrypoints {
            hints.push(NavigationHint {
                subject: fact.clone(),
                reason: HintReason::RuntimeEntrypoint,
                rank: 0,
            });
        }
        for fact in &applicable_governance {
            hints.push(NavigationHint {
                subject: fact.clone(),
                reason: HintReason::ApplicableGovernance,
                rank: 0,
            });
        }
        // Stable priority table (step 28): reason priority, then the
        // subject's own stable ID as the final tie-breaker.
        hints.sort_by(|a, b| {
            a.reason
                .priority()
                .cmp(&b.reason.priority())
                .then_with(|| a.subject.id.cmp(&b.subject.id))
        });
        hints.dedup_by(|a, b| a.subject.id == b.subject.id && a.reason == b.reason);
        hints.truncate(bounds.max_nodes);
        for (index, hint) in hints.iter_mut().enumerate() {
            hint.rank = index as u32 + 1;
        }

        let mut warnings = vec![FIXTURE_COVERAGE_WARNING.to_owned()];
        warnings.extend(context.warnings.clone());
        warnings.extend(dependencies.warnings.clone());
        warnings.extend(dependents_result.warnings.clone());
        warnings.extend(governance_result.warnings.clone());
        warnings.sort();
        warnings.dedup();

        let result = OrientResult {
            identity,
            containment,
            direct_dependencies,
            direct_dependents,
            relevant_tests,
            build_surfaces,
            package_surfaces,
            runtime_entrypoints,
            applicable_governance,
            navigation_hints: hints,
        };
        let returned_nodes = 1
            + result.relevant_tests.len()
            + result.build_surfaces.len()
            + result.package_surfaces.len()
            + result.runtime_entrypoints.len()
            + result.applicable_governance.len();
        let returned_edges = result.containment.len()
            + result.direct_dependencies.len()
            + result.direct_dependents.len();
        self.envelope(
            OrientOutcome::Resolved(result),
            warnings,
            false,
            returned_nodes,
            returned_edges,
            None,
        )
    }
}

fn normalized_relation(
    edge: &GraphEdge,
    from: String,
    to: String,
    provenance: RelationProvenance,
) -> RelationFact {
    RelationFact {
        from,
        to,
        kind: edge.kind,
        layer: edge.layer,
        role: edge.relation_role.clone(),
        derivation: edge.derivation,
        source: edge.source.clone(),
        location: edge.location,
        provenance,
    }
}
