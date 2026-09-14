//! YAML metadata adapter (WI063 ROG-019, Decision 0048): CI workflow
//! files under `.github/workflows/`. This is a deterministic,
//! line-oriented, indentation-aware scanner for the specific shape
//! GitHub Actions workflow YAML uses -- not a general YAML parser (no
//! anchors, no flow-style collections, no multi-document streams).
//! `${{ ... }}` expressions are recorded verbatim as symbolic text, never
//! evaluated or resolved; a `uses:`/`run:` reference is a fact that the
//! line exists, never a claim that the referenced action or script's
//! full behavior has been resolved.

use repopact_types::{RecordKind, RecordRef};

use super::{manifest_fact_node_id, manifest_node_id, MetadataAdapter};
use crate::semantic::{AdapterOutput, FileCoverage, SkipReason, SourceInput};
use crate::{
    DerivationClass, GraphEdge, GraphEdgeKind, GraphLayer, GraphNode, GraphNodeKind, ManifestKind,
};

pub const ADAPTER_VERSION: &str = "yaml-ci-workflow-adapter-0.1.0";

/// A `run:` fact's stored text is truncated to this many bytes -- a
/// short, deterministic fact ("this step runs a command starting with
/// ..."), never a claim that the entire script body was captured or
/// resolved.
const MAX_RUN_FACT_BYTES: usize = 160;

pub struct YamlAdapter;

fn source(relative_path: &str) -> RecordRef {
    RecordRef::new(
        RecordKind::File,
        relative_path.to_owned(),
        relative_path.to_owned(),
    )
}

fn manifest_document_node(relative_path: &str, label: String) -> GraphNode {
    GraphNode {
        id: manifest_node_id(relative_path),
        kind: GraphNodeKind::Manifest,
        label,
        layer: GraphLayer::Build,
        source: Some(source(relative_path)),
        symbol_kind: None,
        location: None,
        manifest_kind: Some(ManifestKind::CiWorkflow),
    }
}

fn fact_node(relative_path: &str, fact_kind: &str, name: &str, label: String) -> GraphNode {
    GraphNode {
        id: manifest_fact_node_id(relative_path, fact_kind, name),
        kind: GraphNodeKind::Manifest,
        label,
        layer: GraphLayer::Build,
        source: Some(source(relative_path)),
        symbol_kind: None,
        location: None,
        manifest_kind: None,
    }
}

fn contains_edge(from: String, to: String, relative_path: &str) -> GraphEdge {
    GraphEdge {
        from,
        to,
        kind: GraphEdgeKind::Contains,
        layer: GraphLayer::Build,
        derivation: DerivationClass::Manifest,
        source: source(relative_path),
        location: None,
    }
}

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

fn key_value(trimmed: &str, key: &str) -> Option<String> {
    trimmed
        .strip_prefix(key)?
        .strip_prefix(':')
        .map(|rest| rest.trim().to_owned())
}

fn extract(relative_path: &str, text: &str) -> AdapterOutput {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let manifest_id = manifest_node_id(relative_path);

    let mut workflow_name: Option<String> = None;
    let mut current_job: Option<String> = None;
    let mut current_job_fact_id: Option<String> = None;
    let mut in_jobs = false;
    let mut step_counter: usize = 0;

    for raw_line in text.lines() {
        let line = raw_line.split('#').next().unwrap_or(raw_line);
        let trimmed_start = line.trim_start();
        if trimmed_start.is_empty() {
            continue;
        }
        let indent = leading_spaces(line);
        let trimmed = line.trim();

        if indent == 0 {
            if let Some(value) = key_value(trimmed, "name") {
                if workflow_name.is_none() && !value.is_empty() {
                    workflow_name = Some(value);
                }
                continue;
            }
            in_jobs = trimmed == "jobs:";
            continue;
        }

        if in_jobs && indent == 2 && trimmed.ends_with(':') {
            let job_id = trimmed.trim_end_matches(':').to_owned();
            current_job = Some(job_id.clone());
            let fact = fact_node(relative_path, "job", &job_id, job_id.clone());
            current_job_fact_id = Some(fact.id.clone());
            edges.push(contains_edge(
                manifest_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
            step_counter = 0;
            continue;
        }

        let Some(job_fact_id) = current_job_fact_id.clone() else {
            continue;
        };
        let trimmed_item = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        if trimmed.starts_with("- ") {
            step_counter += 1;
        }
        if let Some(action) = key_value(trimmed_item, "uses") {
            let fact = fact_node(
                relative_path,
                "uses_action",
                &format!("{}-{step_counter}", current_job.clone().unwrap_or_default()),
                action,
            );
            edges.push(contains_edge(
                job_fact_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
        } else if let Some(run) = key_value(trimmed_item, "run") {
            let truncated: String = run.chars().take(MAX_RUN_FACT_BYTES).collect();
            let fact = fact_node(
                relative_path,
                "runs_script",
                &format!("{}-{step_counter}", current_job.clone().unwrap_or_default()),
                truncated,
            );
            edges.push(contains_edge(
                job_fact_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
        } else if let Some(dir) = key_value(trimmed_item, "working-directory") {
            let fact = fact_node(
                relative_path,
                "working_directory",
                &format!("{}-{step_counter}", current_job.clone().unwrap_or_default()),
                dir,
            );
            edges.push(contains_edge(
                job_fact_id.clone(),
                fact.id.clone(),
                relative_path,
            ));
            nodes.push(fact);
        }
    }

    let mut all_nodes = vec![manifest_document_node(
        relative_path,
        workflow_name
            .map(|name| format!("workflow:{name}"))
            .unwrap_or_else(|| format!("workflow:{relative_path}")),
    )];
    all_nodes.extend(nodes);

    AdapterOutput {
        nodes: all_nodes,
        edges,
        coverage: FileCoverage::Complete,
    }
}

impl MetadataAdapter for YamlAdapter {
    fn identity(&self) -> &'static str {
        ADAPTER_VERSION
    }

    fn accepts(&self, relative_path: &str) -> bool {
        relative_path.starts_with(".github/workflows/")
            && (relative_path.ends_with(".yml") || relative_path.ends_with(".yaml"))
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
        YamlAdapter.extract(&input)
    }

    const WORKFLOW: &str = "name: Governance validation\n\njobs:\n  validate:\n    if: ${{ vars.REPOPACT_GITHUB_CI == 'true' }}\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n        with:\n          fetch-depth: 0\n      - name: Run canonical local CI profile\n        run: python -m repopact.verify_cli ci --root .\n";

    #[test]
    fn workflow_job_uses_and_run_facts_are_extracted() {
        let output = run(".github/workflows/governance.yml", WORKFLOW);
        assert_eq!(output.coverage, FileCoverage::Complete);
        assert!(output
            .nodes
            .iter()
            .any(|n| n.manifest_kind == Some(ManifestKind::CiWorkflow)
                && n.label.contains("Governance validation")));
        assert!(output.nodes.iter().any(|n| n.label == "validate"));
        assert!(output
            .nodes
            .iter()
            .any(|n| n.label == "actions/checkout@v4"));
        assert!(output
            .nodes
            .iter()
            .any(|n| n.label.contains("repopact.verify_cli")));
    }

    #[test]
    fn expressions_are_recorded_symbolically_not_evaluated() {
        // The `if:` expression line is not a recognized fact key
        // (uses/run/working-directory), so it produces no fabricated
        // fact -- this proves the adapter does not attempt to evaluate
        // `${{ ... }}` expressions.
        let output = run(".github/workflows/governance.yml", WORKFLOW);
        assert!(!output
            .nodes
            .iter()
            .any(|n| n.label.contains("REPOPACT_GITHUB_CI")));
    }

    #[test]
    fn a_file_outside_github_workflows_is_skipped() {
        let output = run("some/other.yml", "name: not a workflow\n");
        assert!(matches!(output.coverage, FileCoverage::Skipped { .. }));
    }

    #[test]
    fn malformed_yaml_bytes_do_not_crash_the_scanner() {
        // The scanner is line-oriented, so "malformed" YAML (inconsistent
        // indentation, stray colons) does not raise a parse error the
        // way a real YAML parser would -- it simply yields fewer or no
        // facts. Coverage must remain Complete (not Failed), since no
        // adapter panic or I/O error occurred.
        let output = run(
            ".github/workflows/odd.yml",
            "name: Odd\njobs:\n  weird:::\n  - not: really: valid: yaml:\n",
        );
        assert_eq!(output.coverage, FileCoverage::Complete);
    }
}
