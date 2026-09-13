"""CLI entry point for WI063 Repository Orientation Graph foundation operations.

Mirrors verify_cli.py's shape: a thin argparse wrapper delegating semantics
entirely to the canonical Rust engine's graph.status/graph.build/graph.verify
operations (Decision 0044). No client may gain a competing graph
implementation here; this module renders and dispatches only.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="RepoPact Repository Orientation Graph (WI063 foundation) operations"
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

    args = parser.parse_args(argv)
    return args.handler(args)


if __name__ == "__main__":
    raise SystemExit(main())
