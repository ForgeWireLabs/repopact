"""Conformance corpus test (work items 006 and 019).

Validates the published suite under conformance/: the valid baseline must be
accepted, and each invalid overlay must be rejected with its declared message.
Fixtures do not vendor schemas; the canonical schemas/ are injected here so a
fixture can never pass against a stale schema copy (no drift).
"""

from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

import jsonschema

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from validate_repo import validate  # noqa: E402
import generate_dashboard  # noqa: E402

import run_conformance  # noqa: E402

MANIFEST = ROOT / "conformance" / "manifest.json"
FIXTURES = ROOT / "conformance" / "fixtures"
VALID = FIXTURES / "valid"
INVALID = FIXTURES / "invalid"


def build_repo(dst: Path, overlay: Path | None = None) -> Path:
    shutil.copytree(VALID, dst)
    shutil.copytree(ROOT / "schemas", dst / "schemas")
    generate_dashboard.write_dashboard(dst)
    if overlay is not None:
        for src in overlay.rglob("*"):
            if src.is_dir() or src.name == "meta.json":
                continue
            target = dst / src.relative_to(overlay)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, target)
    return dst


class ConformanceTests(unittest.TestCase):
    def test_manifest_is_structurally_valid(self) -> None:
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        schema = json.loads((ROOT / "schemas" / "conformance-manifest.schema.json").read_text(encoding="utf-8"))
        jsonschema.Draft202012Validator(schema).validate(manifest)
        self.assertEqual((ROOT / "VERSION").read_text(encoding="utf-8").strip(), manifest["suite_version"])
        for case in manifest["cases"]:
            self.assertTrue((FIXTURES / case["path"]).exists(), case["path"])

    def test_manifest_matches_reference_suite(self) -> None:
        results = run_conformance.run_suite(
            f'"{sys.executable}" "{ROOT / "scripts" / "validate_repo.py"}" --root "{{repo}}"',
            MANIFEST,
        )
        self.assertTrue(results, "no conformance cases found")
        self.assertEqual([], [result.detail for result in results if not result.passed])

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
