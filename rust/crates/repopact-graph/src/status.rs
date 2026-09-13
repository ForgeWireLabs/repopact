//! Typed graph freshness/capability status (WI063 ROG-010, Decision 0044
//! section 10).

use repopact_repository::Repository;
use serde::{Deserialize, Serialize};

use crate::durable;
use crate::projection::SourceProjection;
use crate::validate::{self, GraphDiagnostic};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// No `rog/manifest.json` exists. The repository remains a fully
    /// valid RepoPact repository (Decision 0044 section 9).
    Absent,
    Fresh,
    /// Reserved for the future watcher/dirty-tree overlay (ROG-013).
    /// Never emitted by this session's implementation.
    WorkingOverlay,
    /// Reserved for a future semantic adapter reporting an explicit
    /// coverage gap while otherwise fresh. Never emitted by this
    /// session's implementation (physical-only coverage is complete by
    /// construction).
    Partial,
    Stale,
    Unsupported,
    Corrupt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatus {
    pub freshness: Freshness,
    pub manifest: Option<durable::Manifest>,
    pub diagnostics: Vec<GraphDiagnosticView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDiagnosticView {
    pub code: String,
    pub message: String,
}

impl From<GraphDiagnostic> for GraphDiagnosticView {
    fn from(diagnostic: GraphDiagnostic) -> Self {
        Self {
            code: diagnostic.code,
            message: diagnostic.message,
        }
    }
}

/// Compute current graph status for `repository`. This recomputes the
/// source projection fingerprint (a full projection walk) when the
/// durable graph is otherwise structurally sound, in order to detect
/// staleness -- callers that only need structural validity without a
/// fresh walk should use [`crate::validate::validate_structure`] directly.
pub fn status(repository: &Repository) -> GraphStatus {
    let diagnostics = validate::validate_structure(repository.root());
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "graph.absent")
    {
        return GraphStatus {
            freshness: Freshness::Absent,
            manifest: None,
            diagnostics: Vec::new(),
        };
    }
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "graph.schema-unsupported")
    {
        return GraphStatus {
            freshness: Freshness::Unsupported,
            manifest: durable::read_manifest(repository.root()).ok().flatten(),
            diagnostics: diagnostics
                .into_iter()
                .map(GraphDiagnosticView::from)
                .collect(),
        };
    }
    if !diagnostics.is_empty() {
        return GraphStatus {
            freshness: Freshness::Corrupt,
            manifest: durable::read_manifest(repository.root()).ok().flatten(),
            diagnostics: diagnostics
                .into_iter()
                .map(GraphDiagnosticView::from)
                .collect(),
        };
    }

    let manifest = match durable::read_manifest(repository.root()) {
        Ok(Some(manifest)) => manifest,
        _ => {
            return GraphStatus {
                freshness: Freshness::Corrupt,
                manifest: None,
                diagnostics: vec![GraphDiagnosticView {
                    code: "graph.manifest-missing".to_owned(),
                    message: "manifest disappeared between structural checks".to_owned(),
                }],
            }
        }
    };

    let topology = repository.topology();
    let current_fingerprint = SourceProjection::build(repository, &topology).fingerprint();
    let freshness = if current_fingerprint == manifest.source_projection_fingerprint {
        Freshness::Fresh
    } else {
        Freshness::Stale
    };
    GraphStatus {
        freshness,
        manifest: Some(manifest),
        diagnostics: Vec::new(),
    }
}
