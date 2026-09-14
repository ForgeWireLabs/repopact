//! Durable-graph query freshness gate (Decision 0050 section 7). A
//! stale durable graph is never silently queried as if current; an
//! unsupported/corrupt graph fails closed before any query runs.

use std::path::Path;

use repopact_repository::Repository;

use crate::capability::CapabilityState;
use crate::overlay::{DurableFreshness, EffectiveGraphStatus, GraphBasis, GraphCoverageState};
use crate::semantic::SemanticCoverage;
use crate::status::{self, Freshness};
use crate::{durable, RepositoryGraph};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryOpenError {
    /// No durable graph exists yet (`LegacyAbsent` or `ExplicitDisabled`
    /// capability -- both mean "no graph, valid").
    Absent,
    /// The durable graph declares a schema major this build does not
    /// support.
    Unsupported,
    /// The durable graph failed structural validation.
    Corrupt,
    /// The durable graph is stale (source has changed since the last
    /// build) and the caller did not pass `allow_stale`.
    Stale { graph_fingerprint: String },
    /// ROG-039: capability declares `rog=enabled` but no durable graph
    /// exists. Never conflated with [`Self::Absent`] -- this is a hard,
    /// binding failure, not "no graph, valid."
    EnabledButMissing,
}

impl std::fmt::Display for QueryOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Absent => write!(
                f,
                "no durable graph exists; run `repopact graph build` first"
            ),
            Self::Unsupported => write!(
                f,
                "durable graph schema version is unsupported by this build"
            ),
            Self::Corrupt => write!(f, "durable graph failed structural validation"),
            Self::Stale { graph_fingerprint } => write!(
                f,
                "durable graph (fingerprint {graph_fingerprint}) is stale; \
                 pass allow_stale=true to query it anyway, or run `repopact graph update`"
            ),
            Self::EnabledButMissing => write!(
                f,
                "capability declares rog=enabled but no durable graph exists; \
                 run `repopact graph build` or `repopact graph disable`"
            ),
        }
    }
}

/// Everything a [`super::GraphQueryEngine`] result envelope needs to
/// disclose about the graph state it queried, independent of whether
/// that graph came from the durable baseline or a session's working
/// overlay (Decision 0050 section 8).
#[derive(Debug, Clone)]
pub struct GraphQueryContext {
    pub graph_schema_version: u32,
    pub graph_fingerprint: String,
    pub status: EffectiveGraphStatus,
    pub semantic_coverage: SemanticCoverage,
    pub capability_state: CapabilityState,
}

#[derive(Debug)]
pub struct LoadedGraph {
    pub graph: RepositoryGraph,
    pub context: GraphQueryContext,
}

/// Open the durable graph at `root/rog/` for querying, applying the
/// stale-by-default freshness gate. `allow_stale` lets a caller who
/// knowingly wants a stale answer proceed anyway -- the returned
/// context's `status.durable_freshness` still always reports `stale`;
/// nothing here or downstream may hide that.
pub fn open_durable_graph(root: &Path, allow_stale: bool) -> Result<LoadedGraph, QueryOpenError> {
    let repository = Repository::open(root);
    let graph_status = status::status(&repository);
    if graph_status.capability_state == CapabilityState::EnabledMissing {
        return Err(QueryOpenError::EnabledButMissing);
    }
    match graph_status.freshness {
        Freshness::Absent => return Err(QueryOpenError::Absent),
        Freshness::Unsupported => return Err(QueryOpenError::Unsupported),
        Freshness::Corrupt => return Err(QueryOpenError::Corrupt),
        Freshness::Stale => {
            if !allow_stale {
                let fingerprint = graph_status
                    .manifest
                    .as_ref()
                    .map(|manifest| manifest.source_projection_fingerprint.clone())
                    .unwrap_or_default();
                return Err(QueryOpenError::Stale {
                    graph_fingerprint: fingerprint,
                });
            }
        }
        Freshness::Fresh | Freshness::Partial | Freshness::WorkingOverlay => {}
    }

    let manifest = graph_status.manifest.ok_or(QueryOpenError::Corrupt)?;
    let graph = durable::load_graph(root, &manifest).map_err(|_error| QueryOpenError::Corrupt)?;

    let durable_freshness = match graph_status.freshness {
        Freshness::Stale => DurableFreshness::Stale,
        _ => DurableFreshness::Fresh,
    };
    let coverage = if matches!(graph_status.freshness, Freshness::Partial)
        || status::has_semantic_coverage_gap(&manifest)
    {
        GraphCoverageState::Partial
    } else {
        GraphCoverageState::Complete
    };
    let fingerprint = manifest.source_projection_fingerprint.clone();
    let context = GraphQueryContext {
        graph_schema_version: manifest.graph_schema_version,
        graph_fingerprint: fingerprint.clone(),
        status: EffectiveGraphStatus {
            basis: GraphBasis::Durable,
            durable_freshness,
            coverage,
            baseline_fingerprint: Some(fingerprint.clone()),
            effective_fingerprint: fingerprint,
            changed_path_count: 0,
            overlay_generation: 0,
        },
        semantic_coverage: manifest.semantic_coverage.clone().unwrap_or_default(),
        capability_state: graph_status.capability_state,
    };
    Ok(LoadedGraph { graph, context })
}
