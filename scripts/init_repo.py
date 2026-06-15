"""Bootstrap RepoPact into a new repository (work item 003, B1).

Writes the minimal set of valid source records into a target directory and copies
the schemas and tooling, then validates the result. The goal: ``init_repo.py
--target ../newrepo`` yields a directory that passes ``validate_repo.py``.
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from datetime import date, timedelta
from pathlib import Path

SOURCE = Path(__file__).resolve().parents[1]
LIFECYCLE = ("active", "blocked", "deferred", "completed")


def _write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def _json(path: Path, data: object) -> None:
    _write(path, json.dumps(data, indent=2) + "\n")


def bootstrap(target: Path, today: date | None = None) -> Path:
    today = today or date.today()
    next_review = (today + timedelta(days=90)).isoformat()
    target.mkdir(parents=True, exist_ok=True)

    # Tooling and schemas come from the installed RepoPact.
    shutil.copytree(SOURCE / "schemas", target / "schemas", dirs_exist_ok=True)
    (target / "scripts").mkdir(exist_ok=True)
    for script in SOURCE.glob("scripts/*.py"):
        shutil.copy2(script, target / "scripts" / script.name)

    _write(target / "VERSION", (SOURCE / "VERSION").read_text(encoding="utf-8"))
    _write(target / "AGENTS.md",
           "# Agent Contract\n\n"
           "The repository is the durable coordination surface. The invariants in\n"
           "`governance/invariants.json` are binding; weakening one requires operator\n"
           "approval. Read every `AGENTS.md` from root to the file you touch.\n")

    _write(target / "governance" / "charter.md",
           "# Charter\n\n## Principles (human judgment)\n\n"
           "1. Systems before sessions.\n2. Completion requires proof.\n\n"
           "## Invariants (binding)\n\nSee `invariants.json`.\n")
    _write(target / "governance" / "workflow.md",
           "# Operating Workflow\n\nCapture intent in a work item, resolve authority,\n"
           "implement in scope, produce evidence, reconcile, then transition state.\n")
    _json(target / "governance" / "invariants.json", {
        "$schema": "../schemas/invariants.schema.json",
        "version": 1,
        "invariants": [{
            "id": "INV-1",
            "statement": "No critical state exists only in conversation; it lives in versioned files.",
            "rationale": "State must be recoverable without a prior conversation.",
            "escalation": "If a task would leave load-bearing state only in chat, record it as a file first.",
            "enforced_by": None,
        }],
    })
    _json(target / "governance" / "frozen-surface.json", {
        "$schema": "../schemas/frozen-surface.schema.json",
        "version": 1,
        "protected": [{
            "glob": "governance/invariants.json",
            "reason": "Invariants are the pact; weakening requires operator approval.",
            "symbols": [],
        }],
    })
    _json(target / "governance" / "owners.json", {
        "version": 2,
        "scopes": [{"id": "governance", "paths": ["AGENTS.md", "governance/**", "schemas/**"], "owner": "governance-owner"}],
        "roles": [{"id": "governance-owner", "description": "Maintains the pact and schemas.", "scopes": ["governance"]}],
        "concurrency": {"enforce_disjoint_active_scopes": False},
    })

    _json(target / "audits" / "registry.json", {
        "version": 1,
        "scopes": [{
            "path": ".", "owner": "governance-owner", "contract": "AGENTS.md",
            "last_reviewed": today.isoformat(), "next_review": next_review,
            "alignment": "current", "notes": "Bootstrapped repository contract.",
        }],
    })

    for status in LIFECYCLE:
        (target / "work" / status).mkdir(parents=True, exist_ok=True)
    for empty in ("evidence/runs", "decisions", "governance/policies", "audits/findings", "audits/reports"):
        (target / empty).mkdir(parents=True, exist_ok=True)

    _write(target / "README.md",
           "# Repository\n\nBootstrapped with RepoPact. Run `python scripts/validate_repo.py`.\n")
    return target


def main() -> int:
    parser = argparse.ArgumentParser(description="Bootstrap RepoPact into a target directory")
    parser.add_argument("--target", type=Path, required=True)
    args = parser.parse_args()
    target = args.target.resolve()
    bootstrap(target)

    sys.path.insert(0, str(target / "scripts"))
    import validate_repo  # noqa: E402  (loaded from the freshly seeded repo)

    problems = validate_repo.validate(target)
    if problems:
        for problem in problems:
            print(f"ERROR {problem.path.relative_to(target)}: {problem.message}")
        print(f"\nBootstrap produced an invalid repository: {len(problems)} error(s).")
        return 1
    print(f"Bootstrapped a valid RepoPact at {target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
