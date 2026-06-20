"""`repopact takeover` — let RepoPact fully replace a legacy planning method.

Once `adopt` + `import-plan` + validation are done, a homegrown plan directory
(`todos/`, `tasks/`, …) sitting beside `work/` is just confusing. `takeover` retires
such a directory — **archiving** it under `archive/` (default) or **deleting** it
(`--delete`) — but only when it is safe to do so:

1. the repository validates as a conformant RepoPact, and
2. every plan item the directory contains is provably represented as a work item,
   matched via each work item's `source` provenance.

A directory with any *un-migrated* item is left untouched and reported, so planning
that RepoPact has not captured (e.g. a `tracking/` system import-plan never read) is
never lost. Re-validates after any change.

`--delete` is documented and git-guarded: it deletes only when the directory is
fully tracked and clean (so it is **recoverable from git history**), and it first
writes a `decisions/` ADR recording why the directory was retired and how to
restore it (`git checkout <sha> -- <dir>`). If the directory is not git-recoverable
(not a repo, untracked, or has uncommitted changes), the delete is downgraded to an
archive so nothing unrecoverable is lost.

    repopact takeover [--root PATH] [--delete] [--dry-run]
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from datetime import date
from pathlib import Path

import adopt_repo
import plan_import
import validate_repo


def _git(root: Path, args: list[str]) -> str | None:
    """Run a git command in ``root``; return stdout, or None on any failure."""
    try:
        result = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)
    except OSError:
        return None
    if result.returncode != 0:
        return None
    return result.stdout


def git_recoverable(root: Path, d: Path) -> tuple[bool, str | None, str | None]:
    """Is directory ``d`` safe to delete because git can fully restore it?

    Recoverable means: ``root`` is a git work tree, every file under ``d`` is
    tracked, and there are no uncommitted changes or untracked files inside it —
    so the tree exists at ``HEAD`` and ``git checkout <sha> -- <dir>`` brings it
    back. Returns ``(ok, head_sha, reason_if_not)``.
    """
    if _git(root, ["rev-parse", "--show-toplevel"]) is None:
        return (False, None, "not a git repository")
    rel = str(d.relative_to(root)).replace("\\", "/")
    tracked = _git(root, ["ls-files", "--", rel])
    if not tracked or not tracked.strip():
        return (False, None, "directory is not tracked in git")
    # --ignored so a present-but-git-ignored file (not in history, e.g. build
    # output under the dir) also blocks deletion — it would not be recoverable.
    dirty = _git(root, ["status", "--porcelain", "--ignored", "--", rel])
    if dirty is None:
        return (False, None, "could not read git status")
    if dirty.strip():
        return (False, None, "directory has uncommitted, untracked, or git-ignored files")
    sha = _git(root, ["rev-parse", "HEAD"])
    if not sha or not sha.strip():
        return (False, None, "repository has no commits")
    return (True, sha.strip(), None)


def _next_decision_id(root: Path) -> str:
    used: set[str] = set()
    decisions = root / "decisions"
    if decisions.is_dir():
        for p in decisions.glob("*.md"):
            m = re.match(r"(\d{4,})", p.name)
            if m:
                used.add(m.group(1))
    n = 1
    while f"{n:04d}" in used:
        n += 1
    return f"{n:04d}"


def write_retire_decision(root: Path, rel: str, item_count: int, sha: str,
                          dry_run: bool, report: dict) -> str:
    """Record an ADR explaining why a fully-migrated plan dir was deleted."""
    did = _next_decision_id(root)
    slug = adopt_repo._slug(f"retire-{rel}-legacy-plan-directory")
    path = root / "decisions" / f"{did}-{slug}.md"
    today = date.today().isoformat()
    text = (
        f"---\n"
        f"id: {did}\n"
        f"title: Retire legacy plan directory `{rel}/`\n"
        f"status: accepted\n"
        f"date: {today}\n"
        f"supersedes: []\n"
        f"---\n\n"
        f"# {did}: Retire legacy plan directory `{rel}/`\n\n"
        f"## Context\n\n"
        f"`{rel}/` was a homegrown planning method that has been fully imported into\n"
        f"the RepoPact `work/` ledger: every one of its {item_count} plan item(s) is\n"
        f"provably represented as a work item, matched via each work item's `source`\n"
        f"provenance. A migrated plan directory sitting beside `work/` is a duplicate\n"
        f"source of truth and a recurring source of confusion about which is\n"
        f"authoritative.\n\n"
        f"## Decision\n\n"
        f"Delete `{rel}/` rather than archive it. At retirement the directory was fully\n"
        f"tracked and clean at commit `{sha}`, so the content is **recoverable from git\n"
        f"history**. Leaving an `archive/{rel}/` copy in the working tree would add\n"
        f"confusion without adding durability.\n\n"
        f"## Recovery\n\n"
        f"The directory and all of its detail files remain in git history:\n\n"
        f"```sh\n"
        f"git show {sha}:{rel}            # browse the retired tree\n"
        f"git checkout {sha} -- {rel}     # restore it into the working tree\n"
        f"```\n\n"
        f"## Consequences\n\n"
        f"- `work/` is the single source of truth for planning.\n"
        f"- Item-level narrative lives in each work item's README; finer-grained detail\n"
        f"  files live in git history at `{sha}` and can be restored if needed.\n"
    )
    report["decisions"].append(str(path.relative_to(root)).replace("\\", "/"))
    if not dry_run:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    return did


def migrated_sources(root: Path) -> set[str]:
    """Source paths recorded by import-plan in work items' `source` field."""
    out: set[str] = set()
    for manifest in root.glob("work/*/*/work-item.json"):
        try:
            src = json.loads(manifest.read_text(encoding="utf-8")).get("source")
        except (OSError, ValueError):
            continue
        if src:
            out.add(str(src).replace("\\", "/"))
    return out


def plan_dir_status(root: Path) -> list[tuple[Path, list[str], list[str]]]:
    """For each present plan directory: (dir, migrated source_rels, unmigrated source_rels)."""
    migrated = migrated_sources(root)
    result = []
    for name in plan_import.PLAN_DIR_NAMES:
        d = root / name
        if not d.is_dir():
            continue
        items = plan_import.collect_from_dir(d, root)
        done = [it.source_rel for it in items if it.source_rel in migrated]
        todo = [it.source_rel for it in items if it.source_rel not in migrated]
        result.append((d, done, todo))
    return result


# Mixed-content or RepoPact-owned artifacts we never auto-retire; report for review.
_REVIEW_HINTS = ("tracking", "ROADMAP.md", "BACKLOG.md", "CHANGELOG.md", "PLAN.md")


def registry_scopes_inside(root: Path, d: Path) -> list[str]:
    """Audit-scope paths or contracts registered inside ``d`` (``audits/registry.json``).

    Retiring a directory that an audit scope points into would dangle that scope and
    break validation, so takeover refuses until the operator relocates it.
    """
    reg = root / "audits" / "registry.json"
    if not reg.is_file():
        return []
    try:
        data = json.loads(reg.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return []
    rel = str(d.relative_to(root)).replace("\\", "/").rstrip("/")
    prefix = rel + "/"
    hits: set[str] = set()
    for scope in data.get("scopes", []):
        for key in ("path", "contract"):
            value = str(scope.get(key, "")).replace("\\", "/")
            if value == rel or value.startswith(prefix):
                hits.add(value)
    return sorted(hits)


def takeover(root: Path, delete: bool = False, dry_run: bool = False) -> dict:
    report: dict = {"validated": True, "retired": [], "skipped": [], "review": [],
                    "actions": [], "decisions": [], "downgraded": [], "blocked": []}

    problems = validate_repo.validate(root)
    if problems:
        report["validated"] = False
        report["problems"] = [f"{p.path.relative_to(root)}: {p.message}" for p in problems]
        return report

    archive_root = root / "archive"
    for d, done, todo in plan_dir_status(root):
        rel = d.name
        if todo:
            report["skipped"].append({"dir": rel, "migrated": len(done), "unmigrated": todo})
            continue
        if not done:
            continue  # empty / nothing imported from here; leave it
        scoped = registry_scopes_inside(root, d)
        if scoped:
            # Retiring the dir would leave a registered audit scope (or its contract)
            # dangling and break validation; the operator must relocate it first.
            report["blocked"].append({"dir": rel, "scopes": scoped})
            continue
        if delete:
            ok, sha, why = git_recoverable(root, d)
            if ok:
                did = write_retire_decision(root, rel, len(done), sha, dry_run, report)
                report["actions"].append(
                    f"delete {rel}/ ({len(done)} migrated item(s); documented in "
                    f"decisions/{did}-*, recoverable at {sha[:10]})")
                if not dry_run:
                    shutil.rmtree(d)
            else:
                # Deleting would not be recoverable (the git caveat), so fall back to
                # archiving rather than risk losing un-committed content.
                dest = archive_root / rel
                report["downgraded"].append({"dir": rel, "reason": why})
                report["actions"].append(
                    f"archive {rel}/ -> archive/{rel}/ ({len(done)} migrated item(s); "
                    f"delete downgraded to archive — {why})")
                if not dry_run:
                    archive_root.mkdir(exist_ok=True)
                    if dest.exists():
                        shutil.rmtree(dest)
                    shutil.move(str(d), str(dest))
        else:
            dest = archive_root / rel
            report["actions"].append(f"archive {rel}/ -> archive/{rel}/ ({len(done)} migrated item(s))")
            if not dry_run:
                archive_root.mkdir(exist_ok=True)
                if dest.exists():
                    shutil.rmtree(dest)
                shutil.move(str(d), str(dest))
        report["retired"].append(rel)

    for hint in _REVIEW_HINTS:
        p = root / hint
        if p.exists():
            report["review"].append(hint)

    if not dry_run and report["retired"]:
        report["post_validate_ok"] = not validate_repo.validate(root)
    return report


def _print(report: dict, dry_run: bool) -> int:
    if not report["validated"]:
        print("repopact takeover: repository does not validate; resolve these first:")
        for p in report.get("problems", []):
            print(f"  INVALID {p}")
        return 1
    verb = "Would " if dry_run else ""
    for a in report["actions"]:
        print(f"  ~ {verb}{a}")
    for dec in report.get("decisions", []):
        print(f"  + {verb}document why in {dec}")
    for dg in report.get("downgraded", []):
        print(f"  i {dg['dir']}/ not git-recoverable ({dg['reason']}) — archived instead of deleted")
    for s in report["skipped"]:
        print(f"  ! kept {s['dir']}/ — {len(s['unmigrated'])} item(s) not yet imported "
              f"(run `repopact import-plan` first): {', '.join(s['unmigrated'][:5])}"
              + (" …" if len(s['unmigrated']) > 5 else ""))
    for b in report.get("blocked", []):
        print(f"  ! kept {b['dir']}/ — audit scope(s) registered inside "
              f"(relocate in audits/registry.json first): {', '.join(b['scopes'][:5])}"
              + (" …" if len(b['scopes']) > 5 else ""))
    if report["review"]:
        print("  i review (not auto-retired; may hold un-migrated content): " + ", ".join(report["review"]))
    if not report["actions"] and not report["skipped"]:
        print("repopact takeover: no legacy plan directories to retire.")
    if report.get("post_validate_ok") is False:
        print("\nWARNING: repository no longer validates after retirement — inspect.")
        return 1
    if not dry_run and report["retired"]:
        print(f"\nRetired {len(report['retired'])} legacy plan dir(s); repository still validates.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Retire legacy planning sources RepoPact has fully imported")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--delete", action="store_true",
                        help="Delete instead of archiving (git-guarded; writes a decisions/ ADR; "
                             "downgrades to archive if not recoverable from git)")
    parser.add_argument("--dry-run", action="store_true", help="Report the plan without changing files")
    args = parser.parse_args()
    report = takeover(args.root.resolve(), delete=args.delete, dry_run=args.dry_run)
    rc = _print(report, args.dry_run)
    if args.dry_run:
        print("\nDry run: nothing changed.")
    return rc


if __name__ == "__main__":
    sys.exit(main())
