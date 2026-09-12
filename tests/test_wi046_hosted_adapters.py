from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
GOVERNANCE = ROOT / ".github" / "workflows" / "governance.yml"
RELEASE = ROOT / ".github" / "workflows" / "release.yml"


class HostedAdapterContractTests(unittest.TestCase):
    def test_governance_adapter_is_explicitly_default_off_and_thin(self):
        text = GOVERNANCE.read_text(encoding="utf-8")
        self.assertIn("vars.REPOPACT_GITHUB_CI == 'true'", text)
        self.assertIn("python -m repopact.verify_cli ci --root .", text)
        self.assertNotIn("python -m unittest discover", text)
        self.assertNotIn("python -m repopact.run_conformance", text)
        self.assertNotIn("cargo test", text)
        self.assertNotIn("repopact dashboard", text)
        self.assertNotIn("repopact spec", text)
        self.assertNotIn("REPOPACT_GITHUB_CD", text)

    def test_release_adapter_is_independently_default_off_and_uses_local_build(self):
        text = RELEASE.read_text(encoding="utf-8")
        guard = "vars.REPOPACT_GITHUB_CD == 'true'"
        self.assertGreaterEqual(text.count(guard), 2)
        self.assertIn(
            "python -m repopact.release_local build --root . --outdir release-out",
            text,
        )
        self.assertNotIn("python -m build", text)
        self.assertNotIn("repopact release-build", text)
        self.assertNotIn("REPOPACT_GITHUB_CI", text)
        # Provider-specific OIDC is confined to the final hosted publication job.
        self.assertIn("id-token: write", text)
        self.assertIn("pypa/gh-action-pypi-publish@release/v1", text)

    def test_workflow_presence_does_not_encode_an_always_on_job(self):
        for path, variable in (
            (GOVERNANCE, "REPOPACT_GITHUB_CI"),
            (RELEASE, "REPOPACT_GITHUB_CD"),
        ):
            text = path.read_text(encoding="utf-8")
            self.assertIn(f"vars.{variable} == 'true'", text)
            self.assertNotIn("if: ${{ true }}", text)
            self.assertNotIn("if: ${{ always() }}", text)


if __name__ == "__main__":
    unittest.main()
