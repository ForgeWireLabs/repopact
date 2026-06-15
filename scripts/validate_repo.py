from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from datetime import date, datetime
from pathlib import Path

from frontmatter import FrontMatterError, parse_file
from repo_model import STATUSES, discover_evidence_ids, discover_work_items, load_json


REQUIRED_WORK_FIELDS = {
    "id", "title", "status", "owner_scope", "affected_scopes", "depends_on",
    "acceptance_criteria", "created", "updated",
}

DECISION_STATUSES = ("proposed", "accepted", "superseded", "deprecated")
POLICY_STATUSES = ("active", "retired")


@dataclass(frozen=True)
class Problem:
    path: Path
    message: str


def validate_dates(value: object, field: str, path: Path, problems: list[Problem]) -> None:
    try:
        date.fromisoformat(str(value))
    except ValueError:
        problems.append(Problem(path, f"{field} must be an ISO date"))


def registered_contracts(root: Path) -> set[Path]:
    """Contract directories declared in the audit registry.

    Coverage is declared once, in the registry, rather than re-asserted as a
    hand-maintained _audit triple beside every contract (INV-7).
    """
    try:
        data = load_json(root / "audits" / "registry.json")
    except (OSError, ValueError, json.JSONDecodeError):
        return set()
    result: set[Path] = set()
    for entry in data.get("scopes", []):
        contract = entry.get("contract")
        if contract:
            result.add((root / str(contract)).resolve().parent)
    return result


def validate_contracts(root: Path, problems: list[Problem]) -> None:
    contracts = sorted(root.rglob("AGENTS.md"))
    if root / "AGENTS.md" not in contracts:
        problems.append(Problem(root, "missing root AGENTS.md"))
    covered = registered_contracts(root)
    for contract in contracts:
        if contract.parent == root:
            continue
        # Every nested contract must be registered for audit coverage.
        if contract.parent.resolve() not in covered:
            problems.append(Problem(contract, "nested contract is not registered in audits/registry.json"))
        # An _audit companion is optional, but if present it must be complete.
        audit = contract.parent / "_audit"
        if audit.is_dir():
            for name in ("README.md", "inventory.md", "alignment-report.md"):
                if not (audit / name).is_file():
                    problems.append(Problem(contract, f"incomplete _audit companion, missing _audit/{name}"))


def validate_invariants(root: Path, problems: list[Problem]) -> None:
    path = root / "governance" / "invariants.json"
    try:
        data = load_json(path)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(path, str(exc)))
        return
    invariants = data.get("invariants")
    if not isinstance(invariants, list) or not invariants:
        problems.append(Problem(path, "invariants must be a non-empty list"))
        return
    seen: set[str] = set()
    for entry in invariants:
        if not isinstance(entry, dict):
            problems.append(Problem(path, "each invariant must be an object"))
            continue
        inv_id = str(entry.get("id", ""))
        if not re.fullmatch(r"INV-[0-9]+", inv_id):
            problems.append(Problem(path, f"invariant id '{inv_id}' must match INV-<n>"))
        elif inv_id in seen:
            problems.append(Problem(path, f"duplicate invariant id '{inv_id}'"))
        seen.add(inv_id)
        for field in ("statement", "rationale", "escalation"):
            if not str(entry.get(field, "")).strip():
                problems.append(Problem(path, f"invariant {inv_id} is missing {field}"))
        if "enforced_by" not in entry:
            problems.append(Problem(path, f"invariant {inv_id} must declare enforced_by (string or null)"))


def validate_frozen_surface(root: Path, problems: list[Problem]) -> None:
    path = root / "governance" / "frozen-surface.json"
    try:
        data = load_json(path)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(path, str(exc)))
        return
    protected = data.get("protected")
    if not isinstance(protected, list):
        problems.append(Problem(path, "protected must be a list"))
        return
    for entry in protected:
        if not isinstance(entry, dict) or not str(entry.get("glob", "")).strip():
            problems.append(Problem(path, "each protected entry needs a non-empty glob"))
        elif not str(entry.get("reason", "")).strip():
            problems.append(Problem(path, f"protected entry '{entry.get('glob')}' needs a reason"))


def validate_owners(root: Path, problems: list[Problem]) -> tuple[set[str], bool]:
    path = root / "governance" / "owners.json"
    try:
        data = load_json(path)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(path, str(exc)))
        return set(), False
    scopes = data.get("scopes", [])
    ids = [scope.get("id") for scope in scopes if isinstance(scope, dict)]
    if len(ids) != len(set(ids)):
        problems.append(Problem(path, "scope IDs must be unique"))
    scope_ids = {str(value) for value in ids if value}
    for role in data.get("roles", []):
        if not isinstance(role, dict):
            problems.append(Problem(path, "each role must be an object"))
            continue
        for scope in role.get("scopes", []):
            if scope not in scope_ids:
                problems.append(Problem(path, f"role '{role.get('id')}' references unknown scope '{scope}'"))
    concurrency = data.get("concurrency", {})
    enforce_disjoint = bool(concurrency.get("enforce_disjoint_active_scopes", False))
    return scope_ids, enforce_disjoint


def validate_work(root: Path, owner_scopes: set[str], enforce_disjoint: bool, problems: list[Problem]) -> set[str]:
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
        for scope in item.data.get("affected_scopes", []):
            if scope not in owner_scopes:
                problems.append(Problem(manifest, f"unknown affected_scope '{scope}'"))
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

    if enforce_disjoint:
        validate_disjoint_scopes(items, problems)
    return all_ids


def validate_disjoint_scopes(items: list, problems: list[Problem]) -> None:
    """No two non-terminal items may share an owner or affected scope."""
    active = [item for item in items if item.status in ("active", "blocked")]
    for i, left in enumerate(active):
        left_scopes = {left.data.get("owner_scope")} | set(left.data.get("affected_scopes", []))
        for right in active[i + 1:]:
            right_scopes = {right.data.get("owner_scope")} | set(right.data.get("affected_scopes", []))
            overlap = left_scopes & right_scopes
            if overlap:
                problems.append(Problem(
                    left.directory / "work-item.json",
                    f"active scope conflict with {right.item_id} on {', '.join(sorted(map(str, overlap)))}",
                ))


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


def _validate_records(root: Path, directory: Path, pattern: str, statuses: tuple[str, ...],
                      required: tuple[str, ...], problems: list[Problem]) -> dict[str, Path]:
    seen: dict[str, Path] = {}
    if not directory.is_dir():
        return seen
    for path in sorted(directory.glob("*.md")):
        if path.name.upper() == "README.MD":
            continue
        try:
            front = parse_file(path)
        except FrontMatterError as exc:
            problems.append(Problem(path, str(exc)))
            continue
        for field in required:
            if not str(front.get(field, "")).strip():
                problems.append(Problem(path, f"front matter missing '{field}'"))
        record_id = str(front.get("id", ""))
        if record_id and not re.fullmatch(pattern, record_id):
            problems.append(Problem(path, f"id '{record_id}' must match {pattern}"))
        if record_id and not path.name.startswith(f"{record_id}-"):
            problems.append(Problem(path, "filename prefix must match record id"))
        if record_id in seen:
            problems.append(Problem(path, f"duplicate id also used by {seen[record_id]}"))
        elif record_id:
            seen[record_id] = path
        if front.get("status") not in statuses:
            problems.append(Problem(path, f"status '{front.get('status')}' must be one of {', '.join(statuses)}"))
    return seen


def validate_decisions(root: Path, problems: list[Problem]) -> None:
    directory = root / "decisions"
    ids = _validate_records(root, directory, r"[0-9]{4,}", DECISION_STATUSES,
                            ("id", "title", "status", "date"), problems)
    for path in sorted(directory.glob("*.md")) if directory.is_dir() else []:
        if path.name.upper() == "README.MD":
            continue
        try:
            front = parse_file(path)
        except FrontMatterError:
            continue
        validate_dates(front.get("date"), "date", path, problems)
        supersedes = front.get("supersedes", [])
        if isinstance(supersedes, list):
            for target in supersedes:
                if target not in ids:
                    problems.append(Problem(path, f"supersedes unknown decision '{target}'"))


def validate_policies(root: Path, problems: list[Problem]) -> None:
    _validate_records(root, root / "governance" / "policies", r"[0-9]{3,}", POLICY_STATUSES,
                      ("id", "title", "status", "applies_to"), problems)


def validate(root: Path) -> list[Problem]:
    problems: list[Problem] = []
    validate_contracts(root, problems)
    validate_invariants(root, problems)
    validate_frozen_surface(root, problems)
    owner_scopes, enforce_disjoint = validate_owners(root, problems)
    work_ids = validate_work(root, owner_scopes, enforce_disjoint, problems)
    validate_evidence(root, work_ids, problems)
    validate_audit_registry(root, problems)
    validate_decisions(root, problems)
    validate_policies(root, problems)
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
