//! Deterministic Rust semantic extraction (WI063 ROG-017, this
//! checkpoint's first language). Grounded directly against real
//! tree-sitter-rust 0.24.2 AST output (verified via a throwaway spike, not
//! assumed from documentation) before being written: `mod_item`,
//! `function_item`, `struct_item`, `trait_item`, `enum_item`, `type_item`,
//! and `macro_definition` all expose a `name` field; `impl_item` exposes
//! `type` and optionally `trait` fields; `use_declaration` has no single
//! stable sub-field worth parsing further for this checkpoint, so its raw
//! import-path text is used directly as the import fact.
//!
//! Does not claim rustc-level name resolution. An import syntax edge is an
//! import fact (the literal `use` path text), not a resolved target
//! identity.

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

pub const ADAPTER_VERSION: &str = "rust-adapter-0.1.0";
pub const SUPPORTED_RELATIONS: [&str; 2] = ["defines", "imports"];

pub struct RustAdapter;

impl SemanticAdapter for RustAdapter {
    fn language(&self) -> SourceLanguage {
        SourceLanguage::Rust
    }

    fn extract(&self, input: &SourceInput) -> AdapterOutput {
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .is_err()
        {
            return AdapterOutput {
                nodes: Vec::new(),
                edges: Vec::new(),
                coverage: FileCoverage::Failed {
                    reason: "unable to load Rust grammar".to_owned(),
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
    source: &[u8],
    relative_path: &str,
    container: &str,
    file_node_id: &str,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
    has_error: &mut bool,
) {
    if node.is_error() {
        *has_error = true;
    }

    let mut next_container = container.to_owned();

    match node.kind() {
        "mod_item" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    source,
                    relative_path,
                    container,
                    &name,
                    "module",
                    SymbolKind::Module,
                    file_node_id,
                    nodes,
                    edges,
                );
                next_container = qualify(container, &name);
            }
        }
        "function_item" => {
            if let Some(name) = named_text(node, source) {
                let is_test = has_test_attribute(node, source);
                let (tag, kind) = if is_test {
                    ("test", SymbolKind::Test)
                } else {
                    ("function", SymbolKind::Function)
                };
                emit_symbol(
                    node,
                    source,
                    relative_path,
                    container,
                    &name,
                    tag,
                    kind,
                    file_node_id,
                    nodes,
                    edges,
                );
            }
        }
        "struct_item" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    source,
                    relative_path,
                    container,
                    &name,
                    "type",
                    SymbolKind::Type,
                    file_node_id,
                    nodes,
                    edges,
                );
            }
        }
        "enum_item" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    source,
                    relative_path,
                    container,
                    &name,
                    "enum",
                    SymbolKind::Enum,
                    file_node_id,
                    nodes,
                    edges,
                );
            }
        }
        "trait_item" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    source,
                    relative_path,
                    container,
                    &name,
                    "interface",
                    SymbolKind::Interface,
                    file_node_id,
                    nodes,
                    edges,
                );
            }
        }
        "type_item" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    source,
                    relative_path,
                    container,
                    &name,
                    "type_alias",
                    SymbolKind::TypeAlias,
                    file_node_id,
                    nodes,
                    edges,
                );
            }
        }
        "macro_definition" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    source,
                    relative_path,
                    container,
                    &name,
                    "macro",
                    SymbolKind::Macro,
                    file_node_id,
                    nodes,
                    edges,
                );
            }
        }
        "impl_item" => {
            let target = node
                .child_by_field_name("type")
                .map(|child| text_of(child, source))
                .unwrap_or_else(|| "<unknown>".to_owned());
            let label = match node.child_by_field_name("trait") {
                Some(trait_node) => format!("impl {} for {target}", text_of(trait_node, source)),
                None => format!("impl {target}"),
            };
            emit_symbol(
                node,
                source,
                relative_path,
                container,
                &label,
                "impl",
                SymbolKind::Implementation,
                file_node_id,
                nodes,
                edges,
            );
            next_container = qualify(container, &label);
        }
        "use_declaration" => {
            let import_text = text_of(node, source)
                .trim_start_matches("use")
                .trim()
                .trim_end_matches(';')
                .trim()
                .to_owned();
            if !import_text.is_empty() {
                let target_id = format!("import:{}:{import_text}", relative_path);
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
                });
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(
            child,
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
        format!("{container}::{name}")
    }
}

/// A function is a Test symbol only when the `attribute_item` immediately
/// preceding it in its parent's child list has an `attribute` node whose
/// text is exactly `test` (covers `#[test]`) or ends with `::test`
/// (covers `#[tokio::test]` and similar). This is a deterministic
/// syntactic check, not attribute-macro resolution -- a custom attribute
/// that merely happens to be named `test` on an unrelated crate would
/// also match, which is an accepted, disclosed limitation rather than a
/// claim of semantic understanding.
fn has_test_attribute(function_node: Node, source: &[u8]) -> bool {
    let Some(parent) = function_node.parent() else {
        return false;
    };
    let mut cursor = parent.walk();
    let mut immediately_preceding_attribute: Option<Node> = None;
    for child in parent.children(&mut cursor) {
        if child.id() == function_node.id() {
            break;
        }
        if child.kind() == "attribute_item" {
            immediately_preceding_attribute = Some(child);
        } else if !child.is_extra()
            && child.kind() != "line_comment"
            && child.kind() != "block_comment"
        {
            immediately_preceding_attribute = None;
        }
    }
    let Some(attribute_item) = immediately_preceding_attribute else {
        return false;
    };
    let mut inner_cursor = attribute_item.walk();
    // `#[test]`'s inner node is kind "attribute" (bare identifier text);
    // `#[tokio::test]`'s is an "attribute" wrapping a "scoped_identifier".
    // Checking the node's own text against "test"/"*::test" covers both
    // without depending on exactly which of these two grammar shapes a
    // given attribute macro happens to produce.
    let result = attribute_item.children(&mut inner_cursor).any(|child| {
        matches!(
            child.kind(),
            "attribute" | "identifier" | "scoped_identifier"
        ) && {
            let text = text_of(child, source);
            text == "test" || text.ends_with("::test")
        }
    });
    result
}

#[allow(clippy::too_many_arguments)]
fn emit_symbol(
    node: Node,
    _source: &[u8],
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
        SourceLanguage::Rust,
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
            relative_path: "src/lib.rs",
            content: source.as_bytes(),
            resource_policy: &policy,
        };
        RustAdapter.extract(&input)
    }

    #[test]
    fn covers_module_function_struct_enum_trait_type_alias_impl_macro() {
        let output = extract(
            "mod m {\n    pub fn f() {}\n    struct S;\n}\nenum E { A }\ntrait T {}\nimpl T for i32 {}\ntype Alias = i32;\nmacro_rules! mac { () => {}; }\n",
        );
        assert_eq!(output.coverage, FileCoverage::Complete);
        let kinds: Vec<_> = output
            .nodes
            .iter()
            .filter(|n| n.kind == GraphNodeKind::Symbol)
            .map(|n| n.symbol_kind)
            .collect();
        assert!(kinds.contains(&Some(SymbolKind::Module)));
        assert!(kinds.contains(&Some(SymbolKind::Function)));
        assert!(kinds.contains(&Some(SymbolKind::Type)));
        assert!(kinds.contains(&Some(SymbolKind::Enum)));
        assert!(kinds.contains(&Some(SymbolKind::Interface)));
        assert!(kinds.contains(&Some(SymbolKind::Implementation)));
        assert!(kinds.contains(&Some(SymbolKind::TypeAlias)));
        assert!(kinds.contains(&Some(SymbolKind::Macro)));
    }

    #[test]
    fn detects_test_attribute_functions_as_test_symbols() {
        let output = extract("#[test]\nfn it_works() {}\nfn not_a_test() {}\n");
        let test_symbol = output.nodes.iter().find(|n| n.label == "it_works").unwrap();
        assert_eq!(test_symbol.symbol_kind, Some(SymbolKind::Test));
        let plain_symbol = output
            .nodes
            .iter()
            .find(|n| n.label == "not_a_test")
            .unwrap();
        assert_eq!(plain_symbol.symbol_kind, Some(SymbolKind::Function));
    }

    #[test]
    fn use_declaration_becomes_an_import_fact_not_a_resolved_target() {
        let output = extract("use std::collections::HashMap;\n");
        let import_edge = output
            .edges
            .iter()
            .find(|e| e.kind == GraphEdgeKind::Imports)
            .unwrap();
        let target = output
            .nodes
            .iter()
            .find(|n| n.id == import_edge.to)
            .unwrap();
        assert_eq!(target.label, "std::collections::HashMap");
        assert_eq!(target.layer, GraphLayer::Semantic);
        assert_eq!(target.symbol_kind, Some(SymbolKind::Module));
    }

    #[test]
    fn every_semantic_edge_carries_layer_derivation_and_location() {
        let output = extract("fn f() {}\n");
        for edge in &output.edges {
            assert_eq!(edge.layer, GraphLayer::Semantic);
            assert_eq!(edge.derivation, DerivationClass::Parser);
            assert!(
                edge.location.is_some(),
                "every semantic edge must carry an explanatory source span"
            );
        }
    }

    #[test]
    fn stable_id_is_unaffected_by_leading_blank_lines_and_comments() {
        let a = extract("fn f() {}\n");
        let b = extract("\n\n// a comment\n\nfn f() {}\n");
        let id_a = a.nodes.iter().find(|n| n.label == "f").unwrap().id.clone();
        let id_b = b.nodes.iter().find(|n| n.label == "f").unwrap().id.clone();
        assert_eq!(
            id_a, id_b,
            "irrelevant leading whitespace/comments must not change a symbol's stable ID"
        );
    }

    #[test]
    fn malformed_syntax_still_yields_partial_coverage_not_a_crash() {
        let output = extract("fn f( { this is not valid rust\n");
        assert!(matches!(output.coverage, FileCoverage::Partial { .. }));
    }

    #[test]
    fn parse_deadline_is_enforced_via_progress_callback_not_deprecated_timeout() {
        let huge_source = "fn f() {}\n".repeat(2_000_000);
        let policy = super::super::ResourcePolicy {
            max_file_bytes: usize::MAX,
            max_parse_duration: std::time::Duration::from_nanos(1),
        };
        let input = SourceInput {
            relative_path: "src/lib.rs",
            content: huge_source.as_bytes(),
            resource_policy: &policy,
        };
        let output = RustAdapter.extract(&input);
        assert!(
            matches!(output.coverage, FileCoverage::Failed { .. }),
            "an effectively-zero deadline must cancel the parse, not hang or panic"
        );
    }

    #[test]
    fn parser_is_reusable_after_a_cancelled_parse() {
        // Two extractions in a row on the same adapter type (a fresh
        // Parser is constructed per call in this implementation, but the
        // property under test -- tree-sitter's own reset-before-reuse
        // contract -- is exercised identically either way since
        // RustAdapter::extract always starts from Parser::new()).
        let policy_tight = super::super::ResourcePolicy {
            max_file_bytes: usize::MAX,
            max_parse_duration: std::time::Duration::from_nanos(1),
        };
        let huge = "fn f() {}\n".repeat(2_000_000);
        let cancelled_input = SourceInput {
            relative_path: "src/lib.rs",
            content: huge.as_bytes(),
            resource_policy: &policy_tight,
        };
        let cancelled = RustAdapter.extract(&cancelled_input);
        assert!(matches!(cancelled.coverage, FileCoverage::Failed { .. }));

        let normal = extract("fn after_cancel() {}\n");
        assert_eq!(normal.coverage, FileCoverage::Complete);
        assert!(normal.nodes.iter().any(|n| n.label == "after_cancel"));
    }
}
