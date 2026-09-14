//! Physical repository topology (WI063 ROG-003/004), built from the
//! Decision 0044 source projection. No second filesystem crawl: directory
//! structure is derived purely from the already-collected file paths, and
//! the only additional filesystem touch is a bounded `.git` existence
//! check per implied directory for nested-repository classification.

use std::collections::{BTreeMap, BTreeSet};

use repopact_repository::Repository;
use repopact_types::{RecordKind, RecordRef};

use crate::projection::SourceProjection;
use crate::{
    DerivationClass, GraphEdge, GraphEdgeKind, GraphLayer, GraphNode, GraphNodeKind,
    RepositoryGraph,
};

const WORKSPACE_MANIFESTS: [&str; 3] = ["Cargo.toml", "pyproject.toml", "package.json"];

pub fn directory_node(relative_path: &str) -> String {
    if relative_path.is_empty() {
        "repository".to_owned()
    } else {
        format!("dir:{relative_path}")
    }
}

pub fn file_node(relative_path: &str) -> String {
    format!("file:{relative_path}")
}

/// Populate `graph` with the physical layer from an already-computed
/// `projection` (never rebuilt here -- rebuilding would recompute
/// `RepositoryTopology` and add unbounded Git invocations, see
/// `projection::SourceProjection::build`'s doc comment). `frozen_globs` is
/// `(frozen_surface_node_id, glob_pattern)` -- already collected by the
/// governance pass so this function performs no extra I/O to find them.
/// The only I/O this function performs itself is a bounded `.git`
/// existence check per implied directory, for nested-repository
/// classification.
pub fn extend(
    graph: &mut RepositoryGraph,
    repository: &Repository,
    projection: &SourceProjection,
    frozen_globs: &[(String, String)],
) {
    // Directories are derived from file paths' ancestor components, not a
    // second walk: every proper prefix of a file's path is an implied
    // directory.
    let mut directories: BTreeSet<String> = BTreeSet::new();
    for file in &projection.files {
        for ancestor in ancestors(&file.relative_path) {
            directories.insert(ancestor);
        }
    }

    // A directory is a Workspace if it directly contains a recognized
    // manifest file (Decision 0044: manifest presence is a deterministic
    // fact, not a heuristic guess). A directory is a NestedRepository if it
    // directly contains its own `.git` -- checked once per implied
    // directory, never per file, and never descended into.
    let files_by_directory = group_by_directory(&projection.files);
    let mut workspace_dirs: BTreeSet<String> = BTreeSet::new();
    let mut configuration_files: BTreeSet<String> = BTreeSet::new();
    for (directory, files) in &files_by_directory {
        for file in files {
            let name = basename(file);
            if WORKSPACE_MANIFESTS.contains(&name.as_str()) {
                workspace_dirs.insert(directory.clone());
                configuration_files.insert(file.clone());
            }
        }
    }
    let mut nested_repositories: BTreeSet<String> = BTreeSet::new();
    for directory in &directories {
        if directory.is_empty() {
            continue;
        }
        if repository.root().join(directory).join(".git").exists() {
            nested_repositories.insert(directory.clone());
        }
    }

    // Nodes: repository root already exists as GraphNodeKind::Repository
    // from the governance pass; every other directory gets a physical
    // Directory/Workspace/NestedRepository node.
    for directory in &directories {
        if directory.is_empty() {
            continue;
        }
        let id = directory_node(directory);
        let kind = if nested_repositories.contains(directory) {
            GraphNodeKind::NestedRepository
        } else if workspace_dirs.contains(directory) {
            GraphNodeKind::Workspace
        } else {
            GraphNodeKind::Directory
        };
        graph.node(GraphNode {
            id: id.clone(),
            kind,
            label: basename(directory),
            symbol_kind: None,
            manifest_kind: None,
            node_role: None,
            location: None,
            layer: GraphLayer::Physical,
            source: Some(RecordRef::new(
                RecordKind::Directory,
                directory.clone(),
                directory.clone(),
            )),
        });
        let parent = directory_node(&parent_of(directory));
        graph.edge(GraphEdge {
            from: parent,
            to: id,
            kind: GraphEdgeKind::Contains,
            layer: GraphLayer::Physical,
            derivation: DerivationClass::Filesystem,
            location: None,
            relation_role: None,
            source: RecordRef::new(RecordKind::Directory, directory.clone(), directory.clone()),
        });
    }

    for file in &projection.files {
        let id = file_node(&file.relative_path);
        let kind = if configuration_files.contains(&file.relative_path) {
            GraphNodeKind::ConfigurationFile
        } else {
            GraphNodeKind::File
        };
        graph.node(GraphNode {
            id: id.clone(),
            kind,
            label: basename(&file.relative_path),
            symbol_kind: None,
            manifest_kind: None,
            node_role: None,
            location: None,
            layer: GraphLayer::Physical,
            source: Some(RecordRef::new(
                RecordKind::File,
                file.relative_path.clone(),
                file.relative_path.clone(),
            )),
        });
        let parent = directory_node(&parent_of(&file.relative_path));
        graph.edge(GraphEdge {
            from: parent.clone(),
            to: id.clone(),
            kind: GraphEdgeKind::Contains,
            layer: GraphLayer::Physical,
            derivation: DerivationClass::Filesystem,
            location: None,
            relation_role: None,
            source: RecordRef::new(
                RecordKind::File,
                file.relative_path.clone(),
                file.relative_path.clone(),
            ),
        });

        if let Some(workspace) = nearest_workspace_ancestor(&file.relative_path, &workspace_dirs) {
            graph.edge(GraphEdge {
                from: id.clone(),
                to: directory_node(&workspace),
                kind: GraphEdgeKind::BelongsToWorkspace,
                layer: GraphLayer::Physical,
                derivation: DerivationClass::Manifest,
                location: None,
                relation_role: None,
                source: RecordRef::new(RecordKind::Directory, workspace.clone(), workspace.clone()),
            });
        }

        if configuration_files.contains(&file.relative_path) {
            let directory = parent_of(&file.relative_path);
            graph.edge(GraphEdge {
                from: directory_node(&directory),
                to: id.clone(),
                kind: GraphEdgeKind::ConfiguredBy,
                layer: GraphLayer::Physical,
                derivation: DerivationClass::Manifest,
                location: None,
                relation_role: None,
                source: RecordRef::new(
                    RecordKind::File,
                    file.relative_path.clone(),
                    file.relative_path.clone(),
                ),
            });
        }

        for (frozen_node, glob) in frozen_globs {
            if matches_frozen_glob(glob, &file.relative_path) {
                graph.edge(GraphEdge {
                    from: id.clone(),
                    to: frozen_node.clone(),
                    kind: GraphEdgeKind::Intersects,
                    layer: GraphLayer::Physical,
                    derivation: DerivationClass::Filesystem,
                    location: None,
                    relation_role: None,
                    source: RecordRef::new(
                        RecordKind::File,
                        file.relative_path.clone(),
                        file.relative_path.clone(),
                    ),
                });
            }
        }
    }
}

/// Every proper ancestor directory of `relative_path`, root-most first,
/// including the empty string for the repository root's own directory
/// slot (already represented by the `"repository"` node, so callers must
/// skip the empty string when emitting a node).
fn ancestors(relative_path: &str) -> Vec<String> {
    let mut result = vec![String::new()];
    let mut current = String::new();
    let parts: Vec<&str> = relative_path.split('/').collect();
    for part in &parts[..parts.len().saturating_sub(1)] {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(part);
        result.push(current.clone());
    }
    result
}

fn parent_of(relative_path: &str) -> String {
    match relative_path.rfind('/') {
        Some(index) => relative_path[..index].to_owned(),
        None => String::new(),
    }
}

fn basename(relative_path: &str) -> String {
    relative_path
        .rsplit('/')
        .next()
        .unwrap_or(relative_path)
        .to_owned()
}

fn group_by_directory(files: &[crate::projection::ProjectedFile]) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files {
        map.entry(parent_of(&file.relative_path))
            .or_default()
            .push(file.relative_path.clone());
    }
    map
}

fn nearest_workspace_ancestor(
    relative_path: &str,
    workspace_dirs: &BTreeSet<String>,
) -> Option<String> {
    let mut current = parent_of(relative_path);
    loop {
        if workspace_dirs.contains(&current) {
            return Some(current);
        }
        if current.is_empty() {
            return None;
        }
        current = parent_of(&current);
    }
}

/// Minimal glob matcher covering exactly the patterns RepoPact's own
/// frozen-surface records use today: exact repository-relative paths and
/// `<prefix>/**`. Mirrors `repopact/admission.py`'s
/// `fnmatch.fnmatch(path, pattern) or (pattern.endswith("/**") and
/// path.startswith(pattern[:-3]))` for those two shapes; does not attempt
/// full `fnmatch` character-class/`?` semantics, which no current frozen
/// surface pattern uses.
fn matches_frozen_glob(pattern: &str, path: &str) -> bool {
    if pattern == path {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ancestors_are_root_most_first() {
        assert_eq!(
            ancestors("a/b/c.txt"),
            vec!["".to_owned(), "a".to_owned(), "a/b".to_owned()]
        );
    }

    #[test]
    fn frozen_glob_prefix_matches() {
        assert!(matches_frozen_glob(
            ".github/workflows/**",
            ".github/workflows/ci.yml"
        ));
        assert!(!matches_frozen_glob(
            ".github/workflows/**",
            ".github/other/ci.yml"
        ));
        assert!(matches_frozen_glob(
            "governance/charter.md",
            "governance/charter.md"
        ));
        assert!(!matches_frozen_glob(
            "governance/charter.md",
            "governance/other.md"
        ));
    }
}
