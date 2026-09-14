//! Deterministic structured-metadata and operational-topology extraction
//! (WI063 metadata/operational-topology checkpoint, Decision 0048).
//!
//! Parallels `crate::semantic`'s `SemanticAdapter` boundary for
//! non-source-language files: JSON/TOML/YAML/Markdown are structured or
//! declarative formats, not programming languages with symbol/scope
//! semantics, so they are dispatched to a `MetadataAdapter` here rather
//! than a Tree-sitter `SemanticAdapter`. Both adapter families are
//! reached from the same per-file classification point
//! (`semantic::build_file_contribution`), so there is exactly one
//! contribution-generation pipeline, never a second parallel one.

mod json_adapter;
mod markdown_adapter;
mod toml_adapter;
mod yaml_adapter;

pub mod policy;

use crate::semantic::{AdapterOutput, SourceInput, SourceLanguage};

/// The parser-neutral metadata adapter boundary (Decision 0048 section
/// 1). No `serde_json`/`toml`/YAML-parser AST type crosses into a
/// canonical graph DTO through this trait -- `extract` returns the same
/// `AdapterOutput` shape `SemanticAdapter` does.
pub trait MetadataAdapter {
    fn identity(&self) -> &'static str;
    fn accepts(&self, relative_path: &str) -> bool;
    fn extract(&self, input: &SourceInput) -> AdapterOutput;
}

pub const SUPPORTED_RELATIONS_JSON: [&str; 2] = ["defines", "depends_on"];
pub const SUPPORTED_RELATIONS_TOML: [&str; 3] = ["defines", "depends_on", "belongs_to_workspace"];
pub const SUPPORTED_RELATIONS_YAML: [&str; 1] = ["defines"];
pub const SUPPORTED_RELATIONS_MARKDOWN: [&str; 1] = ["references"];

fn adapter_for(language: SourceLanguage) -> Option<Box<dyn MetadataAdapter>> {
    match language {
        SourceLanguage::Json => Some(Box::new(json_adapter::JsonAdapter)),
        SourceLanguage::Toml => Some(Box::new(toml_adapter::TomlAdapter)),
        SourceLanguage::Yaml => Some(Box::new(yaml_adapter::YamlAdapter)),
        SourceLanguage::Markdown => Some(Box::new(markdown_adapter::MarkdownAdapter)),
        _ => None,
    }
}

/// Dispatch one already-classified metadata file to its adapter. Called
/// from `semantic::build_file_contribution` after the shared size/binary/
/// minified/generated policy checks (Decision 0048 section on metadata
/// resource policy: structured parsers reuse the exact same bounds as
/// source-language parsers, not a separate, weaker set).
pub fn extract(language: SourceLanguage, input: &SourceInput) -> AdapterOutput {
    let Some(adapter) = adapter_for(language) else {
        return AdapterOutput {
            nodes: Vec::new(),
            edges: Vec::new(),
            coverage: crate::semantic::FileCoverage::Skipped {
                reason: crate::semantic::SkipReason::UnsupportedLanguage,
            },
        };
    };
    let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| adapter.extract(input)));
    match output {
        Ok(output) => output,
        Err(_) => AdapterOutput {
            nodes: Vec::new(),
            edges: Vec::new(),
            coverage: crate::semantic::FileCoverage::Failed {
                reason: "metadata adapter panicked while parsing this file".to_owned(),
            },
        },
    }
}

pub fn adapter_versions() -> std::collections::BTreeMap<String, String> {
    let mut map = std::collections::BTreeMap::new();
    map.insert("json".to_owned(), json_adapter::ADAPTER_VERSION.to_owned());
    map.insert("toml".to_owned(), toml_adapter::ADAPTER_VERSION.to_owned());
    map.insert("yaml".to_owned(), yaml_adapter::ADAPTER_VERSION.to_owned());
    map.insert(
        "markdown".to_owned(),
        markdown_adapter::ADAPTER_VERSION.to_owned(),
    );
    map
}

pub fn relations_supported() -> std::collections::BTreeMap<String, Vec<String>> {
    let mut map = std::collections::BTreeMap::new();
    map.insert(
        "json".to_owned(),
        SUPPORTED_RELATIONS_JSON
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    map.insert(
        "toml".to_owned(),
        SUPPORTED_RELATIONS_TOML
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    map.insert(
        "yaml".to_owned(),
        SUPPORTED_RELATIONS_YAML
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    map.insert(
        "markdown".to_owned(),
        SUPPORTED_RELATIONS_MARKDOWN
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    map
}

/// Deterministic, repository-relative "fact" node/edge id helpers, kept
/// here so every metadata adapter builds ids the same way (mirrors
/// `semantic::node_id_for_symbol`'s role for source symbols).
pub(crate) fn manifest_node_id(relative_path: &str) -> String {
    format!("manifest:{relative_path}")
}

pub(crate) fn manifest_fact_node_id(relative_path: &str, fact_kind: &str, name: &str) -> String {
    format!("manifest-fact:{relative_path}:{fact_kind}:{name}")
}
