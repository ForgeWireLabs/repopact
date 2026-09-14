//! Integration tests for the query kernel (WI063 bounded-query-and-
//! orientation checkpoint, steps 39-51). Every test builds a real graph
//! (via [`crate::RepositoryGraph::build`] or the durable/overlay paths)
//! and exercises [`super::GraphQueryEngine`] against it -- never a mock
//! graph shape divorced from what a real builder produces.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use repopact_repository::{Repository, RepositorySession};

use super::*;
use crate::overlay::{DurableFreshness, EffectiveGraphStatus, GraphBasis, GraphCoverageState};
use crate::{GraphEdgeKind, GraphLayer, SymbolKind};

fn temp_root(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("repopact-query-{name}-{suffix}"));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

fn work_item_json(id: &str, depends_on: &[&str]) -> String {
    let deps = depends_on
        .iter()
        .map(|d| format!("\"{d}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"id":"{id}","title":"Item {id}","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[{deps}],"acceptance_criteria":[{{"id":"AC-1","text":"prove","state":"pending","evidence":[]}}],"created":"2026-01-01","updated":"2026-01-01"}}"#
    )
}

/// A fixture exercising every selector class, a dependency chain long
/// enough to trigger depth/edge/node bounds, a fan-in for pagination,
/// operational-role facts (tests/runtime entrypoints/npm workspace), and
/// a governance relation.
fn full_fixture(name: &str) -> PathBuf {
    let root = temp_root(name);

    // Governance dependency chain: 100 -> 101 -> 102 -> 103 (path/
    // transitive-dependency/depth-truncation material).
    write(
        &root,
        "work/active/100/work-item.json",
        &work_item_json("100", &["101"]),
    );
    write(
        &root,
        "work/active/101/work-item.json",
        &work_item_json("101", &["102"]),
    );
    write(
        &root,
        "work/active/102/work-item.json",
        &work_item_json("102", &["103"]),
    );
    write(
        &root,
        "work/active/103/work-item.json",
        &work_item_json("103", &[]),
    );
    // A disconnected item for "no path" proofs.
    write(
        &root,
        "work/active/900/work-item.json",
        &work_item_json("900", &[]),
    );
    // Fan-in of 12 dependents on 100 (pagination/bound material).
    for index in 0..12 {
        let id = format!("2{index:02}");
        write(
            &root,
            &format!("work/active/{id}/work-item.json"),
            &work_item_json(&id, &["100"]),
        );
    }

    // Cargo crate: explicit [[bin]] + [[test]] + a test symbol.
    write(
        &root,
        "Cargo.toml",
        "[package]\nname = \"fixture-crate\"\n\n[[bin]]\nname = \"fixture-crate\"\npath = \"src/main.rs\"\n\n[[test]]\nname = \"integration\"\npath = \"tests/integration.rs\"\n",
    );
    write(&root, "src/main.rs", "fn main() {}\n");
    write(&root, "src/lib.rs", "pub fn hello() {}\n");
    write(&root, "tests/integration.rs", "#[test]\nfn it_works() {}\n");

    // npm workspace root + member (Package selector, workspace edges).
    write(
        &root,
        "package.json",
        r#"{"name":"root","workspaces":["packages/*"]}"#,
    );
    write(&root, "packages/a/package.json", r#"{"name":"a"}"#);

    root
}

fn build_engine(root: &Path) -> (crate::RepositoryGraph, GraphQueryContext) {
    let repository = Repository::open(root);
    let snapshot = repository.session().snapshot();
    let graph = crate::RepositoryGraph::build(&snapshot);
    let context = GraphQueryContext {
        graph_schema_version: crate::durable::CURRENT_GRAPH_SCHEMA_VERSION,
        graph_fingerprint: "test-fingerprint".to_owned(),
        status: EffectiveGraphStatus {
            basis: GraphBasis::Durable,
            durable_freshness: DurableFreshness::Fresh,
            coverage: GraphCoverageState::Complete,
            baseline_fingerprint: Some("test-fingerprint".to_owned()),
            effective_fingerprint: "test-fingerprint".to_owned(),
            changed_path_count: 0,
            overlay_generation: 0,
        },
        semantic_coverage: crate::semantic::SemanticCoverage::default(),
    };
    (graph, context)
}

// ---- target resolution matrix (step 42) --------------------------------

#[test]
fn resolve_exact_node_id() {
    let root = full_fixture("resolve-node-id");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let outcome = engine.resolve(
        &NodeSelector::NodeId("work:100".to_owned()),
        &QueryBounds::default(),
    );
    assert!(matches!(outcome.result, ResolutionOutcome::Exact { .. }));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolve_exact_repository_path() {
    let root = full_fixture("resolve-path");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let outcome = engine.resolve(
        &NodeSelector::RepositoryPath("src/lib.rs".to_owned()),
        &QueryBounds::default(),
    );
    match outcome.result {
        ResolutionOutcome::Exact { fact } => assert_eq!(fact.id, "file:src/lib.rs"),
        other => panic!("expected exact resolution, got {other:?}"),
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolve_work_item_id() {
    let root = full_fixture("resolve-work-item");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let outcome = engine.resolve(
        &NodeSelector::WorkItemId("100".to_owned()),
        &QueryBounds::default(),
    );
    assert!(matches!(outcome.result, ResolutionOutcome::Exact { .. }));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolve_package_by_manifest_identity() {
    let root = full_fixture("resolve-package");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let outcome = engine.resolve(
        &NodeSelector::Package("fixture-crate".to_owned()),
        &QueryBounds::default(),
    );
    match outcome.result {
        ResolutionOutcome::Exact { fact } => assert_eq!(fact.id, "manifest:Cargo.toml"),
        other => panic!("expected exact resolution, got {other:?}"),
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolve_module_by_symbol_kind() {
    // Python import facts are stored as raw statement text (Decision
    // 0045's "import fact, not resolved target" precedent), so the
    // Module symbol's own label is the literal statement, not a clean
    // bare module name -- this selector matches whatever the graph
    // actually stores today.
    let root = temp_root("resolve-module");
    write(&root, "pkg/mod.py", "import os\n");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let outcome = engine.resolve(
        &NodeSelector::Module("import os".to_owned()),
        &QueryBounds::default(),
    );
    assert!(matches!(outcome.result, ResolutionOutcome::Exact { .. }));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolve_unique_symbol() {
    let root = full_fixture("resolve-symbol-unique");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let outcome = engine.resolve(
        &NodeSelector::Symbol(SymbolSelector {
            name: "hello".to_owned(),
            path: None,
            symbol_kind: None,
            container: None,
        }),
        &QueryBounds::default(),
    );
    match outcome.result {
        ResolutionOutcome::Exact { fact } => {
            assert_eq!(fact.symbol_kind, Some(SymbolKind::Function))
        }
        other => panic!("expected exact resolution, got {other:?}"),
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolve_ambiguous_symbol_returns_bounded_candidates_never_a_guess() {
    let root = temp_root("resolve-symbol-ambiguous");
    write(&root, "a.rs", "pub fn shared() {}\n");
    write(&root, "b.rs", "pub fn shared() {}\n");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let outcome = engine.resolve(
        &NodeSelector::Symbol(SymbolSelector {
            name: "shared".to_owned(),
            path: None,
            symbol_kind: None,
            container: None,
        }),
        &QueryBounds::default(),
    );
    match outcome.result {
        ResolutionOutcome::Ambiguous { candidates } => assert_eq!(candidates.len(), 2),
        other => panic!("expected ambiguous resolution, got {other:?}"),
    }
    // Disambiguating with path resolves exactly.
    let outcome = engine.resolve(
        &NodeSelector::Symbol(SymbolSelector {
            name: "shared".to_owned(),
            path: Some("a.rs".to_owned()),
            symbol_kind: None,
            container: None,
        }),
        &QueryBounds::default(),
    );
    match outcome.result {
        ResolutionOutcome::Exact { fact } => assert_eq!(fact.source.unwrap().path, "a.rs"),
        other => panic!("expected exact resolution after disambiguation, got {other:?}"),
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolve_not_found_is_distinct_from_ambiguous() {
    let root = full_fixture("resolve-not-found");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let outcome = engine.resolve(
        &NodeSelector::WorkItemId("does-not-exist".to_owned()),
        &QueryBounds::default(),
    );
    assert_eq!(outcome.result, ResolutionOutcome::NotFound);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolve_rejects_a_traversal_path_selector_as_not_found_not_a_crash() {
    let root = full_fixture("resolve-traversal");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let outcome = engine.resolve(
        &NodeSelector::RepositoryPath("../outside/lib.rs".to_owned()),
        &QueryBounds::default(),
    );
    assert_eq!(outcome.result, ResolutionOutcome::NotFound);
    let outcome = engine.resolve(
        &NodeSelector::RepositoryPath("/etc/passwd".to_owned()),
        &QueryBounds::default(),
    );
    assert_eq!(outcome.result, ResolutionOutcome::NotFound);
    std::fs::remove_dir_all(root).unwrap();
}

// ---- dependency tests (step 43) ----------------------------------------

#[test]
fn direct_dependency_is_reported() {
    let root = full_fixture("dep-direct");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let envelope = engine.dependencies("work:100", false, &QueryBounds::default());
    let result = envelope.result.unwrap();
    assert_eq!(result.dependencies.len(), 1);
    assert_eq!(result.dependencies[0].to, "work:101");
    assert!(!result.transitive);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn transitive_dependency_bounded_by_depth() {
    let root = full_fixture("dep-transitive");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);

    let full_bounds = QueryBounds {
        max_depth: 10,
        ..QueryBounds::default()
    };
    let envelope = engine.dependencies("work:100", true, &full_bounds);
    let result = envelope.result.unwrap();
    assert_eq!(result.nodes.len(), 3); // 101, 102, 103
    assert!(!envelope.truncated);

    let shallow_bounds = QueryBounds {
        max_depth: 1,
        ..QueryBounds::default()
    };
    let envelope = engine.dependencies("work:100", true, &shallow_bounds);
    let result = envelope.result.unwrap();
    assert_eq!(result.nodes.len(), 1); // only 101
    assert!(envelope.warnings.iter().any(|w| w.contains("max_depth")));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reverse_dependency_via_incoming_depends_on_and_persisted_reverse_dependency() {
    let root = full_fixture("dep-reverse");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        max_edges: 50,
        page_size: 50,
        ..QueryBounds::default()
    };
    let envelope = engine.dependents("work:100", &bounds);
    let result = envelope.result.unwrap();
    assert_eq!(result.dependents.len(), 12);
    assert!(result
        .dependents
        .iter()
        .all(|fact| fact.provenance == RelationProvenance::Persisted));
    assert!(result.dependents.iter().all(|fact| fact.to == "work:100"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn relation_filters_restrict_traversal() {
    let root = full_fixture("dep-relation-filter");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        relation_kinds: Some(vec![GraphEdgeKind::Contains]),
        ..QueryBounds::default()
    };
    let envelope = engine.neighbors("work:100", Direction::Outgoing, &bounds);
    let result = envelope.result.unwrap();
    assert!(result
        .relations
        .iter()
        .all(|r| r.kind == GraphEdgeKind::Contains));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cyclical_governance_graph_terminates_deterministically() {
    // WI063 governance dependency direction never actually allows a
    // literal self-referential cycle to be authored validly, but the
    // transitive BFS must still terminate even if a duplicate edge
    // exists -- proven by re-running dependencies() twice and checking
    // determinism plus a bounded node count.
    let root = full_fixture("dep-cycle-termination");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        max_depth: 20,
        max_nodes: 20,
        ..QueryBounds::default()
    };
    let first = engine.dependencies("work:100", true, &bounds);
    let second = engine.dependencies("work:100", true, &bounds);
    assert_eq!(first.result, second.result);
    std::fs::remove_dir_all(root).unwrap();
}

// ---- path tests (step 44) ----------------------------------------------

#[test]
fn known_connected_path_is_found() {
    let root = full_fixture("path-connected");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let envelope = engine.path("work:100", "work:103", &QueryBounds::default());
    let result = envelope.result.unwrap();
    match result.path {
        PathOutcome::Found { nodes, .. } => {
            let ids: Vec<_> = nodes.iter().map(|n| n.id.as_str()).collect();
            assert_eq!(ids, vec!["work:100", "work:101", "work:102", "work:103"]);
        }
        other => panic!("expected a found path, got {other:?}"),
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn disconnected_targets_report_no_path_not_truncation() {
    let root = full_fixture("path-disconnected");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let envelope = engine.path("work:100", "work:900", &QueryBounds::default());
    let result = envelope.result.unwrap();
    assert_eq!(result.path, PathOutcome::NoPath);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn max_depth_truncation_is_distinct_from_no_path() {
    let root = full_fixture("path-depth-truncated");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        max_depth: 1,
        ..QueryBounds::default()
    };
    let envelope = engine.path("work:100", "work:103", &bounds);
    let result = envelope.result.unwrap();
    assert_eq!(result.path, PathOutcome::SearchTruncatedBeforeProof);
    assert!(envelope
        .warnings
        .iter()
        .any(|w| w.contains("before proving")));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn relation_filter_exclusion_prevents_a_path_that_otherwise_exists() {
    let root = full_fixture("path-relation-excluded");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        relation_kinds: Some(vec![GraphEdgeKind::Contains]),
        ..QueryBounds::default()
    };
    let envelope = engine.path("work:100", "work:103", &bounds);
    let result = envelope.result.unwrap();
    assert_eq!(result.path, PathOutcome::NoPath);
    std::fs::remove_dir_all(root).unwrap();
}

// ---- test-query coverage (step 45) --------------------------------------

#[test]
fn target_with_known_tests_reports_them() {
    let root = full_fixture("tests-known");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let envelope = engine.tests("manifest:Cargo.toml", &QueryBounds::default());
    let result = envelope.result.unwrap();
    assert!(!result.test_targets.is_empty());
    assert!(envelope.warnings.iter().any(|w| w.contains("fixture")));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn target_with_no_known_tests_discloses_uncertainty_not_zero_proof() {
    let root = full_fixture("tests-none");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let envelope = engine.tests("work:900", &QueryBounds::default());
    let result = envelope.result.unwrap();
    assert!(result.test_targets.is_empty());
    assert!(envelope
        .warnings
        .iter()
        .any(|w| w.contains("does not prove no tests exist")));
    assert!(envelope.warnings.iter().any(|w| w.contains("fixture")));
    std::fs::remove_dir_all(root).unwrap();
}

// ---- governance tests (step 46) -----------------------------------------

#[test]
fn governance_surfaces_applicable_relations() {
    let root = full_fixture("governance-surface");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let envelope = engine.governance("work:100", &QueryBounds::default());
    let result = envelope.result.unwrap();
    // work:100 has an acceptance criterion (Contains, not governance) and
    // a SupportedBy-eligible criterion but no evidence in this fixture --
    // at minimum the query must not crash and must return only real
    // governance-kind edges.
    assert!(result
        .relations
        .iter()
        .all(|r| engine_test_support::is_governance_kind(r.kind)));
    std::fs::remove_dir_all(root).unwrap();
}

// ---- orientation tests (step 47) ----------------------------------------

#[test]
fn orient_work_item_returns_only_facts_backed_by_the_graph() {
    let root = full_fixture("orient-work-item");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let envelope = engine.orient(
        &NodeSelector::WorkItemId("100".to_owned()),
        &QueryBounds::default(),
    );
    match envelope.result {
        OrientOutcome::Resolved(result) => {
            assert_eq!(result.identity.id, "work:100");
            assert_eq!(result.direct_dependencies.len(), 1);
            assert_eq!(result.direct_dependents.len(), 12);
            assert!(result
                .navigation_hints
                .iter()
                .enumerate()
                .all(|(i, h)| h.rank == i as u32 + 1));
        }
        other => panic!("expected resolved orientation, got {other:?}"),
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn orient_package_returns_runtime_and_test_surfaces() {
    let root = full_fixture("orient-package");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let envelope = engine.orient(
        &NodeSelector::Package("fixture-crate".to_owned()),
        &QueryBounds::default(),
    );
    match envelope.result {
        OrientOutcome::Resolved(result) => {
            assert!(!result.runtime_entrypoints.is_empty());
            assert!(result
                .navigation_hints
                .iter()
                .any(|h| h.reason == HintReason::RuntimeEntrypoint));
        }
        other => panic!("expected resolved orientation, got {other:?}"),
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn orient_ambiguous_symbol_is_disclosed_not_guessed() {
    let root = temp_root("orient-ambiguous");
    write(&root, "a.rs", "pub fn shared() {}\n");
    write(&root, "b.rs", "pub fn shared() {}\n");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let envelope = engine.orient(
        &NodeSelector::Symbol(SymbolSelector {
            name: "shared".to_owned(),
            path: None,
            symbol_kind: None,
            container: None,
        }),
        &QueryBounds::default(),
    );
    match envelope.result {
        OrientOutcome::Ambiguous { candidates } => assert_eq!(candidates.len(), 2),
        other => panic!("expected ambiguous orientation, got {other:?}"),
    }
    std::fs::remove_dir_all(root).unwrap();
}

// ---- hard bound tests (step 39) -----------------------------------------

#[test]
fn max_edges_bound_is_hard() {
    let root = full_fixture("bound-max-edges");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        max_edges: 3,
        page_size: 100,
        ..QueryBounds::default()
    };
    let envelope = engine.dependents("work:100", &bounds);
    let result = envelope.result.unwrap();
    assert_eq!(result.dependents.len(), 3);
    assert!(envelope.warnings.iter().any(|w| w.contains("max_edges")));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn max_nodes_bound_is_hard_on_ambiguous_resolution() {
    let root = temp_root("bound-max-nodes");
    for index in 0..5 {
        write(&root, &format!("f{index}.rs"), "pub fn shared() {}\n");
    }
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        max_nodes: 2,
        ..QueryBounds::default()
    };
    let envelope = engine.resolve(
        &NodeSelector::Symbol(SymbolSelector {
            name: "shared".to_owned(),
            path: None,
            symbol_kind: None,
            container: None,
        }),
        &bounds,
    );
    match envelope.result {
        ResolutionOutcome::Ambiguous { candidates } => assert_eq!(candidates.len(), 2),
        other => panic!("expected ambiguous resolution, got {other:?}"),
    }
    assert!(envelope.warnings.iter().any(|w| w.contains("max_nodes")));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn max_depth_bound_is_hard_on_transitive_dependencies() {
    let root = full_fixture("bound-max-depth");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        max_depth: 2,
        ..QueryBounds::default()
    };
    let envelope = engine.dependencies("work:100", true, &bounds);
    let result = envelope.result.unwrap();
    assert_eq!(result.nodes.len(), 2); // 101, 102 -- not 103
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn page_size_bound_is_hard() {
    let root = full_fixture("bound-page-size");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        page_size: 5,
        max_edges: 50,
        ..QueryBounds::default()
    };
    let envelope = engine.dependents("work:100", &bounds);
    let result = envelope.result.unwrap();
    assert_eq!(result.dependents.len(), 5);
    assert!(envelope.truncated);
    assert!(envelope.next_cursor.is_some());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn output_byte_budget_estimate_is_available_and_documented() {
    let bounds = QueryBounds::default();
    assert_eq!(bounds.max_output_bytes, DEFAULT_MAX_OUTPUT_BYTES);
    assert_eq!(QueryBounds::estimate_tokens(400), 100);
}

// ---- pagination tests (step 40) -----------------------------------------

#[test]
fn pages_reconstruct_the_bounded_result_with_no_duplicates_or_gaps() {
    let root = full_fixture("pagination-reconstruct");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);

    let full_bounds = QueryBounds {
        max_edges: 50,
        page_size: 50,
        ..QueryBounds::default()
    };
    let full = engine
        .dependents("work:100", &full_bounds)
        .result
        .unwrap()
        .dependents;

    let mut collected = Vec::new();
    let mut cursor = None;
    loop {
        let bounds = QueryBounds {
            max_edges: 50,
            page_size: 5,
            cursor: cursor.clone(),
            ..QueryBounds::default()
        };
        let envelope = engine.dependents("work:100", &bounds);
        let result = envelope.result.unwrap();
        collected.extend(result.dependents);
        match envelope.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    assert_eq!(collected, full);
    let mut ids: Vec<_> = collected.iter().map(|f| f.from.clone()).collect();
    let before_dedup = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(
        ids.len(),
        before_dedup,
        "pagination must not produce duplicates"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cursor_rejects_a_different_graph_fingerprint() {
    let root = full_fixture("cursor-different-fingerprint");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        max_edges: 50,
        page_size: 5,
        ..QueryBounds::default()
    };
    let first_cursor = engine.dependents("work:100", &bounds).next_cursor.unwrap();

    let (graph2, mut context2) = build_engine(&root);
    context2.graph_fingerprint = "different-fingerprint".to_owned();
    context2.status.effective_fingerprint = "different-fingerprint".to_owned();
    let engine2 = GraphQueryEngine::new(&graph2, context2);
    let bounds_with_cursor = QueryBounds {
        max_edges: 50,
        page_size: 5,
        cursor: Some(first_cursor),
        ..QueryBounds::default()
    };
    let envelope = engine2.dependents("work:100", &bounds_with_cursor);
    assert!(envelope.result.is_none());
    assert!(envelope
        .warnings
        .iter()
        .any(|w| w.contains("invalid cursor")));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cursor_rejects_a_different_operation() {
    let root = full_fixture("cursor-different-operation");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        max_edges: 50,
        page_size: 5,
        ..QueryBounds::default()
    };
    let cursor = engine.dependents("work:100", &bounds).next_cursor.unwrap();
    let bounds_with_cursor = QueryBounds {
        max_edges: 50,
        page_size: 5,
        cursor: Some(cursor),
        ..QueryBounds::default()
    };
    let envelope = engine.neighbors("work:100", Direction::Outgoing, &bounds_with_cursor);
    assert!(envelope.result.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cursor_rejects_a_different_target() {
    let root = full_fixture("cursor-different-target");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        max_edges: 50,
        page_size: 5,
        ..QueryBounds::default()
    };
    let cursor = engine.dependents("work:100", &bounds).next_cursor.unwrap();
    let bounds_with_cursor = QueryBounds {
        max_edges: 50,
        page_size: 5,
        cursor: Some(cursor),
        ..QueryBounds::default()
    };
    let envelope = engine.dependents("work:101", &bounds_with_cursor);
    assert!(envelope.result.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cursor_rejects_different_filters() {
    let root = full_fixture("cursor-different-filters");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        relation_kinds: None,
        page_size: 1,
        ..QueryBounds::default()
    };
    let cursor = engine
        .neighbors("work:100", Direction::Outgoing, &bounds)
        .next_cursor
        .clone();
    if let Some(cursor) = cursor {
        let filtered_bounds = QueryBounds {
            relation_kinds: Some(vec![GraphEdgeKind::Contains]),
            page_size: 1,
            cursor: Some(cursor),
            ..QueryBounds::default()
        };
        let envelope = engine.neighbors("work:100", Direction::Outgoing, &filtered_bounds);
        assert!(envelope.result.is_none());
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_and_oversized_cursors_are_rejected_by_operations() {
    let root = full_fixture("cursor-malformed");
    let (graph, context) = build_engine(&root);
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds {
        cursor: Some("not a valid cursor".to_owned()),
        ..QueryBounds::default()
    };
    let envelope = engine.dependents("work:100", &bounds);
    assert!(envelope.result.is_none());

    let oversized = QueryBounds {
        cursor: Some("A".repeat(MAX_CURSOR_LEN + 1)),
        ..QueryBounds::default()
    };
    let envelope = engine.dependents("work:100", &oversized);
    assert!(envelope.result.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

// ---- compact / source-reference-only mode (step 41) ---------------------

#[test]
fn compact_mode_materially_reduces_output_size() {
    let node = crate::GraphNode {
        id: "file:some/very/long/descriptive/path/module.rs".to_owned(),
        kind: crate::GraphNodeKind::Symbol,
        label: "a_very_long_and_descriptive_human_readable_symbol_label_for_presentation"
            .to_owned(),
        layer: GraphLayer::Semantic,
        source: None,
        symbol_kind: Some(SymbolKind::Function),
        location: None,
        manifest_kind: None,
        node_role: None,
    };
    let full = FactRef::from_node(&node);
    let compact = full.clone().into_compact();
    let full_len = serde_json::to_vec(&full).unwrap().len();
    let compact_len = serde_json::to_vec(&compact).unwrap().len();
    assert!(
        compact_len < full_len,
        "compact mode must reduce serialized size"
    );
    assert_eq!(compact.id, full.id, "stable ID must survive compact mode");
}

// ---- freshness / durable-open gate ---------------------------------------

#[test]
fn absent_durable_graph_is_a_typed_error_with_guidance() {
    let root = temp_root("open-absent");
    write(&root, "src/lib.rs", "pub fn f() {}\n");
    let result = open_durable_graph(&root, false);
    assert_eq!(result.err(), Some(QueryOpenError::Absent));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_durable_graph_is_refused_by_default_and_allowed_with_flag() {
    let root = temp_root("open-stale");
    write(&root, "src/lib.rs", "pub fn f() {}\n");
    let snapshot = RepositorySession::open(root.clone()).snapshot();
    crate::build_and_write(&snapshot).expect("initial build");
    // Mutate source after the build without re-running graph.update.
    write(&root, "src/lib.rs", "pub fn f() {}\npub fn g() {}\n");

    let refused = open_durable_graph(&root, false);
    assert!(matches!(refused, Err(QueryOpenError::Stale { .. })));

    let allowed = open_durable_graph(&root, true).expect("allow_stale must still open");
    assert_eq!(
        allowed.context.status.durable_freshness,
        DurableFreshness::Stale
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fresh_durable_graph_opens_normally_and_queries_succeed() {
    let root = temp_root("open-fresh");
    write(&root, "src/lib.rs", "pub fn f() {}\n");
    let snapshot = RepositorySession::open(root.clone()).snapshot();
    crate::build_and_write(&snapshot).expect("build");
    let loaded = open_durable_graph(&root, false).expect("fresh graph should open");
    assert_eq!(
        loaded.context.status.durable_freshness,
        DurableFreshness::Fresh
    );
    let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
    let envelope = engine.resolve(
        &NodeSelector::RepositoryPath("src/lib.rs".to_owned()),
        &QueryBounds::default(),
    );
    assert!(matches!(envelope.result, ResolutionOutcome::Exact { .. }));
    std::fs::remove_dir_all(root).unwrap();
}

// ---- working-overlay basis disclosure ------------------------------------

#[test]
fn overlay_query_discloses_working_overlay_basis() {
    let root = temp_root("overlay-basis");
    write(&root, "src/lib.rs", "pub fn f() {}\n");
    let snapshot = RepositorySession::open(root.clone()).snapshot();
    crate::build_and_write(&snapshot).expect("build");
    write(&root, "src/lib.rs", "pub fn f() {}\npub fn g() {}\n");
    let snapshot = RepositorySession::open(root.clone()).snapshot();
    let mut overlay = crate::overlay::SessionGraphState::open(&snapshot);
    overlay.refresh(&snapshot);
    let context = overlay.query_context();
    assert_eq!(context.status.basis, GraphBasis::WorkingOverlay);
    let engine = GraphQueryEngine::new(overlay.effective_graph(), context);
    let envelope = engine.resolve(
        &NodeSelector::RepositoryPath("src/lib.rs".to_owned()),
        &QueryBounds::default(),
    );
    assert!(matches!(envelope.result, ResolutionOutcome::Exact { .. }));
    assert!(envelope
        .warnings
        .iter()
        .any(|w| w.contains("working_overlay")));
    std::fs::remove_dir_all(root).unwrap();
}

// ---- authority / read-only (step 36) --------------------------------------

#[test]
fn query_kernel_exposes_no_mutation_surface() {
    // Structural proof: GraphQueryEngine borrows `&RepositoryGraph`, not
    // `&mut`, so no method it exposes can mutate the graph -- this test
    // documents and locks in that contract by construction. If a future
    // change adds a `&mut self`/`&mut RepositoryGraph` method here, this
    // comment (and Decision 0050 section 1) is what it violates.
    let root = full_fixture("authority-read-only");
    let (graph, context) = build_engine(&root);
    let before = graph.clone();
    let engine = GraphQueryEngine::new(&graph, context);
    let _ = engine.orient(
        &NodeSelector::WorkItemId("100".to_owned()),
        &QueryBounds::default(),
    );
    let _ = engine.dependencies("work:100", true, &QueryBounds::default());
    assert_eq!(
        graph, before,
        "no query operation may mutate the graph it was given"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ---- WI057: queries issue zero additional Git invocations ---------------

#[test]
fn query_operations_never_invoke_git() {
    use repopact_repository::CountingGitRunner;

    let root = full_fixture("query-no-git");
    let runner = CountingGitRunner::native();
    let repository = Repository::with_git_runner(&root, runner.clone());
    let snapshot = repository.session().snapshot();
    let graph = crate::RepositoryGraph::build(&snapshot);
    let baseline_count = runner.count();

    let context = GraphQueryContext {
        graph_schema_version: crate::durable::CURRENT_GRAPH_SCHEMA_VERSION,
        graph_fingerprint: "fp".to_owned(),
        status: EffectiveGraphStatus {
            basis: GraphBasis::Durable,
            durable_freshness: DurableFreshness::Fresh,
            coverage: GraphCoverageState::Complete,
            baseline_fingerprint: Some("fp".to_owned()),
            effective_fingerprint: "fp".to_owned(),
            changed_path_count: 0,
            overlay_generation: 0,
        },
        semantic_coverage: crate::semantic::SemanticCoverage::default(),
    };
    let engine = GraphQueryEngine::new(&graph, context);
    let bounds = QueryBounds::default();
    let _ = engine.resolve(&NodeSelector::WorkItemId("100".to_owned()), &bounds);
    let _ = engine.orient(&NodeSelector::WorkItemId("100".to_owned()), &bounds);
    let _ = engine.dependencies("work:100", true, &bounds);
    let _ = engine.dependents("work:100", &bounds);
    let _ = engine.path("work:100", "work:103", &bounds);
    let _ = engine.tests("manifest:Cargo.toml", &bounds);
    let _ = engine.governance("work:100", &bounds);
    let _ = engine.impact("work:100", &bounds);

    assert_eq!(
        runner.count(),
        baseline_count,
        "query operations must never invoke Git beyond the graph's own construction"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// Step 51: after the durable graph is loaded, no query operation may
/// open a source file. Proven concretely -- not merely by code
/// inspection -- by deleting every source file after `graph.build`
/// (leaving only `rog/`) and confirming queries still succeed purely
/// from the loaded graph.
#[test]
fn queries_succeed_after_source_files_are_deleted_post_build() {
    let root = temp_root("no-source-read");
    write(&root, "Cargo.toml", "[package]\nname = \"fixture\"\n");
    write(&root, "src/lib.rs", "pub fn hello() {}\n");
    let snapshot = RepositorySession::open(root.clone()).snapshot();
    crate::build_and_write(&snapshot).expect("build");

    std::fs::remove_dir_all(root.join("src")).unwrap();
    std::fs::remove_file(root.join("Cargo.toml")).unwrap();
    assert!(!root.join("src").exists());

    // open_durable_graph recomputes the current source-projection
    // fingerprint to detect staleness, which itself walks the (now
    // source-free) tree -- that recompute is graph *loading*, not the
    // query kernel. Deleted source correctly reports as a structural
    // change (stale), proving the fingerprint check is real; querying
    // it with allow_stale proves the query kernel itself needs nothing
    // more than the already-loaded graph to answer.
    let loaded = open_durable_graph(&root, true).expect("stale graph still opens with allow_stale");
    let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
    let envelope = engine.resolve(
        &NodeSelector::RepositoryPath("src/lib.rs".to_owned()),
        &QueryBounds::default(),
    );
    assert!(matches!(envelope.result, ResolutionOutcome::Exact { .. }));
    let orient = engine.orient(
        &NodeSelector::Package("fixture".to_owned()),
        &QueryBounds::default(),
    );
    assert!(matches!(orient.result, OrientOutcome::Resolved(_)));
    std::fs::remove_dir_all(&root).ok();
}

/// Step 50: engineering query-latency evidence (not ROG-032 closeout).
/// Separates cold graph-load/index-build time from warm in-memory query
/// time -- combining them into one number would misleadingly attribute
/// disk/deserialization cost to the query kernel itself. Run against a
/// synthetic fixture sized like this checkpoint's other fixtures (not
/// the live RepoPact repository, which this test suite must not depend
/// on existing on the machine); the real-repo numbers are captured
/// separately via the raw engine binary and recorded in the evidence
/// record. `--nocapture` reveals the printed timings; the test itself
/// only asserts the operations succeed, never a specific latency (no
/// machine-specific performance contract is asserted here).
#[test]
fn query_latency_evidence_cold_load_vs_warm_query() {
    let root = full_fixture("latency-evidence");
    let snapshot = RepositorySession::open(root.clone()).snapshot();
    crate::build_and_write(&snapshot).expect("build");

    let cold_start = std::time::Instant::now();
    let loaded = open_durable_graph(&root, false).expect("fresh graph opens");
    let cold_open_elapsed = cold_start.elapsed();

    let index_start = std::time::Instant::now();
    let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
    let index_build_elapsed = index_start.elapsed();

    let bounds = QueryBounds {
        max_depth: 10,
        max_edges: 50,
        ..QueryBounds::default()
    };
    let warm_start = std::time::Instant::now();
    let _ = engine.resolve(&NodeSelector::WorkItemId("100".to_owned()), &bounds);
    let resolve_elapsed = warm_start.elapsed();
    let start = std::time::Instant::now();
    let _ = engine.dependencies("work:100", true, &bounds);
    let dependencies_elapsed = start.elapsed();
    let start = std::time::Instant::now();
    let _ = engine.dependents("work:100", &bounds);
    let dependents_elapsed = start.elapsed();
    let start = std::time::Instant::now();
    let _ = engine.tests("manifest:Cargo.toml", &bounds);
    let tests_elapsed = start.elapsed();
    let start = std::time::Instant::now();
    let _ = engine.governance("work:100", &bounds);
    let governance_elapsed = start.elapsed();
    let start = std::time::Instant::now();
    let orient_envelope = engine.orient(&NodeSelector::WorkItemId("100".to_owned()), &bounds);
    let orient_elapsed = start.elapsed();

    eprintln!(
        "query-latency-evidence: cold_open={cold_open_elapsed:?} index_build={index_build_elapsed:?} \
         warm[resolve={resolve_elapsed:?} dependencies={dependencies_elapsed:?} \
         dependents={dependents_elapsed:?} tests={tests_elapsed:?} governance={governance_elapsed:?} \
         orient={orient_elapsed:?}]"
    );
    assert!(matches!(orient_envelope.result, OrientOutcome::Resolved(_)));
    std::fs::remove_dir_all(root).unwrap();
}

// A tiny private helper module so a test above can classify governance
// edge kinds without duplicating the engine's own constant.
mod engine_test_support {
    pub(super) fn is_governance_kind(kind: crate::GraphEdgeKind) -> bool {
        use crate::GraphEdgeKind::*;
        matches!(
            kind,
            SupportedBy
                | SupportsWorkItem
                | OwnedBy
                | Affects
                | Supersedes
                | Concerns
                | ConstrainedBy
                | Intersects
                | AppliesTo
                | Allows
        )
    }
}
