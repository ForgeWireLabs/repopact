//! Typed target resolution (Decision 0050 section 2): the canonical
//! resolver every other query operation reuses.

use crate::{GraphNodeKind, ManifestKind};

use super::dto::{FactRef, ResolutionOutcome};
use super::index::GraphQueryIndex;
use super::selector::{validate_repository_relative_path, NodeSelector, SymbolSelector};
use crate::RepositoryGraph;

pub(super) fn resolve(
    graph: &RepositoryGraph,
    index: &GraphQueryIndex,
    selector: &NodeSelector,
) -> ResolutionOutcome {
    match selector {
        NodeSelector::NodeId(id) => match graph.nodes.get(id) {
            Some(node) => ResolutionOutcome::Exact {
                fact: FactRef::from_node(node),
            },
            None => ResolutionOutcome::NotFound,
        },
        NodeSelector::RepositoryPath(raw) => {
            let Ok(normalized) = validate_repository_relative_path(raw) else {
                return ResolutionOutcome::NotFound;
            };
            let candidates = index.nodes_by_path(&normalized);
            resolve_from_candidates(graph, candidates)
        }
        NodeSelector::WorkItemId(id) => {
            let node_id = format!("work:{id}");
            match graph.nodes.get(&node_id) {
                Some(node) => ResolutionOutcome::Exact {
                    fact: FactRef::from_node(node),
                },
                None => ResolutionOutcome::NotFound,
            }
        }
        NodeSelector::Package(name) => resolve_package(graph, index, name),
        NodeSelector::Module(name) => resolve_module(graph, index, name),
        NodeSelector::Symbol(symbol) => resolve_symbol(graph, index, symbol),
    }
}

fn resolve_from_candidates(graph: &RepositoryGraph, candidates: &[String]) -> ResolutionOutcome {
    match candidates {
        [] => ResolutionOutcome::NotFound,
        [only] => match graph.nodes.get(only) {
            Some(node) => ResolutionOutcome::Exact {
                fact: FactRef::from_node(node),
            },
            None => ResolutionOutcome::NotFound,
        },
        many => ResolutionOutcome::Ambiguous {
            candidates: many
                .iter()
                .filter_map(|id| graph.nodes.get(id))
                .map(FactRef::from_node)
                .collect(),
        },
    }
}

/// A package is a manifest document node whose identity label names it
/// (e.g. `"cargo:repopact-graph"`, `"npm:root"`,
/// `"python-project:repopact"`). Matched by exact suffix after the
/// ecosystem prefix, never a substring/fuzzy match.
fn resolve_package(
    graph: &RepositoryGraph,
    index: &GraphQueryIndex,
    name: &str,
) -> ResolutionOutcome {
    let manifest_ids = index.nodes_of_kind(GraphNodeKind::Manifest);
    let matches: Vec<String> = manifest_ids
        .iter()
        .filter(|id| {
            graph
                .nodes
                .get(*id)
                .is_some_and(|node| package_label_matches(node.manifest_kind, &node.label, name))
        })
        .cloned()
        .collect();
    resolve_from_candidates(graph, &matches)
}

fn package_label_matches(manifest_kind: Option<ManifestKind>, label: &str, name: &str) -> bool {
    let Some(kind) = manifest_kind else {
        return false;
    };
    let prefix = match kind {
        ManifestKind::CargoPackage => "cargo:",
        ManifestKind::NodePackage => "npm:",
        ManifestKind::PythonProject => "python-project:",
        _ => return false,
    };
    label.strip_prefix(prefix) == Some(name)
}

/// A module is a `Symbol` node with `symbol_kind = Module` whose label
/// equals the given name.
fn resolve_module(
    graph: &RepositoryGraph,
    index: &GraphQueryIndex,
    name: &str,
) -> ResolutionOutcome {
    let symbol_ids = index.nodes_of_kind(GraphNodeKind::Symbol);
    let matches: Vec<String> = symbol_ids
        .iter()
        .filter(|id| {
            graph.nodes.get(*id).is_some_and(|node| {
                node.symbol_kind == Some(crate::SymbolKind::Module) && node.label == name
            })
        })
        .cloned()
        .collect();
    resolve_from_candidates(graph, &matches)
}

fn resolve_symbol(
    graph: &RepositoryGraph,
    index: &GraphQueryIndex,
    selector: &SymbolSelector,
) -> ResolutionOutcome {
    let symbol_ids = index.nodes_of_kind(GraphNodeKind::Symbol);
    let matches: Vec<String> = symbol_ids
        .iter()
        .filter(|id| {
            let Some(node) = graph.nodes.get(*id) else {
                return false;
            };
            if node.label != selector.name {
                return false;
            }
            if let Some(expected_path) = &selector.path {
                let Ok(normalized) = validate_repository_relative_path(expected_path) else {
                    return false;
                };
                if node.source.as_ref().map(|source| source.path.as_str())
                    != Some(normalized.as_str())
                {
                    return false;
                }
            }
            if let Some(expected_kind) = selector.symbol_kind {
                if node.symbol_kind != Some(expected_kind) {
                    return false;
                }
            }
            if let Some(container) = &selector.container {
                // Best-effort disambiguation: symbol node IDs embed their
                // container as a colon-delimited segment
                // (`node_id_for_symbol`); a substring check is not a
                // claim of exact structural parsing, only a filter.
                if !node.id.contains(&format!(":{container}:")) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();
    resolve_from_candidates(graph, &matches)
}
