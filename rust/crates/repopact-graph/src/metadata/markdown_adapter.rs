//! Markdown metadata adapter (WI063 ROG-018, Decision 0048). Structural
//! only, never NLP: heading hierarchy, repository-relative links,
//! code-fence languages, and front-matter presence. Entire prose bodies,
//! embeddings, and LLM summaries are never stored -- only short,
//! bounded structural facts (a heading's text, a link's target, a
//! fence's declared language).

use repopact_types::{RecordKind, RecordRef};

use super::{manifest_fact_node_id, manifest_node_id, MetadataAdapter};
use crate::semantic::{AdapterOutput, FileCoverage, SourceInput};
use crate::{
    DerivationClass, GraphEdge, GraphEdgeKind, GraphLayer, GraphNode, GraphNodeKind, ManifestKind,
};

pub const ADAPTER_VERSION: &str = "markdown-metadata-adapter-0.1.0";

/// Headings/links beyond this count are not emitted -- a bound against a
/// pathologically large document, consistent with ROG-022's resource
/// discipline. The document's coverage remains `Complete`; this is a
/// per-document output cap, not a parse failure.
const MAX_FACTS_PER_DOCUMENT: usize = 500;

pub struct MarkdownAdapter;

fn source(relative_path: &str) -> RecordRef {
    RecordRef::new(
        RecordKind::File,
        relative_path.to_owned(),
        relative_path.to_owned(),
    )
}

fn manifest_document_node(relative_path: &str) -> GraphNode {
    GraphNode {
        id: manifest_node_id(relative_path),
        kind: GraphNodeKind::Manifest,
        label: format!("markdown:{relative_path}"),
        layer: GraphLayer::Package,
        source: Some(source(relative_path)),
        symbol_kind: None,
        location: None,
        manifest_kind: Some(ManifestKind::MarkdownDocument),
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

fn references_edge(from: String, to: String, relative_path: &str) -> GraphEdge {
    GraphEdge {
        from,
        to,
        kind: GraphEdgeKind::References,
        layer: GraphLayer::Package,
        derivation: DerivationClass::Manifest,
        source: source(relative_path),
        location: None,
        relation_role: None,
    }
}

/// Extract a Markdown inline link target: `[text](target)`. Deterministic
/// bracket/paren scanning, not a full CommonMark parser -- sufficient
/// for the concrete fact this checkpoint claims (a link target string
/// appears in this document), not a claim of full Markdown fidelity.
fn extract_links(line: &str) -> Vec<String> {
    let mut links = Vec::new();
    let bytes = line.as_bytes();
    let mut index = 0;
    while let Some(open_bracket) = line[index..].find('[') {
        let bracket_pos = index + open_bracket;
        let Some(close_bracket_rel) = line[bracket_pos..].find(']') else {
            break;
        };
        let close_bracket = bracket_pos + close_bracket_rel;
        if close_bracket + 1 >= bytes.len() || bytes[close_bracket + 1] != b'(' {
            index = close_bracket + 1;
            continue;
        }
        let paren_start = close_bracket + 2;
        let Some(close_paren_rel) = line[paren_start..].find(')') else {
            break;
        };
        let close_paren = paren_start + close_paren_rel;
        let target = line[paren_start..close_paren].trim();
        if !target.is_empty() {
            links.push(target.to_owned());
        }
        index = close_paren + 1;
    }
    links
}

fn extract(relative_path: &str, text: &str) -> AdapterOutput {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let manifest_id = manifest_node_id(relative_path);
    nodes.push(manifest_document_node(relative_path));

    let mut lines = text.lines().enumerate();
    let mut fact_count = 0usize;
    let mut in_front_matter = false;
    let mut in_fence = false;

    if text.starts_with("---\n") || text == "---" {
        in_front_matter = true;
        let fact = fact_node(
            relative_path,
            "front_matter",
            "present",
            "front matter present".to_owned(),
        );
        edges.push(contains_edge(
            manifest_id.clone(),
            fact.id.clone(),
            relative_path,
        ));
        nodes.push(fact);
        fact_count += 1;
    }

    while let Some((line_index, line)) = lines.next() {
        if fact_count >= MAX_FACTS_PER_DOCUMENT {
            break;
        }
        if in_front_matter {
            if line_index > 0 && line.trim() == "---" {
                in_front_matter = false;
            }
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(fence_lang) = trimmed.strip_prefix("```") {
            if !in_fence {
                in_fence = true;
                let lang = fence_lang.trim();
                if !lang.is_empty() {
                    let fact = fact_node(
                        relative_path,
                        "code_fence",
                        &format!("{lang}-{line_index}"),
                        lang.to_owned(),
                    );
                    edges.push(contains_edge(
                        manifest_id.clone(),
                        fact.id.clone(),
                        relative_path,
                    ));
                    nodes.push(fact);
                    fact_count += 1;
                }
            } else {
                in_fence = false;
            }
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(heading_text) = trimmed.strip_prefix("#") {
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            let text = heading_text.trim_start_matches('#').trim();
            if !text.is_empty() && level <= 6 {
                let fact = fact_node(
                    relative_path,
                    "heading",
                    &format!("{level}-{line_index}"),
                    text.to_owned(),
                );
                edges.push(contains_edge(
                    manifest_id.clone(),
                    fact.id.clone(),
                    relative_path,
                ));
                nodes.push(fact);
                fact_count += 1;
            }
        }
        for target in extract_links(line) {
            if fact_count >= MAX_FACTS_PER_DOCUMENT {
                break;
            }
            // Only repository-relative-looking targets (no scheme, not an
            // anchor-only fragment) are recorded as references -- external
            // URLs and pure in-page anchors are still real links, but this
            // checkpoint only claims repository-path references as facts.
            if target.contains("://") || target.starts_with('#') {
                continue;
            }
            let fact = fact_node(
                relative_path,
                "link",
                &format!("{target}-{line_index}"),
                target.clone(),
            );
            edges.push(references_edge(
                manifest_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
            fact_count += 1;
        }
    }

    AdapterOutput {
        nodes,
        edges,
        coverage: FileCoverage::Complete,
    }
}

impl MetadataAdapter for MarkdownAdapter {
    fn identity(&self) -> &'static str {
        ADAPTER_VERSION
    }

    fn accepts(&self, relative_path: &str) -> bool {
        relative_path.ends_with(".md") || relative_path.ends_with(".markdown")
    }

    fn extract(&self, input: &SourceInput) -> AdapterOutput {
        let Ok(text) = std::str::from_utf8(input.content) else {
            return AdapterOutput {
                nodes: Vec::new(),
                edges: Vec::new(),
                coverage: FileCoverage::Failed {
                    reason: "not valid UTF-8".to_owned(),
                },
            };
        };
        extract(input.relative_path, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::ResourcePolicy;

    fn run(relative_path: &str, content: &str) -> AdapterOutput {
        let policy = ResourcePolicy::default();
        let input = SourceInput {
            relative_path,
            content: content.as_bytes(),
            resource_policy: &policy,
        };
        MarkdownAdapter.extract(&input)
    }

    #[test]
    fn headings_links_and_fences_are_extracted() {
        let content = "# Title\n\nSee [the README](./README.md) for more.\n\n```rust\nfn f() {}\n```\n\n## Section\n";
        let output = run("docs/guide.md", content);
        assert_eq!(output.coverage, FileCoverage::Complete);
        assert!(output.nodes.iter().any(|n| n.label == "Title"));
        assert!(output.nodes.iter().any(|n| n.label == "Section"));
        assert!(output.nodes.iter().any(|n| n.label == "./README.md"));
        assert!(output.nodes.iter().any(|n| n.label == "rust"));
    }

    #[test]
    fn front_matter_presence_is_detected() {
        let content = "---\ntitle: test\n---\n\n# Body\n";
        let output = run("docs/page.md", content);
        assert!(output
            .nodes
            .iter()
            .any(|n| n.label.contains("front matter")));
    }

    #[test]
    fn external_urls_and_anchors_are_not_recorded_as_reference_facts() {
        let content = "[external](https://example.com) and [anchor](#section)\n";
        let output = run("docs/page.md", content);
        assert!(!output.nodes.iter().any(|n| n.label.contains("example.com")));
        assert!(!output.nodes.iter().any(|n| n.label == "#section"));
    }

    #[test]
    fn code_inside_a_fence_is_not_scanned_for_headings() {
        let content = "```text\n# not a heading\n```\n";
        let output = run("docs/page.md", content);
        assert!(!output.nodes.iter().any(|n| n.label == "not a heading"));
    }

    #[test]
    fn no_prose_body_is_stored_only_structural_facts() {
        let content = "# Title\n\nThis is a long paragraph of prose that must never appear verbatim in the durable graph, only the heading text and any links/fences above.\n";
        let output = run("docs/page.md", content);
        assert!(!output
            .nodes
            .iter()
            .any(|n| n.label.contains("long paragraph")));
    }
}
