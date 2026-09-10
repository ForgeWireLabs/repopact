use std::collections::BTreeMap;
use std::path::Path;

use repopact_repository::{IndexedRecord, RepositorySnapshot};
use repopact_types::{RecordKind, RecordRef, SourceRef, WorkItem};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: GraphNodeKind,
    pub label: String,
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
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: GraphEdgeKind,
    pub source: SourceRef,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryGraph {
    pub nodes: BTreeMap<String, GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl RepositoryGraph {
    pub fn build(snapshot: &RepositorySnapshot) -> Self {
        let mut graph = Self::default();
        let repository_source = RecordRef::new(RecordKind::Repository, "repository", "<root>");
        graph.node(GraphNode {
            id: "repository".to_owned(),
            kind: GraphNodeKind::Repository,
            label: "RepoPact repository".to_owned(),
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
                source: Some(record.reference.clone()),
            });
            for criterion in &item.acceptance_criteria {
                let criterion_id = criterion_node(&item.id, &criterion.id);
                graph.node(GraphNode {
                    id: criterion_id.clone(),
                    kind: GraphNodeKind::AcceptanceCriterion,
                    label: criterion.text.clone(),
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
                            source: Some(evidence.reference.clone()),
                        });
                        graph.edge(GraphEdge {
                            from: criterion_id.clone(),
                            to: evidence_node_id.clone(),
                            kind: GraphEdgeKind::SupportedBy,
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
                    source: record.reference.clone(),
                });
                graph.edge(GraphEdge {
                    from: dependency_node,
                    to: work_id.clone(),
                    kind: GraphEdgeKind::ReverseDependency,
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
                source: Some(owner_source.clone()),
            });
            graph.edge(GraphEdge {
                from: work_id.clone(),
                to: owner_node,
                kind: GraphEdgeKind::OwnedBy,
                source: record.reference.clone(),
            });
            for scope in &item.affected_scopes {
                let scope_node_id = scope_node(scope);
                graph.node(GraphNode {
                    id: scope_node_id.clone(),
                    kind: GraphNodeKind::Scope,
                    label: scope.clone(),
                    source: Some(owner_source.clone()),
                });
                graph.edge(GraphEdge {
                    from: work_id.clone(),
                    to: scope_node_id,
                    kind: GraphEdgeKind::Affects,
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
                        source: Some(contract.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: work_id.clone(),
                        to: contract_node(&contract.reference.id),
                        kind: GraphEdgeKind::ConstrainedBy,
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
                source: Some(evidence.reference.clone()),
            });
            graph.edge(GraphEdge {
                from: evidence_node(&evidence.reference.id),
                to: work_node(work_id),
                kind: GraphEdgeKind::SupportsWorkItem,
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
                source: Some(record.reference.clone()),
            });
            if let Ok(matter) = &record.front_matter {
                for superseded in string_values(matter.get("supersedes")) {
                    graph.edge(GraphEdge {
                        from: record_node(&record.reference.kind, &record.reference.id),
                        to: record_node(&RecordKind::Decision, &superseded),
                        kind: GraphEdgeKind::Supersedes,
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
                        source: Some(owners.reference.clone()),
                    });
                    if let Some(owner) = scope.get("owner").and_then(Value::as_str) {
                        let role_id = role_node(owner);
                        graph.node(GraphNode {
                            id: role_id.clone(),
                            kind: GraphNodeKind::Role,
                            label: owner.to_owned(),
                            source: Some(owners.reference.clone()),
                        });
                        graph.edge(GraphEdge {
                            from: role_id,
                            to: scope_node(id),
                            kind: GraphEdgeKind::Allows,
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
                        source: Some(invariants.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: "repository".to_owned(),
                        to: node_id,
                        kind: GraphEdgeKind::ConstrainedBy,
                        source: invariants.reference.clone(),
                    });
                }
            }
        }
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
                        source: Some(frozen.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: "repository".to_owned(),
                        to: node_id,
                        kind: GraphEdgeKind::ConstrainedBy,
                        source: frozen.reference.clone(),
                    });
                }
            }
        }
        for finding in &snapshot.index().audit_findings {
            let finding_id = format!("finding:{}", finding.reference.id);
            graph.node(GraphNode {
                id: finding_id.clone(),
                kind: GraphNodeKind::AuditFinding,
                label: finding.reference.id.clone(),
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
                        source: Some(finding.reference.clone()),
                    });
                    graph.edge(GraphEdge {
                        from: finding_id,
                        to: scope_node(scope),
                        kind: GraphEdgeKind::Concerns,
                        source: finding.reference.clone(),
                    });
                }
            }
        }
        graph.edges.sort();
        graph.edges.dedup();
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
}

pub fn build(snapshot: &RepositorySnapshot) -> RepositoryGraph {
    RepositoryGraph::build(snapshot)
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

#[cfg(test)]
mod tests {
    use super::*;
    use repopact_repository::RepositorySession;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn graph_is_deterministic_and_keeps_source_context() {
        let root = std::env::temp_dir().join(format!(
            "repopact-graph-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
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
}
