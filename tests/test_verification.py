from __future__ import annotations

import json
import sys
import tempfile
import unittest
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

    def config(self, steps: list[dict]) -> dict:
        return {
            "$schema": "../schemas/verification-profile.schema.json",
            "version": 1,
            "default_profile": "test",
            "execution_policy": {
                "local_primary": True,
                "hosted_ci_default": False,
                "hosted_cd_default": False,
            },
            "profiles": {
                "test": {
                    "description": "test profile",
                    "steps": steps,
                }
            },
        }

    def test_default_contract_is_local_first_and_hosted_off(self):
        config = verification.default_verification_config()
        self.assertTrue(config["execution_policy"]["local_primary"])
        self.assertFalse(config["execution_policy"]["hosted_ci_default"])
        self.assertFalse(config["execution_policy"]["hosted_cd_default"])

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

    def test_platform_mismatch_is_explicit_skip(self):
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
                ]
            )
        )
        report = verification.run_profile(root, "test")
        self.assertEqual("pass", report.status)
        self.assertEqual("skipped", report.steps[0].status)
        self.assertIn(current, report.steps[0].summary)

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

    def test_json_report_has_stable_status_fields(self):
        root = self.make_root(
            self.config([{"id": "probe", "argv": ["{python}", "-c", "pass"]}])
        )
        report = verification.run_profile(root)
        payload = json.loads(verification.render_json(report))
        self.assertEqual("test", payload["profile"])
        self.assertEqual("local", payload["executor"])
        self.assertEqual("pass", payload["status"])
        self.assertEqual(0, payload["exit_code"])
        self.assertEqual("probe", payload["steps"][0]["id"])


if __name__ == "__main__":
    unittest.main()
