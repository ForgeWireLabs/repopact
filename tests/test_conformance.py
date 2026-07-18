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
        run_conformance.validate_manifest_coverage(manifest)
        self.assertEqual((ROOT / "VERSION").read_text(encoding="utf-8").strip(), manifest["suite_version"])
        for case in manifest["cases"]:
            self.assertTrue((FIXTURES / case["path"]).exists(), case["path"])

    def test_rule_coverage_is_bidirectional_and_every_fixture_is_declared(self) -> None:
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        declared = {case["path"] for case in manifest["cases"]}
        fixture_paths = {"valid"}
        fixture_paths.update(path.name for path in FIXTURES.iterdir() if path.is_dir() and path.name.startswith("valid-"))
        fixture_paths.update(f"invalid/{path.name}" for path in INVALID.iterdir() if path.is_dir())
        self.assertEqual(fixture_paths, declared)

        missing = json.loads(json.dumps(manifest))
        missing["cases"] = [case for case in missing["cases"] if "SPEC-4-version" not in case["rules"]]
        with self.assertRaisesRegex(ValueError, "rules without conformance cases: SPEC-4-version"):
            run_conformance.validate_manifest_coverage(missing)

        unknown = json.loads(json.dumps(manifest))
        unknown["cases"][0]["rules"].append("SPEC-999-unknown")
        with self.assertRaisesRegex(ValueError, "cases reference unknown rules: SPEC-999-unknown"):
            run_conformance.validate_manifest_coverage(unknown)

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
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        cases = [case for case in manifest["cases"] if case["expect"] == "reject"]
        self.assertTrue(cases, "no invalid fixtures found")
        for case in cases:
            with self.subTest(case=case["id"]):
                with tempfile.TemporaryDirectory() as tmp:
                    repo = run_conformance.materialize_case(Path(tmp), FIXTURES, case)
                    messages = [p.message for p in validate(repo)]
                    expect = case["expected_message"]
                    self.assertEqual(1, len(messages), f"{case['id']}: unexpected violations: {messages}")
                    self.assertIn(expect, messages[0])

    def test_runner_reports_unexpected_fixture_violations(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            suite = Path(tmp) / "conformance"
            shutil.copytree(ROOT / "conformance", suite)
            overlay = suite / "fixtures" / "invalid" / "completed-with-pending"
            (overlay / "VERSION").write_text("not-semver\n", encoding="utf-8")
            results = run_conformance.run_suite(
                f'"{sys.executable}" "{ROOT / "scripts" / "validate_repo.py"}" --root "{{repo}}"',
                suite / "manifest.json",
            )
        result = next(item for item in results if item.case_id == "completed-with-pending")
        self.assertFalse(result.passed)
        self.assertIn("unexpected violations", result.detail)
        self.assertIn("must be semantic", result.detail)


if __name__ == "__main__":
    unittest.main()
