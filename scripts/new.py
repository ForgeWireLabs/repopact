"""Stamp a new record from a template (work item 003, B2).

    python scripts/new.py work-item "Harden the widget"
    python scripts/new.py decision "Adopt event sourcing"
    python scripts/new.py policy "No silent retries"

Fills the next free id and today's date, then writes the record to its canonical
location. The stamped record is a valid starting point; fill in the body.
"""

from __future__ import annotations

import argparse
import re
import sys
from datetime import date, datetime
from pathlib import Path

import generate_dashboard
from repo_model import STATUSES

ROOT = Path(__file__).resolve().parents[1]


def slugify(title: str) -> str:
    return re.sub(r"-+", "-", re.sub(r"[^a-z0-9]+", "-", title.lower())).strip("-")


def _next_numeric(paths: list[Path], width: int, start: int) -> str:
    used = []
    for path in paths:
        match = re.match(r"([0-9]+)", path.name)
        if match:
            used.append(int(match.group(1)))
    return str(max(used + [start - 1]) + 1).zfill(width)


def new_work_item(title: str, today: date, root: Path = ROOT, status: str = "active") -> Path:
    if status not in STATUSES:
        raise ValueError(f"unknown work-item status '{status}'")
    slug = slugify(title)
    existing = list((root / "work").glob("*/*/work-item.json"))
    item_id = _next_numeric([p.parent for p in existing], 3, 1)
    directory = root / "work" / status / f"{item_id}-{slug}"
    directory.mkdir(parents=True)
    # Mandatory preflight (decision 0021): creating the work item with `new` IS the
    # preflight act, so stamp the marker and concrete provenance up front. No work
    # should begin until this record exists and propagates through the pact.
    created_at = datetime.combine(today, datetime.min.time()).isoformat() + "Z"
    manifest = {
        "$schema": "../../../schemas/work-item.schema.json",
        "id": item_id, "title": title, "status": status,
        "owner_scope": "governance", "affected_scopes": [], "depends_on": [],
        "provenance": "concrete",
        "preflight": {
            "created_before_work_started": True,
            "created_at": created_at,
            "note": "Stamped by `repopact new`; the work item was recorded before implementation began.",
        },
        "acceptance_criteria": [{"id": "AC-1", "text": "TODO: observable outcome", "state": "pending", "evidence": []}],
        "created": today.isoformat(), "updated": today.isoformat(),
    }
    import json
    (manifest_path := directory / "work-item.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (directory / "README.md").write_text(
        (root / "templates" / "work-item.README.md").read_text(encoding="utf-8")
        .replace("NNN", item_id).replace("Title Of The Work", title), encoding="utf-8")
    generate_dashboard.write_dashboard(root, today=today)
    return manifest_path


def new_markdown(kind: str, title: str, today: date, root: Path = ROOT) -> Path:
    slug = slugify(title)
    if kind == "decision":
        directory, width, template = root / "decisions", 4, "decision.md"
    else:
        directory, width, template = root / "governance" / "policies", 3, "policy.md"
    record_id = _next_numeric(list(directory.glob("*.md")), width, 1)
    text = (root / "templates" / template).read_text(encoding="utf-8")
    text = text.replace("NNNN", record_id).replace("NNN", record_id)
    text = text.replace("YYYY-MM-DD", today.isoformat())
    text = text.replace("Decision Title", title).replace("Policy Title", title)
    path = directory / f"{record_id}-{slug}.md"
    path.write_text(text, encoding="utf-8")
    generate_dashboard.write_dashboard(root, today=today)
    return path


def main() -> int:
    parser = argparse.ArgumentParser(description="Stamp a new record from a template")
    parser.add_argument("kind", choices=["work-item", "decision", "policy"])
    parser.add_argument("title")
    parser.add_argument("--status", choices=STATUSES, default="active",
                        help="Lifecycle status for a new work item")
    args = parser.parse_args()
    today = date.today()
    if args.kind == "work-item":
        path = new_work_item(args.title, today, status=args.status)
    else:
        path = new_markdown(args.kind, args.title, today)
    print(f"Created {path.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
