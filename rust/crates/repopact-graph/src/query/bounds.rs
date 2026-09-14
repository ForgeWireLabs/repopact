//! Shared, reusable query bounds (Decision 0050 section 4). Every
//! listing/traversal operation takes the same [`QueryBounds`] type with
//! centralized, conservative defaults -- no operation walks the graph
//! without a bound.

use serde::{Deserialize, Serialize};

use crate::{GraphEdgeKind, GraphLayer};

pub const DEFAULT_MAX_NODES: usize = 200;
pub const DEFAULT_MAX_EDGES: usize = 400;
pub const DEFAULT_MAX_DEPTH: usize = 6;
/// A hard, authoritative, serialized-UTF-8 byte budget (Decision 0050
/// section 6). 256 KiB comfortably fits a typical orientation result
/// while remaining far short of a full graph dump.
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 262_144;
pub const DEFAULT_PAGE_SIZE: usize = 50;
/// An opaque cursor is bounded in length so it can never smuggle
/// arbitrarily large state (Decision 0050 section 5/16).
pub const MAX_CURSOR_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryBounds {
    #[serde(default = "default_max_nodes")]
    pub max_nodes: usize,
    #[serde(default = "default_max_edges")]
    pub max_edges: usize,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    /// Restrict traversal/listing to these layers only, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<GraphLayer>>,
    /// Restrict traversal/listing to these edge kinds only, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_kinds: Option<Vec<GraphEdgeKind>>,
    #[serde(default = "default_max_output_bytes")]
    pub max_output_bytes: usize,
    /// An explicitly named, documented, deterministic estimate -- never a
    /// real provider tokenizer (Decision 0050 section 6). The byte budget
    /// above always remains authoritative if the two disagree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_token_budget: Option<usize>,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// Compact / source-reference-only mode (ROG-025 step 41): omit
    /// nonessential labels/presentation details, retain stable IDs,
    /// source references, relation kinds/roles, and freshness/coverage.
    #[serde(default)]
    pub compact: bool,
    /// Explicitly allow querying a stale durable graph (Decision 0050
    /// section 7). Ignored for overlay queries, which have no separate
    /// staleness concept beyond `DurableFreshness`.
    #[serde(default)]
    pub allow_stale: bool,
}

fn default_max_nodes() -> usize {
    DEFAULT_MAX_NODES
}
fn default_max_edges() -> usize {
    DEFAULT_MAX_EDGES
}
fn default_max_depth() -> usize {
    DEFAULT_MAX_DEPTH
}
fn default_max_output_bytes() -> usize {
    DEFAULT_MAX_OUTPUT_BYTES
}
fn default_page_size() -> usize {
    DEFAULT_PAGE_SIZE
}

impl Default for QueryBounds {
    fn default() -> Self {
        Self {
            max_nodes: DEFAULT_MAX_NODES,
            max_edges: DEFAULT_MAX_EDGES,
            max_depth: DEFAULT_MAX_DEPTH,
            layers: None,
            relation_kinds: None,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
            estimated_token_budget: None,
            page_size: DEFAULT_PAGE_SIZE,
            cursor: None,
            compact: false,
            allow_stale: false,
        }
    }
}

impl QueryBounds {
    /// A deterministic, documented estimate: roughly 4 UTF-8 bytes per
    /// token for typical JSON/source text. This is intentionally crude
    /// and disclosed as an estimate, never a real tokenizer result.
    pub fn estimate_tokens(byte_len: usize) -> usize {
        byte_len.div_ceil(4)
    }

    /// Whether `edge` passes this bound's layer/relation-kind filters.
    pub fn allows_layer(&self, layer: GraphLayer) -> bool {
        self.layers
            .as_ref()
            .map_or(true, |layers| layers.contains(&layer))
    }

    pub fn allows_relation_kind(&self, kind: GraphEdgeKind) -> bool {
        self.relation_kinds
            .as_ref()
            .map_or(true, |kinds| kinds.contains(&kind))
    }
}
