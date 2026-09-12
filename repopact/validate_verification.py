"""Python compatibility validator for the optional WI046 verification contract.

Canonical product validation is Rust. This module deliberately remains small and
independent so the legacy-Python conformance comparator can continue checking the
same repository rule without making Python a second product authority.
"""

from __future__ import annotations

from pathlib import Path

from . import validate_repo
from . import verification


def validate(root: Path) -> list[validate_repo.Problem]:
    root = root.resolve()
    path = root / verification.CONFIG_REL
    if not path.is_file():
        return []
    try:
        verification.load_contract(root)
    except verification.VerificationConfigError as exc:
        return [validate_repo.Problem(path, str(exc))]
    return []
