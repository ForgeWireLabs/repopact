from __future__ import annotations

import json
import sys
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path

from repopact import verification


class VerificationRunnerTests(unittest.TestCase):
    def make_root(self, config: dict) -> Path:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name).resolve()
        (root / "governance").mkdir(parents=True)
        (root / "governance" / "verification.json").write_text(
            json.dumps(config, indent=2) + "\n", encoding="utf-8"
        )
        return root

    def config(self, steps: list[dict], *, coverage: str | None = None) -> dict:
        profile = {
            "description": "test profile",
            "steps": steps,
        }
        if coverage is not None:
            profile["coverage"] = coverage
        return {
            "$schema": "../schemas/verification-profile.schema.json",
            "version": 1,
            "default_profile": "test",
            "execution_policy": {
                "local_primary": True,
                "hosted_ci_default": False,
                "hosted_cd_default": False,
            },
            "profiles": {"test": profile},
        }

    def add_work_item(self, root: Path, work_item: str = "046") -> None:
        directory = root / "work" / "active" / f"{work_item}-verification"
        directory.mkdir(parents=True)
        (directory / "work-item.json").write_text(
            json.dumps({"id": work_item}) + "\n", encoding="utf-8"
        )

    def test_default_contract_is_local_first_and_hosted_off(self):
        config = verification.default_verification_config()
        self.assertTrue(config["execution_policy"]["local_primary"])
        self.assertFalse(config["execution_policy"]["hosted_ci_default"])
        self.assertFalse(config["execution_policy"]["hosted_cd_default"])
        self.assertEqual("host", config["profiles"]["governance"]["coverage"])

    def test_command_profile_passes_without_shell(self):
        root = self.make_root(
            self.config(
                [
                    {
                        "id": "probe",
                        "argv": ["{python}", "-c", "print('ok')"],
                        "required": True,
                    }
                ]
            )
        )
        report = verification.run_profile(root, "test")
        self.assertEqual("pass", report.status)
        self.assertEqual(0, report.exit_code)
        self.assertEqual("passed", report.steps[0].status)
        self.assertIn("ok", report.steps[0].stdout)
        self.assertTrue(report.coverage.satisfied)
        self.assertTrue(report.coverage.all_declared_executed)

    def test_required_nonzero_is_failure(self):
        root = self.make_root(
            self.config(
                [
                    {
                        "id": "probe",
                        "argv": ["{python}", "-c", "raise SystemExit(7)"],
                        "required": True,
                    }
                ]
            )
        )
        report = verification.run_profile(root, "test")
        self.assertEqual("fail", report.status)
        self.assertEqual(1, report.exit_code)
        self.assertEqual(7, report.steps[0].exit_code)
        self.assertEqual(1, report.coverage.required_failed)

    def test_required_missing_executable_is_incomplete(self):
        root = self.make_root(
            self.config(
                [
                    {
                        "id": "missing",
                        "argv": ["repopact-executable-that-does-not-exist-046"],
                        "required": True,
                    }
                ]
            )
        )
        report = verification.run_profile(root, "test")
        self.assertEqual("incomplete", report.status)
        self.assertEqual(2, report.exit_code)
        self.assertEqual("unavailable", report.steps[0].status)
        self.assertFalse(report.coverage.satisfied)
        self.assertEqual(1, report.coverage.required_unavailable)

    def test_declared_capability_is_aggregated(self):
        root = self.make_root(
            self.config(
                [
                    {
                        "id": "missing-capability",
                        "argv": ["repopact-executable-that-does-not-exist-046"],
                        "capability": "repopact-executable-that-does-not-exist-046",
                        "required": True,
                    }
                ]
            )
        )
        report = verification.run_profile(root, "test")
        self.assertEqual(
            "unavailable",
            report.coverage.capabilities["repopact-executable-that-does-not-exist-046"],
        )

    def test_optional_failure_does_not_fail_profile(self):
        root = self.make_root(
            self.config(
                [
                    {
                        "id": "required",
                        "argv": ["{python}", "-c", "raise SystemExit(0)"],
                        "required": True,
                    },
                    {
                        "id": "advisory",
                        "argv": ["{python}", "-c", "raise SystemExit(9)"],
                        "required": False,
                    },
                ]
            )
        )
        report = verification.run_profile(root, "test")
        self.assertEqual("pass", report.status)
        self.assertEqual("failed", report.steps[1].status)

    def test_platform_mismatch_is_explicit_skip_under_host_coverage(self):
        current = verification.current_platform()
        other = next(platform for platform in sorted(verification.KNOWN_PLATFORMS) if platform != current)
        root = self.make_root(
            self.config(
                [
                    {
                        "id": "other-platform",
                        "argv": ["{python}", "-c", "raise SystemExit(99)"],
                        "platforms": [other],
                        "required": True,
                    }
                ],
                coverage="host",
            )
        )
        report = verification.run_profile(root, "test")
        self.assertEqual("pass", report.status)
        self.assertEqual("skipped", report.steps[0].status)
        self.assertIn(current, report.steps[0].summary)
        self.assertTrue(report.coverage.satisfied)
        self.assertFalse(report.coverage.all_declared_executed)
        self.assertEqual(1, report.coverage.required_not_applicable)

    def test_complete_coverage_is_incomplete_when_required_platform_did_not_run(self):
        current = verification.current_platform()
        other = next(platform for platform in sorted(verification.KNOWN_PLATFORMS) if platform != current)
        root = self.make_root(
            self.config(
                [
                    {
                        "id": "other-platform",
                        "argv": ["{python}", "-c", "raise SystemExit(99)"],
                        "platforms": [other],
                        "required": True,
                    }
                ],
                coverage="complete",
            )
        )
        report = verification.run_profile(root, "test")
        self.assertEqual("incomplete", report.status)
        self.assertFalse(report.coverage.satisfied)
        self.assertEqual((other,), report.coverage.declared_platforms)

    def test_repository_escape_cwd_is_rejected(self):
        root = self.make_root(
            self.config(
                [
                    {
                        "id": "escape",
                        "argv": ["{python}", "-c", "print('never')"],
                        "cwd": "../outside",
                    }
                ]
            )
        )
        with self.assertRaises(verification.VerificationConfigError):
            verification.load_contract(root)

    def test_unknown_placeholder_is_rejected(self):
        root = self.make_root(
            self.config(
                [
                    {
                        "id": "placeholder",
                        "argv": ["{provider_secret}", "oops"],
                    }
                ]
            )
        )
        with self.assertRaises(verification.VerificationConfigError):
            verification.load_contract(root)

    def test_metacharacters_are_literal_arguments_not_shell(self):
        marker = "x; echo this-must-not-be-a-shell"
        root = self.make_root(
            self.config(
                [
                    {
                        "id": "literal",
                        "argv": [
                            "{python}",
                            "-c",
                            "import sys; raise SystemExit(0 if sys.argv[1] == %r else 4)" % marker,
                            marker,
                        ],
                    }
                ]
            )
        )
        report = verification.run_profile(root, "test")
        self.assertEqual("pass", report.status)
        self.assertEqual(marker, report.steps[0].command[-1])

    def test_json_report_has_stable_status_and_coverage_fields(self):
        root = self.make_root(
            self.config([{"id": "probe", "argv": ["{python}", "-c", "pass"]}])
        )
        report = verification.run_profile(root)
        payload = json.loads(verification.render_json(report))
        self.assertEqual("test", payload["profile"])
        self.assertEqual("local", payload["executor"])
        self.assertEqual("pass", payload["status"])
        self.assertEqual(0, payload["exit_code"])
        self.assertEqual("host", payload["coverage"]["mode"])
        self.assertTrue(payload["coverage"]["satisfied"])
        self.assertEqual("probe", payload["steps"][0]["id"])

    def test_build_evidence_captures_profile_coverage_and_candidate_identity(self):
        root = self.make_root(
            self.config([{"id": "probe", "argv": ["{python}", "-c", "pass"]}])
        )
        self.add_work_item(root)
        report = verification.run_profile(root)
        record = verification.build_evidence(
            root,
            report,
            "046",
            evidence_id="20260912-test-verification",
            timestamp=datetime(2026, 9, 12, 20, 0, tzinfo=timezone.utc),
        )
        self.assertEqual("046", record["work_item"])
        self.assertEqual("passed", record["result"])
        self.assertEqual("concrete", record["provenance"])
        self.assertEqual("git-recording", record["timestamp_basis"])
        self.assertEqual("test", record["environment"]["verification"]["profile"])
        self.assertTrue(record["environment"]["verification"]["coverage"]["satisfied"])
        self.assertIn("candidate", record["environment"])
        self.assertEqual(1, len(record["commands"]))

    def test_record_evidence_is_immutable_and_does_not_invent_missing_work(self):
        root = self.make_root(
            self.config([{"id": "probe", "argv": ["{python}", "-c", "pass"]}])
        )
        report = verification.run_profile(root)
        with self.assertRaises(verification.VerificationConfigError):
            verification.record_evidence(
                root,
                report,
                "046",
                evidence_id="20260912-missing-work",
                refresh_dashboard=False,
            )
        self.add_work_item(root)
        path = verification.record_evidence(
            root,
            report,
            "046",
            evidence_id="20260912-local-profile",
            refresh_dashboard=False,
        )
        self.assertEqual("evidence/runs/20260912-local-profile.json", path)
        with self.assertRaises(verification.VerificationConfigError):
            verification.record_evidence(
                root,
                report,
                "046",
                evidence_id="20260912-local-profile",
                refresh_dashboard=False,
            )


if __name__ == "__main__":
    unittest.main()
