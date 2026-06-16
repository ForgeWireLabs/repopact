"""Import a repository's existing plan items into the RepoPact ``work/`` ledger.

`adopt` scaffolds governance but leaves `work/` empty, so an adopted repo ends up with
a hollow `work/` next to the team's real planning (e.g. a `todos/` tree). This module
detects the common ways developers track plan items and imports them into `work/` as
work items — non-destructively (the source is preserved), idempotently (re-running
skips items already imported), and honestly (completed items become `waived`, never
fabricated `satisfied`-with-evidence).

Supported sources (extensible):

* **Plan directories** — `todos/`, `todo/`, `tasks/`, `plan/`, `planning/`, `backlog/`
  with one subdirectory or markdown file per item, optionally grouped under
  `completed/`, `deferred/`, `blocked/`, `active/` lifecycle folders.
* **Checklist files** — `TODO.md`, `TODOS.md`, `ROADMAP.md`, `BACKLOG.md`, `PLAN.md`,
  `TASKS.md` at the root: each `- [ ]` / `- [x]` line becomes a work item.

Run via ``repopact import-plan --target <repo> [--dry-run]``.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path

import adopt_repo  # reuse Report and _slug

PLAN_DIR_NAMES = ("todos", "todo", "tasks", "plan", "planning", "backlog")
CHECKLIST_FILES = ("TODO.md", "TODOS.md", "ROADMAP.md", "BACKLOG.md", "PLAN.md", "TASKS.md")
LIFECYCLE_DIRS = {"completed", "deferred", "blocked", "active"}

# source lifecycle -> (work status, acceptance-criterion state)
_STATUS = {
    "completed": ("completed", "waived"),
    "deferred": ("deferred", "pending"),
    "blocked": ("blocked", "pending"),
    "active": ("active", "pending"),
}
_EMOJI = {"completed": "✅ Complete", "deferred": "⏸️ Deferred", "blocked": "⛔ Blocked", "active": "📋 Active"}

_SKIP_DIRS = {".git", ".venv", "node_modules", "__pycache__"}


@dataclass
class PlanItem:
    lifecycle: str          # completed | deferred | blocked | active
    slug: str               # RepoPact slug (no leading number)
    title: str
    narrative: str          # markdown body to preserve
    source_rel: str         # path the item came from (provenance)
    num: str | None = None  # original leading number, if any


def _first_heading(text: str, fallback: str) -> str:
    for line in text.splitlines():
        m = re.match(r"\s*#\s+(.+?)\s*$", line)
        if m:
            return m.group(1).strip()
    return fallback


def _split_num(name: str) -> tuple[str | None, str]:
    m = re.match(r"(\d+)[-_ ]+(.*)$", name)
    if m:
        return m.group(1), adopt_repo._slug(m.group(2))
    return None, adopt_repo._slug(name)


def _read_md(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def _dir_item(directory: Path, lifecycle: str, root: Path) -> PlanItem:
    readme = directory / "README.md"
    body = _read_md(readme) if readme.is_file() else ""
    if not body:  # fall back to the first markdown file in the directory
        mds = sorted(p for p in directory.glob("*.md") if p.name.lower() != "readme.md")
        if mds:
            body = _read_md(mds[0])
    num, slug = _split_num(directory.name)
    title = _first_heading(body, slug.replace("-", " ").title())
    return PlanItem(lifecycle, slug, title, body, str(directory.relative_to(root)).replace("\\", "/"), num)


def collect_from_dir(plan_dir: Path, root: Path) -> list[PlanItem]:
    items: list[PlanItem] = []
    for entry in sorted(plan_dir.iterdir()):
        if entry.name in _SKIP_DIRS or entry.name.lower() == "readme.md":
            continue
        if entry.is_dir() and entry.name.lower() in LIFECYCLE_DIRS:
            lifecycle = entry.name.lower()
            for sub in sorted(entry.iterdir()):
                if sub.name.lower() == "readme.md":
                    continue
                if sub.is_dir():
                    items.append(_dir_item(sub, lifecycle, root))
                elif sub.suffix.lower() == ".md":
                    body = _read_md(sub)
                    num, slug = _split_num(sub.stem)
                    items.append(PlanItem(lifecycle, slug, _first_heading(body, slug.replace("-", " ").title()),
                                          body, str(sub.relative_to(root)).replace("\\", "/"), num))
        elif entry.is_dir():
            items.append(_dir_item(entry, "active", root))
        elif entry.suffix.lower() == ".md":
            body = _read_md(entry)
            num, slug = _split_num(entry.stem)
            items.append(PlanItem("active", slug, _first_heading(body, slug.replace("-", " ").title()),
                                  body, str(entry.relative_to(root)).replace("\\", "/"), num))
    return items


# Section-heading keyword -> lifecycle, for roadmap/backlog files without checkboxes.
_SECTION_LIFECYCLE = (
    ("completed", ("done", "complete", "shipped", "released", "delivered")),
    ("deferred", ("later", "future", "someday", "deferred", "icebox", "parking", "wishlist")),
    ("blocked", ("blocked", "on hold", "waiting")),
    ("active", ("now", "next", "in progress", "doing", "active", "todo", "to do", "planned", "backlog")),
)


def _section_lifecycle(heading: str) -> str | None:
    low = heading.lower()
    for lifecycle, keywords in _SECTION_LIFECYCLE:
        if any(k in low for k in keywords):
            return lifecycle
    return None


def _clean_inline(text: str) -> str:
    return re.sub(r"[`*_]", "", text).strip()


def collect_from_checklist(path: Path, root: Path) -> list[PlanItem]:
    """Parse a roadmap/checklist file. Checkbox lines use their checkbox state;
    plain top-level bullets inherit the lifecycle of their `##` section heading."""
    items: list[PlanItem] = []
    rel = str(path.relative_to(root)).replace("\\", "/")
    section: str | None = None
    for raw in _read_md(path).splitlines():
        heading = re.match(r"#{1,3}\s+(.+?)\s*$", raw)
        if heading:
            section = _section_lifecycle(heading.group(1))
            continue
        box = re.match(r"[-*]\s+\[([ xX])\]\s+(.+?)\s*$", raw)   # top-level checkbox
        if box:
            text = _clean_inline(box.group(2))
            lifecycle = "completed" if box.group(1).lower() == "x" else (section or "active")
            items.append(PlanItem(lifecycle, adopt_repo._slug(text)[:60] or "item", text, raw.strip() + "\n", rel))
            continue
        bullet = re.match(r"[-*]\s+(.+?)\s*$", raw)              # top-level plain bullet (no checkbox)
        if bullet and section is not None:                       # only inside a recognized section
            text = _clean_inline(bullet.group(1))
            items.append(PlanItem(section, adopt_repo._slug(text)[:60] or "item", text, raw.strip() + "\n", rel))
    return items


def collect_from_github(root: Path, limit: int = 200) -> list[PlanItem]:
    """Import GitHub issues as plan items (opt-in). open -> active, closed -> completed.
    Best-effort: returns [] if gh is unavailable or there is no GitHub remote."""
    import json as _json
    import subprocess
    try:
        result = subprocess.run(
            ["gh", "issue", "list", "--state", "all", "--limit", str(limit),
             "--json", "number,title,body,state,url"],
            cwd=root, capture_output=True, text=True)
    except OSError:
        return []
    if result.returncode != 0:
        return []
    try:
        issues = _json.loads(result.stdout or "[]")
    except ValueError:
        return []
    items: list[PlanItem] = []
    for issue in issues:
        lifecycle = "completed" if str(issue.get("state", "")).upper() == "CLOSED" else "active"
        title = str(issue.get("title", "")).strip() or f"issue-{issue.get('number')}"
        body = str(issue.get("body") or "")
        narrative = f"{issue.get('url', '')}\n\n{body}\n"
        items.append(PlanItem(lifecycle, adopt_repo._slug(title)[:60] or "issue", title,
                              narrative, f"github:#{issue.get('number')}"))
    return items


def discover(root: Path, import_issues: bool = False) -> list[PlanItem]:
    items: list[PlanItem] = []
    for name in PLAN_DIR_NAMES:
        d = root / name
        if d.is_dir():
            items.extend(collect_from_dir(d, root))
    for fname in CHECKLIST_FILES:
        f = root / fname
        if f.is_file():
            items.extend(collect_from_checklist(f, root))
    if import_issues:
        items.extend(collect_from_github(root))
    return items


def _existing(root: Path) -> tuple[set[str], set[str]]:
    """Return (used ids, used slugs) across all work/ lifecycles."""
    ids: set[str] = set()
    slugs: set[str] = set()
    for manifest in root.glob("work/*/*/work-item.json"):
        try:
            ids.add(str(json.loads(manifest.read_text(encoding="utf-8")).get("id", "")))
        except (OSError, ValueError):
            pass
        name = manifest.parent.name
        m = re.match(r"\d+-(.*)$", name)
        slugs.add(m.group(1) if m else name)
    return ids, slugs


def _allocate(num: str | None, used: set[str]) -> str:
    if num and num != "0" * len(num):
        candidate = num.zfill(3)
        if candidate not in used:
            used.add(candidate)
            return candidate
    n = 200
    while f"{n:03d}" in used:
        n += 1
    used.add(f"{n:03d}")
    return f"{n:03d}"


def import_plan(root: Path, today: date | None = None, dry_run: bool = False,
                import_issues: bool = False) -> adopt_repo.Report:
    today = today or date.today()
    rep = adopt_repo.Report(dry_run)
    used_ids, used_slugs = _existing(root)
    for item in discover(root, import_issues=import_issues):
        if item.slug in used_slugs:        # idempotent: already imported (or name clash) -> skip
            rep.skipped.append(f"work/* /{item.slug} (already present)")
            continue
        used_slugs.add(item.slug)
        item_id = _allocate(item.num, used_ids)
        status, ac_state = _STATUS[item.lifecycle]
        wi_dir = root / "work" / status / f"{item_id}-{item.slug}"
        if ac_state == "waived":
            ac_text = ("Delivered prior to RepoPact adoption; imported from "
                       f"`{item.source_rel}`. Waived: no RepoPact evidence was captured at the time.")
        else:
            ac_text = f"Deliver the outcome described in the imported plan narrative (`{item.source_rel}`)."
        rep.json(wi_dir / "work-item.json", {
            "$schema": "../../../schemas/work-item.schema.json",
            "id": item_id, "title": item.title, "status": status,
            "owner_scope": "governance", "affected_scopes": [], "depends_on": [],
            "acceptance_criteria": [{"id": "AC-1", "text": ac_text, "state": ac_state, "evidence": []}],
            "created": today.isoformat(), "updated": today.isoformat(),
            "source": item.source_rel,
        }, root)
        header = (f"# {item_id} — {item.title}\n\n> **Status**: {_EMOJI[item.lifecycle]}\n"
                  f"> Imported into RepoPact from `{item.source_rel}`; the source is preserved.\n\n"
                  "## Imported plan narrative\n\n")
        rep.write(wi_dir / "README.md", header + (item.narrative or "_(no narrative in source)_\n"), root)
    return rep


def _print(rep: adopt_repo.Report) -> None:
    verb = "Would import" if rep.dry_run else "Imported"
    print(f"{verb} {len(rep.created) // 2} plan item(s) into work/; skipped {len(rep.skipped)}.")
    for rel in rep.created:
        if rel.endswith("work-item.json"):
            print(f"  + {rel.rsplit('/', 1)[0]}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Import existing plan items into the RepoPact work/ ledger")
    parser.add_argument("--target", type=Path, default=Path.cwd())
    parser.add_argument("--dry-run", action="store_true", help="Report the plan without writing files")
    parser.add_argument("--issues", action="store_true", help="Also import GitHub issues (needs gh + a GitHub remote)")
    args = parser.parse_args()
    root = args.target.resolve()
    rep = import_plan(root, dry_run=args.dry_run, import_issues=args.issues)
    _print(rep)
    if args.dry_run:
        print("\nDry run: nothing written.")
        return 0
    import validate_repo
    problems = validate_repo.validate(root)
    if problems:
        for p in problems:
            print(f"ERROR {p.path.relative_to(root)}: {p.message}")
        print(f"\nImport produced {len(problems)} validation error(s).")
        return 1
    print("\nwork/ ledger imported; repository validates as a conformant RepoPact.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
