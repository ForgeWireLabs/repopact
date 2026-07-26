"""Bootstrap RepoPact into a new repository (work item 003 B1; CLI in 005).

Writes the minimal set of valid source records into a target directory and copies
the schemas and templates, then validation passes through the installed package.
Works both from a source checkout and from an installed wheel, where seed data
is loaded from package resources.
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import date, timedelta
from importlib.resources import files
from importlib.resources.abc import Traversable
from pathlib import Path

from . import __version__
from . import generate_dashboard

HERE = Path(__file__).resolve().parent          # the installed/checked-out package
CHECKOUT = HERE.parent                          # repo root when running from a checkout
LIFECYCLE = ("proposed", "active", "blocked", "deferred", "completed")


def _seed_dir(name: str) -> Traversable:
    """Return packaged seed content through the standard resource API."""
    candidate = files("repopact").joinpath(name)
    if candidate.is_dir():
        return candidate
    raise FileNotFoundError(f"package resource '{name}' is missing")


def _copy_seed_dir(name: str, target: Path) -> None:
    """Copy one packaged resource tree into a repository."""
    for source in _seed_dir(name).iterdir():
        destination = target / source.name
        if source.is_dir():
            _copy_traversable(source, destination)
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(source.read_bytes())


def _copy_traversable(source: Traversable, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    for item in source.iterdir():
        target = destination / item.name
        if item.is_dir():
            _copy_traversable(item, target)
        else:
            target.write_bytes(item.read_bytes())


def _write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def _json(path: Path, data: object) -> None:
    _write(path, json.dumps(data, indent=2) + "\n")


def bootstrap(target: Path, today: date | None = None) -> Path:
    today = today or date.today()
    next_review = (today + timedelta(days=90)).isoformat()
    target.mkdir(parents=True, exist_ok=True)

    # Schemas and templates come from the installed RepoPact (or checkout). Tooling
    # is deliberately *not* copied: a seeded repository holds its own state and runs
    # the installed `repopact` command against it (decision 0029). Vendoring the
    # modules made a second, unversioned distribution channel whose only test ran it
    # in a mode it is never used in, which is how a seeded validator shipped broken.
    _copy_seed_dir("schemas", target / "schemas")
    _copy_seed_dir("templates", target / "templates")

    version = ((CHECKOUT / "VERSION").read_text(encoding="utf-8").strip()
               if (CHECKOUT / "VERSION").is_file() else __version__)
    _write(target / "VERSION", f"{version}\n")
    _write(target / "requirements.txt", "jsonschema>=4.20\n")
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
        # Mandatory preflight (decision 0021). The bootstrap date is the epoch: work created
        # after setup must carry a marker (`repopact new` stamps it); same-day setup records
        # and anything imported at adoption time are grandfathered.
        "preflight": {"enabled": True, "required_from_date": today.isoformat()},
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
           "# Repository\n\nBootstrapped with RepoPact. Run `repopact validate` to check "
           "the records, and `repopact dashboard` to regenerate the derived projection.\n")
    generate_dashboard.write_dashboard(target, today=today)
    return target


def main() -> int:
    parser = argparse.ArgumentParser(description="Bootstrap RepoPact into a target directory")
    parser.add_argument("--target", type=Path, required=True)
    args = parser.parse_args()
    target = args.target.resolve()
    bootstrap(target)

    from . import validate_repo

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
