use std::collections::BTreeMap;
use std::path::Path;

use repopact_repository::{IndexedRecord, RepositorySnapshot};
use repopact_types::{RecordKind, RecordRef, SourceRef, WorkItem};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod durable;
pub mod physical;
pub mod projection;
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
    pub fn build_with_fingerprint(snapshot: &RepositorySnapshot) -> (Self, String) {
        let mut graph = Self::default();
        let repository_source = RecordRef::new(RecordKind::Repository, "repository", "<root>");
        graph.node(GraphNode {
            id: "repository".to_owned(),
            kind: GraphNodeKind::Repository,
            label: "RepoPact repository".to_owned(),
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
                layer: GraphLayer::Governance,
                source: Some(record.reference.clone()),
            });
            for criterion in &item.acceptance_criteria {
                let criterion_id = criterion_node(&item.id, &criterion.id);
                graph.node(GraphNode {
                    id: criterion_id.clone(),
                    kind: GraphNodeKind::AcceptanceCriterion,
                    label: criterion.text.clone(),
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
                            layer: GraphLayer::Governance,
                            source: Some(evidence.reference.clone()),
                        });
                        graph.edge(GraphEdge {
                            from: criterion_id.clone(),
                            to: evidence_node_id.clone(),
                            kind: GraphEdgeKind::SupportedBy,
                            layer: GraphLayer::Governance,
                            derivation: DerivationClass::CanonicalRecord,
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
                    source: record.reference.clone(),
                });
                graph.edge(GraphEdge {
                    from: dependency_node,
                    to: work_id.clone(),
                    kind: GraphEdgeKind::ReverseDependency,
                    layer: GraphLayer::Governance,
                    derivation: DerivationClass::CanonicalRecord,
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
                layer: GraphLayer::Governance,
                source: Some(owner_source.clone()),
            });
            graph.edge(GraphEdge {
                from: work_id.clone(),
                to: owner_node,
                kind: GraphEdgeKind::OwnedBy,
                layer: GraphLayer::Governance,
                derivation: DerivationClass::CanonicalRecord,
                source: record.reference.clone(),
            });
            for scope in &item.affected_scopes {
                let scope_node_id = scope_node(scope);
                graph.node(GraphNode {
                    id: scope_node_id.clone(),
                    kind: GraphNodeKind::Scope,
                    label: scope.clone(),
                    layer: GraphLayer::Governance,
                    source: Some(owner_source.clone()),
                });
                graph.edge(GraphEdge {
                    from: work_id.clone(),
                    to: scope_node_id,
                    kind: GraphEdgeKind::Affects,
                    layer: GraphLayer::Governance,
                    derivation: DerivationClass::CanonicalRecord,
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
                        layer: GraphLayer::Governance,
                        source: Some(contract.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: work_id.clone(),
                        to: contract_node(&contract.reference.id),
                        kind: GraphEdgeKind::ConstrainedBy,
                        layer: GraphLayer::Governance,
                        derivation: DerivationClass::CanonicalRecord,
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
                layer: GraphLayer::Governance,
                source: Some(evidence.reference.clone()),
            });
            graph.edge(GraphEdge {
                from: evidence_node(&evidence.reference.id),
                to: work_node(work_id),
                kind: GraphEdgeKind::SupportsWorkItem,
                layer: GraphLayer::Governance,
                derivation: DerivationClass::CanonicalRecord,
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
                        layer: GraphLayer::Governance,
                        source: Some(owners.reference.clone()),
                    });
                    if let Some(owner) = scope.get("owner").and_then(Value::as_str) {
                        let role_id = role_node(owner);
                        graph.node(GraphNode {
                            id: role_id.clone(),
                            kind: GraphNodeKind::Role,
                            label: owner.to_owned(),
                            layer: GraphLayer::Governance,
                            source: Some(owners.reference.clone()),
                        });
                        graph.edge(GraphEdge {
                            from: role_id,
                            to: scope_node(id),
                            kind: GraphEdgeKind::Allows,
                            layer: GraphLayer::Governance,
                            derivation: DerivationClass::CanonicalRecord,
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
                        layer: GraphLayer::Governance,
                        source: Some(invariants.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: "repository".to_owned(),
                        to: node_id,
                        kind: GraphEdgeKind::ConstrainedBy,
                        layer: GraphLayer::Governance,
                        derivation: DerivationClass::CanonicalRecord,
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
                        layer: GraphLayer::Governance,
                        source: Some(frozen.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: "repository".to_owned(),
                        to: node_id.clone(),
                        kind: GraphEdgeKind::ConstrainedBy,
                        layer: GraphLayer::Governance,
                        derivation: DerivationClass::CanonicalRecord,
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
                        layer: GraphLayer::Governance,
                        source: Some(finding.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: finding_id,
                        to: scope_node(scope),
                        kind: GraphEdgeKind::Concerns,
                        layer: GraphLayer::Governance,
                        derivation: DerivationClass::CanonicalRecord,
                        source: finding.reference.clone(),
                    });
                }
            }
        }

        let source_projection =
            projection::SourceProjection::build(snapshot.repository(), snapshot.topology());
        physical::extend(
            &mut graph,
            snapshot.repository(),
            &source_projection,
            &frozen_globs,
        );

        graph.edges.sort();
        graph.edges.dedup();
        (graph, source_projection.fingerprint())
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
    let (graph, fingerprint) = RepositoryGraph::build_with_fingerprint(snapshot);
    durable::write(snapshot.repository().root(), &graph, &fingerprint)
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
