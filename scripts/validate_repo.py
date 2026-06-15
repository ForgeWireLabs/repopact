from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from datetime import date, datetime
from pathlib import Path

from repo_model import STATUSES, discover_evidence_ids, discover_work_items, load_json


REQUIRED_WORK_FIELDS = {
    "id", "title", "status", "owner_scope", "affected_scopes", "depends_on",
    "acceptance_criteria", "created", "updated",
}


@dataclass(frozen=True)
class Problem:
    path: Path
    message: str


def validate_dates(value: object, field: str, path: Path, problems: list[Problem]) -> None:
    try:
        date.fromisoformat(str(value))
    except ValueError:
        problems.append(Problem(path, f"{field} must be an ISO date"))


def validate_contracts(root: Path, problems: list[Problem]) -> None:
    contracts = sorted(root.rglob("AGENTS.md"))
    if root / "AGENTS.md" not in contracts:
        problems.append(Problem(root, "missing root AGENTS.md"))
    for contract in contracts:
        if contract.parent == root:
            continue
        audit = contract.parent / "_audit"
        for name in ("README.md", "inventory.md", "alignment-report.md"):
            if not (audit / name).is_file():
                problems.append(Problem(contract, f"missing audit companion _audit/{name}"))


def validate_owners(root: Path, problems: list[Problem]) -> set[str]:
    path = root / "governance" / "owners.json"
    try:
        data = load_json(path)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(path, str(exc)))
        return set()
    scopes = data.get("scopes", [])
    ids = [scope.get("id") for scope in scopes if isinstance(scope, dict)]
    if len(ids) != len(set(ids)):
        problems.append(Problem(path, "scope IDs must be unique"))
    return {str(value) for value in ids if value}


def validate_work(root: Path, owner_scopes: set[str], problems: list[Problem]) -> set[str]:
    try:
        items = discover_work_items(root)
        evidence_ids = discover_evidence_ids(root)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(root, str(exc)))
        return set()

    seen: dict[str, Path] = {}
    for item in items:
        manifest = item.directory / "work-item.json"
        missing = REQUIRED_WORK_FIELDS - item.data.keys()
        if missing:
            problems.append(Problem(manifest, f"missing fields: {', '.join(sorted(missing))}"))
            continue

        expected_status = item.directory.parent.name
        if item.status != expected_status or item.status not in STATUSES:
            problems.append(Problem(manifest, f"status '{item.status}' does not match directory '{expected_status}'"))
        if not re.fullmatch(r"[0-9]{3,}", item.item_id):
            problems.append(Problem(manifest, "id must contain at least three digits"))
        elif item.item_id in seen:
            problems.append(Problem(manifest, f"duplicate id also used by {seen[item.item_id]}"))
        else:
            seen[item.item_id] = manifest

        if item.directory.name.split("-", 1)[0] != item.item_id:
            problems.append(Problem(manifest, "directory prefix must match work-item id"))
        if not (item.directory / "README.md").is_file():
            problems.append(Problem(item.directory, "missing README.md narrative"))
        if item.data["owner_scope"] not in owner_scopes:
            problems.append(Problem(manifest, f"unknown owner_scope '{item.data['owner_scope']}'"))
        validate_dates(item.data["created"], "created", manifest, problems)
        validate_dates(item.data["updated"], "updated", manifest, problems)

        criteria = item.data.get("acceptance_criteria")
        if not isinstance(criteria, list) or not criteria:
            problems.append(Problem(manifest, "acceptance_criteria must be a non-empty list"))
            continue
        criterion_ids: set[str] = set()
        for criterion in criteria:
            if not isinstance(criterion, dict):
                problems.append(Problem(manifest, "each acceptance criterion must be an object"))
                continue
            criterion_id = str(criterion.get("id", ""))
            if not criterion_id or criterion_id in criterion_ids:
                problems.append(Problem(manifest, "acceptance criterion IDs must be present and unique"))
            criterion_ids.add(criterion_id)
            state = criterion.get("state")
            linked = criterion.get("evidence", [])
            if state not in ("pending", "satisfied", "waived"):
                problems.append(Problem(manifest, f"criterion {criterion_id} has invalid state '{state}'"))
            if state == "satisfied" and not linked:
                problems.append(Problem(manifest, f"criterion {criterion_id} is satisfied without evidence"))
            for evidence_id in linked:
                if evidence_id not in evidence_ids:
                    problems.append(Problem(manifest, f"criterion {criterion_id} references unknown evidence '{evidence_id}'"))
            if item.status == "completed" and state == "pending":
                problems.append(Problem(manifest, f"completed item has pending criterion {criterion_id}"))

    all_ids = set(seen)
    for item in items:
        for dependency in item.data.get("depends_on", []):
            if dependency not in all_ids:
                problems.append(Problem(item.directory / "work-item.json", f"unknown dependency '{dependency}'"))
    return all_ids


def validate_evidence(root: Path, work_ids: set[str], problems: list[Problem]) -> None:
    seen: dict[str, Path] = {}
    for path in sorted((root / "evidence" / "runs").glob("*.json")):
        try:
            data = load_json(path)
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            problems.append(Problem(path, str(exc)))
            continue
        required = {"id", "timestamp", "work_item", "result", "commands", "artifacts", "environment"}
        missing = required - data.keys()
        if missing:
            problems.append(Problem(path, f"missing evidence fields: {', '.join(sorted(missing))}"))
            continue
        evidence_id = str(data["id"])
        if evidence_id != path.stem:
            problems.append(Problem(path, "evidence id must match filename"))
        if evidence_id in seen:
            problems.append(Problem(path, f"duplicate evidence id also used by {seen[evidence_id]}"))
        seen[evidence_id] = path
        try:
            datetime.fromisoformat(str(data["timestamp"]))
        except ValueError:
            problems.append(Problem(path, "timestamp must be ISO 8601"))
        if data["work_item"] not in work_ids:
            problems.append(Problem(path, f"unknown work_item '{data['work_item']}'"))
        if data["result"] not in ("passed", "failed", "partial", "blocked"):
            problems.append(Problem(path, f"invalid result '{data['result']}'"))
        commands = data["commands"]
        if not isinstance(commands, list) or not commands:
            problems.append(Problem(path, "commands must be a non-empty list"))
        else:
            for command in commands:
                if not isinstance(command, dict) or "command" not in command or not isinstance(command.get("exit_code"), int):
                    problems.append(Problem(path, "each command needs command text and integer exit_code"))


def validate_audit_registry(root: Path, problems: list[Problem]) -> None:
    path = root / "audits" / "registry.json"
    try:
        data = load_json(path)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(path, str(exc)))
        return
    for entry in data.get("scopes", []):
        scope_path = root if entry.get("path") == "." else root / str(entry.get("path", ""))
        if not scope_path.exists():
            problems.append(Problem(path, f"audit scope does not exist: {entry.get('path')}"))
        validate_dates(entry.get("last_reviewed"), "last_reviewed", path, problems)
        validate_dates(entry.get("next_review"), "next_review", path, problems)


def validate(root: Path) -> list[Problem]:
    problems: list[Problem] = []
    validate_contracts(root, problems)
    owner_scopes = validate_owners(root, problems)
    work_ids = validate_work(root, owner_scopes, problems)
    validate_evidence(root, work_ids, problems)
    validate_audit_registry(root, problems)
    return sorted(problems, key=lambda problem: (str(problem.path), problem.message))


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate repository governance records")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    root = args.root.resolve()
    problems = validate(root)
    if problems:
        for problem in problems:
            print(f"ERROR {problem.path.relative_to(root)}: {problem.message}")
        print(f"\nValidation failed with {len(problems)} error(s).")
        return 1
    print("Repository governance validation passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
