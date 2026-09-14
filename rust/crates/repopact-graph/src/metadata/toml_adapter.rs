//! TOML metadata adapter (WI063 ROG-019, Decision 0048): `Cargo.toml` and
//! `pyproject.toml`. Facts only -- dependency/workspace-member references
//! are recorded as local fact nodes (raw name/path text), never as
//! resolved edges into another file's own manifest node, since resolving
//! which other in-repo file a dependency name refers to (or whether it
//! is even in this repository at all, e.g. a crates.io/PyPI dependency)
//! would require cross-file lookup this per-file adapter deliberately
//! does not perform -- consistent with Decision 0045's "import fact, not
//! resolved target" precedent for source-language imports.

use repopact_types::{RecordKind, RecordRef};

use super::{manifest_fact_node_id, manifest_node_id, MetadataAdapter};
use crate::semantic::{AdapterOutput, FileCoverage, SkipReason, SourceInput};
use crate::{
    DerivationClass, GraphEdge, GraphEdgeKind, GraphLayer, GraphNode, GraphNodeKind, ManifestKind,
};

pub const ADAPTER_VERSION: &str = "toml-metadata-adapter-0.1.0";

pub struct TomlAdapter;

fn file_name(relative_path: &str) -> &str {
    relative_path.rsplit('/').next().unwrap_or(relative_path)
}

fn source(relative_path: &str) -> RecordRef {
    RecordRef::new(
        RecordKind::File,
        relative_path.to_owned(),
        relative_path.to_owned(),
    )
}

fn manifest_document_node(relative_path: &str, label: String, kind: ManifestKind) -> GraphNode {
    GraphNode {
        id: manifest_node_id(relative_path),
        kind: GraphNodeKind::Manifest,
        label,
        layer: GraphLayer::Package,
        source: Some(source(relative_path)),
        symbol_kind: None,
        location: None,
        manifest_kind: Some(kind),
    }
}

fn fact_node(relative_path: &str, fact_kind: &str, name: &str, label: String) -> GraphNode {
    GraphNode {
        id: manifest_fact_node_id(relative_path, fact_kind, name),
        kind: GraphNodeKind::Manifest,
        label,
        layer: GraphLayer::Package,
        source: Some(source(relative_path)),
        symbol_kind: None,
        location: None,
        manifest_kind: None,
    }
}

fn depends_on_edge(from: String, to: String, relative_path: &str) -> GraphEdge {
    GraphEdge {
        from,
        to,
        kind: GraphEdgeKind::DependsOn,
        layer: GraphLayer::Package,
        derivation: DerivationClass::Manifest,
        source: source(relative_path),
        location: None,
    }
}

fn contains_edge(from: String, to: String, relative_path: &str) -> GraphEdge {
    GraphEdge {
        from,
        to,
        kind: GraphEdgeKind::Contains,
        layer: GraphLayer::Package,
        derivation: DerivationClass::Manifest,
        source: source(relative_path),
        location: None,
    }
}

/// Strip a Cargo/PEP-508-style version/extras specifier off a dependency
/// name, leaving just the bare package name. Deterministic string
/// splitting, not a real PEP 508 parser -- sufficient for the fact this
/// checkpoint claims (the declared dependency name), not a claim of full
/// specifier parsing.
fn bare_dependency_name(raw: &str) -> String {
    raw.trim()
        .split(|c: char| {
            c == '='
                || c == '>'
                || c == '<'
                || c == '!'
                || c == '~'
                || c == '['
                || c == ';'
                || c.is_whitespace()
        })
        .next()
        .unwrap_or(raw)
        .trim()
        .to_owned()
}

fn extract_cargo_toml(relative_path: &str, value: &toml::Value) -> AdapterOutput {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let package_name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str());
    let label = package_name
        .map(|name| format!("cargo:{name}"))
        .unwrap_or_else(|| format!("cargo-workspace:{relative_path}"));
    let manifest_id = manifest_node_id(relative_path);
    nodes.push(manifest_document_node(
        relative_path,
        label,
        ManifestKind::CargoPackage,
    ));

    if let Some(members) = value
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
    {
        for member in members.iter().filter_map(|m| m.as_str()) {
            let fact = fact_node(relative_path, "workspace_member", member, member.to_owned());
            edges.push(contains_edge(
                manifest_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
        }
    }

    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(table) = value.get(section).and_then(|t| t.as_table()) else {
            continue;
        };
        for name in table.keys() {
            let fact = fact_node(relative_path, "dependency", name, name.clone());
            edges.push(depends_on_edge(
                manifest_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
        }
    }

    if let Some(deps) = value
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table())
    {
        for name in deps.keys() {
            let fact = fact_node(relative_path, "workspace_dependency", name, name.clone());
            edges.push(depends_on_edge(
                manifest_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
        }
    }

    AdapterOutput {
        nodes,
        edges,
        coverage: FileCoverage::Complete,
    }
}

fn extract_pyproject_toml(relative_path: &str, value: &toml::Value) -> AdapterOutput {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let project_name = value
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str());
    let label = project_name
        .map(|name| format!("python-project:{name}"))
        .unwrap_or_else(|| format!("python-project:{relative_path}"));
    let manifest_id = manifest_node_id(relative_path);
    nodes.push(manifest_document_node(
        relative_path,
        label,
        ManifestKind::PythonProject,
    ));

    if let Some(deps) = value
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
    {
        for raw in deps.iter().filter_map(|d| d.as_str()) {
            let name = bare_dependency_name(raw);
            if name.is_empty() {
                continue;
            }
            let fact = fact_node(relative_path, "dependency", &name, raw.to_owned());
            edges.push(depends_on_edge(
                manifest_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
        }
    }

    if let Some(groups) = value
        .get("project")
        .and_then(|p| p.get("optional-dependencies"))
        .and_then(|g| g.as_table())
    {
        for (group, list) in groups {
            let Some(list) = list.as_array() else {
                continue;
            };
            for raw in list.iter().filter_map(|d| d.as_str()) {
                let name = bare_dependency_name(raw);
                if name.is_empty() {
                    continue;
                }
                let fact = fact_node(
                    relative_path,
                    "optional_dependency",
                    &format!("{group}:{name}"),
                    raw.to_owned(),
                );
                edges.push(depends_on_edge(
                    manifest_id.clone(),
                    fact.id.clone(),
                    relative_path,
                ));
                nodes.push(fact);
            }
        }
    }

    if let Some(scripts) = value
        .get("project")
        .and_then(|p| p.get("scripts"))
        .and_then(|s| s.as_table())
    {
        for (name, target) in scripts {
            let target_text = target.as_str().unwrap_or_default();
            let fact = fact_node(
                relative_path,
                "entrypoint",
                name,
                format!("{name} = {target_text}"),
            );
            edges.push(contains_edge(
                manifest_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
        }
    }

    if let Some(backend) = value
        .get("build-system")
        .and_then(|b| b.get("build-backend"))
        .and_then(|b| b.as_str())
    {
        let fact = fact_node(relative_path, "build_backend", backend, backend.to_owned());
        edges.push(contains_edge(
            manifest_id.clone(),
            fact.id.clone(),
            relative_path,
        ));
        nodes.push(fact);
    }

    AdapterOutput {
        nodes,
        edges,
        coverage: FileCoverage::Complete,
    }
}

impl MetadataAdapter for TomlAdapter {
    fn identity(&self) -> &'static str {
        ADAPTER_VERSION
    }

    fn accepts(&self, relative_path: &str) -> bool {
        matches!(file_name(relative_path), "Cargo.toml" | "pyproject.toml")
    }

    fn extract(&self, input: &SourceInput) -> AdapterOutput {
        if !self.accepts(input.relative_path) {
            return AdapterOutput {
                nodes: Vec::new(),
                edges: Vec::new(),
                coverage: FileCoverage::Skipped {
                    reason: SkipReason::UnrecognizedMetadataSchema,
                },
            };
        }
        let Ok(text) = std::str::from_utf8(input.content) else {
            return AdapterOutput {
                nodes: Vec::new(),
                edges: Vec::new(),
                coverage: FileCoverage::Failed {
                    reason: "not valid UTF-8".to_owned(),
                },
            };
        };
        let value: toml::Value = match text.parse() {
            Ok(value) => value,
            Err(error) => {
                return AdapterOutput {
                    nodes: Vec::new(),
                    edges: Vec::new(),
                    coverage: FileCoverage::Partial {
                        reason: error.to_string(),
                    },
                }
            }
        };
        match file_name(input.relative_path) {
            "Cargo.toml" => extract_cargo_toml(input.relative_path, &value),
            "pyproject.toml" => extract_pyproject_toml(input.relative_path, &value),
            _ => unreachable!("accepts() already filtered to these two file names"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::ResourcePolicy;

    fn extract(relative_path: &str, content: &str) -> AdapterOutput {
        let policy = ResourcePolicy::default();
        let input = SourceInput {
            relative_path,
            content: content.as_bytes(),
            resource_policy: &policy,
        };
        TomlAdapter.extract(&input)
    }

    #[test]
    fn cargo_toml_yields_package_workspace_and_dependency_facts() {
        let output = extract(
            "Cargo.toml",
            r#"
[workspace]
members = ["crates/a", "crates/b"]

[workspace.dependencies]
serde = "1.0"
"#,
        );
        assert_eq!(output.coverage, FileCoverage::Complete);
        assert!(output
            .nodes
            .iter()
            .any(|n| n.manifest_kind == Some(ManifestKind::CargoPackage)));
        assert!(output.nodes.iter().any(|n| n.label == "crates/a"));
        assert!(output.nodes.iter().any(|n| n.label == "serde"));
        assert!(output
            .edges
            .iter()
            .any(|e| e.kind == GraphEdgeKind::DependsOn));
    }

    #[test]
    fn pyproject_toml_yields_project_dependency_and_script_facts() {
        let output = extract(
            "pyproject.toml",
            r#"
[project]
name = "repopact"
dependencies = ["jsonschema>=4.20"]

[project.optional-dependencies]
dev = ["pytest>=8"]

[project.scripts]
repopact = "repopact.cli:main"

[build-system]
build-backend = "maturin"
"#,
        );
        assert_eq!(output.coverage, FileCoverage::Complete);
        assert!(output
            .nodes
            .iter()
            .any(|n| n.manifest_kind == Some(ManifestKind::PythonProject)
                && n.label.contains("repopact")));
        assert!(output.nodes.iter().any(|n| n.label.contains("jsonschema")));
        assert!(output.nodes.iter().any(|n| n.label.contains("pytest")));
        assert!(output
            .nodes
            .iter()
            .any(|n| n.label.contains("repopact.cli")));
    }

    #[test]
    fn malformed_toml_yields_partial_not_a_crash() {
        let output = extract("Cargo.toml", "this is not [ valid toml");
        assert!(matches!(output.coverage, FileCoverage::Partial { .. }));
    }

    #[test]
    fn an_unrecognized_toml_file_is_skipped_not_fabricated() {
        let output = extract("some/other.toml", "[table]\nkey = \"value\"\n");
        assert!(matches!(output.coverage, FileCoverage::Skipped { .. }));
        assert!(output.nodes.is_empty());
    }
}
