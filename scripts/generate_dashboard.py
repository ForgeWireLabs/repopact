from __future__ import annotations

from collections import Counter
from datetime import date
from pathlib import Path

from repo_model import STATUSES, discover_evidence_ids, discover_work_items, load_json


def generate(root: Path) -> str:
    items = discover_work_items(root)
    counts = Counter(item.status for item in items)
    contracts = list(root.rglob("AGENTS.md"))
    evidence_count = len(discover_evidence_ids(root))
    audit_entries = load_json(root / "audits" / "registry.json").get("scopes", [])

    lines = [
        "# Repository Dashboard",
        "",
        "> Generated from source records. Do not edit manually.",
        f"> Generated: {date.today().isoformat()}",
        "",
        "## Health",
        "",
        "| Metric | Count |",
        "| --- | ---: |",
        f"| Scope contracts | {len(contracts)} |",
        f"| Audit registry entries | {len(audit_entries)} |",
        f"| Evidence runs | {evidence_count} |",
        "",
        "## Work",
        "",
        "| Status | Count |",
        "| --- | ---: |",
    ]
    lines.extend(f"| {status} | {counts[status]} |" for status in STATUSES)
    lines.extend(["", "## Active items", ""])
    active = [item for item in items if item.status in ("active", "blocked")]
    if not active:
        lines.append("No active or blocked work.")
    else:
        lines.extend(f"- {item.item_id}: {item.data['title']} ({item.status})" for item in active)
    return "\n".join(lines) + "\n"


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    output = root / "audits" / "reports" / "dashboard.md"
    output.write_text(generate(root), encoding="utf-8")
    print(f"Generated {output.relative_to(root)}")


if __name__ == "__main__":
    main()

