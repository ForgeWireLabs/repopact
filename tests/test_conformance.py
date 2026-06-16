"""Conformance corpus test (work item 006, issue #4).

Validates the fixtures under tests/fixtures/: the valid baseline must be accepted,
and each invalid overlay must be rejected with its declared message. Fixtures do
not vendor schemas; the canonical schemas/ are injected here so a fixture can never
pass against a stale schema copy (no drift).
"""

from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from validate_repo import validate  # noqa: E402

FIXTURES = ROOT / "tests" / "fixtures"
VALID = FIXTURES / "valid"
INVALID = FIXTURES / "invalid"


def build_repo(dst: Path, overlay: Path | None = None) -> Path:
    shutil.copytree(VALID, dst)
    shutil.copytree(ROOT / "schemas", dst / "schemas")
    if overlay is not None:
        for src in overlay.rglob("*"):
            if src.is_dir() or src.name == "meta.json":
                continue
            target = dst / src.relative_to(overlay)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, target)
    return dst


class ConformanceTests(unittest.TestCase):
    def test_valid_fixture_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = build_repo(Path(tmp) / "repo")
            self.assertEqual([], [p.message for p in validate(repo)])

    def test_invalid_fixtures_are_rejected(self) -> None:
        cases = sorted(d for d in INVALID.iterdir() if d.is_dir())
        self.assertTrue(cases, "no invalid fixtures found")
        for case in cases:
            with self.subTest(case=case.name):
                expect = json.loads((case / "meta.json").read_text(encoding="utf-8"))["expect"]
                with tempfile.TemporaryDirectory() as tmp:
                    repo = build_repo(Path(tmp) / "repo", overlay=case)
                    messages = [p.message for p in validate(repo)]
                    self.assertTrue(messages, f"{case.name}: expected rejection, got none")
                    self.assertTrue(
                        any(expect in m for m in messages),
                        f"{case.name}: expected '{expect}' in {messages}",
                    )


if __name__ == "__main__":
    unittest.main()
