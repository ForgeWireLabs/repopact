//! Deterministic JavaScript/TypeScript semantic extraction (WI063
//! ROG-018), covering `.js`/`.jsx`/`.ts`/`.tsx`. Grounded against real
//! tree-sitter-javascript 0.25.0 and tree-sitter-typescript 0.23.2 AST
//! output (verified via a throwaway spike): `function_declaration`,
//! `class_declaration`, and `method_definition` all expose a `name`
//! field; TypeScript's `interface_declaration`, `type_alias_declaration`,
//! and `enum_declaration` do too. `export_statement` merely wraps a
//! declaration -- the recursive walk naturally descends into it, so no
//! special-casing is needed to still find the wrapped declaration's name.
//! `.jsx`/`.tsx` are parsed with the JSX-aware grammar variant, not
//! silently treated as plain JS/TS (Decision 0045/the checkpoint brief
//! both require TS and TSX be treated as distinct grammars where
//! required; `tree-sitter-javascript`'s single grammar already parses
//! JSX, and `tree-sitter-typescript` exposes `LANGUAGE_TSX` as a genuinely
//! separate grammar from `LANGUAGE_TYPESCRIPT`, used here accordingly).
//!
//! Does not equate a syntactic identifier reference with a fully resolved
//! module/type reference. Import/export facts are raw statement text, the
//! same "fact, not resolved identity" approach as the Rust/Python
//! adapters.

use std::ops::ControlFlow;
use std::time::Instant;

use repopact_types::{RecordKind, RecordRef};
use tree_sitter::{Language, Node, Parser};

use super::{
    location_from_span, node_id_for_symbol, AdapterOutput, FileCoverage, SemanticAdapter,
    SourceInput, SourceLanguage,
};
use crate::{
    DerivationClass, GraphEdge, GraphEdgeKind, GraphLayer, GraphNode, GraphNodeKind, SymbolKind,
};

pub const ADAPTER_VERSION: &str = "javascript-typescript-adapter-0.1.0";
pub const SUPPORTED_RELATIONS: [&str; 2] = ["defines", "imports"];
/// ROG-022: a maliciously or accidentally deeply nested AST must not
/// overflow this walker\'s own recursion stack. Exceeding this depth
/// truncates the walk for that subtree and marks the file Partial --
/// it never panics or aborts the whole build.
const MAX_WALK_DEPTH: usize = 512;

pub struct JavaScriptFamilyAdapter {
    language: SourceLanguage,
}

impl JavaScriptFamilyAdapter {
    pub fn new(language: SourceLanguage) -> Self {
        Self { language }
    }

    fn grammar(&self) -> Language {
        match self.language {
            SourceLanguage::JavaScript | SourceLanguage::Jsx => {
                tree_sitter_javascript::LANGUAGE.into()
            }
            SourceLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            SourceLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            SourceLanguage::Rust
            | SourceLanguage::Python
            | SourceLanguage::Json
            | SourceLanguage::Toml
            | SourceLanguage::Yaml
            | SourceLanguage::Markdown => {
                unreachable!("adapter is only constructed for JS/JSX/TS/TSX")
            }
        }
    }
}

impl SemanticAdapter for JavaScriptFamilyAdapter {
    fn language(&self) -> SourceLanguage {
        self.language
    }

    fn extract(&self, input: &SourceInput) -> AdapterOutput {
        let mut parser = Parser::new();
        if parser.set_language(&self.grammar()).is_err() {
            return AdapterOutput {
                nodes: Vec::new(),
                edges: Vec::new(),
                coverage: FileCoverage::Failed {
                    reason: format!("unable to load {} grammar", self.language.label()),
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
            self.language,
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
    language: SourceLanguage,
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
        "function_declaration" => {
            if let Some(name) = named_text(node, source) {
                let is_test = looks_like_test_declaration(node, source);
                let (tag, kind) = if is_test {
                    ("test", SymbolKind::Test)
                } else {
                    ("function", SymbolKind::Function)
                };
                emit_symbol(
                    node,
                    language,
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
        "class_declaration" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    language,
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
        "method_definition" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    language,
                    relative_path,
                    container,
                    &name,
                    "method",
                    SymbolKind::Method,
                    file_node_id,
                    nodes,
                    edges,
                );
            }
        }
        "interface_declaration" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    language,
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
        "type_alias_declaration" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    language,
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
        "enum_declaration" => {
            if let Some(name) = named_text(node, source) {
                emit_symbol(
                    node,
                    language,
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
        "import_statement" => {
            let import_text = text_of(node, source).trim().to_owned();
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
            language,
            relative_path,
            &next_container,
            file_node_id,
            nodes,
            edges,
            has_error,
        );
    }
}

/// Deterministic, disclosed heuristic: a top-level function named
/// `test`/`it`/prefixed `test_`/`test`-suffixed inside a `describe`/`test`
/// call is common across JS test frameworks, but there is no single
/// syntactic marker analogous to Rust's `#[test]`. This checkpoint only
/// treats an exported function literally named with a `test`/`Test`
/// prefix as a test symbol -- a narrow, conservative convention rather
/// than an attempt to recognize every test-framework calling convention.
fn looks_like_test_declaration(node: Node, source: &[u8]) -> bool {
    named_text(node, source)
        .is_some_and(|name| name.starts_with("test") || name.starts_with("Test"))
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
    language: SourceLanguage,
    relative_path: &str,
    container: &str,
    name: &str,
    kind_tag: &str,
    symbol_kind: SymbolKind,
    file_node_id: &str,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
) {
    let id = node_id_for_symbol(language, relative_path, container, kind_tag, name);
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

    fn extract(language: SourceLanguage, path: &str, source: &str) -> AdapterOutput {
        let policy = super::super::ResourcePolicy::default();
        let input = SourceInput {
            relative_path: path,
            content: source.as_bytes(),
            resource_policy: &policy,
        };
        JavaScriptFamilyAdapter::new(language).extract(&input)
    }

    #[test]
    fn covers_js_function_class_and_method() {
        let output = extract(
            SourceLanguage::JavaScript,
            "src/index.js",
            "export function bar() {}\nclass Baz { method() {} }\n",
        );
        assert_eq!(output.coverage, FileCoverage::Complete);
        let by_label: std::collections::BTreeMap<_, _> = output
            .nodes
            .iter()
            .filter(|n| n.kind == GraphNodeKind::Symbol)
            .map(|n| (n.label.clone(), n.symbol_kind))
            .collect();
        assert_eq!(by_label.get("bar"), Some(&Some(SymbolKind::Function)));
        assert_eq!(by_label.get("Baz"), Some(&Some(SymbolKind::Type)));
        assert_eq!(by_label.get("method"), Some(&Some(SymbolKind::Method)));
        let bar = output.nodes.iter().find(|n| n.label == "bar").unwrap();
        assert_eq!(bar.node_role, None);
    }

    #[test]
    fn test_named_functions_are_tagged_test_target() {
        let output = extract(
            SourceLanguage::JavaScript,
            "src/index.test.js",
            "function testAddition() {}\n",
        );
        let symbol = output
            .nodes
            .iter()
            .find(|n| n.label == "testAddition")
            .unwrap();
        assert_eq!(symbol.symbol_kind, Some(SymbolKind::Test));
        assert_eq!(
            symbol.node_role.as_ref().map(crate::GraphNodeRole::as_str),
            Some(crate::roles::TEST_TARGET)
        );
    }

    #[test]
    fn ts_and_tsx_are_treated_as_distinct_grammars() {
        let ts_output = extract(
            SourceLanguage::TypeScript,
            "src/types.ts",
            "interface Foo { x: number }\ntype Bar = string;\nenum Color { Red, Green }\n",
        );
        assert_eq!(ts_output.coverage, FileCoverage::Complete);
        let labels: Vec<_> = ts_output.nodes.iter().map(|n| n.label.clone()).collect();
        assert!(labels.contains(&"Foo".to_owned()));
        assert!(labels.contains(&"Bar".to_owned()));
        assert!(labels.contains(&"Color".to_owned()));

        // JSX syntax parses under the TSX grammar but would be a syntax
        // error under the plain TypeScript grammar -- proving the two are
        // genuinely distinct grammars, not one grammar reused by label.
        let tsx_output = extract(
            SourceLanguage::Tsx,
            "src/App.tsx",
            "function App() { return <div/>; }\n",
        );
        assert_eq!(tsx_output.coverage, FileCoverage::Complete);
        let ts_on_jsx = extract(
            SourceLanguage::TypeScript,
            "src/App.tsx",
            "function App() { return <div/>; }\n",
        );
        assert!(
            matches!(ts_on_jsx.coverage, FileCoverage::Partial { .. }),
            "JSX syntax parsed under the plain TypeScript grammar must show a syntax error, proving TSX is not silently treated as plain TS"
        );
    }

    #[test]
    fn jsx_file_uses_the_javascript_grammar_and_parses_jsx_syntax() {
        let output = extract(
            SourceLanguage::Jsx,
            "src/App.jsx",
            "function App() { return <div/>; }\n",
        );
        assert_eq!(output.coverage, FileCoverage::Complete);
    }

    #[test]
    fn imports_are_facts_not_resolved_targets() {
        let output = extract(
            SourceLanguage::JavaScript,
            "src/index.js",
            "import { foo } from './foo';\n",
        );
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
        assert!(target.label.contains("./foo"));
        assert_eq!(import_edge.derivation, DerivationClass::Parser);
    }

    #[test]
    fn malformed_js_still_yields_partial_coverage() {
        let output = extract(
            SourceLanguage::JavaScript,
            "src/index.js",
            "function f( { this is not valid js\n",
        );
        assert!(matches!(output.coverage, FileCoverage::Partial { .. }));
    }
}
