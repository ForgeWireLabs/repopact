use std::collections::BTreeMap;
use std::path::Path;

use repopact_repository::{IndexedRecord, RepositorySnapshot};
use repopact_types::{RecordKind, RecordRef, SourceRef, WorkItem};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod capability;
pub mod durable;
pub mod incremental;
pub mod metadata;
pub mod overlay;
pub mod physical;
pub mod projection;
pub mod query;
pub mod semantic;
pub mod status;
pub mod validate;

/// Which orientation domain a node/edge belongs to (WI063 ROG-002/003).
/// `Governance` is the pre-existing WI054 domain and is the default so
/// every governance node/edge constructed before this field existed keeps
/// its exact prior meaning. Only `Governance` and `Physical` are populated
/// by any builder in this session; the remaining variants are declared now
/// so later phases (semantic language adapters, build/test/runtime
/// topology) extend the same typed vocabulary instead of inventing a
/// parallel one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphLayer {
    #[default]
    Governance,
    Physical,
    Semantic,
    Build,
    Test,
    Runtime,
    Package,
}

/// Why a relationship exists (WI063 ROG-005/006). `CanonicalRecord` is the
/// default and describes every governance edge WI054 already produces
/// (derived directly from a governed RepoPact record, not a heuristic).
/// `Heuristic`/`Inferred` are declared for future use but are never
/// produced by any builder in this session -- LLM or heuristic output is
/// never silently promoted to concrete graph authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivationClass {
    #[default]
    CanonicalRecord,
    Filesystem,
    Manifest,
    Parser,
    BuildMetadata,
    Heuristic,
    Inferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphNodeKind {
    Repository,
    WorkItem,
    AcceptanceCriterion,
    EvidenceRun,
    Scope,
    Role,
    Decision,
    Policy,
    Contract,
    Invariant,
    FrozenSurface,
    AuditFinding,
    // Physical topology (WI063 ROG-003).
    Directory,
    File,
    Workspace,
    ConfigurationFile,
    NestedRepository,
    // Code semantics (WI063 ROG-003, Decision 0045; schema v2). A single
    // Symbol kind plus a typed SymbolKind field, deliberately not one
    // GraphNodeKind variant per language/symbol-category -- see Decision
    // 0045 section 3 for why.
    Symbol,
    // Structured metadata / operational topology (WI063 ROG-018/019,
    // Decision 0048; schema v3). A single Manifest kind plus a typed
    // ManifestKind field, the same "generic node + typed sub-kind"
    // pattern as Symbol/SymbolKind above -- see Decision 0048 section 1.
    Manifest,
}

/// Structured-metadata/operational category (Decision 0048). New
/// ordinary metadata categories are added here, not as new
/// `GraphNodeKind` variants -- mirroring the `SymbolKind` precedent
/// (Decision 0045 section 3), with the same caveat disclosed in Decision
/// 0048's Consequences: adding a variant here still requires a schema-
/// major bump for readers compiled before it existed, exactly like
/// `SymbolKind` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestKind {
    CargoPackage,
    PythonProject,
    NodePackage,
    TypeScriptConfig,
    CiWorkflow,
    JsonDocument,
    TomlDocument,
    YamlDocument,
    MarkdownDocument,
}

/// Language-neutral symbol category (Decision 0045 section 3). New
/// ordinary symbol categories in an already-supported language are added
/// here, not as new `GraphNodeKind` variants, so growing language
/// coverage does not require another schema-major bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Module,
    Function,
    Method,
    Type,
    Enum,
    Interface,
    Implementation,
    TypeAlias,
    Constant,
    Macro,
    Test,
}

/// Deterministic, UTF-8-byte-offset source location for a graph node/edge
/// (Decision 0045 section 4). Explanatory metadata only -- never part of a
/// symbol's primary identity, never a validation precondition. Kept
/// separate from the shared governance `SourceRef`/`RecordRef` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GraphSourceLocation {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_row: usize,
    pub start_column: usize,
    pub end_row: usize,
    pub end_column: usize,
}

/// A bounded, syntax-validated, **open** vocabulary token (Decision 0049)
/// -- deliberately not a closed enum. Serializes as an ordinary string;
/// an unrecognized-but-well-formed value is never a deserialization
/// error, so growing this vocabulary never requires a schema-major bump.
/// Construction (not deserialization) enforces the bounded syntax: a
/// non-empty, at most 64-byte, lowercase-ASCII-letters/digits/
/// underscores token starting with a letter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphNodeRole(String);

impl GraphNodeRole {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        is_valid_role_token(&value).then_some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The relation-side counterpart to [`GraphNodeRole`] (Decision 0049).
/// Same bounded-syntax, open-vocabulary contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphRelationRole(String);

impl GraphRelationRole {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        is_valid_role_token(&value).then_some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_valid_role_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_lowercase())
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Well-known role constants used by RepoPact's own emitters (Decision
/// 0049 section 4). This is a *starting* vocabulary, not a closed list --
/// a producer may construct any syntactically valid `GraphNodeRole`/
/// `GraphRelationRole`, known to this module or not.
pub mod roles {
    pub const TEST_TARGET: &str = "test_target";
    pub const GENERATED_SURFACE: &str = "generated_surface";
    pub const INSTALLER_SURFACE: &str = "installer_surface";
    pub const RUNTIME_ENTRYPOINT: &str = "runtime_entrypoint";

    pub const TESTS: &str = "tests";
    pub const GENERATED_BY: &str = "generated_by";
    pub const ENTRY_POINT_FOR: &str = "entry_point_for";
    pub const INSTALLER_FOR: &str = "installer_for";
    pub const WORKSPACE_MEMBER: &str = "workspace_member";
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: GraphNodeKind,
    pub label: String,
    #[serde(default)]
    pub layer: GraphLayer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_kind: Option<SymbolKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<GraphSourceLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_kind: Option<ManifestKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_role: Option<GraphNodeRole>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphEdgeKind {
    DependsOn,
    ReverseDependency,
    Contains,
    SupportedBy,
    SupportsWorkItem,
    OwnedBy,
    Affects,
    Supersedes,
    Concerns,
    ConstrainedBy,
    Intersects,
    AppliesTo,
    Allows,
    // Physical topology (WI063 ROG-004).
    BelongsToWorkspace,
    ConfiguredBy,
    // Code semantics (WI063 ROG-004, Decision 0045; schema v2). Only
    // Defines and Imports are actually emitted this checkpoint; the rest
    // are predeclared per Decision 0045 section 3 and must not be emitted
    // until a real adapter backs them.
    Defines,
    Imports,
    Exports,
    Implements,
    Extends,
    References,
    Calls,
    UsesType,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: GraphEdgeKind,
    #[serde(default)]
    pub layer: GraphLayer,
    #[serde(default)]
    pub derivation: DerivationClass,
    pub source: SourceRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<GraphSourceLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_role: Option<GraphRelationRole>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryGraph {
    pub nodes: BTreeMap<String, GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl RepositoryGraph {
    pub fn build(snapshot: &RepositorySnapshot) -> Self {
        Self::build_with_fingerprint(snapshot).0
    }

    /// Same as [`Self::build`], but also returns the Decision 0044 source
    /// projection fingerprint computed along the way, so callers that need
    /// both (durable writes) never recompute the projection a second time
    /// -- recomputing it would mean a second `RepositoryTopology`-consuming
    /// walk, which callers building on top of an already-open
    /// `RepositorySnapshot` must avoid to keep the WI057 bounded-git-
    /// invocation guarantee intact.
    pub fn build_with_fingerprint(
        snapshot: &RepositorySnapshot,
    ) -> (Self, String, semantic::SemanticCoverage) {
        let (mut graph, source_projection) = Self::build_governance_and_physical(snapshot);
        let semantic_coverage =
            semantic::extend(&mut graph, snapshot.repository(), &source_projection);
        graph.edges.sort();
        graph.edges.dedup();
        (graph, source_projection.fingerprint(), semantic_coverage)
    }

    /// Governance + physical topology only (WI063 incremental-equivalence
    /// checkpoint, step 1): both layers are deterministic and cheap enough
    /// to rebuild globally on every `graph.update`, so only semantic
    /// extraction needs contribution-level reuse. [`crate::incremental`]
    /// calls this directly rather than duplicating governance/physical
    /// construction; a full build ([`Self::build_with_fingerprint`]) is
    /// this plus an unconditional full semantic pass. Callers must apply
    /// their own `graph.edges.sort(); graph.edges.dedup();` after adding
    /// semantic edges -- this function does not, since an incremental
    /// caller still has more edges to add.
    pub(crate) fn build_governance_and_physical(
        snapshot: &RepositorySnapshot,
    ) -> (Self, projection::SourceProjection) {
        let source_projection =
            projection::SourceProjection::build(snapshot.repository(), snapshot.topology());
        let graph =
            Self::build_governance_and_physical_with_projection(snapshot, &source_projection);
        (graph, source_projection)
    }

    /// Same as [`Self::build_governance_and_physical`], but takes an
    /// already-computed [`projection::SourceProjection`] instead of
    /// building a fresh one. `incremental::update` already computes the
    /// projection once (to compare fingerprints and classify the delta)
    /// before deciding an incremental path is safe; without this, it
    /// would otherwise pay for a second full projection walk (hashing
    /// every projected file's content again) purely to rebuild the
    /// always-global governance/physical layers -- on a large repository
    /// that walk, not semantic parsing, dominates wall time, so avoiding
    /// the duplicate is a real cost win, not a cosmetic one.
    pub(crate) fn build_governance_and_physical_with_projection(
        snapshot: &RepositorySnapshot,
        source_projection: &projection::SourceProjection,
    ) -> Self {
        let mut graph = Self::default();
        let repository_source = RecordRef::new(RecordKind::Repository, "repository", "<root>");
        graph.node(GraphNode {
            id: "repository".to_owned(),
            kind: GraphNodeKind::Repository,
            label: "RepoPact repository".to_owned(),
            symbol_kind: None,
            manifest_kind: None,
            node_role: None,
            location: None,
            layer: GraphLayer::Governance,
            source: Some(repository_source.clone()),
        });

        for record in &snapshot.index().work_items {
            let Some(item) = typed_work(record) else {
                continue;
            };
            let work_id = work_node(&item.id);
            graph.node(GraphNode {
                id: work_id.clone(),
                kind: GraphNodeKind::WorkItem,
                label: item.title.clone(),
                symbol_kind: None,
                manifest_kind: None,
                node_role: None,
                location: None,
                layer: GraphLayer::Governance,
                source: Some(record.reference.clone()),
            });
            for criterion in &item.acceptance_criteria {
                let criterion_id = criterion_node(&item.id, &criterion.id);
                graph.node(GraphNode {
                    id: criterion_id.clone(),
                    kind: GraphNodeKind::AcceptanceCriterion,
                    label: criterion.text.clone(),
                    symbol_kind: None,
                    manifest_kind: None,
                    node_role: None,
                    location: None,
                    layer: GraphLayer::Governance,
                    source: Some(RecordRef::new(
                        RecordKind::AcceptanceCriterion,
                        format!("{}:{}", item.id, criterion.id),
                        record.reference.path.clone(),
                    )),
                });
                graph.edge(GraphEdge {
                    from: work_id.clone(),
                    to: criterion_id.clone(),
                    kind: GraphEdgeKind::Contains,
                    layer: GraphLayer::Governance,
                    derivation: DerivationClass::CanonicalRecord,
                    location: None,
                    relation_role: None,
                    source: record.reference.clone(),
                });
                for evidence_id in &criterion.evidence {
                    let evidence_node_id = evidence_node(evidence_id);
                    if let Some(evidence) = snapshot
                        .index()
                        .evidence
                        .iter()
                        .find(|candidate| candidate.reference.id == *evidence_id)
                    {
                        graph.node(GraphNode {
                            id: evidence_node_id.clone(),
                            kind: GraphNodeKind::EvidenceRun,
                            label: evidence_id.clone(),
                            symbol_kind: None,
                            manifest_kind: None,
                            node_role: None,
                            location: None,
                            layer: GraphLayer::Governance,
                            source: Some(evidence.reference.clone()),
                        });
                        graph.edge(GraphEdge {
                            from: criterion_id.clone(),
                            to: evidence_node_id.clone(),
                            kind: GraphEdgeKind::SupportedBy,
                            layer: GraphLayer::Governance,
                            derivation: DerivationClass::CanonicalRecord,
                            location: None,
                            relation_role: None,
                            source: RecordRef::new(
                                RecordKind::AcceptanceCriterion,
                                format!("{}:{}", item.id, criterion.id),
                                record.reference.path.clone(),
                            ),
                        });
                    }
                }
            }
            for dependency in &item.depends_on {
                let dependency_node = work_node(dependency);
                graph.edge(GraphEdge {
                    from: work_id.clone(),
                    to: dependency_node.clone(),
                    kind: GraphEdgeKind::DependsOn,
                    layer: GraphLayer::Governance,
                    derivation: DerivationClass::CanonicalRecord,
                    location: None,
                    relation_role: None,
                    source: record.reference.clone(),
                });
                graph.edge(GraphEdge {
                    from: dependency_node,
                    to: work_id.clone(),
                    kind: GraphEdgeKind::ReverseDependency,
                    layer: GraphLayer::Governance,
                    derivation: DerivationClass::CanonicalRecord,
                    location: None,
                    relation_role: None,
                    source: record.reference.clone(),
                });
            }
            let owner_source = snapshot
                .index()
                .owners
                .as_ref()
                .map(|owners| owners.reference.clone())
                .unwrap_or_else(|| record.reference.clone());
            let owner_node = scope_node(&item.owner_scope);
            graph.node(GraphNode {
                id: owner_node.clone(),
                kind: GraphNodeKind::Scope,
                label: item.owner_scope.clone(),
                symbol_kind: None,
                manifest_kind: None,
                node_role: None,
                location: None,
                layer: GraphLayer::Governance,
                source: Some(owner_source.clone()),
            });
            graph.edge(GraphEdge {
                from: work_id.clone(),
                to: owner_node,
                kind: GraphEdgeKind::OwnedBy,
                layer: GraphLayer::Governance,
                derivation: DerivationClass::CanonicalRecord,
                location: None,
                relation_role: None,
                source: record.reference.clone(),
            });
            for scope in &item.affected_scopes {
                let scope_node_id = scope_node(scope);
                graph.node(GraphNode {
                    id: scope_node_id.clone(),
                    kind: GraphNodeKind::Scope,
                    label: scope.clone(),
                    symbol_kind: None,
                    manifest_kind: None,
                    node_role: None,
                    location: None,
                    layer: GraphLayer::Governance,
                    source: Some(owner_source.clone()),
                });
                graph.edge(GraphEdge {
                    from: work_id.clone(),
                    to: scope_node_id,
                    kind: GraphEdgeKind::Affects,
                    layer: GraphLayer::Governance,
                    derivation: DerivationClass::CanonicalRecord,
                    location: None,
                    relation_role: None,
                    source: record.reference.clone(),
                });
            }
            for contract in &snapshot.index().contracts {
                if path_is_under(
                    &record.path,
                    contract.path.parent().unwrap_or(&contract.path),
                ) {
                    graph.node(GraphNode {
                        id: contract_node(&contract.reference.id),
                        kind: GraphNodeKind::Contract,
                        label: contract.reference.id.clone(),
                        symbol_kind: None,
                        manifest_kind: None,
                        node_role: None,
                        location: None,
                        layer: GraphLayer::Governance,
                        source: Some(contract.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: work_id.clone(),
                        to: contract_node(&contract.reference.id),
                        kind: GraphEdgeKind::ConstrainedBy,
                        layer: GraphLayer::Governance,
                        derivation: DerivationClass::CanonicalRecord,
                        location: None,
                        relation_role: None,
                        source: contract.reference.clone(),
                    });
                }
            }
        }

        for evidence in &snapshot.index().evidence {
            let Some(value) = evidence.value.as_ref().ok() else {
                continue;
            };
            let Some(work_id) = value.get("work_item").and_then(Value::as_str) else {
                continue;
            };
            graph.node(GraphNode {
                id: evidence_node(&evidence.reference.id),
                kind: GraphNodeKind::EvidenceRun,
                label: evidence.reference.id.clone(),
                symbol_kind: None,
                manifest_kind: None,
                node_role: None,
                location: None,
                layer: GraphLayer::Governance,
                source: Some(evidence.reference.clone()),
            });
            graph.edge(GraphEdge {
                from: evidence_node(&evidence.reference.id),
                to: work_node(work_id),
                kind: GraphEdgeKind::SupportsWorkItem,
                layer: GraphLayer::Governance,
                derivation: DerivationClass::CanonicalRecord,
                location: None,
                relation_role: None,
                source: evidence.reference.clone(),
            });
        }

        for record in snapshot
            .index()
            .decisions
            .iter()
            .chain(snapshot.index().policies.iter())
        {
            let Some(kind) = (record.reference.kind == RecordKind::Decision)
                .then_some(GraphNodeKind::Decision)
                .or_else(|| {
                    (record.reference.kind == RecordKind::Policy).then_some(GraphNodeKind::Policy)
                })
            else {
                continue;
            };
            graph.node(GraphNode {
                id: record_node(&record.reference.kind, &record.reference.id),
                kind,
                label: record.reference.id.clone(),
                symbol_kind: None,
                manifest_kind: None,
                node_role: None,
                location: None,
                layer: GraphLayer::Governance,
                source: Some(record.reference.clone()),
            });
            if let Ok(matter) = &record.front_matter {
                for superseded in string_values(matter.get("supersedes")) {
                    graph.edge(GraphEdge {
                        from: record_node(&record.reference.kind, &record.reference.id),
                        to: record_node(&RecordKind::Decision, &superseded),
                        kind: GraphEdgeKind::Supersedes,
                        layer: GraphLayer::Governance,
                        derivation: DerivationClass::CanonicalRecord,
                        location: None,
                        relation_role: None,
                        source: record.reference.clone(),
                    });
                }
            }
        }

        if let Some(owners) = &snapshot.index().owners {
            if let Ok(value) = &owners.value {
                for scope in value
                    .get("scopes")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let Some(id) = scope.get("id").and_then(Value::as_str) else {
                        continue;
                    };
                    graph.node(GraphNode {
                        id: scope_node(id),
                        kind: GraphNodeKind::Scope,
                        label: id.to_owned(),
                        symbol_kind: None,
                        manifest_kind: None,
                        node_role: None,
                        location: None,
                        layer: GraphLayer::Governance,
                        source: Some(owners.reference.clone()),
                    });
                    if let Some(owner) = scope.get("owner").and_then(Value::as_str) {
                        let role_id = role_node(owner);
                        graph.node(GraphNode {
                            id: role_id.clone(),
                            kind: GraphNodeKind::Role,
                            label: owner.to_owned(),
                            symbol_kind: None,
                            manifest_kind: None,
                            node_role: None,
                            location: None,
                            layer: GraphLayer::Governance,
                            source: Some(owners.reference.clone()),
                        });
                        graph.edge(GraphEdge {
                            from: role_id,
                            to: scope_node(id),
                            kind: GraphEdgeKind::Allows,
                            layer: GraphLayer::Governance,
                            derivation: DerivationClass::CanonicalRecord,
                            location: None,
                            relation_role: None,
                            source: owners.reference.clone(),
                        });
                    }
                }
            }
        }

        if let Some(invariants) = &snapshot.index().invariants {
            if let Ok(value) = &invariants.value {
                for entry in value
                    .get("invariants")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let id = entry.get("id").and_then(Value::as_str).unwrap_or("unknown");
                    let node_id = format!("invariant:{id}");
                    graph.node(GraphNode {
                        id: node_id.clone(),
                        kind: GraphNodeKind::Invariant,
                        label: id.to_owned(),
                        symbol_kind: None,
                        manifest_kind: None,
                        node_role: None,
                        location: None,
                        layer: GraphLayer::Governance,
                        source: Some(invariants.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: "repository".to_owned(),
                        to: node_id,
                        kind: GraphEdgeKind::ConstrainedBy,
                        layer: GraphLayer::Governance,
                        derivation: DerivationClass::CanonicalRecord,
                        location: None,
                        relation_role: None,
                        source: invariants.reference.clone(),
                    });
                }
            }
        }
        let mut frozen_globs: Vec<(String, String)> = Vec::new();
        if let Some(frozen) = &snapshot.index().frozen_surface {
            if let Ok(value) = &frozen.value {
                for (index, entry) in value
                    .get("protected")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .enumerate()
                {
                    let glob = entry
                        .get("glob")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown");
                    let node_id = format!("frozen:{index}:{glob}");
                    graph.node(GraphNode {
                        id: node_id.clone(),
                        kind: GraphNodeKind::FrozenSurface,
                        label: glob.to_owned(),
                        symbol_kind: None,
                        manifest_kind: None,
                        node_role: None,
                        location: None,
                        layer: GraphLayer::Governance,
                        source: Some(frozen.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: "repository".to_owned(),
                        to: node_id.clone(),
                        kind: GraphEdgeKind::ConstrainedBy,
                        layer: GraphLayer::Governance,
                        derivation: DerivationClass::CanonicalRecord,
                        location: None,
                        relation_role: None,
                        source: frozen.reference.clone(),
                    });
                    frozen_globs.push((node_id, glob.to_owned()));
                }
            }
        }
        for finding in &snapshot.index().audit_findings {
            let finding_id = format!("finding:{}", finding.reference.id);
            graph.node(GraphNode {
                id: finding_id.clone(),
                kind: GraphNodeKind::AuditFinding,
                label: finding.reference.id.clone(),
                symbol_kind: None,
                manifest_kind: None,
                node_role: None,
                location: None,
                layer: GraphLayer::Governance,
                source: Some(finding.reference.clone()),
            });
            if let Ok(value) = &finding.value {
                if let Some(scope) = value
                    .get("scope")
                    .or_else(|| value.get("path"))
                    .and_then(Value::as_str)
                {
                    graph.node(GraphNode {
                        id: scope_node(scope),
                        kind: GraphNodeKind::Scope,
                        label: scope.to_owned(),
                        symbol_kind: None,
                        manifest_kind: None,
                        node_role: None,
                        location: None,
                        layer: GraphLayer::Governance,
                        source: Some(finding.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: finding_id,
                        to: scope_node(scope),
                        kind: GraphEdgeKind::Concerns,
                        layer: GraphLayer::Governance,
                        derivation: DerivationClass::CanonicalRecord,
                        location: None,
                        relation_role: None,
                        source: finding.reference.clone(),
                    });
                }
            }
        }

        physical::extend(
            &mut graph,
            snapshot.repository(),
            source_projection,
            &frozen_globs,
        );
        graph
    }

    pub fn node(&mut self, node: GraphNode) {
        self.nodes.entry(node.id.clone()).or_insert(node);
    }

    pub fn edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    pub fn dependencies(&self, work_id: &str) -> Vec<GraphEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.from == work_node(work_id) && edge.kind == GraphEdgeKind::DependsOn)
            .cloned()
            .collect()
    }

    pub fn reverse_dependencies(&self, work_id: &str) -> Vec<GraphEdge> {
        self.edges
            .iter()
            .filter(|edge| {
                edge.to == work_node(work_id) && edge.kind == GraphEdgeKind::ReverseDependency
            })
            .cloned()
            .collect()
    }

    pub fn acceptance_criteria(&self, work_id: &str) -> Vec<GraphEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.from == work_node(work_id) && edge.kind == GraphEdgeKind::Contains)
            .cloned()
            .collect()
    }

    pub fn evidence_for_criterion(&self, work_id: &str, criterion_id: &str) -> Vec<GraphEdge> {
        let criterion = criterion_node(work_id, criterion_id);
        self.edges
            .iter()
            .filter(|edge| edge.from == criterion && edge.kind == GraphEdgeKind::SupportedBy)
            .cloned()
            .collect()
    }

    pub fn edges_from(&self, node_id: &str) -> Vec<GraphEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.from == node_id)
            .cloned()
            .collect()
    }

    pub fn nodes_in_layer(&self, layer: GraphLayer) -> Vec<&GraphNode> {
        self.nodes
            .values()
            .filter(|node| node.layer == layer)
            .collect()
    }
}

pub fn build(snapshot: &RepositorySnapshot) -> RepositoryGraph {
    RepositoryGraph::build(snapshot)
}

/// Build the full (governance + physical) graph and write it as the
/// durable ROG representation. This is the single entry point
/// `repopact graph build` and the `graph.build` engine operation both
/// call; see [`durable::write`] for the atomic build-then-swap sequence.
pub fn build_and_write(
    snapshot: &RepositorySnapshot,
) -> Result<durable::Manifest, durable::DurableError> {
    // Decision 0051 section 7 / step 12: refuse to report an enabled
    // build as successful if Git would ignore the artifacts a clean
    // clone needs to actually carry it.
    durable::check_enablement_not_ignored(snapshot.repository())?;
    let (graph, fingerprint, semantic_coverage) = RepositoryGraph::build_with_fingerprint(snapshot);
    durable::write(
        snapshot.repository().root(),
        &graph,
        &fingerprint,
        semantic_coverage,
        incremental::current_semantic_compatibility(),
    )
}

/// Explicit disable (`repopact graph disable`, Decision 0051 section 4).
/// See [`durable::disable`] for the exact ordering/idempotency
/// guarantees.
pub fn disable_graph(repository_root: &std::path::Path) -> Result<(), durable::DurableError> {
    durable::disable(repository_root)
}

fn typed_work(record: &IndexedRecord) -> Option<WorkItem> {
    serde_json::from_value(record.value.clone().ok()?).ok()
}

fn string_values(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn path_is_under(path: &Path, ancestor: &Path) -> bool {
    path == ancestor || path.strip_prefix(ancestor).is_ok()
}

fn work_node(id: &str) -> String {
    format!("work:{id}")
}
fn criterion_node(work: &str, criterion: &str) -> String {
    format!("criterion:{work}:{criterion}")
}
fn evidence_node(id: &str) -> String {
    format!("evidence:{id}")
}
fn scope_node(id: &str) -> String {
    format!("scope:{id}")
}
fn role_node(id: &str) -> String {
    format!("role:{id}")
}
fn contract_node(id: &str) -> String {
    format!("contract:{id}")
}
fn record_node(kind: &RecordKind, id: &str) -> String {
    format!(
        "{}:{id}",
        serde_json::to_value(kind)
            .unwrap()
            .as_str()
            .unwrap_or("record")
    )
}

/// Re-export so callers building a physical-only graph (tests, tooling)
/// don't need to know the module path.
pub use physical::{directory_node, file_node};

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("repopact-graph-{name}-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_root;
    use repopact_repository::{Repository, RepositorySession};
    use std::path::PathBuf;

    #[test]
    fn graph_is_deterministic_and_keeps_source_context() {
        let root = temp_root("determinism");
        std::fs::create_dir_all(root.join("work/active/001-one")).unwrap();
        std::fs::write(root.join("work/active/001-one/work-item.json"), r#"{"id":"001","title":"One","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-01"}"#).unwrap();
        let snapshot = RepositorySession::open(PathBuf::from(&root)).snapshot();
        let first = build(&snapshot);
        let second = build(&snapshot);
        assert_eq!(first, second);
        assert_eq!(first.acceptance_criteria("001").len(), 1);
        assert_eq!(first.acceptance_criteria("001")[0].source.id, "001");
        std::fs::remove_dir_all(root).unwrap();
    }

    fn seeded_repo(name: &str) -> PathBuf {
        let root = temp_root(name);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("README.md"), "# fixture\n").unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]\nname=\"fixture\"\n").unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn hello() {}\n").unwrap();
        root
    }

    // WI063 ROG-021: explicit graph-level boundary/exclusion evidence,
    // not merely inherited-and-trusted walker behavior. Each test builds
    // a real graph and inspects its actual nodes/edges rather than
    // asserting on `repopact-repository`'s own internal exclusion logic
    // (already tested at that layer separately).

    #[test]
    fn excluded_build_dependency_and_venv_trees_produce_no_graph_nodes() {
        let root = seeded_repo("rog021-excluded-trees");
        std::fs::create_dir_all(root.join("target/debug")).unwrap();
        std::fs::write(root.join("target/debug/output.bin"), "binary").unwrap();
        std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        std::fs::write(
            root.join("node_modules/pkg/index.js"),
            "module.exports = {};\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join(".venv/lib")).unwrap();
        std::fs::write(root.join(".venv/lib/site.py"), "def vendored(): pass\n").unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let graph = build(&snapshot);
        for node in graph.nodes.values() {
            let path = node
                .source
                .as_ref()
                .map(|source| source.path.as_str())
                .unwrap_or("");
            assert!(
                !path.contains("target/")
                    && !path.contains("node_modules/")
                    && !path.contains(".venv/"),
                "excluded-tree path leaked into the graph: {path} (node {})",
                node.id
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_internals_are_never_indexed_as_source() {
        let root = seeded_repo("rog021-git-internals");
        std::fs::create_dir_all(root.join(".git/objects")).unwrap();
        std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            root.join(".git/objects/pretend-object"),
            "not real git data",
        )
        .unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let graph = build(&snapshot);
        for node in graph.nodes.values() {
            let path = node
                .source
                .as_ref()
                .map(|source| source.path.as_str())
                .unwrap_or("");
            assert!(
                !path.contains(".git/"),
                ".git internals must never be indexed as source: {path}"
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_secret_looking_file_is_never_content_ingested_into_a_fact() {
        // A `.env`-shaped file is not excluded by name (only directory
        // patterns are excluded), so it is still physically listed (a
        // File node with a content digest) -- but that digest is never
        // reversible to the secret value, and no adapter ever parses a
        // plain `.env` file's key=value content into a fact (it has no
        // recognized extension), so the actual secret text must never
        // appear as any node's label anywhere in the graph.
        let root = seeded_repo("rog021-secret-file");
        let secret_value = "SUPER_SECRET_TOKEN_VALUE_9f8e7d6c5b4a";
        std::fs::write(root.join(".env"), format!("API_KEY={secret_value}\n")).unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let graph = build(&snapshot);
        assert!(
            !graph.nodes.values().any(|node| node.label.contains(secret_value)),
            "a secret value must never appear as a node label merely because the file exists locally"
        );
        // The file is still truthfully listed as a physical fact (a
        // digest, not the content) -- confirming this is a containment
        // boundary decision, not an accidental omission.
        assert!(graph.nodes.contains_key("file:.env"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_nested_repository_is_classified_and_its_contents_are_bounded() {
        let root = seeded_repo("rog021-nested-repo");
        std::fs::create_dir_all(root.join("vendor/nested/.git")).unwrap();
        std::fs::write(
            root.join("vendor/nested/.git/HEAD"),
            "ref: refs/heads/main\n",
        )
        .unwrap();
        std::fs::write(root.join("vendor/nested/inner.rs"), "pub fn inner() {}\n").unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let graph = build(&snapshot);
        let nested_node = graph
            .nodes
            .get("dir:vendor/nested")
            .expect("the nested repository's own directory must still be classified");
        assert_eq!(nested_node.kind, GraphNodeKind::NestedRepository);
        for node in graph.nodes.values() {
            let path = node
                .source
                .as_ref()
                .map(|source| source.path.as_str())
                .unwrap_or("");
            assert!(
                !path.contains("vendor/nested/.git/"),
                "a nested repository's own .git internals must never be indexed: {path}"
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_symlink_pointing_outside_the_repository_is_not_traversed() {
        let root = temp_root("rog021-symlink-outside");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn hello() {}\n").unwrap();
        let outside = temp_root("rog021-symlink-outside-target");
        let secret_marker = "outside_repository_marker_fn";
        let outside_file = outside.join("secret.rs");
        std::fs::write(&outside_file, format!("pub fn {secret_marker}() {{}}\n")).unwrap();
        let link = root.join("linked.rs");

        #[cfg(windows)]
        fn make_symlink(target: &Path, link: &Path) -> bool {
            std::os::windows::fs::symlink_file(target, link).is_ok()
        }
        #[cfg(not(windows))]
        fn make_symlink(target: &Path, link: &Path) -> bool {
            std::os::unix::fs::symlink(target, link).is_ok()
        }

        if !make_symlink(&outside_file, &link) {
            eprintln!(
                "skipping symlink boundary test: platform/permissions do not allow \
                 creating a file symlink in this environment"
            );
            std::fs::remove_dir_all(root).unwrap();
            std::fs::remove_dir_all(outside).unwrap();
            return;
        }

        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let graph = build(&snapshot);
        assert!(
            !graph
                .nodes
                .values()
                .any(|node| node.label.contains(secret_marker)),
            "content reached only through a symlink pointing outside the repository \
             must never appear in the graph"
        );
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn physical_node_ids_are_stable_and_platform_independent() {
        let root = seeded_repo("stable-ids");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let graph = build(&snapshot);
        // Forward-slash normalized regardless of the host path separator.
        assert!(graph.nodes.contains_key("file:src/lib.rs"));
        assert!(graph.nodes.contains_key("dir:src"));
        assert!(!graph.nodes.keys().any(|id| id.contains('\\')));
        // A repeated build over the same source yields byte-identical IDs.
        let second = build(&snapshot);
        assert_eq!(graph, second);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_and_configuration_file_are_classified() {
        let root = seeded_repo("workspace-classification");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let graph = build(&snapshot);
        let root_dir = graph.nodes.get("repository").unwrap();
        assert_eq!(root_dir.kind, GraphNodeKind::Repository);
        let manifest = graph.nodes.get("file:Cargo.toml").unwrap();
        assert_eq!(manifest.kind, GraphNodeKind::ConfigurationFile);
        let lib_file = graph.nodes.get("file:src/lib.rs").unwrap();
        assert!(graph.edges.iter().any(|edge| edge.from == "file:src/lib.rs"
            && edge.kind == GraphEdgeKind::BelongsToWorkspace
            && edge.to == "repository"));
        assert_eq!(lib_file.layer, GraphLayer::Physical);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_absolute_path_leaks_into_physical_nodes() {
        let root = seeded_repo("no-absolute-leak");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let graph = build(&snapshot);
        for node in graph.nodes.values() {
            if let Some(source) = &node.source {
                assert!(
                    !source.path.starts_with('/'),
                    "leaked absolute path: {}",
                    source.path
                );
                assert!(
                    !(source.path.len() > 1 && source.path.as_bytes()[1] == b':'),
                    "leaked Windows absolute path: {}",
                    source.path
                );
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn durable_round_trip_and_repeated_full_build_are_byte_identical() {
        let root = seeded_repo("durable-round-trip");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();

        let manifest_a = build_and_write(&snapshot).expect("first build");
        let bytes_a: Vec<_> = manifest_a
            .node_shards
            .iter()
            .map(|entry| durable::shard_bytes(&root, "nodes", &entry.shard).unwrap())
            .collect();

        // Rebuild from the same source a second time. This must be
        // byte-identical (ROG-011) and must not have absorbed its own
        // prior durable output into the source projection (self-exclusion,
        // ROG-009) -- if it had, node/edge counts would grow every build.
        let snapshot = repository.session().snapshot();
        let manifest_b = build_and_write(&snapshot).expect("second build");
        let bytes_b: Vec<_> = manifest_b
            .node_shards
            .iter()
            .map(|entry| durable::shard_bytes(&root, "nodes", &entry.shard).unwrap())
            .collect();

        assert_eq!(manifest_a.node_count, manifest_b.node_count);
        assert_eq!(manifest_a.edge_count, manifest_b.edge_count);
        assert_eq!(
            manifest_a.source_projection_fingerprint,
            manifest_b.source_projection_fingerprint
        );
        assert_eq!(bytes_a, bytes_b);

        // The rog/ directory itself must never appear as a physical node.
        let loaded_snapshot = repository.session().snapshot();
        let fresh_graph = build(&loaded_snapshot);
        assert!(!fresh_graph.nodes.contains_key("dir:rog"));
        assert!(!fresh_graph
            .nodes
            .keys()
            .any(|id| id.starts_with("file:rog/")));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_fingerprint_is_detected_after_source_change() {
        let root = seeded_repo("stale-detection");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        build_and_write(&snapshot).expect("build");

        let fresh_status = status::status(&repository);
        assert_eq!(fresh_status.freshness, status::Freshness::Fresh);

        std::fs::write(
            root.join("src/lib.rs"),
            "pub fn hello() { /* changed */ }\n",
        )
        .unwrap();
        let stale_status = status::status(&repository);
        assert_eq!(stale_status.freshness, status::Freshness::Stale);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn graph_disabled_repository_reports_absent_not_error() {
        let root = seeded_repo("graph-disabled");
        let repository = Repository::open(&root);
        let disabled_status = status::status(&repository);
        assert_eq!(disabled_status.freshness, status::Freshness::Absent);
        assert!(disabled_status.diagnostics.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unknown_major_schema_version_is_rejected_not_interpreted() {
        let root = seeded_repo("unknown-major");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        build_and_write(&snapshot).expect("build");

        let manifest_path = durable::manifest_path(&root);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["graph_schema_version"] = serde_json::Value::from(999);
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let status = status::status(&repository);
        assert_eq!(status.freshness, status::Freshness::Unsupported);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_genuine_v1_physical_only_graph_remains_readable_and_valid() {
        // build_and_write always writes the current major (2) once the
        // binary has semantic vocabulary (Decision 0045 section 2), so a
        // "genuine v1 graph" is simulated by declaring version 1 on a
        // graph whose actual content is pure Decision-0044 physical
        // vocabulary -- exactly what an honest historical v1 build would
        // have produced. This is not cheating the test: the whole point
        // of the compatibility guarantee is that v1's *vocabulary* (not
        // its version number in isolation) must still validate cleanly.
        let root = seeded_repo("v1-remains-readable");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        build_and_write(&snapshot).expect("build");

        let manifest_path = durable::manifest_path(&root);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["graph_schema_version"] = serde_json::Value::from(1);
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let diagnostics = validate::validate_structure(&root);
        assert!(
            diagnostics.is_empty(),
            "a schema-v1-declared graph containing only Decision 0044 \
             physical vocabulary must remain structurally valid under \
             the schema-v2-aware validator: {diagnostics:?}"
        );
        let status = status::status(&repository);
        assert_eq!(status.freshness, status::Freshness::Fresh);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_v1_graph_rebuilds_deterministically_into_a_valid_v2_graph() {
        let root = seeded_repo("v1-to-v2-rebuild");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        build_and_write(&snapshot).expect("initial build");

        let manifest_path = durable::manifest_path(&root);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["graph_schema_version"] = serde_json::Value::from(1);
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        // The only supported migration path is a full deterministic
        // rebuild (Decision 0045 section 2) -- never an in-place bump of
        // the version field alone.
        let snapshot = repository.session().snapshot();
        let rebuilt = build_and_write(&snapshot).expect("rebuild");
        assert_eq!(
            rebuilt.graph_schema_version,
            durable::CURRENT_GRAPH_SCHEMA_VERSION
        );
        assert_eq!(rebuilt.graph_schema_version, 3);

        let diagnostics = validate::validate_structure(&root);
        assert!(
            diagnostics.is_empty(),
            "rebuilt v2 graph must validate cleanly: {diagnostics:?}"
        );
        let status = status::status(&repository);
        assert_eq!(status.freshness, status::Freshness::Fresh);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_genuine_v2_semantic_graph_remains_readable_and_valid() {
        // Mirrors a_genuine_v1_physical_only_graph_remains_readable_and_valid
        // one major up: a schema-v2-declared graph containing only
        // Decision 0045 semantic vocabulary (no metadata/ManifestKind
        // content) must remain structurally valid and Fresh under the
        // schema-v3-aware validator.
        let root = seeded_repo("v2-remains-readable");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        build_and_write(&snapshot).expect("build");

        let manifest_path = durable::manifest_path(&root);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["graph_schema_version"] = serde_json::Value::from(2);
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let diagnostics = validate::validate_structure(&root);
        assert!(
            diagnostics.is_empty(),
            "a schema-v2-declared graph containing only Decision 0045 \
             semantic vocabulary must remain structurally valid under \
             the schema-v3-aware validator: {diagnostics:?}"
        );
        let status = status::status(&repository);
        assert_eq!(status.freshness, status::Freshness::Fresh);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_v2_graph_rebuilds_deterministically_into_a_valid_v3_graph() {
        let root = seeded_repo("v2-to-v3-rebuild");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        build_and_write(&snapshot).expect("initial build");

        let manifest_path = durable::manifest_path(&root);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["graph_schema_version"] = serde_json::Value::from(2);
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let snapshot = repository.session().snapshot();
        let rebuilt = build_and_write(&snapshot).expect("rebuild");
        assert_eq!(rebuilt.graph_schema_version, 3);

        let diagnostics = validate::validate_structure(&root);
        assert!(
            diagnostics.is_empty(),
            "rebuilt v3 graph must validate cleanly: {diagnostics:?}"
        );
        let status = status::status(&repository);
        assert_eq!(status.freshness, status::Freshness::Fresh);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupted_shard_hash_is_detected() {
        let root = seeded_repo("corrupt-shard");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let manifest = build_and_write(&snapshot).expect("build");

        let shard_path = root.join("rog").join("nodes").join(
            &manifest
                .node_shards
                .first()
                .expect("at least one node shard")
                .shard,
        );
        let mut bytes = std::fs::read(&shard_path).unwrap();
        bytes.push(b'\n');
        bytes.extend_from_slice(b"{\"tampered\":true}");
        std::fs::write(&shard_path, bytes).unwrap();

        let diagnostics = validate::validate_structure(&root);
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "graph.shard-hash-mismatch"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_node_id_across_shards_is_rejected() {
        let root = seeded_repo("duplicate-node");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let manifest = build_and_write(&snapshot).expect("build");

        // Duplicate the first node record into every other shard so at
        // least one duplicate lands somewhere, then recompute hashes so
        // the corruption under test is the duplicate, not a hash mismatch.
        let first_shard = &manifest.node_shards[0];
        let sample_line = std::fs::read_to_string(root.join("rog/nodes").join(&first_shard.shard))
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_owned();
        for entry in manifest.node_shards.iter().skip(1).take(1) {
            let path = root.join("rog/nodes").join(&entry.shard);
            let mut bytes = std::fs::read(&path).unwrap();
            bytes.extend_from_slice(sample_line.as_bytes());
            bytes.push(b'\n');
            let new_hash = {
                use sha2::{Digest, Sha256};
                Sha256::digest(&bytes)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            };
            std::fs::write(&path, &bytes).unwrap();
            let manifest_path = durable::manifest_path(&root);
            let mut manifest_value: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
            for shard_entry in manifest_value["node_shards"].as_array_mut().unwrap() {
                if shard_entry["shard"] == serde_json::Value::from(entry.shard.clone()) {
                    shard_entry["sha256"] = serde_json::Value::from(new_hash.clone());
                    shard_entry["count"] = serde_json::Value::from(
                        shard_entry["count"].as_u64().unwrap() as usize + 1,
                    );
                }
            }
            manifest_value["node_count"] = serde_json::Value::from(
                manifest_value["node_count"].as_u64().unwrap() as usize + 1,
            );
            std::fs::write(
                &manifest_path,
                serde_json::to_string_pretty(&manifest_value).unwrap(),
            )
            .unwrap();
        }

        let diagnostics = validate::validate_structure(&root);
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "graph.duplicate-node"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dangling_edge_endpoint_is_rejected() {
        let root = seeded_repo("dangling-edge");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let manifest = build_and_write(&snapshot).expect("build");

        let entry = &manifest.edge_shards[0];
        let path = root.join("rog/edges").join(&entry.shard);
        let bogus_edge = serde_json::json!({
            "from": "file:does/not/exist.rs",
            "to": "repository",
            "kind": "contains",
            "layer": "physical",
            "derivation": "filesystem",
            "source": {"kind": "file", "id": "does/not/exist.rs", "path": "does/not/exist.rs"}
        });
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.extend_from_slice(serde_json::to_string(&bogus_edge).unwrap().as_bytes());
        bytes.push(b'\n');
        let new_hash = {
            use sha2::{Digest, Sha256};
            Sha256::digest(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        std::fs::write(&path, &bytes).unwrap();
        let manifest_path = durable::manifest_path(&root);
        let mut manifest_value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        for shard_entry in manifest_value["edge_shards"].as_array_mut().unwrap() {
            if shard_entry["shard"] == serde_json::Value::from(entry.shard.clone()) {
                shard_entry["sha256"] = serde_json::Value::from(new_hash.clone());
                shard_entry["count"] =
                    serde_json::Value::from(shard_entry["count"].as_u64().unwrap() as usize + 1);
            }
        }
        manifest_value["edge_count"] =
            serde_json::Value::from(manifest_value["edge_count"].as_u64().unwrap() as usize + 1);
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest_value).unwrap(),
        )
        .unwrap();

        let diagnostics = validate::validate_structure(&root);
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "graph.dangling-edge"));

        std::fs::remove_dir_all(root).unwrap();
    }

    // ---- ROG-014/015/039: capability enable/disable lifecycle -------

    #[test]
    fn build_persists_explicit_enabled_capability() {
        let root = seeded_repo("capability-build-enables");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        build_and_write(&snapshot).expect("build");
        assert_eq!(
            crate::capability::current_state(&root).unwrap(),
            crate::capability::CapabilityState::ExplicitEnabled
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disable_removes_graph_and_persists_explicit_disabled() {
        let root = seeded_repo("capability-disable");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        build_and_write(&snapshot).expect("build");
        assert!(durable::rog_root(&root).exists());

        disable_graph(&root).expect("disable");
        assert!(!durable::rog_root(&root).exists());
        assert_eq!(
            crate::capability::current_state(&root).unwrap(),
            crate::capability::CapabilityState::ExplicitDisabled
        );

        let status_result = status::status(&Repository::open(&root));
        assert_eq!(status_result.freshness, status::Freshness::Absent);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disable_is_idempotent() {
        let root = seeded_repo("capability-disable-idempotent");
        disable_graph(&root).expect("first disable");
        disable_graph(&root).expect("second disable on an already-disabled repo");
        assert_eq!(
            crate::capability::current_state(&root).unwrap(),
            crate::capability::CapabilityState::ExplicitDisabled
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn build_disable_rebuild_re_enables() {
        let root = seeded_repo("capability-cycle");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();

        build_and_write(&snapshot).expect("first build");
        assert_eq!(
            crate::capability::current_state(&root).unwrap(),
            crate::capability::CapabilityState::ExplicitEnabled
        );

        disable_graph(&root).expect("disable");
        assert_eq!(
            crate::capability::current_state(&root).unwrap(),
            crate::capability::CapabilityState::ExplicitDisabled
        );

        let snapshot = Repository::open(&root).session().snapshot();
        build_and_write(&snapshot).expect("second build re-enables");
        assert_eq!(
            crate::capability::current_state(&root).unwrap(),
            crate::capability::CapabilityState::ExplicitEnabled
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_graph_migrates_to_explicit_enabled_on_next_build_with_no_manual_step() {
        let root = seeded_repo("capability-legacy-migration");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        build_and_write(&snapshot).expect("build");
        // Simulate a pre-Decision-0051 graph: strip the capability
        // record this checkpoint's own build just wrote.
        std::fs::remove_file(root.join(crate::capability::CAPABILITY_RECORD_RELATIVE_PATH))
            .unwrap();
        assert_eq!(
            crate::capability::current_state(&root).unwrap(),
            crate::capability::CapabilityState::LegacyEnabled
        );

        // The explicit backfill/migration path is exactly `graph build`
        // again -- no manual deletion, no doctor step required.
        let snapshot = Repository::open(&root).session().snapshot();
        build_and_write(&snapshot).expect("migration build");
        assert_eq!(
            crate::capability::current_state(&root).unwrap(),
            crate::capability::CapabilityState::ExplicitEnabled
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ignored_durable_graph_refuses_to_report_enablement_success() {
        let root = seeded_repo("capability-ignored-artifact");
        std::fs::write(root.join(".gitignore"), "rog/\n").unwrap();
        // A real Git repository is required for `git check-ignore` to
        // have anything to consult.
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&root)
            .output()
            .expect("git init");

        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let result = build_and_write(&snapshot);
        assert!(
            result.is_err(),
            "build must refuse to report enablement success when .gitignore swallows rog/"
        );
        assert_eq!(
            crate::capability::current_state(&root).unwrap(),
            crate::capability::CapabilityState::LegacyAbsent,
            "capability must not be persisted as enabled when the graph would be git-ignored"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn randomized_git_invocation_count_is_still_bounded_after_graph_build() {
        use repopact_repository::CountingGitRunner;

        let root = seeded_repo("git-bound");
        let runner = CountingGitRunner::native();
        let repository = Repository::with_git_runner(&root, runner.clone());
        let snapshot = repository.session().snapshot();
        build_and_write(&snapshot).expect("build");
        // WI057's bound is <= 4 git invocations per snapshot(); a graph
        // build must not add any additional per-file or per-node git
        // calls beyond what RepositorySession::snapshot() already issues.
        assert!(
            runner.count() <= 4,
            "graph build must not add git invocations beyond the bounded snapshot cost, got {}",
            runner.count()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
