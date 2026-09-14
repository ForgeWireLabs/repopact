//! JSON metadata adapter (WI063 ROG-019, Decision 0048): `package.json`
//! and `tsconfig*.json`. Only these recognized manifest shapes are given
//! real structural extraction -- arbitrary JSON is never flattened into
//! graph semantics (Decision 0048), so an unrecognized `.json` file is
//! explicitly skipped, not silently ignored and not fabricated into
//! facts it does not actually contain.

use repopact_types::{RecordKind, RecordRef};
use serde_json::Value;

use super::{manifest_fact_node_id, manifest_node_id, MetadataAdapter};
use crate::semantic::{AdapterOutput, FileCoverage, SkipReason, SourceInput};
use crate::{
    DerivationClass, GraphEdge, GraphEdgeKind, GraphLayer, GraphNode, GraphNodeKind, ManifestKind,
};

pub const ADAPTER_VERSION: &str = "json-metadata-adapter-0.1.0";

pub struct JsonAdapter;

fn file_name(relative_path: &str) -> &str {
    relative_path.rsplit('/').next().unwrap_or(relative_path)
}

fn is_tsconfig(name: &str) -> bool {
    name.starts_with("tsconfig") && name.ends_with(".json")
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
        node_role: None,
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
        node_role: None,
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
        relation_role: None,
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
        relation_role: None,
    }
}

fn configured_by_edge(from: String, to: String, relative_path: &str) -> GraphEdge {
    GraphEdge {
        from,
        to,
        kind: GraphEdgeKind::ConfiguredBy,
        layer: GraphLayer::Package,
        derivation: DerivationClass::Manifest,
        source: source(relative_path),
        location: None,
        relation_role: None,
    }
}

fn extract_package_json(relative_path: &str, value: &Value) -> AdapterOutput {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let name = value.get("name").and_then(Value::as_str);
    let label = name
        .map(|name| format!("npm:{name}"))
        .unwrap_or_else(|| format!("npm-package:{relative_path}"));
    let manifest_id = manifest_node_id(relative_path);
    nodes.push(manifest_document_node(
        relative_path,
        label,
        ManifestKind::NodePackage,
    ));

    for section in ["dependencies", "devDependencies", "peerDependencies"] {
        let Some(deps) = value.get(section).and_then(Value::as_object) else {
            continue;
        };
        for name in deps.keys() {
            let fact = fact_node(relative_path, "dependency", name, name.clone());
            edges.push(depends_on_edge(
                manifest_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
        }
    }

    if let Some(scripts) = value.get("scripts").and_then(Value::as_object) {
        for (name, command) in scripts {
            let command_text = command.as_str().unwrap_or_default();
            let fact = fact_node(
                relative_path,
                "script",
                name,
                format!("{name} = {command_text}"),
            );
            edges.push(contains_edge(
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

fn extract_tsconfig_json(relative_path: &str, value: &Value) -> AdapterOutput {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let manifest_id = manifest_node_id(relative_path);
    nodes.push(manifest_document_node(
        relative_path,
        format!("tsconfig:{relative_path}"),
        ManifestKind::TypeScriptConfig,
    ));

    if let Some(extends) = value.get("extends").and_then(Value::as_str) {
        let fact = fact_node(relative_path, "extends", extends, extends.to_owned());
        edges.push(configured_by_edge(
            manifest_id.clone(),
            fact.id.clone(),
            relative_path,
        ));
        nodes.push(fact);
    }

    if let Some(references) = value.get("references").and_then(Value::as_array) {
        for reference in references {
            let Some(path) = reference.get("path").and_then(Value::as_str) else {
                continue;
            };
            let fact = fact_node(relative_path, "project_reference", path, path.to_owned());
            edges.push(configured_by_edge(
                manifest_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
        }
    }

    if let Some(options) = value.get("compilerOptions").and_then(Value::as_object) {
        for key in ["outDir", "rootDir", "baseUrl"] {
            let Some(entry) = options.get(key).and_then(Value::as_str) else {
                continue;
            };
            let fact = fact_node(
                relative_path,
                "compiler_option",
                key,
                format!("{key} = {entry}"),
            );
            edges.push(contains_edge(
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

impl MetadataAdapter for JsonAdapter {
    fn identity(&self) -> &'static str {
        ADAPTER_VERSION
    }

    fn accepts(&self, relative_path: &str) -> bool {
        let name = file_name(relative_path);
        name == "package.json" || is_tsconfig(name)
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
        let value: Value = match serde_json::from_slice(input.content) {
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
        let name = file_name(input.relative_path);
        if name == "package.json" {
            extract_package_json(input.relative_path, &value)
        } else {
            extract_tsconfig_json(input.relative_path, &value)
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
        JsonAdapter.extract(&input)
    }

    #[test]
    fn package_json_yields_identity_dependency_and_script_facts() {
        let output = extract(
            "package.json",
            r#"{"name":"repopact-desktop","dependencies":{"react":"19.3.0"},"scripts":{"build":"vite build"}}"#,
        );
        assert_eq!(output.coverage, FileCoverage::Complete);
        assert!(output
            .nodes
            .iter()
            .any(|n| n.manifest_kind == Some(ManifestKind::NodePackage)
                && n.label.contains("repopact-desktop")));
        assert!(output.nodes.iter().any(|n| n.label == "react"));
        assert!(output.nodes.iter().any(|n| n.label.contains("vite build")));
    }

    #[test]
    fn tsconfig_json_yields_extends_and_reference_facts() {
        let output = extract(
            "tsconfig.json",
            r#"{"extends":"./tsconfig.base.json","references":[{"path":"./packages/lib"}],"compilerOptions":{"outDir":"dist"}}"#,
        );
        assert_eq!(output.coverage, FileCoverage::Complete);
        assert!(output
            .nodes
            .iter()
            .any(|n| n.manifest_kind == Some(ManifestKind::TypeScriptConfig)));
        assert!(output
            .nodes
            .iter()
            .any(|n| n.label.contains("tsconfig.base.json")));
        assert!(output
            .nodes
            .iter()
            .any(|n| n.label.contains("packages/lib")));
        assert!(output.nodes.iter().any(|n| n.label.contains("dist")));
    }

    #[test]
    fn malformed_json_yields_partial_not_a_crash() {
        let output = extract("package.json", "{ not valid json");
        assert!(matches!(output.coverage, FileCoverage::Partial { .. }));
    }

    #[test]
    fn an_unrecognized_json_file_is_skipped_not_fabricated() {
        let output = extract("some/data.json", r#"{"arbitrary":"content"}"#);
        assert!(matches!(output.coverage, FileCoverage::Skipped { .. }));
        assert!(output.nodes.is_empty());
    }
}
