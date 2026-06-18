from __future__ import annotations

from collections import Counter
from datetime import date
from pathlib import Path

from repo_model import STATUSES, discover_evidence_ids, discover_work_items, iter_contracts, load_json


def _count_markdown_records(directory: Path) -> int:
    if not directory.is_dir():
        return 0
    return sum(1 for path in directory.glob("*.md") if path.name.upper() != "README.MD")


def _count_json_records(directory: Path) -> int:
    if not directory.is_dir():
        return 0
    return len(list(directory.glob("*.json")))


def _spec_version(root: Path) -> str:
    # Prefer a vendored RepoPact version marker (scripts/REPOPACT_VERSION) so an
    # adopter repository can use its root VERSION file for its own product version
    # without mislabelling the dashboard. Falls back to VERSION, which is the
    # RepoPact version in this repository.
    for candidate in (root / "scripts" / "REPOPACT_VERSION", root / "VERSION"):
        if candidate.is_file():
            return candidate.read_text(encoding="utf-8").strip()
    return "unknown"


def _overdue_scopes(root: Path, today: date) -> list[tuple[str, str]]:
    try:
        entries = load_json(root / "audits" / "registry.json").get("scopes", [])
    except (OSError, ValueError):
        return []
    overdue: list[tuple[str, str]] = []
    for entry in entries:
        next_review = str(entry.get("next_review", ""))
        try:
            if date.fromisoformat(next_review) < today:
                overdue.append((str(entry.get("path", "")), next_review))
        except ValueError:
            continue
    return overdue


def generate(root: Path) -> str:
    today = date.today()
    items = discover_work_items(root)
    counts = Counter(item.status for item in items)
    contracts = iter_contracts(root)
    evidence_count = len(discover_evidence_ids(root))
    audit_entries = load_json(root / "audits" / "registry.json").get("scopes", [])
    invariant_count = len(load_json(root / "governance" / "invariants.json").get("invariants", []))
    frozen_count = len(load_json(root / "governance" / "frozen-surface.json").get("protected", []))
    decision_count = _count_markdown_records(root / "decisions")
    policy_count = _count_markdown_records(root / "governance" / "policies")
    finding_count = _count_json_records(root / "audits" / "findings")
    overdue = _overdue_scopes(root, today)

    lines = [
        "# Repository Dashboard",
        "",
        "> Generated from source records. Do not edit manually.",
        f"> Generated: {today.isoformat()}",
        f"> RepoPact spec version: {_spec_version(root)}",
        "",
        "## Health",
        "",
        "| Metric | Count |",
        "| --- | ---: |",
        f"| Invariants | {invariant_count} |",
        f"| Frozen-surface entries | {frozen_count} |",
        f"| Scope contracts | {len(contracts)} |",
        f"| Audit registry entries | {len(audit_entries)} |",
        f"| Audit findings | {finding_count} |",
        f"| Decision records | {decision_count} |",
        f"| Policy records | {policy_count} |",
        f"| Evidence runs | {evidence_count} |",
        "",
        "## Work",
        "",
        "| Status | Count |",
        "| --- | ---: |",
    ]
    lines.extend(f"| {status} | {counts[status]} |" for status in STATUSES)

    lines.extend(["", "## Audit freshness", ""])
    if not overdue:
        lines.append("All audit scopes are within their review cadence.")
    else:
        lines.append("| Scope | Review was due |")
        lines.append("| --- | --- |")
        lines.extend(f"| {path} | {due} |" for path, due in overdue)

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
