//! Deterministic semantic extraction (WI063 semantic-adapter checkpoint,
//! Decision 0045). Parser-neutral orchestrator + adapter boundary: no
//! Tree-sitter type crosses the [`SemanticAdapter`] trait boundary or
//! appears in any canonical graph DTO. Adapters receive already-loaded,
//! already-bounded content; they perform no repository I/O, Git
//! invocation, or symlink traversal themselves (Decision 0045 section 7).

mod javascript_adapter;
mod python_adapter;
mod rust_adapter;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use repopact_repository::Repository;
use serde::{Deserialize, Serialize};

use crate::projection::SourceProjection;
use crate::{GraphEdge, GraphNode, RepositoryGraph};

/// A language this checkpoint can identify by file extension. Adapters are
/// selected by this identity, never by adapters independently sniffing or
/// walking anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLanguage {
    Rust,
    Python,
    JavaScript,
    Jsx,
    TypeScript,
    Tsx,
    // Structured metadata (WI063 metadata/operational-topology checkpoint,
    // Decision 0048; schema v3). These are not programming languages with
    // symbol/scope semantics -- they are dispatched to a MetadataAdapter
    // (`crate::metadata`) rather than a Tree-sitter SemanticAdapter, but
    // share the same classification/coverage machinery so they never fall
    // into `SkipReason::UnsupportedLanguage` merely for lacking a
    // tree-sitter grammar.
    Json,
    Toml,
    Yaml,
    Markdown,
}

impl SourceLanguage {
    /// Identify a language by file extension only (ROG-021: this is a
    /// classification decision, not a parse attempt). Returns `None` for
    /// every extension this checkpoint does not adapt -- those files are
    /// reported as `SkipReason::UnsupportedLanguage` coverage, never
    /// silently ignored.
    pub fn from_extension(relative_path: &str) -> Option<Self> {
        let extension = relative_path.rsplit('.').next()?.to_ascii_lowercase();
        match extension.as_str() {
            "rs" => Some(Self::Rust),
            "py" | "pyi" => Some(Self::Python),
            "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            "jsx" => Some(Self::Jsx),
            "ts" | "mts" | "cts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            "yml" | "yaml" => Some(Self::Yaml),
            "md" | "markdown" => Some(Self::Markdown),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::Jsx => "jsx",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::Json => "json",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Markdown => "markdown",
        }
    }

    /// Whether this language is dispatched to a `MetadataAdapter`
    /// (`crate::metadata`) rather than a Tree-sitter `SemanticAdapter`.
    pub fn is_metadata(self) -> bool {
        matches!(self, Self::Json | Self::Toml | Self::Yaml | Self::Markdown)
    }
}

/// ROG-022 resource policy. Conservative starting constants, deliberately
/// centralized here rather than scattered per adapter, chosen without
/// production measurement evidence (none exists yet for this checkpoint)
/// but easy to revisit centrally once real measurements (see
/// `implementation-progress.md`) suggest otherwise.
#[derive(Debug, Clone, Copy)]
pub struct ResourcePolicy {
    /// Files larger than this are skipped by policy, not parsed. 1 MiB
    /// comfortably covers real source files; a legitimate source file this
    /// large is exceptionally rare and more likely generated/vendored
    /// content this checkpoint would want to exclude anyway.
    pub max_file_bytes: usize,
    /// Wall-clock budget per file parse, enforced via Tree-sitter's
    /// `parse_with_options` progress callback (Decision 0045 section 1),
    /// not the removed `set_timeout_micros` API.
    pub max_parse_duration: Duration,
}

impl Default for ResourcePolicy {
    fn default() -> Self {
        Self {
            max_file_bytes: 1_048_576,
            max_parse_duration: Duration::from_secs(2),
        }
    }
}

/// A file's semantic coverage outcome (ROG-020/023): unsupported language,
/// excluded/oversized/binary, complete, partial, or genuinely failed are
/// kept as distinct values -- never collapsed into one "skipped" bucket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum FileCoverage {
    Complete,
    /// Parsed, but at least one region failed (syntax error the grammar's
    /// error-recovery could not fully resolve, or cancellation mid-parse).
    /// `nodes_emitted`/`edges_emitted` still reflect real, valid partial
    /// output -- this is not corruption.
    Partial {
        reason: String,
    },
    Skipped {
        reason: SkipReason,
    },
    Failed {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    UnsupportedLanguage,
    OversizedFile,
    BinaryContent,
    /// WI063 ROG-022, Decision 0048: content classified as minified by a
    /// deterministic, centralized heuristic (`metadata::policy`) -- a
    /// distinct, named skip reason rather than being folded into
    /// `OversizedFile`/`BinaryContent`.
    MinifiedContent,
    /// WI063 ROG-022, Decision 0048: content classified as generated by a
    /// deterministic, centralized heuristic (a recognized generator
    /// header/marker) -- a distinct, named skip reason so coverage can
    /// truthfully say what happened rather than silently discarding it.
    GeneratedContent,
    /// WI063 ROG-018, Decision 0048: the file's language is a recognized
    /// metadata format (JSON/TOML/YAML), but this specific file does not
    /// match any manifest shape this checkpoint extracts facts from (e.g.
    /// an arbitrary `.json` data file that is not `package.json`/
    /// `tsconfig*.json`). Distinct from `UnsupportedLanguage`: the
    /// *language* is supported, this particular file's *schema* is not
    /// recognized -- arbitrary JSON/TOML/YAML keys are never flattened
    /// into graph semantics merely because the extension matched.
    UnrecognizedMetadataSchema,
}

/// One file's semantic-extraction result, reported in the durable coverage
/// summary regardless of outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCoverageEntry {
    pub relative_path: String,
    pub language: Option<SourceLanguage>,
    pub coverage: FileCoverage,
    pub nodes_emitted: usize,
    pub edges_emitted: usize,
    /// The `SourceProjection` content digest this entry was computed
    /// against (WI063 incremental-equivalence checkpoint, Decision 0046).
    /// `None` only for a durable graph written before this field existed;
    /// an old schema-v2 graph without per-file digests cannot be diffed
    /// for incremental reuse and must fall back to a full rebuild -- see
    /// `crate::incremental`. Additive and optional so a pre-existing v2
    /// reader that does not know this field still deserializes cleanly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_digest: Option<String>,
}

/// Bounded, already-approved input handed to an adapter. Adapters do not
/// look beyond this -- no directory walk, no Git call, no reopening a
/// different path (Decision 0045 section 7).
pub struct SourceInput<'a> {
    pub relative_path: &'a str,
    pub content: &'a [u8],
    pub resource_policy: &'a ResourcePolicy,
}

/// What an adapter contributes for one file: normalized graph content plus
/// its own coverage verdict. No Tree-sitter type appears here.
pub struct AdapterOutput {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub coverage: FileCoverage,
}

/// The parser-neutral adapter boundary (Decision 0045 section 6). A
/// future non-Tree-sitter adapter (or a higher-fidelity language-specific
/// tool) implements this same trait without any canonical graph DTO
/// changing.
pub trait SemanticAdapter {
    fn language(&self) -> SourceLanguage;
    fn extract(&self, input: &SourceInput) -> AdapterOutput;
}

fn is_probably_binary(content: &[u8]) -> bool {
    content.iter().take(8192).any(|byte| *byte == 0)
}

fn adapter_for(language: SourceLanguage) -> Box<dyn SemanticAdapter> {
    match language {
        SourceLanguage::Rust => Box::new(rust_adapter::RustAdapter),
        SourceLanguage::Python => Box::new(python_adapter::PythonAdapter),
        SourceLanguage::JavaScript | SourceLanguage::Jsx => {
            Box::new(javascript_adapter::JavaScriptFamilyAdapter::new(language))
        }
        SourceLanguage::TypeScript | SourceLanguage::Tsx => {
            Box::new(javascript_adapter::JavaScriptFamilyAdapter::new(language))
        }
        SourceLanguage::Json
        | SourceLanguage::Toml
        | SourceLanguage::Yaml
        | SourceLanguage::Markdown => {
            unreachable!(
                "metadata languages are dispatched to crate::metadata::extract, \
                 never to a SemanticAdapter -- build_file_contribution checks \
                 language.is_metadata() before ever calling adapter_for"
            )
        }
    }
}

/// Coverage summary recorded in the durable manifest (ROG-020/021/023).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticCoverage {
    pub adapter_versions: BTreeMap<String, String>,
    pub files_considered: usize,
    pub files_complete: usize,
    pub files_partial: usize,
    pub files_skipped_unsupported_language: usize,
    pub files_skipped_policy: usize,
    pub files_failed: usize,
    pub relations_supported: BTreeMap<String, Vec<String>>,
    pub per_file: Vec<FileCoverageEntry>,
}

/// One file's deterministic semantic contribution to the graph (WI063
/// incremental-equivalence checkpoint, ROG-012). This is the single
/// contribution-generation primitive: a clean full build calls it for
/// every projected file, and an incremental update
/// (`crate::incremental::update`) calls it only for added/modified files,
/// reusing a prior contribution unchanged for everything else. There is
/// deliberately no second, parallel per-file extraction path -- if full
/// build and incremental update did not both funnel through this one
/// function, byte-equivalence between them would be a coincidence rather
/// than a structural guarantee.
pub struct FileContribution {
    pub entry: FileCoverageEntry,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Build the semantic contribution for exactly one already-approved
/// projected file. `digest` is the `SourceProjection` content digest for
/// this file, recorded on the resulting entry so later incremental runs
/// can detect whether the file has changed without reparsing it. Reads
/// the file's content itself (through `repository.root()`, the same
/// containment the projection was already built under) -- no new
/// discovery, no path re-resolution beyond what the projection already
/// approved.
pub fn build_file_contribution(
    repository: &Repository,
    relative_path: &str,
    digest: &str,
    policy: &ResourcePolicy,
) -> FileContribution {
    let Some(language) = SourceLanguage::from_extension(relative_path) else {
        return FileContribution {
            entry: FileCoverageEntry {
                relative_path: relative_path.to_owned(),
                language: None,
                coverage: FileCoverage::Skipped {
                    reason: SkipReason::UnsupportedLanguage,
                },
                nodes_emitted: 0,
                edges_emitted: 0,
                source_digest: Some(digest.to_owned()),
            },
            nodes: Vec::new(),
            edges: Vec::new(),
        };
    };

    let absolute = repository.root().join(relative_path);
    let content = match std::fs::read(&absolute) {
        Ok(bytes) => bytes,
        Err(error) => {
            return FileContribution {
                entry: FileCoverageEntry {
                    relative_path: relative_path.to_owned(),
                    language: Some(language),
                    coverage: FileCoverage::Failed {
                        reason: error.to_string(),
                    },
                    nodes_emitted: 0,
                    edges_emitted: 0,
                    source_digest: Some(digest.to_owned()),
                },
                nodes: Vec::new(),
                edges: Vec::new(),
            };
        }
    };

    if content.len() > policy.max_file_bytes {
        return FileContribution {
            entry: FileCoverageEntry {
                relative_path: relative_path.to_owned(),
                language: Some(language),
                coverage: FileCoverage::Skipped {
                    reason: SkipReason::OversizedFile,
                },
                nodes_emitted: 0,
                edges_emitted: 0,
                source_digest: Some(digest.to_owned()),
            },
            nodes: Vec::new(),
            edges: Vec::new(),
        };
    }
    if is_probably_binary(&content) {
        return FileContribution {
            entry: FileCoverageEntry {
                relative_path: relative_path.to_owned(),
                language: Some(language),
                coverage: FileCoverage::Skipped {
                    reason: SkipReason::BinaryContent,
                },
                nodes_emitted: 0,
                edges_emitted: 0,
                source_digest: Some(digest.to_owned()),
            },
            nodes: Vec::new(),
            edges: Vec::new(),
        };
    }
    // WI063 ROG-022, Decision 0048: minified/generated classification
    // applies uniformly to every language (source and metadata alike),
    // ahead of adapter dispatch -- these are policy skips, not adapter
    // failures, and are reported as distinct, named coverage states.
    if crate::metadata::policy::is_probably_minified(&content) {
        return FileContribution {
            entry: FileCoverageEntry {
                relative_path: relative_path.to_owned(),
                language: Some(language),
                coverage: FileCoverage::Skipped {
                    reason: SkipReason::MinifiedContent,
                },
                nodes_emitted: 0,
                edges_emitted: 0,
                source_digest: Some(digest.to_owned()),
            },
            nodes: Vec::new(),
            edges: Vec::new(),
        };
    }
    if crate::metadata::policy::is_probably_generated(&content) {
        return FileContribution {
            entry: FileCoverageEntry {
                relative_path: relative_path.to_owned(),
                language: Some(language),
                coverage: FileCoverage::Skipped {
                    reason: SkipReason::GeneratedContent,
                },
                nodes_emitted: 0,
                edges_emitted: 0,
                source_digest: Some(digest.to_owned()),
            },
            nodes: Vec::new(),
            edges: Vec::new(),
        };
    }

    let input = SourceInput {
        relative_path,
        content: &content,
        resource_policy: policy,
    };
    let started = Instant::now();
    let output = if language.is_metadata() {
        crate::metadata::extract(language, &input)
    } else {
        let adapter = adapter_for(language);
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| adapter.extract(&input)));
        match result {
            Ok(output) => output,
            Err(_) => AdapterOutput {
                nodes: Vec::new(),
                edges: Vec::new(),
                coverage: FileCoverage::Failed {
                    reason: "adapter panicked while parsing this file".to_owned(),
                },
            },
        }
    };
    let _elapsed = started.elapsed();

    let node_count = output.nodes.len();
    let edge_count = output.edges.len();
    FileContribution {
        entry: FileCoverageEntry {
            relative_path: relative_path.to_owned(),
            language: Some(language),
            coverage: output.coverage,
            nodes_emitted: node_count,
            edges_emitted: edge_count,
            source_digest: Some(digest.to_owned()),
        },
        nodes: output.nodes,
        edges: output.edges,
    }
}

/// Fixed adapter-identity/relation-taxonomy metadata (independent of any
/// particular file). Shared by the full build and by incremental update
/// so both populate `SemanticCoverage.adapter_versions`/
/// `relations_supported` identically.
pub fn adapter_metadata() -> (BTreeMap<String, String>, BTreeMap<String, Vec<String>>) {
    let mut adapter_versions = BTreeMap::new();
    adapter_versions.insert("rust".to_owned(), rust_adapter::ADAPTER_VERSION.to_owned());
    adapter_versions.insert(
        "python".to_owned(),
        python_adapter::ADAPTER_VERSION.to_owned(),
    );
    adapter_versions.insert(
        "javascript_typescript".to_owned(),
        javascript_adapter::ADAPTER_VERSION.to_owned(),
    );
    let mut relations_supported = BTreeMap::new();
    relations_supported.insert(
        "rust".to_owned(),
        rust_adapter::SUPPORTED_RELATIONS
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    relations_supported.insert(
        "python".to_owned(),
        python_adapter::SUPPORTED_RELATIONS
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    relations_supported.insert(
        "javascript_typescript".to_owned(),
        javascript_adapter::SUPPORTED_RELATIONS
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    (adapter_versions, relations_supported)
}

/// Recompute the aggregate `SemanticCoverage` counters deterministically
/// from a final per-file inventory (WI063 incremental-equivalence
/// checkpoint, step 14). Deliberately not incremental increment/decrement
/// bookkeeping: a full build and an incremental update both end with some
/// final `Vec<FileCoverageEntry>` (assembled differently, one entry per
/// file either way) and must derive numerically identical aggregates from
/// it by the same counting rule, or a coverage-equality proof between the
/// two paths would depend on bookkeeping happening to stay in sync rather
/// than on this single shared computation.
pub fn aggregate_coverage(per_file: Vec<FileCoverageEntry>) -> SemanticCoverage {
    let (adapter_versions, relations_supported) = adapter_metadata();
    let mut coverage = SemanticCoverage {
        adapter_versions,
        relations_supported,
        ..SemanticCoverage::default()
    };
    for entry in &per_file {
        coverage.files_considered += 1;
        match &entry.coverage {
            FileCoverage::Complete => coverage.files_complete += 1,
            FileCoverage::Partial { .. } => coverage.files_partial += 1,
            FileCoverage::Failed { .. } => coverage.files_failed += 1,
            FileCoverage::Skipped {
                reason: SkipReason::UnsupportedLanguage,
            } => coverage.files_skipped_unsupported_language += 1,
            FileCoverage::Skipped { .. } => coverage.files_skipped_policy += 1,
        }
    }
    coverage.per_file = per_file;
    coverage
}

/// Run a full semantic extraction over every one of `projection`'s files,
/// appending `GraphLayer::Semantic` nodes/edges to `graph` and returning
/// the coverage summary. This is the correctness oracle (Decision 0046):
/// it calls [`build_file_contribution`] for every file unconditionally,
/// never reusing a prior result, so its output is always ground truth to
/// compare an incremental update against.
pub fn extend(
    graph: &mut RepositoryGraph,
    repository: &Repository,
    projection: &SourceProjection,
) -> SemanticCoverage {
    let policy = ResourcePolicy::default();
    let mut per_file = Vec::with_capacity(projection.files.len());
    for file in &projection.files {
        let contribution =
            build_file_contribution(repository, &file.relative_path, &file.digest, &policy);
        for node in contribution.nodes {
            graph.node(node);
        }
        for edge in contribution.edges {
            graph.edge(edge);
        }
        per_file.push(contribution.entry);
    }
    aggregate_coverage(per_file)
}

pub(crate) fn node_id_for_symbol(
    language: SourceLanguage,
    relative_path: &str,
    container: &str,
    kind_tag: &str,
    name: &str,
) -> String {
    if container.is_empty() {
        format!(
            "symbol:{}:{relative_path}:{kind_tag}:{name}",
            language.label()
        )
    } else {
        format!(
            "symbol:{}:{relative_path}:{container}:{kind_tag}:{name}",
            language.label()
        )
    }
}

pub(crate) fn location_from_span(
    start_byte: usize,
    end_byte: usize,
    start_row: usize,
    start_column: usize,
    end_row: usize,
    end_column: usize,
) -> crate::GraphSourceLocation {
    crate::GraphSourceLocation {
        start_byte,
        end_byte,
        start_row,
        start_column,
        end_row,
        end_column,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use repopact_repository::Repository;

    use super::*;
    use crate::{status, RepositoryGraph};

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("repopact-semantic-{name}-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn unsupported_language_is_a_distinct_coverage_state_not_silently_skipped() {
        // `.md` is now a recognized metadata language (WI063 metadata/
        // operational-topology checkpoint) -- use a genuinely
        // unrecognized extension for this fixture instead.
        let root = temp_root("unsupported-language");
        std::fs::write(root.join("notes.xyz"), "# notes\n").unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let (_, _, coverage) = RepositoryGraph::build_with_fingerprint(&snapshot);
        assert_eq!(coverage.files_skipped_unsupported_language, 1);
        let entry = coverage
            .per_file
            .iter()
            .find(|e| e.relative_path == "notes.xyz")
            .unwrap();
        assert_eq!(
            entry.coverage,
            FileCoverage::Skipped {
                reason: SkipReason::UnsupportedLanguage
            }
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn oversized_file_is_skipped_by_policy_not_parsed() {
        let root = temp_root("oversized");
        let big = "fn f() {}\n".repeat(200_000); // well over the 1 MiB default policy
        std::fs::write(root.join("big.rs"), &big).unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let (_, _, coverage) = RepositoryGraph::build_with_fingerprint(&snapshot);
        let entry = coverage
            .per_file
            .iter()
            .find(|e| e.relative_path == "big.rs")
            .unwrap();
        assert_eq!(
            entry.coverage,
            FileCoverage::Skipped {
                reason: SkipReason::OversizedFile
            }
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn binary_content_with_a_source_extension_is_skipped_not_parsed() {
        let root = temp_root("binary-content");
        let mut bytes = b"fn f".to_vec();
        bytes.extend_from_slice(&[0u8, 1, 2, 3]);
        std::fs::write(root.join("weird.rs"), &bytes).unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let (_, _, coverage) = RepositoryGraph::build_with_fingerprint(&snapshot);
        let entry = coverage
            .per_file
            .iter()
            .find(|e| e.relative_path == "weird.rs")
            .unwrap();
        assert_eq!(
            entry.coverage,
            FileCoverage::Skipped {
                reason: SkipReason::BinaryContent
            }
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn one_malformed_file_does_not_destroy_valid_output_from_unrelated_files() {
        let root = temp_root("failure-isolation");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/good.rs"), "pub fn good() {}\n").unwrap();
        std::fs::write(root.join("src/bad.rs"), "fn f( { totally not valid rust\n").unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let (graph, _, coverage) = RepositoryGraph::build_with_fingerprint(&snapshot);

        assert!(
            graph.nodes.values().any(|node| node.label == "good"),
            "valid content from an unrelated file must still appear in the graph"
        );
        let bad_entry = coverage
            .per_file
            .iter()
            .find(|e| e.relative_path == "src/bad.rs")
            .unwrap();
        assert!(matches!(bad_entry.coverage, FileCoverage::Partial { .. }));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_corrected_full_rebuild_clears_a_prior_partial_coverage_gap() {
        let root = temp_root("failure-then-fix");
        std::fs::write(root.join("mixed.rs"), "fn f( { not valid\n").unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let (_, _, first_coverage) = RepositoryGraph::build_with_fingerprint(&snapshot);
        assert!(first_coverage.files_partial >= 1);

        std::fs::write(root.join("mixed.rs"), "fn f() {}\n").unwrap();
        let snapshot = repository.session().snapshot();
        let (_, _, second_coverage) = RepositoryGraph::build_with_fingerprint(&snapshot);
        assert_eq!(second_coverage.files_partial, 0);
        assert_eq!(second_coverage.files_complete, 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn freshness_becomes_partial_on_a_genuine_supported_file_coverage_gap() {
        // A malformed *supported-language* file is a real gap: the
        // repository-wide freshness must say so explicitly (Partial),
        // never silently collapse into Fresh.
        let root = temp_root("partial-on-malformed");
        std::fs::write(root.join("bad.rs"), "fn f( { not valid\n").unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        crate::build_and_write(&snapshot).expect("build");
        let result = status::status(&repository);
        assert_eq!(result.freshness, status::Freshness::Partial);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unsupported_language_or_policy_exclusion_alone_does_not_make_the_graph_partial() {
        // An unsupported-language file or a policy-excluded file is an
        // expected, intentional non-coverage state, not a gap in what
        // the graph should have covered -- Fresh remains correct even
        // though semantic coverage is incomplete by design. (`.md` is
        // now a recognized metadata language -- use a genuinely
        // unrecognized extension here instead.)
        let root = temp_root("fresh-despite-unsupported-language");
        std::fs::write(root.join("notes.xyz"), "# notes\n").unwrap();
        std::fs::write(root.join("good.rs"), "fn good() {}\n").unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        crate::build_and_write(&snapshot).expect("build");
        let result = status::status(&repository);
        assert_eq!(result.freshness, status::Freshness::Fresh);
        std::fs::remove_dir_all(root).unwrap();
    }
}
