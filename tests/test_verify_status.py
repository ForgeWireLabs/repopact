from __future__ import annotations

import json
import unittest
from pathlib import Path

from repopact import release_local, verify_status
from repopact.dev_fixtures import open_fixture_repo


class VerifyStatusTests(unittest.TestCase):
    def setUp(self) -> None:
        self.root = open_fixture_repo(self, prefix="repopact-verify-status-", init_git=True)

    def test_local_verification_section_reflects_contract(self) -> None:
        status = verify_status.build_status(self.root)
        self.assertTrue(status["local_verification"]["contract_present"])
        self.assertEqual(status["local_verification"]["default_profile"], "quick")

    def test_no_remote_call_is_ever_made_and_hosted_state_is_unknown(self) -> None:
        status = verify_status.build_status(self.root)
        for section in ("hosted_ci", "hosted_cd"):
            state = status[section]["actual_remote_state"]
            self.assertIn("unknown", state)
            self.assertNotIn("enabled", state)
            self.assertNotIn("enforced", state)

    def test_admission_state_never_claims_effectiveness_or_enforcement_without_evidence(self) -> None:
        status = verify_status.build_status(self.root)
        admission = status["admission_enforcement"]
        self.assertEqual(admission["effectiveness"], "not-proven")
        self.assertEqual(admission["enforcement_closure"], "not-proven")

    def test_admission_policy_present_still_does_not_claim_enforcement(self) -> None:
        governance = self.root / "governance"
        governance.mkdir(parents=True, exist_ok=True)
        (governance / "admission-policy.json").write_text(
            json.dumps({"enabled": True}), encoding="utf-8"
        )
        status = verify_status.build_status(self.root)
        admission = status["admission_enforcement"]
        self.assertEqual(admission["coverage"], "configured")
        self.assertTrue(admission["effectiveness"].startswith("not-proven"))
        self.assertTrue(admission["enforcement_closure"].startswith("not-proven"))

    def test_release_section_without_evidence_reports_not_evidenced(self) -> None:
        status = verify_status.build_status(self.root)
        release = status["release"]
        self.assertIn("not evidenced", release["host_preparation"])
        self.assertFalse(release["aggregate_ready"])
        self.assertEqual(sorted(release["required_platforms"]), ["linux", "macos", "windows"])
        self.assertEqual(sorted(release["missing_platforms"]), ["linux", "macos", "windows"])

    def test_release_section_folds_in_supplied_readiness_evidence(self) -> None:
        outdir = self.root / "readiness-linux"
        outdir.mkdir()
        payload = {
            "format": release_local.READINESS_FORMAT,
            "platform": "linux",
            "candidate": {"commit": "abc123", "version": "1.0.0", "dirty": False},
            "required_platforms": ["linux", "macos", "windows"],
            "missing_platforms": ["macos", "windows"],
            "host_preparation": "passed",
            "artifacts": [],
        }
        (outdir / release_local.READINESS_NAME).write_text(json.dumps(payload), encoding="utf-8")
        status = verify_status.build_status(self.root, release_evidence=[outdir / release_local.READINESS_NAME])
        release = status["release"]
        self.assertEqual(release["platforms_with_evidence"], ["linux"])
        self.assertEqual(release["missing_platforms"], ["macos", "windows"])
        self.assertFalse(release["aggregate_ready"])

    def test_render_json_and_human_do_not_crash_and_stay_consistent(self) -> None:
        status = verify_status.build_status(self.root)
        rendered_json = verify_status.render_json(status)
        rendered_human = verify_status.render_human(status)
        self.assertEqual(json.loads(rendered_json), status)
        self.assertIn("no remote API calls", rendered_human)

    def test_cli_status_subcommand_never_invokes_a_profile_named_status(self) -> None:
        from repopact import verify_cli

        exit_code = verify_cli.main(["status", "--root", str(self.root), "--json"])
        self.assertEqual(exit_code, 0)


if __name__ == "__main__":
    unittest.main()
