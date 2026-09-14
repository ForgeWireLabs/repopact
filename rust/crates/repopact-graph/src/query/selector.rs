//! Typed target selectors (Decision 0050 section 2). Resolution never
//! depends on a fuzzy string parser -- a caller names exactly what kind
//! of thing it means.

use serde::{Deserialize, Serialize};

use crate::SymbolKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum NodeSelector {
    /// An exact, already-known graph node ID (e.g. `"file:src/lib.rs"`,
    /// `"work:063"`). Never a partial/prefix match.
    NodeId(String),
    /// A repository-relative source path (e.g. `"rust/crates/
    /// repopact-graph/src/lib.rs"`). Validated by
    /// [`validate_repository_relative_path`] -- an absolute host path or
    /// a `..` escape is rejected as malformed input, never silently
    /// normalized.
    RepositoryPath(String),
    /// A governance work-item ID (e.g. `"063"`).
    WorkItemId(String),
    /// A package/crate name as it appears in a manifest identity label
    /// (e.g. `"repopact-graph"`, an npm package name, a Python project
    /// name).
    Package(String),
    /// A module name (a `SymbolKind::Module` symbol's label, e.g. a
    /// Python module or a namespace-like construct).
    Module(String),
    /// A source symbol, optionally disambiguated by path/kind/container.
    Symbol(SymbolSelector),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolSelector {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_kind: Option<SymbolKind>,
    /// The symbol's qualified/container name (e.g. the enclosing
    /// class/module), matched as a best-effort substring of the symbol's
    /// own stable ID -- a disambiguation filter, not a claim of exact
    /// structural parsing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSelectorError {
    Empty,
    Absolute,
    ParentEscape,
    Malformed,
}

impl std::fmt::Display for PathSelectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Empty => "path selector must not be empty",
            Self::Absolute => {
                "path selector must be repository-relative, not an absolute host path"
            }
            Self::ParentEscape => "path selector must not contain a '..' component",
            Self::Malformed => "path selector contains an invalid character sequence",
        };
        write!(f, "{message}")
    }
}

/// Validate and normalize a caller-supplied repository-relative path
/// selector (Decision 0050 section 2/37). Rejects absolute host paths
/// (a leading `/`, a Windows drive letter, or a UNC prefix), any `..`
/// component, an empty string, and backslashes (every path stored in the
/// graph is forward-slash-separated, platform-independent -- Decision
/// 0044). Returns the normalized, forward-slash, non-trailing-slash
/// form.
pub fn validate_repository_relative_path(raw: &str) -> Result<String, PathSelectorError> {
    if raw.is_empty() {
        return Err(PathSelectorError::Empty);
    }
    if raw.contains('\\') {
        return Err(PathSelectorError::Malformed);
    }
    if raw.starts_with('/') {
        return Err(PathSelectorError::Absolute);
    }
    // A Windows drive letter (`C:...`) or UNC-style prefix.
    let mut chars = raw.chars();
    if let (Some(first), Some(second)) = (chars.next(), chars.next()) {
        if first.is_ascii_alphabetic() && second == ':' {
            return Err(PathSelectorError::Absolute);
        }
    }
    let trimmed = raw.trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(PathSelectorError::Empty);
    }
    for segment in trimmed.split('/') {
        if segment == ".." {
            return Err(PathSelectorError::ParentEscape);
        }
        if segment.is_empty() {
            // A doubled slash (`a//b`) is malformed, not silently
            // collapsed.
            return Err(PathSelectorError::Malformed);
        }
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_relative_paths_are_accepted() {
        assert_eq!(
            validate_repository_relative_path("rust/crates/repopact-graph/src/lib.rs"),
            Ok("rust/crates/repopact-graph/src/lib.rs".to_owned())
        );
        assert_eq!(
            validate_repository_relative_path("src/lib.rs/"),
            Ok("src/lib.rs".to_owned())
        );
    }

    #[test]
    fn absolute_host_paths_are_rejected() {
        assert_eq!(
            validate_repository_relative_path("/etc/passwd"),
            Err(PathSelectorError::Absolute)
        );
        assert_eq!(
            validate_repository_relative_path("C:/Projects/repopact/src/lib.rs"),
            Err(PathSelectorError::Absolute)
        );
    }

    #[test]
    fn parent_escapes_are_rejected() {
        assert_eq!(
            validate_repository_relative_path("../outside/lib.rs"),
            Err(PathSelectorError::ParentEscape)
        );
        assert_eq!(
            validate_repository_relative_path("src/../../lib.rs"),
            Err(PathSelectorError::ParentEscape)
        );
    }

    #[test]
    fn empty_and_malformed_paths_are_rejected() {
        assert_eq!(
            validate_repository_relative_path(""),
            Err(PathSelectorError::Empty)
        );
        assert_eq!(
            validate_repository_relative_path("src\\lib.rs"),
            Err(PathSelectorError::Malformed)
        );
        assert_eq!(
            validate_repository_relative_path("src//lib.rs"),
            Err(PathSelectorError::Malformed)
        );
    }
}
