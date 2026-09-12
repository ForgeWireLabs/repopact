"""CLI entry point for WI046 local verification profiles.

This module exists independently of hosted CI. The public ``repopact verify``
command delegates here, while hosted adapters call the same local contract instead
of maintaining a second semantic command list.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .verification import (
    VerificationConfigError,
    record_evidence,
    render_human,
    render_json,
    run_profile,
)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Run a repository-defined RepoPact verification profile locally"
    )
    parser.add_argument(
        "profile",
        nargs="?",
        help="Profile name; defaults to governance/verification.json default_profile",
    )
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")
    parser.add_argument(
        "--evidence-work-item",
        metavar="ID",
        help="Record this actual invocation as immutable evidence for the named RepoPact work item",
    )
    parser.add_argument(
        "--evidence-id",
        help="Optional explicit evidence id; never overwrites an existing record",
    )
    args = parser.parse_args(argv)
    try:
        report = run_profile(args.root, args.profile)
        if args.evidence_work_item:
            path = record_evidence(
                args.root,
                report,
                args.evidence_work_item,
                evidence_id=args.evidence_id,
            )
            report = report.with_evidence(path)
        elif args.evidence_id:
            raise VerificationConfigError("--evidence-id requires --evidence-work-item")
    except VerificationConfigError as exc:
        print(f"RepoPact verification configuration error: {exc}", file=sys.stderr)
        return 2
    print(render_json(report) if args.json else render_human(report), end="")
    return report.exit_code


if __name__ == "__main__":
    raise SystemExit(main())
