"""Explicit Python compatibility validator used by conformance comparison.

This is not the product authority. It composes the historical Python repository
validator with small compatibility validators for semantic surfaces that migrated
after the Rust cutover, currently WI046 verification contracts.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from . import validate_repo
from . import validate_verification


def validate(root: Path) -> list[validate_repo.Problem]:
    root = root.resolve()
    problems = list(validate_repo.validate(root))
    problems.extend(validate_verification.validate(root))
    problems.sort(key=lambda item: (str(item.path), item.message))
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Run the explicit Python RepoPact compatibility validator"
    )
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)
    root = args.root.resolve()
    problems = validate(root)
    for problem in problems:
        try:
            path = problem.path.relative_to(root)
        except ValueError:
            path = problem.path
        print(f"ERROR {path}: {problem.message}")
    if problems:
        print(f"\nValidation failed with {len(problems)} error(s).")
        return 1
    print("Repository governance validation passed (Python compatibility comparator).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
