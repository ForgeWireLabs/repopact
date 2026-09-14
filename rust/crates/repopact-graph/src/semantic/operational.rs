//! Orchestrator-level cross-file operational passes (WI063 operational-
//! surface-completion checkpoint, Decision 0049). Individual adapters and
//! metadata extractors are forbidden from crawling the repository (each
//! sees exactly one file's content); any fact that requires knowing about
//! *other* files -- an npm workspace glob's actual matching directories,
//! whether a generated file's known generator source is also present,
//! Rust's implicit default binary, a crate's sibling integration-test
//! directory -- can only be derived here, where the full
//! [`SourceProjection`] and the already-assembled [`RepositoryGraph`] are
//! both available after every per-file contribution has been collected.
//!
//! Every pass in this module is deterministic, bounded, and
//! repository-relative: no filesystem walking beyond what `projection`
//! already lists (which itself already excludes `node_modules` and every
//! other `IGNORED_PARTS` entry, Decision 0044), no npm/cargo/python
//! execution, no network access. Node/edge identity is always derived
//! from an existing repository-relative path -- never a new invented ID
//! shape, never line-number-based.
//!
//! This is called from both `semantic::extend` (the full-build path) and
//! `incremental::plan_reconciliation` (the incremental-update and
//! session-overlay path) -- the same shared contribution pipeline
//! Decision 0049 section 6 requires, not a second graph builder. It is
//! *not* re-run by `overlay::SessionGraphState::reconcile`'s single-file
//! watcher fast path: a workspace-membership, generated-boundary, or
//! implicit-entrypoint fact that changes because of an edit becomes
//! visible on the next full reconcile (`refresh`/`open`), not on the
//! immediately-following targeted patch. This is a disclosed, bounded
//! limitation, not a silent gap.

use std::collections::BTreeSet;

use repopact_types::{RecordKind, RecordRef};

use super::{FileCoverage, FileCoverageEntry, SkipReason};
use crate::projection::SourceProjection;
use crate::{
    roles, DerivationClass, GraphEdge, GraphEdgeKind, GraphLayer, GraphNodeRole, GraphRelationRole,
    ManifestKind, RepositoryGraph,
};

/// The real, source-backed generator contracts this checkpoint can prove
/// against RepoPact's own repository: a generated file with a genuinely
/// known generator source, not a filename guess. Both sides are checked
/// against the graph before an edge is ever emitted -- a synthetic
/// fixture missing either file legitimately emits nothing here.
const KNOWN_GENERATOR_CONTRACTS: &[(&str, &str)] = &[
    (
        "rust/apps/repopact-desktop/src/generated/types.ts",
        "rust/crates/repopact-desktop-api/src/bin/generate-types.rs",
    ),
    (
        "audits/reports/dashboard.md",
        "repopact/generate_dashboard.py",
    ),
];

fn file_node_id(relative_path: &str) -> String {
    format!("file:{relative_path}")
}

fn parent_dir(relative_path: &str) -> &str {
    match relative_path.rfind('/') {
        Some(index) => &relative_path[..index],
        None => "",
    }
}

fn file_name(relative_path: &str) -> &str {
    relative_path.rsplit('/').next().unwrap_or(relative_path)
}

fn source_ref(relative_path: &str) -> RecordRef {
    RecordRef::new(
        RecordKind::File,
        relative_path.to_owned(),
        relative_path.to_owned(),
    )
}

/// Run every cross-file operational pass over an already-assembled graph.
pub(crate) fn apply(
    graph: &mut RepositoryGraph,
    projection: &SourceProjection,
    per_file: &[FileCoverageEntry],
) {
    resolve_npm_workspace_members(graph, projection);
    tag_generated_boundaries(graph, per_file);
    emit_known_generator_contracts(graph);
    tag_rust_implicit_binaries(graph, projection);
    tag_cargo_integration_test_directories(graph, projection);
    tag_naming_convention_test_files(graph, projection);
}

/// npm `workspaces` glob resolution: bounded, deterministic,
/// repository-contained. Supports the two glob shapes real `package.json`
/// `workspaces` arrays overwhelmingly use -- an exact directory path, or a
/// single trailing `/*` wildcard -- against the `package.json` paths
/// `projection` already lists. Never executes npm, never touches the
/// network, never descends into `node_modules` (excluded upstream by
/// `IGNORED_PARTS`).
fn resolve_npm_workspace_members(graph: &mut RepositoryGraph, projection: &SourceProjection) {
    let package_json_paths: BTreeSet<&str> = projection
        .files
        .iter()
        .map(|f| f.relative_path.as_str())
        .filter(|path| file_name(path) == "package.json")
        .collect();

    let root_manifests: Vec<(String, String)> = graph
        .nodes
        .values()
        .filter(|node| node.manifest_kind == Some(ManifestKind::NodePackage))
        .filter_map(|node| {
            node.source
                .as_ref()
                .map(|source| (node.id.clone(), source.path.clone()))
        })
        .collect();

    let mut new_edges = Vec::new();
    for (root_manifest_id, root_relative_path) in root_manifests {
        let root_dir = parent_dir(&root_relative_path);
        let glob_prefix = format!("manifest-fact:{root_relative_path}:workspace_glob:");
        let patterns: Vec<String> = graph
            .nodes
            .values()
            .filter(|node| node.id.starts_with(&glob_prefix))
            .map(|node| node.label.clone())
            .collect();
        if patterns.is_empty() {
            continue;
        }
        for member_path in &package_json_paths {
            if *member_path == root_relative_path {
                continue;
            }
            let member_dir = parent_dir(member_path);
            let matches = patterns
                .iter()
                .any(|pattern| npm_glob_matches(root_dir, pattern, member_dir));
            if !matches {
                continue;
            }
            let member_manifest_id = format!("manifest:{member_path}");
            if !graph.nodes.contains_key(&member_manifest_id) {
                continue;
            }
            new_edges.push(GraphEdge {
                from: member_manifest_id,
                to: root_manifest_id.clone(),
                kind: GraphEdgeKind::BelongsToWorkspace,
                layer: GraphLayer::Package,
                derivation: DerivationClass::Manifest,
                source: source_ref(&root_relative_path),
                location: None,
                relation_role: GraphRelationRole::new(roles::WORKSPACE_MEMBER),
            });
        }
    }
    for edge in new_edges {
        graph.edge(edge);
    }
}

fn npm_glob_matches(root_dir: &str, pattern: &str, candidate_dir: &str) -> bool {
    let full_pattern = if root_dir.is_empty() {
        pattern.to_owned()
    } else {
        format!("{root_dir}/{pattern}")
    };
    if let Some(prefix) = full_pattern.strip_suffix("/*") {
        parent_dir(candidate_dir) == prefix
    } else {
        candidate_dir == full_pattern
    }
}

/// Tag a physical `File` node as a generated surface using the same
/// `SkipReason::GeneratedContent` signal ROG-022's content-skip policy
/// already computed for this file -- never re-reading file content here.
/// "This path is generated" (a `node_role`, applying to the File node
/// itself) is a distinct fact from "this generated content is skipped
/// for semantic parsing" (the coverage entry's `SkipReason`); this pass
/// composes with that policy rather than replacing it.
fn tag_generated_boundaries(graph: &mut RepositoryGraph, per_file: &[FileCoverageEntry]) {
    for entry in per_file {
        if !matches!(
            entry.coverage,
            FileCoverage::Skipped {
                reason: SkipReason::GeneratedContent
            }
        ) {
            continue;
        }
        let id = file_node_id(&entry.relative_path);
        if let Some(node) = graph.nodes.get_mut(&id) {
            node.node_role = GraphNodeRole::new(roles::GENERATED_SURFACE);
        }
    }
}

/// A small, explicit, source-backed table of known generator contracts
/// (real files verified to exist in this repository, not a filename
/// heuristic). Emits a `DependsOn`+`layer=Build` edge, role=generated_by,
/// only when *both* the generated file and its generator source are
/// present in the graph as `File` nodes -- a synthetic fixture missing
/// either side legitimately emits nothing.
fn emit_known_generator_contracts(graph: &mut RepositoryGraph) {
    let mut new_edges = Vec::new();
    for (generated_path, generator_path) in KNOWN_GENERATOR_CONTRACTS {
        let generated_id = file_node_id(generated_path);
        let generator_id = file_node_id(generator_path);
        if !graph.nodes.contains_key(&generated_id) || !graph.nodes.contains_key(&generator_id) {
            continue;
        }
        new_edges.push(GraphEdge {
            from: generated_id,
            to: generator_id,
            kind: GraphEdgeKind::DependsOn,
            layer: GraphLayer::Build,
            derivation: DerivationClass::Manifest,
            source: source_ref(generated_path),
            location: None,
            relation_role: GraphRelationRole::new(roles::GENERATED_BY),
        });
    }
    for edge in new_edges {
        graph.edge(edge);
    }
}

/// Rust's *implicit* default binary: a `Cargo.toml` with no `[[bin]]`
/// array-of-tables (nothing tagged `bin_target` by the TOML adapter) but
/// a sibling `src/main.rs` present in the projection is, by Cargo's own
/// convention, a genuine runtime entry point -- this cannot be detected
/// by the per-file TOML adapter, which never sees whether `src/main.rs`
/// exists.
fn tag_rust_implicit_binaries(graph: &mut RepositoryGraph, projection: &SourceProjection) {
    let projected_paths: BTreeSet<&str> = projection
        .files
        .iter()
        .map(|f| f.relative_path.as_str())
        .collect();

    let cargo_manifests: Vec<(String, String)> = graph
        .nodes
        .values()
        .filter(|node| node.manifest_kind == Some(ManifestKind::CargoPackage))
        .filter_map(|node| {
            node.source
                .as_ref()
                .map(|source| (node.id.clone(), source.path.clone()))
        })
        .collect();

    let mut role_updates = Vec::new();
    let mut new_edges = Vec::new();
    for (manifest_id, cargo_toml_path) in cargo_manifests {
        let bin_prefix = format!("manifest-fact:{cargo_toml_path}:bin_target:");
        let has_explicit_bin = graph.nodes.keys().any(|id| id.starts_with(&bin_prefix));
        if has_explicit_bin {
            continue;
        }
        let dir = parent_dir(&cargo_toml_path);
        let main_rs = if dir.is_empty() {
            "src/main.rs".to_owned()
        } else {
            format!("{dir}/src/main.rs")
        };
        if !projected_paths.contains(main_rs.as_str()) {
            continue;
        }
        let main_id = file_node_id(&main_rs);
        if !graph.nodes.contains_key(&main_id) {
            continue;
        }
        role_updates.push(main_id.clone());
        new_edges.push(GraphEdge {
            from: manifest_id,
            to: main_id,
            kind: GraphEdgeKind::Contains,
            layer: GraphLayer::Runtime,
            derivation: DerivationClass::Manifest,
            source: source_ref(&cargo_toml_path),
            location: None,
            relation_role: GraphRelationRole::new(roles::ENTRY_POINT_FOR),
        });
    }
    for id in role_updates {
        if let Some(node) = graph.nodes.get_mut(&id) {
            node.node_role = GraphNodeRole::new(roles::RUNTIME_ENTRYPOINT);
        }
    }
    for edge in new_edges {
        graph.edge(edge);
    }
}

/// Cargo's own sibling-`tests/` directory convention: any `.rs` file
/// directly inside a crate's `tests/` directory is a real integration
/// test target (Cargo compiles each such file as its own test binary),
/// not a filename-similarity guess. Requires the crate's directory
/// listing, which only the orchestrator has.
fn tag_cargo_integration_test_directories(
    graph: &mut RepositoryGraph,
    projection: &SourceProjection,
) {
    let cargo_manifest_dirs: Vec<String> = graph
        .nodes
        .values()
        .filter(|node| node.manifest_kind == Some(ManifestKind::CargoPackage))
        .filter_map(|node| node.source.as_ref())
        .map(|source| parent_dir(&source.path).to_owned())
        .collect();

    let mut role_updates = Vec::new();
    let mut new_edges = Vec::new();
    for crate_dir in cargo_manifest_dirs {
        let tests_prefix = if crate_dir.is_empty() {
            "tests/".to_owned()
        } else {
            format!("{crate_dir}/tests/")
        };
        let cargo_toml_path = if crate_dir.is_empty() {
            "Cargo.toml".to_owned()
        } else {
            format!("{crate_dir}/Cargo.toml")
        };
        let manifest_id = format!("manifest:{cargo_toml_path}");
        if !graph.nodes.contains_key(&manifest_id) {
            continue;
        }
        for file in &projection.files {
            let relative_path = &file.relative_path;
            let Some(rest) = relative_path.strip_prefix(&tests_prefix) else {
                continue;
            };
            // Only the immediate `tests/*.rs` level -- a nested
            // `tests/support/mod.rs` helper module is not itself a
            // compiled test-target binary under Cargo's convention.
            if rest.contains('/') || !rest.ends_with(".rs") {
                continue;
            }
            let file_id = file_node_id(relative_path);
            if !graph.nodes.contains_key(&file_id) {
                continue;
            }
            role_updates.push(file_id.clone());
            new_edges.push(GraphEdge {
                from: manifest_id.clone(),
                to: file_id,
                kind: GraphEdgeKind::DependsOn,
                layer: GraphLayer::Test,
                derivation: DerivationClass::Manifest,
                source: source_ref(&cargo_toml_path),
                location: None,
                relation_role: GraphRelationRole::new(roles::TESTS),
            });
        }
    }
    for id in role_updates {
        if let Some(node) = graph.nodes.get_mut(&id) {
            node.node_role = GraphNodeRole::new(roles::TEST_TARGET);
        }
    }
    for edge in new_edges {
        graph.edge(edge);
    }
}

/// Frontend (`*.test.ts`/`*.test.tsx`/`*.spec.ts`) and Python
/// (`test_*.py`/`*_test.py`) test-file naming conventions, tagged
/// directly on the physical `File` node. This is an explicitly disclosed
/// naming-convention heuristic -- it never claims a specific function
/// inside the file "covers" anything, only that the file itself is a
/// test target by the ecosystem's own naming convention.
fn tag_naming_convention_test_files(graph: &mut RepositoryGraph, projection: &SourceProjection) {
    for file in &projection.files {
        let name = file_name(&file.relative_path);
        let is_test_file = name.ends_with(".test.ts")
            || name.ends_with(".test.tsx")
            || name.ends_with(".spec.ts")
            || name.ends_with(".spec.tsx")
            || (name.starts_with("test_") && name.ends_with(".py"))
            || (name.ends_with("_test.py"));
        if !is_test_file {
            continue;
        }
        let id = file_node_id(&file.relative_path);
        if let Some(node) = graph.nodes.get_mut(&id) {
            node.node_role = GraphNodeRole::new(roles::TEST_TARGET);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use repopact_repository::Repository;

    use crate::RepositoryGraph;

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("repopact-operational-{name}-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn build(root: &std::path::Path) -> RepositoryGraph {
        let repository = Repository::open(root);
        let snapshot = repository.session().snapshot();
        RepositoryGraph::build(&snapshot)
    }

    #[test]
    fn npm_workspace_members_matching_a_wildcard_glob_are_linked_to_the_root() {
        let root = temp_root("npm-wildcard");
        std::fs::write(
            root.join("package.json"),
            r#"{"name":"root","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.join("packages/a")).unwrap();
        std::fs::write(root.join("packages/a/package.json"), r#"{"name":"a"}"#).unwrap();
        std::fs::create_dir_all(root.join("packages/b")).unwrap();
        std::fs::write(root.join("packages/b/package.json"), r#"{"name":"b"}"#).unwrap();

        let graph = build(&root);
        let member_a_edge = graph.edges.iter().find(|e| {
            e.from == "manifest:packages/a/package.json"
                && e.kind == crate::GraphEdgeKind::BelongsToWorkspace
        });
        assert!(
            member_a_edge.is_some(),
            "expected package a to be linked to root workspace"
        );
        let edge = member_a_edge.unwrap();
        assert_eq!(edge.to, "manifest:package.json");
        assert_eq!(edge.layer, crate::GraphLayer::Package);
        assert_eq!(
            edge.relation_role
                .as_ref()
                .map(crate::GraphRelationRole::as_str),
            Some(crate::roles::WORKSPACE_MEMBER)
        );
        assert!(graph.edges.iter().any(|e| {
            e.from == "manifest:packages/b/package.json"
                && e.kind == crate::GraphEdgeKind::BelongsToWorkspace
        }));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn npm_workspace_glob_never_matches_an_unrelated_directory() {
        let root = temp_root("npm-unrelated");
        std::fs::write(
            root.join("package.json"),
            r#"{"name":"root","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.join("unrelated")).unwrap();
        std::fs::write(
            root.join("unrelated/package.json"),
            r#"{"name":"unrelated"}"#,
        )
        .unwrap();

        let graph = build(&root);
        assert!(!graph.edges.iter().any(|e| {
            e.from == "manifest:unrelated/package.json"
                && e.kind == crate::GraphEdgeKind::BelongsToWorkspace
        }));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_file_with_a_generated_marker_is_tagged_generated_surface() {
        let root = temp_root("generated-boundary");
        std::fs::write(
            root.join("gen.rs"),
            "// Generated by repopact-desktop-api. Do not edit by hand.\npub fn f() {}\n",
        )
        .unwrap();
        let graph = build(&root);
        let node = graph.nodes.get("file:gen.rs").unwrap();
        assert_eq!(
            node.node_role.as_ref().map(crate::GraphNodeRole::as_str),
            Some(crate::roles::GENERATED_SURFACE)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_ordinary_file_carries_no_generated_role() {
        let root = temp_root("not-generated");
        std::fs::write(root.join("plain.rs"), "pub fn f() {}\n").unwrap();
        let graph = build(&root);
        let node = graph.nodes.get("file:plain.rs").unwrap();
        assert_eq!(node.node_role, None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cargo_crate_without_explicit_bin_but_with_sibling_main_rs_is_tagged_runtime_entrypoint() {
        let root = temp_root("implicit-binary");
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"toolcli\"\n").unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();

        let graph = build(&root);
        let node = graph.nodes.get("file:src/main.rs").unwrap();
        assert_eq!(
            node.node_role.as_ref().map(crate::GraphNodeRole::as_str),
            Some(crate::roles::RUNTIME_ENTRYPOINT)
        );
        assert!(graph.edges.iter().any(|e| {
            e.from == "manifest:Cargo.toml"
                && e.to == "file:src/main.rs"
                && e.layer == crate::GraphLayer::Runtime
                && e.relation_role
                    .as_ref()
                    .map(crate::GraphRelationRole::as_str)
                    == Some(crate::roles::ENTRY_POINT_FOR)
        }));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cargo_crate_with_explicit_bin_target_is_not_double_tagged_as_implicit() {
        let root = temp_root("explicit-binary");
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"toolcli\"\n\n[[bin]]\nname = \"toolcli\"\npath = \"src/main.rs\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();

        let graph = build(&root);
        // The implicit-binary pass must not fire a second, redundant
        // Contains/Runtime edge on top of the TOML adapter's own
        // explicit `[[bin]]` fact.
        let runtime_edges: Vec<_> = graph
            .edges
            .iter()
            .filter(|e| e.to == "file:src/main.rs" && e.layer == crate::GraphLayer::Runtime)
            .collect();
        assert!(runtime_edges.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cargo_sibling_tests_directory_rs_files_are_tagged_test_target() {
        let root = temp_root("integration-tests");
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"mycrate\"\n").unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn f() {}\n").unwrap();
        std::fs::create_dir_all(root.join("tests")).unwrap();
        std::fs::write(
            root.join("tests/integration.rs"),
            "#[test]\nfn it_works() {}\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("tests/support")).unwrap();
        std::fs::write(root.join("tests/support/mod.rs"), "pub fn helper() {}\n").unwrap();

        let graph = build(&root);
        let target = graph.nodes.get("file:tests/integration.rs").unwrap();
        assert_eq!(
            target.node_role.as_ref().map(crate::GraphNodeRole::as_str),
            Some(crate::roles::TEST_TARGET)
        );
        assert!(graph.edges.iter().any(|e| {
            e.from == "manifest:Cargo.toml"
                && e.to == "file:tests/integration.rs"
                && e.layer == crate::GraphLayer::Test
                && e.relation_role
                    .as_ref()
                    .map(crate::GraphRelationRole::as_str)
                    == Some(crate::roles::TESTS)
        }));
        // A nested support module under `tests/` is not itself a
        // compiled test-target binary under Cargo's own convention.
        let support = graph.nodes.get("file:tests/support/mod.rs").unwrap();
        assert_eq!(support.node_role, None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn frontend_and_python_test_naming_conventions_are_tagged_test_target() {
        let root = temp_root("naming-convention");
        std::fs::write(root.join("index.test.ts"), "test('x', () => {});\n").unwrap();
        std::fs::write(root.join("app.spec.tsx"), "test('y', () => {});\n").unwrap();
        std::fs::write(root.join("test_addition.py"), "def test_add():\n    pass\n").unwrap();
        std::fs::write(root.join("addition_test.py"), "def test_add():\n    pass\n").unwrap();
        std::fs::write(root.join("index.ts"), "export const x = 1;\n").unwrap();

        let graph = build(&root);
        for path in [
            "file:index.test.ts",
            "file:app.spec.tsx",
            "file:test_addition.py",
            "file:addition_test.py",
        ] {
            let node = graph
                .nodes
                .get(path)
                .unwrap_or_else(|| panic!("missing {path}"));
            assert_eq!(
                node.node_role.as_ref().map(crate::GraphNodeRole::as_str),
                Some(crate::roles::TEST_TARGET),
                "expected {path} to be tagged test_target"
            );
        }
        let plain = graph.nodes.get("file:index.ts").unwrap();
        assert_eq!(plain.node_role, None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn known_generator_contract_emits_no_edge_when_only_one_side_exists() {
        // A realistic synthetic fixture missing the generator source: the
        // documented generator-contract table must never fabricate an
        // edge when only the generated side is present.
        let root = temp_root("generator-one-sided");
        std::fs::create_dir_all(root.join("audits/reports")).unwrap();
        std::fs::write(
            root.join("audits/reports/dashboard.md"),
            "# Repository Dashboard\n\n> Canonically generated from source records. Do not edit manually.\n",
        )
        .unwrap();

        let graph = build(&root);
        assert!(!graph.edges.iter().any(|e| {
            e.from == "file:audits/reports/dashboard.md"
                && e.relation_role
                    .as_ref()
                    .map(crate::GraphRelationRole::as_str)
                    == Some(crate::roles::GENERATED_BY)
        }));
        std::fs::remove_dir_all(root).unwrap();
    }
}
