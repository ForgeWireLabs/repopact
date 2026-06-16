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

    repopact takeover [--root PATH] [--delete] [--dry-run]
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path

import plan_import
import validate_repo


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


def takeover(root: Path, delete: bool = False, dry_run: bool = False) -> dict:
    report: dict = {"validated": True, "retired": [], "skipped": [], "review": [], "actions": []}

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
        if delete:
            report["actions"].append(f"delete {rel}/ ({len(done)} migrated item(s))")
            if not dry_run:
                shutil.rmtree(d)
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
    for s in report["skipped"]:
        print(f"  ! kept {s['dir']}/ — {len(s['unmigrated'])} item(s) not yet imported "
              f"(run `repopact import-plan` first): {', '.join(s['unmigrated'][:5])}"
              + (" …" if len(s['unmigrated']) > 5 else ""))
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
    parser.add_argument("--delete", action="store_true", help="Delete instead of archiving under archive/")
    parser.add_argument("--dry-run", action="store_true", help="Report the plan without changing files")
    args = parser.parse_args()
    report = takeover(args.root.resolve(), delete=args.delete, dry_run=args.dry_run)
    rc = _print(report, args.dry_run)
    if args.dry_run:
        print("\nDry run: nothing changed.")
    return rc


if __name__ == "__main__":
    sys.exit(main())
