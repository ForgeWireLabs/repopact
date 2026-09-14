//! Deterministic Python semantic extraction (WI063 ROG-017). Grounded
//! against real tree-sitter-python 0.25.0 AST output (verified via a
//! throwaway spike): `class_definition` and `function_definition`
//! (`async def` included -- `async` is just a leading sibling token, the
//! `name` field still resolves to the function's identifier) both expose
//! a `name` field. `import_statement`/`import_from_statement` have no
//! single stable sub-field worth decomposing for this checkpoint, so
//! their raw text is used directly as the import fact, matching the Rust
//! adapter's approach.
//!
//! Does not claim runtime import resolution, monkey-patching awareness,
//! dynamic attribute resolution, or whole-program call semantics. A
//! function named `test_*` at module or class scope is classified as a
//! test symbol by the pytest naming convention -- an explicitly disclosed
//! heuristic, not a language-level primitive the way Rust's `#[test]`
//! attribute is.

use std::ops::ControlFlow;
use std::time::Instant;

use repopact_types::{RecordKind, RecordRef};
use tree_sitter::{Node, Parser};

use super::{
    location_from_span, node_id_for_symbol, AdapterOutput, FileCoverage, SemanticAdapter,
    SourceInput, SourceLanguage,
};
use crate::{
    DerivationClass, GraphEdge, GraphEdgeKind, GraphLayer, GraphNode, GraphNodeKind, SymbolKind,
};

pub const ADAPTER_VERSION: &str = "python-adapter-0.2.0";
pub const SUPPORTED_RELATIONS: [&str; 2] = ["defines", "imports"];
/// ROG-022: a maliciously or accidentally deeply nested AST must not
/// overflow this walker\'s own recursion stack. Exceeding this depth
/// truncates the walk for that subtree and marks the file Partial --
/// it never panics or aborts the whole build.
const MAX_WALK_DEPTH: usize = 512;

pub struct PythonAdapter;

impl SemanticAdapter for PythonAdapter {
    fn language(&self) -> SourceLanguage {
        SourceLanguage::Python
    }

    fn extract(&self, input: &SourceInput) -> AdapterOutput {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .is_err()
        {
            return AdapterOutput {
                nodes: Vec::new(),
                edges: Vec::new(),
                coverage: FileCoverage::Failed {
                    reason: "unable to load Python grammar".to_owned(),
                },
            };
        }

        let deadline = Instant::now() + input.resource_policy.max_parse_duration;
        let mut read = |offset: usize, _point: tree_sitter::Point| -> &[u8] {
            if offset < input.content.len() {
                &input.content[offset..]
            } else {
                &[]
            }
        };
        let options = tree_sitter::ParseOptions {
            progress_callback: Some(&mut |_state: &tree_sitter::ParseState| {
                if Instant::now() >= deadline {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            }),
        };
        let Some(tree) = parser.parse_with_options(&mut read, None, Some(options)) else {
            return AdapterOutput {
                nodes: Vec::new(),
                edges: Vec::new(),
                coverage: FileCoverage::Failed {
                    reason: "parse cancelled: exceeded the bounded parse deadline".to_owned(),
                },
            };
        };

        let file_node_id = format!("file:{}", input.relative_path);
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut has_error = false;
        walk(
            tree.root_node(),
            0,
            input.content,
            input.relative_path,
            "",
            &file_node_id,
            &mut nodes,
            &mut edges,
            &mut has_error,
        );

        let coverage = if has_error {
            FileCoverage::Partial { reason: "one or more declarations had a syntax error the grammar could not fully recover from".to_owned() }
        } else {
            FileCoverage::Complete
        };
        AdapterOutput {
            nodes,
            edges,
            coverage,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn walk(
    node: Node,
    depth: usize,
    source: &[u8],
    relative_path: &str,
    container: &str,
    file_node_id: &str,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
    has_error: &mut bool,
) {
    if depth > MAX_WALK_DEPTH {
        *has_error = true;
        return;
    }
    if node.is_error() {
        *has_error = true;
    }

    let mut next_container = container.to_owned();

    match node.kind() {
        "class_definition" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    relative_path,
                    container,
                    &name,
                    "class",
                    SymbolKind::Type,
                    file_node_id,
                    nodes,
                    edges,
                );
                next_container = qualify(container, &name);
            }
        }
        "function_definition" => {
            if let Some(name) = named_text(node, source) {
                let (tag, kind) = if name.starts_with("test_") || name == "test" {
                    ("test", SymbolKind::Test)
                } else if container.is_empty() {
                    ("function", SymbolKind::Function)
                } else {
                    ("method", SymbolKind::Method)
                };
                emit_symbol(
                    node,
                    relative_path,
                    container,
                    &name,
                    tag,
                    kind,
                    file_node_id,
                    nodes,
                    edges,
                );
                next_container = qualify(container, &name);
            }
        }
        "import_statement" | "import_from_statement" => {
            let import_text = text_of(node, source)
                .trim()
                .trim_end_matches('\n')
                .to_owned();
            if !import_text.is_empty() {
                let target_id = format!("import:{relative_path}:{import_text}");
                nodes.push(GraphNode {
                    id: target_id.clone(),
                    kind: GraphNodeKind::Symbol,
                    label: import_text.clone(),
                    layer: GraphLayer::Semantic,
                    source: Some(RecordRef::new(
                        RecordKind::File,
                        relative_path.to_owned(),
                        relative_path.to_owned(),
                    )),
                    symbol_kind: Some(SymbolKind::Module),
                    manifest_kind: None,
                    node_role: None,
                    location: None,
                });
                edges.push(GraphEdge {
                    from: file_node_id.to_owned(),
                    to: target_id,
                    kind: GraphEdgeKind::Imports,
                    layer: GraphLayer::Semantic,
                    derivation: DerivationClass::Parser,
                    source: RecordRef::new(
                        RecordKind::File,
                        relative_path.to_owned(),
                        relative_path.to_owned(),
                    ),
                    location: Some(span_of(node)),
                    relation_role: None,
                });
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(
            child,
            depth + 1,
            source,
            relative_path,
            &next_container,
            file_node_id,
            nodes,
            edges,
            has_error,
        );
    }
}

fn named_text(node: Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .map(|child| text_of(child, source))
}

fn text_of(node: Node, source: &[u8]) -> String {
    String::from_utf8_lossy(&source[node.start_byte()..node.end_byte()]).into_owned()
}

fn qualify(container: &str, name: &str) -> String {
    if container.is_empty() {
        name.to_owned()
    } else {
        format!("{container}.{name}")
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_symbol(
    node: Node,
    relative_path: &str,
    container: &str,
    name: &str,
    kind_tag: &str,
    symbol_kind: SymbolKind,
    file_node_id: &str,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
) {
    let id = node_id_for_symbol(
        SourceLanguage::Python,
        relative_path,
        container,
        kind_tag,
        name,
    );
    let location = span_of(node);
    nodes.push(GraphNode {
        id: id.clone(),
        kind: GraphNodeKind::Symbol,
        label: name.to_owned(),
        layer: GraphLayer::Semantic,
        source: Some(RecordRef::new(
            RecordKind::File,
            relative_path.to_owned(),
            relative_path.to_owned(),
        )),
        symbol_kind: Some(symbol_kind),
        manifest_kind: None,
        node_role: (symbol_kind == SymbolKind::Test)
            .then(|| crate::GraphNodeRole::new(crate::roles::TEST_TARGET))
            .flatten(),
        location: Some(location),
    });
    edges.push(GraphEdge {
        from: file_node_id.to_owned(),
        to: id,
        kind: GraphEdgeKind::Defines,
        layer: GraphLayer::Semantic,
        derivation: DerivationClass::Parser,
        source: RecordRef::new(
            RecordKind::File,
            relative_path.to_owned(),
            relative_path.to_owned(),
        ),
        location: Some(location),
        relation_role: None,
    });
}

fn span_of(node: Node) -> crate::GraphSourceLocation {
    let start = node.start_position();
    let end = node.end_position();
    location_from_span(
        node.start_byte(),
        node.end_byte(),
        start.row,
        start.column,
        end.row,
        end.column,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GraphNodeKind;

    fn extract(source: &str) -> AdapterOutput {
        let policy = super::super::ResourcePolicy::default();
        let input = SourceInput {
            relative_path: "pkg/mod.py",
            content: source.as_bytes(),
            resource_policy: &policy,
        };
        PythonAdapter.extract(&input)
    }

    #[test]
    fn covers_class_function_async_function_and_methods() {
        let output = extract("class Foo:\n    def bar(self):\n        pass\n\ndef top():\n    pass\n\nasync def atop():\n    pass\n");
        assert_eq!(output.coverage, FileCoverage::Complete);
        let by_label: std::collections::BTreeMap<_, _> = output
            .nodes
            .iter()
            .filter(|n| n.kind == GraphNodeKind::Symbol)
            .map(|n| (n.label.clone(), n.symbol_kind))
            .collect();
        assert_eq!(by_label.get("Foo"), Some(&Some(SymbolKind::Type)));
        assert_eq!(by_label.get("bar"), Some(&Some(SymbolKind::Method)));
        assert_eq!(by_label.get("top"), Some(&Some(SymbolKind::Function)));
        assert_eq!(by_label.get("atop"), Some(&Some(SymbolKind::Function)));
    }

    #[test]
    fn pytest_convention_functions_are_test_symbols() {
        let output = extract("def test_addition():\n    assert 1 + 1 == 2\n");
        let symbol = output
            .nodes
            .iter()
            .find(|n| n.label == "test_addition")
            .unwrap();
        assert_eq!(symbol.symbol_kind, Some(SymbolKind::Test));
        assert_eq!(
            symbol.node_role.as_ref().map(crate::GraphNodeRole::as_str),
            Some(crate::roles::TEST_TARGET)
        );
    }

    #[test]
    fn non_test_symbols_carry_no_test_target_role() {
        let output = extract("def top():\n    pass\n");
        let symbol = output.nodes.iter().find(|n| n.label == "top").unwrap();
        assert_eq!(symbol.node_role, None);
    }

    #[test]
    fn imports_are_facts_not_resolved_targets() {
        let output = extract("import os\nfrom typing import List\n");
        let import_edges: Vec<_> = output
            .edges
            .iter()
            .filter(|e| e.kind == GraphEdgeKind::Imports)
            .collect();
        assert_eq!(import_edges.len(), 2);
        for edge in import_edges {
            assert_eq!(edge.layer, GraphLayer::Semantic);
            assert_eq!(edge.derivation, DerivationClass::Parser);
        }
    }

    #[test]
    fn malformed_python_still_yields_partial_coverage() {
        let output = extract("def f(:\n    this is not valid python\n");
        assert!(matches!(output.coverage, FileCoverage::Partial { .. }));
    }
}
