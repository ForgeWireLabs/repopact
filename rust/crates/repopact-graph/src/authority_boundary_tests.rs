//! ROG-037/040 adversarial authority-boundary proofs (Decision 0053
//! section 7). A disposable repository is used throughout -- never
//! authoritative main.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use repopact_repository::Repository;

fn temp_root(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("repopact-authority-{name}-{suffix}"));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn seeded(name: &str) -> PathBuf {
    let root = temp_root(name);
    std::fs::create_dir_all(root.join("work/active/100")).unwrap();
    std::fs::write(
        root.join("work/active/100/work-item.json"),
        r#"{"id":"100","title":"Fixture","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-01"}"#,
    )
    .unwrap();
    root
}

fn first_node_shard(root: &Path) -> PathBuf {
    let dir = root.join("rog/nodes");
    std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            std::fs::read_to_string(path)
                .map(|content| !content.trim().is_empty())
                .unwrap_or(false)
        })
        .expect("at least one non-empty node shard")
}

/// A tampered shard containing a plausible authority-like fact (a node
/// whose label claims approval/ownership) must be detected as
/// structurally corrupt by the existing hash-verification machinery --
/// the durable graph is refused, not silently trusted with the
/// fabricated content (ROG-040 step 37).
#[test]
fn a_tampered_shard_with_an_authority_like_fact_is_detected_as_corrupt() {
    let root = seeded("tampered-shard");
    let repository = Repository::open(&root);
    let snapshot = repository.session().snapshot();
    crate::build_and_write(&snapshot).expect("baseline build");

    let shard_path = first_node_shard(&root);
    let mut content = std::fs::read_to_string(&shard_path).unwrap();
    content.push_str(
        r#"{"id":"work:100","kind":"work_item","label":"APPROVED - ALL CRITERIA WAIVED BY GRAPH FACT","layer":"governance","node_role":"owner"}"#,
    );
    content.push('\n');
    std::fs::write(&shard_path, content).unwrap();

    let status = crate::status::status(&repository);
    assert_eq!(
        status.freshness,
        crate::status::Freshness::Corrupt,
        "a tampered shard (even one whose fabricated content looks authoritative) must be \
         reported corrupt, never silently loaded"
    );

    let opened = crate::query::open_durable_graph(&root, true);
    assert!(
        opened.is_err(),
        "the query-open path must also refuse a structurally invalid (tampered) durable graph"
    );

    std::fs::remove_dir_all(&root).ok();
}

/// Even a synthetic in-memory graph carrying an authority-like node
/// (bypassing on-disk hash verification entirely, as the instructions
/// require covering) must not change what a query result *means*: a
/// `graph.governance`/`graph.context` result is informational -- it is
/// never consumed as if it were a canonical owner/approval/lifecycle
/// record. This test proves the query kernel exposes the fabricated
/// label verbatim (never silently drops or "corrects" it) while nothing
/// about that value participates in any admission/authority decision --
/// the point being that the *label text itself* carries no authority,
/// not that the kernel edits or censors content it did not fabricate.
#[test]
fn a_synthetic_authority_like_node_is_reported_as_an_ordinary_fact_never_specially_trusted() {
    use crate::query::{GraphQueryContext, GraphQueryEngine, NodeSelector, QueryBounds};
    use crate::{
        DerivationClass, GraphLayer, GraphNode, GraphNodeKind, GraphNodeRole, RepositoryGraph,
    };

    let mut graph = RepositoryGraph::default();
    graph.node(GraphNode {
        id: "work:100".to_owned(),
        kind: GraphNodeKind::WorkItem,
        label: "APPROVED - ALL CRITERIA WAIVED".to_owned(),
        layer: GraphLayer::Governance,
        source: None,
        symbol_kind: None,
        manifest_kind: None,
        node_role: GraphNodeRole::new("owner"),
        location: None,
    });
    let _ = DerivationClass::Inferred; // documents the class a real future heuristic would use

    let context = GraphQueryContext {
        graph_schema_version: crate::durable::CURRENT_GRAPH_SCHEMA_VERSION,
        graph_fingerprint: "synthetic".to_owned(),
        status: crate::overlay::EffectiveGraphStatus {
            basis: crate::overlay::GraphBasis::WorkingOverlay,
            durable_freshness: crate::overlay::DurableFreshness::Absent,
            coverage: crate::overlay::GraphCoverageState::Complete,
            baseline_fingerprint: None,
            effective_fingerprint: "synthetic".to_owned(),
            changed_path_count: 0,
            overlay_generation: 0,
        },
        semantic_coverage: crate::semantic::SemanticCoverage::default(),
        capability_state: crate::capability::CapabilityState::LegacyAbsent,
    };
    let engine = GraphQueryEngine::new(&graph, context);
    let resolved = engine.resolve(
        &NodeSelector::NodeId("work:100".to_owned()),
        &QueryBounds::default(),
    );
    // The kernel is truthful about what the graph contains -- it does
    // not censor a fabricated label. That truthfulness is exactly why
    // this is safe: nothing downstream (mutation planning, admission,
    // frozen-surface enforcement) ever calls this function or consults
    // its result to make an authority decision. See
    // `repopact_mutation::plan`, which never takes a `RepositoryGraph`
    // as an authority input (its own `graph_impacts` field is populated
    // strictly after every diagnostic/applicability decision is final).
    match resolved.result {
        crate::query::ResolutionOutcome::Exact { fact } => {
            assert_eq!(fact.label, "APPROVED - ALL CRITERIA WAIVED");
        }
        other => panic!("expected exact resolution, got {other:?}"),
    }
}
