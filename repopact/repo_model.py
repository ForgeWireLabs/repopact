from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


STATUSES = ("proposed", "active", "blocked", "deferred", "completed")

# Directories that are not part of the governed tree: tooling caches, test
# fixtures (self-contained sub-repositories validated on their own), and
# `worktrees` -- the conventional directory name agent tooling (Claude Code's
# Agent isolation, Codex, etc.) uses for a scratch `git worktree` checkout.
# A worktree is a second working copy of the SAME repository; its own
# AGENTS.md files are already governed at their origin and must not be
# re-validated as if they were a nested subtree of this checkout, and a
# worktree can outlive the session that created it (left on disk after an
# agent's task completes) with no reliable, portable signal other than its
# name to distinguish it from a real nested contract.
IGNORED_PARTS = {
    ".git", "__pycache__", "node_modules", ".venv", ".pytest_cache",
    "build", "dist", "fixtures", "worktrees",
}


def iter_contracts(root: Path) -> list[Path]:
    """All AGENTS.md under root, excluding ignored subtrees (relative to root)."""
    result: list[Path] = []
    for path in sorted(root.rglob("AGENTS.md")):
        if any(part in IGNORED_PARTS for part in path.relative_to(root).parts):
            continue
        result.append(path)
    return result


@dataclass(frozen=True)
class WorkItem:
    directory: Path
    data: dict[str, Any]

    @property
    def item_id(self) -> str:
        return str(self.data.get("id", ""))

    @property
    def status(self) -> str:
        return str(self.data.get("status", ""))


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object: {path}")
    return value


def discover_work_items(root: Path) -> list[WorkItem]:
    items: list[WorkItem] = []
    for status in STATUSES:
        status_dir = root / "work" / status
        if not status_dir.exists():
            continue
        for manifest in sorted(status_dir.glob("*/work-item.json")):
            items.append(WorkItem(manifest.parent, load_json(manifest)))
    return items


def discover_evidence_ids(root: Path) -> set[str]:
    result: set[str] = set()
    for path in sorted((root / "evidence" / "runs").glob("*.json")):
        result.add(str(load_json(path).get("id", "")))
    return result
