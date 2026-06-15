"""Diff-time enforcement for the declared frozen surface (INV-6).

The validator reasons about records, not diffs, so it cannot tell whether a given
change touched a protected path or symbol. This script is the backstop: it compares
a diff range against ``governance/frozen-surface.json`` and reports protected paths
that changed and protected symbols that appear in the patch.

It is best-effort: outside a git checkout it reports nothing and exits zero, so it
never blocks environments where a diff is unavailable.
"""

from __future__ import annotations

import argparse
import fnmatch
import re
import subprocess
import sys
from pathlib import Path

from repo_model import load_json


def changed_files(root: Path, base: str) -> list[str] | None:
    return _git(root, ["diff", "--name-only", f"{base}...HEAD"])


def diff_text(root: Path, base: str) -> str | None:
    out = _git(root, ["diff", "--unified=0", f"{base}...HEAD"])
    return "\n".join(out) if out is not None else None


def _git(root: Path, args: list[str]) -> list[str] | None:
    try:
        result = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True, check=True)
    except (OSError, subprocess.CalledProcessError):
        return None
    return [line for line in result.stdout.splitlines()]


def path_hits(files: list[str], protected: list[dict]) -> list[tuple[str, str]]:
    hits: list[tuple[str, str]] = []
    for path in files:
        for entry in protected:
            glob = str(entry.get("glob", ""))
            if fnmatch.fnmatch(path, glob) or fnmatch.fnmatch(path, f"{glob.rstrip('/*')}/**"):
                hits.append((path, str(entry.get("reason", ""))))
    return hits


def symbol_hits(patch: str, protected: list[dict]) -> list[tuple[str, str]]:
    """Protected symbols that appear on added or removed lines of the patch."""
    changed_lines = [
        line[1:] for line in patch.splitlines()
        if line[:1] in {"+", "-"} and not line.startswith(("+++", "---"))
    ]
    body = "\n".join(changed_lines)
    hits: list[tuple[str, str]] = []
    for entry in protected:
        for symbol in entry.get("symbols", []):
            if re.search(rf"\b{re.escape(str(symbol))}\b", body):
                hits.append((str(symbol), str(entry.get("reason", ""))))
    return hits


def violations(root: Path, base: str) -> list[tuple[str, str]]:
    protected = load_json(root / "governance" / "frozen-surface.json").get("protected", [])
    files = changed_files(root, base)
    if files is None:
        return []
    hits = path_hits(files, protected)
    patch = diff_text(root, base)
    if patch:
        hits.extend(symbol_hits(patch, protected))
    return hits


def main() -> int:
    parser = argparse.ArgumentParser(description="Detect frozen-surface changes in a diff range")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--base", default="origin/main", help="Base ref to diff against")
    parser.add_argument("--ack", action="store_true", help="Acknowledge operator approval for the change")
    args = parser.parse_args()
    root = args.root.resolve()
    hits = violations(root, args.base)
    if not hits:
        print("No frozen-surface changes detected.")
        return 0
    print("Frozen-surface changes detected (INV-6 requires operator approval):")
    for name, reason in hits:
        print(f"  {name}: {reason}")
    if args.ack:
        print("\nOperator approval acknowledged (--ack). Proceeding.")
        return 0
    print("\nRe-run with --ack only after a human operator has approved these changes.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
