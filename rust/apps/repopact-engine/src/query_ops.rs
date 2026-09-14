//! Engine-protocol adapter for the canonical Rust query kernel (WI063
//! ROG-023/024/025/026, Decision 0050). This module owns no traversal
//! logic of its own -- every operation parses request params, opens the
//! durable graph through the shared freshness gate
//! (`repopact_graph::query::open_durable_graph`), and delegates entirely
//! to `repopact_graph::query::GraphQueryEngine`.

use std::path::PathBuf;

use repopact_graph::query::{
    Direction, GraphQueryEngine, NodeSelector, QueryBounds, QueryOpenError,
};
use repopact_protocol::{EngineRequest, EngineResponse};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{require_root, semantic_failure_closed, ENGINE_VERSION};

#[derive(Debug, Deserialize)]
struct ResolveParams {
    selector: NodeSelector,
    #[serde(default)]
    bounds: QueryBounds,
}

#[derive(Debug, Deserialize)]
struct NodeParams {
    node_id: String,
    #[serde(default)]
    bounds: QueryBounds,
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    text: String,
    #[serde(default)]
    bounds: QueryBounds,
}

#[derive(Debug, Deserialize)]
struct NeighborsParams {
    node_id: String,
    #[serde(default = "default_direction")]
    direction: Direction,
    #[serde(default)]
    bounds: QueryBounds,
}

fn default_direction() -> Direction {
    Direction::Both
}

#[derive(Debug, Deserialize)]
struct DependenciesParams {
    node_id: String,
    #[serde(default)]
    transitive: bool,
    #[serde(default)]
    bounds: QueryBounds,
}

#[derive(Debug, Deserialize)]
struct PathParams {
    from: String,
    to: String,
    #[serde(default)]
    bounds: QueryBounds,
}

#[derive(Debug, Deserialize)]
struct OrientParams {
    selector: NodeSelector,
    #[serde(default)]
    bounds: QueryBounds,
}

/// Open the durable graph for querying and translate
/// [`QueryOpenError`] into the response shape Decision 0050 section 7
/// requires: `Absent`/`Stale` are typed, `ok: true` results a caller can
/// branch on programmatically (never an exception disguised as data);
/// `Unsupported`/`Corrupt` fail closed (`ok: false`).
enum Opened {
    Ready(repopact_graph::query::LoadedGraph),
    ShortCircuit(EngineResponse),
}

fn open_for_query(request: &EngineRequest, root: PathBuf, allow_stale: bool) -> Opened {
    match repopact_graph::query::open_durable_graph(&root, allow_stale) {
        Ok(loaded) => Opened::Ready(loaded),
        Err(QueryOpenError::Absent) => Opened::ShortCircuit(EngineResponse::success(
            request.request_id.clone(),
            ENGINE_VERSION,
            json!({
                "graph_absent": true,
                "guidance": "run `repopact graph build` first"
            }),
        )),
        Err(QueryOpenError::Stale { graph_fingerprint }) => {
            Opened::ShortCircuit(EngineResponse::success(
                request.request_id.clone(),
                ENGINE_VERSION,
                json!({
                    "stale": true,
                    "graph_fingerprint": graph_fingerprint,
                    "guidance": "pass bounds.allow_stale=true to query it anyway, or run `repopact graph update`"
                }),
            ))
        }
        Err(QueryOpenError::Unsupported) => Opened::ShortCircuit(semantic_failure_closed(
            request,
            "graph.schema-unsupported",
            "durable graph schema version is unsupported by this build",
        )),
        Err(QueryOpenError::Corrupt) => Opened::ShortCircuit(semantic_failure_closed(
            request,
            "graph.corrupt",
            "durable graph failed structural validation",
        )),
        // ROG-039: capability=enabled but rog/ missing is a hard,
        // binding failure -- fail closed exactly like Unsupported/
        // Corrupt, never a typed "absent, therefore valid" result.
        Err(QueryOpenError::EnabledButMissing) => Opened::ShortCircuit(semantic_failure_closed(
            request,
            "graph.capability-enabled-but-missing",
            "capability declares rog=enabled but no durable graph exists; \
             run `repopact graph build` or `repopact graph disable`",
        )),
    }
}

fn parse_params<T: for<'de> Deserialize<'de>>(
    request: &EngineRequest,
) -> Result<T, EngineResponse> {
    serde_json::from_value(request.params.clone()).map_err(|error| {
        semantic_failure_closed(request, "query.invalid-parameters", error.to_string())
    })
}

fn respond<T: serde::Serialize>(request: &EngineRequest, envelope: T) -> EngineResponse {
    EngineResponse::success(
        request.request_id.clone(),
        ENGINE_VERSION,
        serde_json::to_value(envelope).unwrap_or(Value::Null),
    )
}

pub fn graph_resolve(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params: ResolveParams = match parse_params(request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    match open_for_query(request, root, params.bounds.allow_stale) {
        Opened::Ready(loaded) => {
            let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
            respond(request, engine.resolve(&params.selector, &params.bounds))
        }
        Opened::ShortCircuit(response) => response,
    }
}

/// `graph.search` (Decision 0052 section 3): a bounded, deterministic,
/// in-memory search over already-indexed graph fields for an operator
/// search box -- never a repository scan, never a fuzzy/embedding/LLM
/// search. Additive to query contract version 1: existing operations'
/// wire semantics are unchanged.
pub fn graph_search(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params: SearchParams = match parse_params(request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    match open_for_query(request, root, params.bounds.allow_stale) {
        Opened::Ready(loaded) => {
            let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
            respond(request, engine.search(&params.text, &params.bounds))
        }
        Opened::ShortCircuit(response) => response,
    }
}

pub fn graph_context(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params: NodeParams = match parse_params(request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    match open_for_query(request, root, params.bounds.allow_stale) {
        Opened::Ready(loaded) => {
            let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
            respond(request, engine.context(&params.node_id, &params.bounds))
        }
        Opened::ShortCircuit(response) => response,
    }
}

pub fn graph_neighbors(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params: NeighborsParams = match parse_params(request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    match open_for_query(request, root, params.bounds.allow_stale) {
        Opened::Ready(loaded) => {
            let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
            respond(
                request,
                engine.neighbors(&params.node_id, params.direction, &params.bounds),
            )
        }
        Opened::ShortCircuit(response) => response,
    }
}

pub fn graph_path(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params: PathParams = match parse_params(request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    match open_for_query(request, root, params.bounds.allow_stale) {
        Opened::Ready(loaded) => {
            let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
            respond(
                request,
                engine.path(&params.from, &params.to, &params.bounds),
            )
        }
        Opened::ShortCircuit(response) => response,
    }
}

pub fn graph_dependencies(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params: DependenciesParams = match parse_params(request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    match open_for_query(request, root, params.bounds.allow_stale) {
        Opened::Ready(loaded) => {
            let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
            respond(
                request,
                engine.dependencies(&params.node_id, params.transitive, &params.bounds),
            )
        }
        Opened::ShortCircuit(response) => response,
    }
}

pub fn graph_dependents(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params: NodeParams = match parse_params(request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    match open_for_query(request, root, params.bounds.allow_stale) {
        Opened::Ready(loaded) => {
            let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
            respond(request, engine.dependents(&params.node_id, &params.bounds))
        }
        Opened::ShortCircuit(response) => response,
    }
}

pub fn graph_tests(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params: NodeParams = match parse_params(request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    match open_for_query(request, root, params.bounds.allow_stale) {
        Opened::Ready(loaded) => {
            let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
            respond(request, engine.tests(&params.node_id, &params.bounds))
        }
        Opened::ShortCircuit(response) => response,
    }
}

pub fn graph_governance(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params: NodeParams = match parse_params(request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    match open_for_query(request, root, params.bounds.allow_stale) {
        Opened::Ready(loaded) => {
            let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
            respond(request, engine.governance(&params.node_id, &params.bounds))
        }
        Opened::ShortCircuit(response) => response,
    }
}

pub fn graph_impact(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params: NodeParams = match parse_params(request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    match open_for_query(request, root, params.bounds.allow_stale) {
        Opened::Ready(loaded) => {
            let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
            respond(request, engine.impact(&params.node_id, &params.bounds))
        }
        Opened::ShortCircuit(response) => response,
    }
}

pub fn graph_orient(request: &EngineRequest) -> EngineResponse {
    let root = match require_root(request) {
        Ok(root) => root,
        Err(response) => return response,
    };
    let params: OrientParams = match parse_params(request) {
        Ok(params) => params,
        Err(response) => return response,
    };
    match open_for_query(request, root, params.bounds.allow_stale) {
        Opened::Ready(loaded) => {
            let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
            respond(request, engine.orient(&params.selector, &params.bounds))
        }
        Opened::ShortCircuit(response) => response,
    }
}
