"""Diff-time enforcement for the declared frozen surface (INV-6).

The validator reasons about records, not diffs, so it cannot tell whether a
given change touched a protected path. This script is the backstop: it compares
the changed files in a range against ``governance/frozen-surface.json`` and fails
when a protected entry is touched without acknowledgement.

It is intentionally best-effort: outside a git checkout it reports nothing and
exits zero, so it never blocks environments where a diff is unavailable.
"""

from __future__ import annotations

import argparse
import fnmatch
import subprocess
import sys
from pathlib import Path

from repo_model import load_json


def changed_files(root: Path, base: str) -> list[str] | None:
    try:
        result = subprocess.run(
            ["git", "diff", "--name-only", f"{base}...HEAD"],
            cwd=root,
            capture_output=True,
            text=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def violations(root: Path, base: str) -> list[tuple[str, str]]:
    data = load_json(root / "governance" / "frozen-surface.json")
    protected = data.get("protected", [])
    files = changed_files(root, base)
    if files is None:
        return []
    hits: list[tuple[str, str]] = []
    for path in files:
        for entry in protected:
            glob = str(entry.get("glob", ""))
            if fnmatch.fnmatch(path, glob) or fnmatch.fnmatch(path, f"{glob.rstrip('/**')}/**"):
                hits.append((path, str(entry.get("reason", ""))))
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
    for path, reason in hits:
        print(f"  {path}: {reason}")
    if args.ack:
        print("\nOperator approval acknowledged (--ack). Proceeding.")
        return 0
    print("\nRe-run with --ack only after a human operator has approved these changes.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
