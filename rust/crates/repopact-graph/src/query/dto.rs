//! Versioned query result DTOs (Decision 0050 sections 9/11). Internal
//! [`crate::GraphNode`]/[`crate::GraphEdge`] maps are never exposed as an
//! accidental wire format -- every result is one of these explicit
//! types, and facts are always kept structurally separate from
//! [`NavigationHint`]s.

use serde::{Deserialize, Serialize};

use crate::overlay::EffectiveGraphStatus;
use crate::{
    DerivationClass, GraphEdge, GraphEdgeKind, GraphLayer, GraphNode, GraphNodeKind, GraphNodeRole,
    GraphRelationRole, GraphSourceLocation, ManifestKind, SourceRef, SymbolKind,
};

/// A stable, self-contained view of one graph node -- never the internal
/// [`GraphNode`] type re-exported verbatim, so the wire shape can evolve
/// independently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactRef {
    pub id: String,
    pub kind: GraphNodeKind,
    pub label: String,
    pub layer: GraphLayer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<GraphNodeRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_kind: Option<SymbolKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_kind: Option<ManifestKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<GraphSourceLocation>,
}

impl FactRef {
    pub fn from_node(node: &GraphNode) -> Self {
        Self {
            id: node.id.clone(),
            kind: node.kind,
            label: node.label.clone(),
            layer: node.layer,
            role: node.node_role.clone(),
            symbol_kind: node.symbol_kind,
            manifest_kind: node.manifest_kind,
            source: node.source.clone(),
            location: node.location,
        }
    }

    /// Compact mode (ROG-025 step 41): drop the presentation label,
    /// retain the stable ID, source reference, and role/kind metadata.
    pub fn into_compact(mut self) -> Self {
        self.label = String::new();
        self
    }
}

/// Where a [`RelationFact`] actually came from -- persisted in the graph
/// as written, or derived at query time as the inverse of a persisted
/// edge (Decision 0050 section 12 / Decision 0048 section 7 / Decision
/// 0049 section 5). A caller never needs to know which representation
/// was stored to interpret this field correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationProvenance {
    Persisted,
    QueryDerivedInverse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationFact {
    pub from: String,
    pub to: String,
    pub kind: GraphEdgeKind,
    pub layer: GraphLayer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<GraphRelationRole>,
    pub derivation: DerivationClass,
    pub source: SourceRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<GraphSourceLocation>,
    pub provenance: RelationProvenance,
}

impl RelationFact {
    pub fn from_edge(edge: &GraphEdge, provenance: RelationProvenance) -> Self {
        Self {
            from: edge.from.clone(),
            to: edge.to.clone(),
            kind: edge.kind,
            layer: edge.layer,
            role: edge.relation_role.clone(),
            derivation: edge.derivation,
            source: edge.source.clone(),
            location: edge.location,
            provenance,
        }
    }
}

/// Target resolution result (Decision 0050 section 2): never a silent
/// first-match guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ResolutionOutcome {
    Exact { fact: FactRef },
    Ambiguous { candidates: Vec<FactRef> },
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Outgoing,
    Incoming,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeighborsResult {
    pub node: FactRef,
    pub relations: Vec<RelationFact>,
    /// The neighboring node at the far end of each relation in
    /// `relations`, deduplicated and in the same relative order.
    pub neighbors: Vec<FactRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependenciesResult {
    pub node: FactRef,
    pub dependencies: Vec<RelationFact>,
    pub nodes: Vec<FactRef>,
    pub transitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependentsResult {
    pub node: FactRef,
    pub dependents: Vec<RelationFact>,
    pub nodes: Vec<FactRef>,
}

/// A single deterministic shortest path, or a typed reason none was
/// found (Decision 0050 section 3 / step 19): `NoPath` (search completed
/// within bounds and proved no connection) is always distinguished from
/// `SearchTruncated` (the bound was hit before a proof either way).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PathOutcome {
    Found {
        nodes: Vec<FactRef>,
        edges: Vec<RelationFact>,
    },
    NoPath,
    SearchTruncatedBeforeProof,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathResult {
    pub from: FactRef,
    pub to: FactRef,
    pub path: PathOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextResult {
    pub identity: FactRef,
    pub containment: Vec<RelationFact>,
    pub direct_relations: Vec<RelationFact>,
    pub related_nodes: Vec<FactRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestsResult {
    pub node: FactRef,
    pub test_targets: Vec<FactRef>,
    pub relations: Vec<RelationFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceResult {
    pub node: FactRef,
    pub relations: Vec<RelationFact>,
    pub facts: Vec<FactRef>,
}

/// Structural graph impact, never behavioral certainty (Decision 0050 /
/// step 25). `impact_semantics` is always `"structural_only"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactResult {
    pub target: FactRef,
    pub impact_semantics: &'static str,
    pub dependents: Vec<FactRef>,
    pub test_targets: Vec<FactRef>,
    pub build_package_runtime_surfaces: Vec<FactRef>,
    pub applicable_governance: Vec<FactRef>,
}

/// A deterministic, reason-coded "inspect this first" entry (ROG-024
/// step 27). Never mixed with facts, never model-scored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigationHint {
    pub subject: FactRef,
    pub reason: HintReason,
    pub rank: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HintReason {
    TargetSource,
    OwningManifest,
    DirectDependency,
    DirectDependent,
    RelevantTest,
    RuntimeEntrypoint,
    ApplicableGovernance,
}

impl HintReason {
    /// Stable priority table (step 28): lower sorts first. Ties within a
    /// reason break on the subject's own stable ID.
    pub fn priority(self) -> u32 {
        match self {
            Self::TargetSource => 0,
            Self::OwningManifest => 1,
            Self::DirectDependency => 2,
            Self::DirectDependent => 3,
            Self::RelevantTest => 4,
            Self::RuntimeEntrypoint => 5,
            Self::ApplicableGovernance => 6,
        }
    }
}

/// `graph.orient`'s outcome: resolution ambiguity/not-found is surfaced
/// the same honest way `graph.resolve` does, never silently picking a
/// candidate (Decision 0050 section 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum OrientOutcome {
    Resolved(OrientResult),
    Ambiguous { candidates: Vec<FactRef> },
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrientResult {
    pub identity: FactRef,
    pub containment: Vec<RelationFact>,
    pub direct_dependencies: Vec<RelationFact>,
    pub direct_dependents: Vec<RelationFact>,
    pub relevant_tests: Vec<FactRef>,
    pub build_surfaces: Vec<FactRef>,
    pub package_surfaces: Vec<FactRef>,
    pub runtime_entrypoints: Vec<FactRef>,
    pub applicable_governance: Vec<FactRef>,
    pub navigation_hints: Vec<NavigationHint>,
}

/// `graph.search`'s deterministic ranking tier (Decision 0052 section
/// 3): exact match on a searchable field, then an exact match after
/// case/whitespace normalization, then a prefix match, then a substring
/// match. Never a fuzzy/embedding/semantic-similarity score -- search
/// results are navigation candidates over deterministic graph fields
/// only, never a new graph fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchRank {
    Exact,
    ExactNormalized,
    Prefix,
    Substring,
}

/// Which deterministic field a [`SearchMatch`] matched on -- disclosed
/// so an operator (or the UI) can tell *why* a candidate matched, never
/// hidden inside an opaque score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchField {
    StableId,
    RepositoryRelativePath,
    Label,
    NodeRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchMatch {
    pub node: FactRef,
    pub rank: SearchRank,
    pub matched_field: SearchField,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    pub query: String,
    pub matches: Vec<SearchMatch>,
}

/// Every query response envelope: graph state disclosure plus the
/// operation's own typed result. `T` is never allowed to hide freshness/
/// coverage information behind an opaque success (Decision 0050 sections
/// 7/9/10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryEnvelope<T> {
    pub query_contract_version: u32,
    pub graph_schema_version: u32,
    pub graph_fingerprint: String,
    pub status: EffectiveGraphStatus,
    pub warnings: Vec<String>,
    pub truncated: bool,
    pub returned_nodes: usize,
    pub returned_edges: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub result: T,
}
