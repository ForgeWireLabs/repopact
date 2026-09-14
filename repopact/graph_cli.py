"""CLI entry point for WI063 Repository Orientation Graph foundation operations.

Mirrors verify_cli.py's shape: a thin argparse wrapper delegating semantics
entirely to the canonical Rust engine's graph.* operations (Decision 0044,
Decision 0050). No client may gain a competing graph implementation here;
this module renders and dispatches only -- it never scrapes presentation
text, it consumes the engine's typed JSON results directly.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from .engine_client import EngineClient, EngineSemanticError


_FRESHNESS_EXIT_CODES = {
    "absent": 0,
    "fresh": 0,
    "working_overlay": 0,
    "partial": 0,
    "stale": 1,
    "unsupported": 2,
    "corrupt": 2,
}


def _render_status_human(result: dict) -> str:
    freshness = result.get("freshness", "unknown")
    lines = [f"RepoPact repository orientation graph: {freshness}"]
    manifest = result.get("manifest")
    if manifest:
        lines.append(
            f"  schema_version={manifest.get('graph_schema_version')} "
            f"nodes={manifest.get('node_count')} edges={manifest.get('edge_count')} "
            f"shards={manifest.get('shard_count')}"
        )
        coverage = manifest.get("coverage", {})
        nodes_by_layer = coverage.get("nodes_by_layer", {})
        if nodes_by_layer:
            summary = ", ".join(f"{layer}={count}" for layer, count in sorted(nodes_by_layer.items()))
            lines.append(f"  nodes by layer: {summary}")
    for diagnostic in result.get("diagnostics", []):
        lines.append(f"  [{diagnostic['code']}] {diagnostic['message']}")
    return "\n".join(lines) + "\n"


def _status(args: argparse.Namespace) -> int:
    response = EngineClient().call("graph.status", root=args.root)
    result = response["result"]
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(_render_status_human(result), end="")
    return _FRESHNESS_EXIT_CODES.get(result.get("freshness"), 2)


def _build(args: argparse.Namespace) -> int:
    try:
        response = EngineClient().call("graph.build", root=args.root)
    except EngineSemanticError as error:
        print(f"RepoPact graph build failed: {error}", file=sys.stderr)
        return 1
    result = response["result"]
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(
            f"RepoPact orientation graph built: nodes={result.get('node_count')} "
            f"edges={result.get('edge_count')} shards={result.get('shard_count')} "
            f"fingerprint={result.get('source_projection_fingerprint')}"
        )
    return 0


def _verify(args: argparse.Namespace) -> int:
    response = EngineClient().call("graph.verify", root=args.root)
    result = response["result"]
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(_render_status_human(result), end="")
        freshness = result.get("freshness")
        if freshness == "fresh":
            print("This proves the local durable graph only; no remote authority is implied.")
    return _FRESHNESS_EXIT_CODES.get(result.get("freshness"), 2)


def _update(args: argparse.Namespace) -> int:
    try:
        response = EngineClient().call("graph.update", root=args.root)
    except EngineSemanticError as error:
        print(f"RepoPact graph update failed: {error}", file=sys.stderr)
        return 1
    result = response["result"]
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        mode = result.get("mode", "unknown")
        lines = [f"RepoPact orientation graph update: mode={mode}"]
        if result.get("fallback_reason"):
            lines.append(f"  fallback_reason={result['fallback_reason']}")
        lines.append(
            f"  files: added={result.get('files_added')} modified={result.get('files_modified')} "
            f"deleted={result.get('files_deleted')} unchanged={result.get('files_unchanged')}"
        )
        lines.append(
            f"  semantic: reparsed={result.get('semantic_reparsed')} "
            f"reused={result.get('semantic_reused')} skipped={result.get('semantic_skipped')}"
        )
        lines.append(
            f"  graph: nodes={result.get('final_node_count')} edges={result.get('final_edge_count')} "
            f"freshness={result.get('freshness')}"
        )
        print("\n".join(lines))
    return _FRESHNESS_EXIT_CODES.get(result.get("freshness"), 2)


def _disable(args: argparse.Namespace) -> int:
    try:
        response = EngineClient().call("graph.disable", root=args.root)
    except EngineSemanticError as error:
        print(f"RepoPact graph disable failed: {error}", file=sys.stderr)
        return 1
    result = response["result"]
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print("RepoPact orientation graph explicitly disabled.")
    return 0


# ---- ROG-023/024/025/026: typed bounded query/orientation commands --------
#
# CLI convenience syntax maps explicit, mutually-exclusive target flags
# into the same typed NodeSelector the Rust engine/protocol accepts
# (Decision 0050 section 2) -- never a bare positional string a caller
# might mean as a path, a symbol, or a work item. This module never
# guesses.


def _add_selector_flags(parser: argparse.ArgumentParser, prefix: str = "") -> None:
    dest_prefix = prefix.replace("-", "_")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument(f"--{prefix}id", dest=f"{dest_prefix}id", help="Exact graph node ID")
    group.add_argument(f"--{prefix}path", dest=f"{dest_prefix}path", help="Repository-relative source path")
    group.add_argument(f"--{prefix}work-item", dest=f"{dest_prefix}work_item", help="Work item ID")
    group.add_argument(f"--{prefix}package", dest=f"{dest_prefix}package", help="Package/crate name")
    group.add_argument(f"--{prefix}module", dest=f"{dest_prefix}module", help="Module name")
    group.add_argument(f"--{prefix}symbol", dest=f"{dest_prefix}symbol", help="Symbol name")
    parser.add_argument(
        f"--{prefix}symbol-path" if prefix else "--symbol-path",
        dest=f"{dest_prefix}symbol_path",
        default=None,
        help="Disambiguate --symbol by repository-relative path",
    )


def _selector_from_args(args: argparse.Namespace, prefix: str = "") -> dict[str, Any]:
    dest_prefix = prefix.replace("-", "_")
    if getattr(args, f"{dest_prefix}id", None):
        return {"type": "node_id", "value": getattr(args, f"{dest_prefix}id")}
    if getattr(args, f"{dest_prefix}path", None):
        return {"type": "repository_path", "value": getattr(args, f"{dest_prefix}path")}
    if getattr(args, f"{dest_prefix}work_item", None):
        return {"type": "work_item_id", "value": getattr(args, f"{dest_prefix}work_item")}
    if getattr(args, f"{dest_prefix}package", None):
        return {"type": "package", "value": getattr(args, f"{dest_prefix}package")}
    if getattr(args, f"{dest_prefix}module", None):
        return {"type": "module", "value": getattr(args, f"{dest_prefix}module")}
    symbol = getattr(args, f"{dest_prefix}symbol", None)
    if symbol:
        value: dict[str, Any] = {"name": symbol}
        symbol_path = getattr(args, f"{dest_prefix}symbol_path", None)
        if symbol_path:
            value["path"] = symbol_path
        return {"type": "symbol", "value": value}
    raise SystemExit("no target selector provided")


def _bounds_from_args(args: argparse.Namespace) -> dict[str, Any]:
    bounds: dict[str, Any] = {}
    if args.max_nodes is not None:
        bounds["max_nodes"] = args.max_nodes
    if args.max_edges is not None:
        bounds["max_edges"] = args.max_edges
    if args.max_depth is not None:
        bounds["max_depth"] = args.max_depth
    if args.page_size is not None:
        bounds["page_size"] = args.page_size
    if args.cursor is not None:
        bounds["cursor"] = args.cursor
    if args.compact:
        bounds["compact"] = True
    if args.allow_stale:
        bounds["allow_stale"] = True
    return bounds


def _add_bounds_flags(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--max-nodes", type=int, default=None)
    parser.add_argument("--max-edges", type=int, default=None)
    parser.add_argument("--max-depth", type=int, default=None)
    parser.add_argument("--page-size", type=int, default=None)
    parser.add_argument("--cursor", default=None, help="Opaque pagination cursor from a previous call")
    parser.add_argument("--compact", action="store_true", help="Source-reference-only compact mode")
    parser.add_argument("--allow-stale", action="store_true", help="Query a stale durable graph anyway")


def _print_query_result(response: dict, args: argparse.Namespace) -> int:
    # Every query result is printed as its full typed JSON regardless of
    # --json: these are structured facts an agent/script consumes, not a
    # human summary that would need a separate presentation-string
    # formatter (and risk losing freshness/coverage/provenance fields a
    # formatter author forgot to render).
    result = response["result"]
    print(json.dumps(result, indent=2, sort_keys=True))
    if isinstance(result, dict) and result.get("stale"):
        return 1
    return 0


def _run_query(args: argparse.Namespace, operation: str, params: dict[str, Any]) -> int:
    try:
        response = EngineClient().call(operation, root=args.root, params=params)
    except EngineSemanticError as error:
        print(f"RepoPact {operation} failed: {error}", file=sys.stderr)
        return 1
    return _print_query_result(response, args)


def _resolve(args: argparse.Namespace) -> int:
    params = {"selector": _selector_from_args(args), "bounds": _bounds_from_args(args)}
    return _run_query(args, "graph.resolve", params)


def _orient(args: argparse.Namespace) -> int:
    params = {"selector": _selector_from_args(args), "bounds": _bounds_from_args(args)}
    return _run_query(args, "graph.orient", params)


def _resolve_to_node_id(args: argparse.Namespace, prefix: str = "") -> str | None:
    """Resolve the CLI's typed selector flags to an exact node ID via
    graph.resolve, printing an honest ambiguous/not-found result and
    returning None rather than guessing (step 33)."""
    params = {"selector": _selector_from_args(args, prefix), "bounds": _bounds_from_args(args)}
    try:
        response = EngineClient().call("graph.resolve", root=args.root, params=params)
    except EngineSemanticError as error:
        print(f"RepoPact graph.resolve failed: {error}", file=sys.stderr)
        return None
    result = response["result"]
    if isinstance(result, dict) and (result.get("graph_absent") or result.get("stale")):
        print(json.dumps(result, indent=2, sort_keys=True))
        return None
    outcome = result.get("result", {})
    if outcome.get("outcome") == "exact":
        return outcome["fact"]["id"]
    print(json.dumps(result, indent=2, sort_keys=True), file=sys.stderr)
    return None


def _node_target_command(operation: str, extra_params: dict[str, Any] | None = None):
    def handler(args: argparse.Namespace) -> int:
        node_id = _resolve_to_node_id(args)
        if node_id is None:
            return 2
        params = {"node_id": node_id, "bounds": _bounds_from_args(args)}
        if extra_params:
            params.update(extra_params(args))
        return _run_query(args, operation, params)

    return handler


def _neighbors(args: argparse.Namespace) -> int:
    node_id = _resolve_to_node_id(args)
    if node_id is None:
        return 2
    params = {"node_id": node_id, "direction": args.direction, "bounds": _bounds_from_args(args)}
    return _run_query(args, "graph.neighbors", params)


def _dependencies(args: argparse.Namespace) -> int:
    node_id = _resolve_to_node_id(args)
    if node_id is None:
        return 2
    params = {"node_id": node_id, "transitive": args.transitive, "bounds": _bounds_from_args(args)}
    return _run_query(args, "graph.dependencies", params)


def _path(args: argparse.Namespace) -> int:
    from_id = _resolve_to_node_id(args, prefix="from-")
    if from_id is None:
        return 2
    to_id = _resolve_to_node_id(args, prefix="to-")
    if to_id is None:
        return 2
    params = {"from": from_id, "to": to_id, "bounds": _bounds_from_args(args)}
    return _run_query(args, "graph.path", params)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="RepoPact Repository Orientation Graph (WI063) operations"
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p_status = sub.add_parser("status", help="Report current graph capability/freshness state")
    p_status.add_argument("--root", type=Path, default=Path.cwd())
    p_status.add_argument("--json", action="store_true")
    p_status.set_defaults(handler=_status)

    p_build = sub.add_parser("build", help="Build/rebuild the durable orientation graph")
    p_build.add_argument("--root", type=Path, default=Path.cwd())
    p_build.add_argument("--json", action="store_true")
    p_build.set_defaults(handler=_build)

    p_verify = sub.add_parser("verify", help="Verify durable graph structure and freshness")
    p_verify.add_argument("--root", type=Path, default=Path.cwd())
    p_verify.add_argument("--json", action="store_true")
    p_verify.set_defaults(handler=_verify)

    p_update = sub.add_parser(
        "update", help="Incrementally update the durable graph (falls back to a full rebuild when reuse is not safe)"
    )
    p_update.add_argument("--root", type=Path, default=Path.cwd())
    p_update.add_argument("--json", action="store_true")
    p_update.set_defaults(handler=_update)

    p_disable = sub.add_parser(
        "disable",
        help="Explicitly disable the durable orientation graph (removes rog/, persists capability=disabled)",
    )
    p_disable.add_argument("--root", type=Path, default=Path.cwd())
    p_disable.add_argument("--json", action="store_true")
    p_disable.set_defaults(handler=_disable)

    # ---- ROG-023/024/025/026 typed bounded query/orientation commands ----

    def base_query_parser(name: str, help_text: str) -> argparse.ArgumentParser:
        p = sub.add_parser(name, help=help_text)
        p.add_argument("--root", type=Path, default=Path.cwd())
        p.add_argument("--json", action="store_true")
        _add_bounds_flags(p)
        return p

    p_resolve = base_query_parser("resolve", "Resolve a typed target selector to an exact graph fact")
    _add_selector_flags(p_resolve)
    p_resolve.set_defaults(handler=_resolve)

    p_context = base_query_parser("context", "Bounded factual context around a resolved target")
    _add_selector_flags(p_context)
    p_context.set_defaults(handler=_node_target_command("graph.context"))

    p_neighbors = base_query_parser("neighbors", "Bounded adjacent facts (incoming/outgoing/both)")
    _add_selector_flags(p_neighbors)
    p_neighbors.add_argument("--direction", choices=["outgoing", "incoming", "both"], default="both")
    p_neighbors.set_defaults(handler=_neighbors)

    p_path = base_query_parser("path", "Deterministic bounded shortest path between two targets")
    _add_selector_flags(p_path, prefix="from-")
    _add_selector_flags(p_path, prefix="to-")
    p_path.set_defaults(handler=_path)

    p_dependencies = base_query_parser("dependencies", "Direct or transitive dependency facts")
    _add_selector_flags(p_dependencies)
    p_dependencies.add_argument("--transitive", action="store_true")
    p_dependencies.set_defaults(handler=_dependencies)

    p_dependents = base_query_parser("dependents", "Reverse dependency facts (persisted + query-derived)")
    _add_selector_flags(p_dependents)
    p_dependents.set_defaults(handler=_node_target_command("graph.dependents"))

    p_impact = base_query_parser("impact", "Bounded structural impact set (never behavioral proof)")
    _add_selector_flags(p_impact)
    p_impact.set_defaults(handler=_node_target_command("graph.impact"))

    p_tests = base_query_parser("tests", "Known test targets for a resolved target")
    _add_selector_flags(p_tests)
    p_tests.set_defaults(handler=_node_target_command("graph.tests"))

    p_governance = base_query_parser("governance", "Applicable RepoPact governance facts (informational only)")
    _add_selector_flags(p_governance)
    p_governance.set_defaults(handler=_node_target_command("graph.governance"))

    p_orient = base_query_parser("orient", "Compact bounded orientation result plus an inspect-first list")
    _add_selector_flags(p_orient)
    p_orient.set_defaults(handler=_orient)

    args = parser.parse_args(argv)
    return args.handler(args)


if __name__ == "__main__":
    raise SystemExit(main())
