from __future__ import annotations

import json
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any


STATUSES = ("proposed", "active", "blocked", "deferred", "completed")

# Directories that are not part of the governed tree: tooling caches, test
# fixtures (self-contained sub-repositories validated on their own), and
# `worktrees` -- the conventional directory name agent tooling uses for a
# scratch checkout. The name remains a compatibility/performance fallback for
# stale or orphaned scratch trees whose Git metadata is gone; live linked
# worktrees are identified structurally by discover_embedded_worktree_roots.
IGNORED_PARTS = {
    ".git", "__pycache__", "node_modules", ".venv", ".pytest_cache",
    "build", "dist", "fixtures", "worktrees",
}


def _resolved(path: Path) -> Path:
    """Resolve a path without requiring it to exist, with a safe fallback."""
    try:
        return path.resolve()
    except (OSError, RuntimeError):
        return Path(os.path.abspath(path))


def _is_descendant(path: Path, ancestor: Path) -> bool:
    """Return whether ``path`` is strictly below ``ancestor``."""
    try:
        path.relative_to(ancestor)
    except ValueError:
        return False
    return path != ancestor


def _gitdir_from_entry(entry: Path) -> Path | None:
    """Read a Git directory from either a normal directory or linked-worktree file."""
    if entry.is_dir():
        return _resolved(entry)
    if not entry.is_file():
        return None
    try:
        first = entry.read_text(encoding="utf-8").splitlines()[0].strip()
    except (OSError, IndexError, UnicodeDecodeError):
        return None
    if not first.lower().startswith("gitdir:"):
        return None
    target = Path(first.split(":", 1)[1].strip())
    if not target.is_absolute():
        target = entry.parent / target
    return _resolved(target)


def _git_common_dir(root: Path) -> Path | None:
    """Find the primary repository's common Git directory when metadata permits."""
    entry = root / ".git"
    direct = _gitdir_from_entry(entry)
    if direct is not None:
        # A linked checkout's .git file points at <common>/.git/worktrees/<name>.
        if direct.parent.name.lower() == "worktrees":
            return direct.parent.parent
        return direct
    if not entry.exists():
        return None
    try:
        result = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "--git-common-dir"],
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError:
        return None
    if result.returncode != 0 or not result.stdout.strip():
        return None
    common = Path(result.stdout.strip())
    if not common.is_absolute():
        common = root / common
    return _resolved(common)


def _registered_worktree_roots(root: Path) -> set[Path]:
    """Return registered same-repository worktrees below ``root`` when Git is usable."""
    if not (root / ".git").exists():
        return set()
    try:
        result = subprocess.run(
            ["git", "-C", str(root), "worktree", "list", "--porcelain"],
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError:
        return set()
    if result.returncode != 0:
        return set()
    roots: set[Path] = set()
    root_abs = _resolved(root)
    for line in result.stdout.splitlines():
        if not line.startswith("worktree "):
            continue
        candidate = Path(line[len("worktree "):])
        if not candidate.is_absolute():
            candidate = root_abs / candidate
        candidate = _resolved(candidate)
        if _is_descendant(candidate, root_abs):
            roots.add(candidate)
    return roots


def discover_embedded_worktree_roots(root: Path) -> set[Path]:
    """Find linked worktree checkout roots nested below a RepoPact scan root.

    Git's porcelain registry catches live worktrees even when their embedded
    metadata is unusual. The filesystem pass catches stale-but-identifiable
    linked checkouts whose ``.git`` file still points into the primary
    ``.git/worktrees`` directory. Independent nested repositories have a
    ``.git`` directory and are intentionally not returned.
    """
    root_abs = _resolved(root)
    if not (root_abs / ".git").exists():
        return set()
    linked = _registered_worktree_roots(root_abs)
    common = _git_common_dir(root_abs)
    worktrees_dir = common / "worktrees" if common is not None else None

    if worktrees_dir is not None:
        for current, directories, files in os.walk(root_abs, topdown=True, followlinks=False):
            current_path = _resolved(Path(current))
            if current_path != root_abs and any(
                current_path == known or _is_descendant(current_path, known) for known in linked
            ):
                directories[:] = []
                continue
            directories[:] = sorted(
                name for name in directories if name not in IGNORED_PARTS
            )
            if ".git" not in files or current_path == root_abs:
                continue
            git_dir = _gitdir_from_entry(Path(current) / ".git")
            if git_dir is not None and (
                git_dir == worktrees_dir or _is_descendant(git_dir, worktrees_dir)
            ):
                linked.add(current_path)
                directories[:] = []
    return linked


def iter_contracts(root: Path) -> list[Path]:
    """All AGENTS.md under root, excluding ignored and linked-worktree subtrees."""
    root_abs = _resolved(root)
    linked = discover_embedded_worktree_roots(root_abs)
    result: list[Path] = []
    for current, directories, files in os.walk(root_abs, topdown=True, followlinks=False):
        current_abs = _resolved(Path(current))
        if current_abs != root_abs and any(
            current_abs == known or _is_descendant(current_abs, known) for known in linked
        ):
            directories[:] = []
            continue
        directories[:] = sorted(
            name for name in directories
            if name not in IGNORED_PARTS
            and not any(
                _resolved(Path(current) / name) == known
                or _is_descendant(_resolved(Path(current) / name), known)
                for known in linked
            )
        )
        if "AGENTS.md" not in files:
            continue
        relative = current_abs.relative_to(root_abs)
        result.append(root / relative / "AGENTS.md")
    return sorted(result)


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
