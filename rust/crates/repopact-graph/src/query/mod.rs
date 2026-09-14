//! Typed, bounded repository graph query and orientation surface (WI063
//! ROG-023 through ROG-026, Decision 0050). This is a pure query kernel:
//! it never mutates [`RepositoryGraph`], never writes `rog/`, never
//! invokes Git, never opens source files, and never calls any LLM/
//! embedding/network service. It answers bounded questions about facts
//! an existing graph builder already put in the graph -- it is not a
//! second graph builder, and a query result grants no mutation,
//! governance, or lifecycle authority (ROG-037/040).
//!
//! The same kernel operates over either an already-loaded durable
//! [`RepositoryGraph`] or a session's [`crate::overlay::SessionGraphState`]
//! effective graph -- never a second, overlay-specific implementation
//! (Decision 0050 section 8). Every traversal builds a disposable,
//! in-memory [`index::GraphQueryIndex`] fresh per call: non-durable,
//! non-authoritative, never `rog/`-adjacent.

mod bounds;
mod cursor;
mod dto;
mod engine;
mod index;
mod open;
mod resolve;
mod selector;

pub use bounds::{
    QueryBounds, DEFAULT_MAX_DEPTH, DEFAULT_MAX_EDGES, DEFAULT_MAX_NODES, DEFAULT_MAX_OUTPUT_BYTES,
    DEFAULT_PAGE_SIZE, MAX_CURSOR_LEN,
};
pub use cursor::CursorError;
pub use dto::{
    ContextResult, DependenciesResult, DependentsResult, Direction, FactRef, GovernanceResult,
    HintReason, ImpactResult, NavigationHint, NeighborsResult, OrientOutcome, OrientResult,
    PathOutcome, PathResult, QueryEnvelope, RelationFact, RelationProvenance, ResolutionOutcome,
    TestsResult,
};
pub use engine::GraphQueryEngine;
pub use open::{open_durable_graph, GraphQueryContext, LoadedGraph, QueryOpenError};
pub use selector::{
    validate_repository_relative_path, NodeSelector, PathSelectorError, SymbolSelector,
};

/// The query protocol contract version. Independent of
/// [`crate::durable::CURRENT_GRAPH_SCHEMA_VERSION`] (Decision 0050
/// section 11) -- a new query field, bound, or operation bumps this, not
/// the durable schema major.
pub const QUERY_CONTRACT_VERSION: u32 = 1;

#[cfg(test)]
mod tests;
