from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import fleet_verify  # noqa: E402


class FakeRemote:
    def __init__(self, heads: dict, files: dict) -> None:
        self.heads = heads
        self.files = files

    def resolve_head(self, repository: str, branch: str) -> str:
        value = self.heads.get((repository, branch))
        if isinstance(value, Exception):
            raise value
        if value is None:
            raise fleet_verify.RemoteFailure(f"unreachable {repository}@{branch}")
        return value

    def read_file(self, repository: str, revision: str, path: str) -> bytes:
        value = self.files.get((repository, revision, path))
        if isinstance(value, Exception):
            raise value
        if value is None:
            raise fleet_verify.RemoteFailure(f"missing {repository}@{revision}:{path}")
        return value


class FleetVerificationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / "repopact"
        (self.root / "schemas").mkdir(parents=True)
        (self.root / "governance" / "overlays").mkdir(parents=True)
        shutil.copy2(ROOT / "schemas" / "adopter-fleet.schema.json", self.root / "schemas")
        (self.root / "VERSION").write_text("2.2.0\n", encoding="utf-8")

        self.upstream_repo = "ForgeWireLabs/repopact"
        self.pypi_repo = "ForgeWireLabs/package-adopter"
        self.vendor_repo = "ForgeWireLabs/vendor-adopter"
        self.upstream_revision = "a" * 40
        self.pypi_head = "b" * 40
        self.vendor_head = "c" * 40
        self.exact = b"exact\n"
        self.base = b"base\n"
        self.adopted = b"base\ncustom\n"
        self.overlay = (
            b"--- upstream/tool.py\n"
            b"+++ adopter/tool.py\n"
            b"@@ -1 +1,2 @@\n"
            b" base\n"
            b"+custom\n"
        )
        (self.root / "governance" / "overlays" / "vendor.patch").write_bytes(self.overlay)
        self.manifest = {
            "$schema": "../schemas/adopter-fleet.schema.json",
            "version": 1,
            "upstream": {"repository": self.upstream_repo, "version_file": "VERSION"},
            "adopters": [
                {
                    "id": "package",
                    "repository": self.pypi_repo,
                    "default_branch": "main",
                    "consumption": {
                        "type": "pypi",
                        "package": "repopact",
                        "version_file": "requirements-repopact.txt",
                    },
                    "validation_commands": ["repopact validate"],
                },
                {
                    "id": "vendor",
                    "repository": self.vendor_repo,
                    "default_branch": "main",
                    "consumption": {
                        "type": "vendored",
                        "version_file": "scripts/REPOPACT_VERSION",
                        "upstream_version": "2.2.0",
                        "upstream_revision": self.upstream_revision,
                        "files": [
                            {
                                "upstream_path": "scripts/exact.py",
                                "adopter_path": "scripts/exact.py",
                                "mode": "exact",
                                "upstream_sha256": self._sha(self.exact),
                                "adopter_sha256": self._sha(self.exact),
                            },
                            {
                                "upstream_path": "scripts/tool.py",
                                "adopter_path": "scripts/tool.py",
                                "mode": "overlay",
                                "upstream_sha256": self._sha(self.base),
                                "adopter_sha256": self._sha(self.adopted),
                                "overlay_path": "governance/overlays/vendor.patch",
                                "overlay_sha256": self._sha(self.overlay),
                            },
                        ],
                    },
                    "validation_commands": ["python scripts/validate_repo.py"],
                },
            ],
        }
        self.heads = {
            (self.pypi_repo, "main"): self.pypi_head,
            (self.vendor_repo, "main"): self.vendor_head,
        }
        self.files = {
            (self.pypi_repo, self.pypi_head, "requirements-repopact.txt"): b"repopact==2.2.0\n",
            (self.vendor_repo, self.vendor_head, "scripts/REPOPACT_VERSION"): b"2.2.0\n",
            (self.upstream_repo, self.upstream_revision, "scripts/exact.py"): self.exact,
            (self.vendor_repo, self.vendor_head, "scripts/exact.py"): self.exact,
            (self.upstream_repo, self.upstream_revision, "scripts/tool.py"): self.base,
            (self.vendor_repo, self.vendor_head, "scripts/tool.py"): self.adopted,
        }
        self._write_manifest()

    def tearDown(self) -> None:
        self.temp.cleanup()

    @staticmethod
    def _sha(value: bytes) -> str:
        return hashlib.sha256(value).hexdigest()

    def _write_manifest(self) -> None:
        (self.root / "governance" / "adopters.json").write_text(
            json.dumps(self.manifest), encoding="utf-8"
        )

    def _verify(self):
        return fleet_verify.verify_fleet(
            self.root,
            discovery_roots=[],
            client=FakeRemote(self.heads, self.files),
        )

    def test_current_pypi_and_vendored_contracts_pass(self) -> None:
        report = self._verify()
        self.assertTrue(report.ok)
        vendor = next(result for result in report.adopters if result.adopter_id == "vendor")
        self.assertIn("exact checksum parity: scripts/exact.py", vendor.checks)
        self.assertIn("reviewable overlay parity: scripts/tool.py", vendor.checks)

    def test_zero_context_insertion_overlay_reconstructs_bytes(self) -> None:
        patch = b"--- upstream/x\n+++ adopter/x\n@@ -1,0 +2 @@\n+inserted\n"
        self.assertEqual(b"first\ninserted\nsecond\n", fleet_verify._apply_unified_patch(b"first\nsecond\n", patch))

    def test_stale_default_branch_pin_fails_closed(self) -> None:
        self.files[(self.pypi_repo, self.pypi_head, "requirements-repopact.txt")] = b"repopact==2.1.0\n"
        report = self._verify()
        self.assertFalse(report.ok)
        self.assertTrue(any("is stale" in error for error in report.adopters[0].errors))

    def test_missing_declared_version_file_fails_closed(self) -> None:
        del self.files[(self.pypi_repo, self.pypi_head, "requirements-repopact.txt")]
        report = self._verify()
        self.assertFalse(report.ok)
        self.assertTrue(any("missing" in error for error in report.adopters[0].errors))

    def test_unreachable_declared_branch_fails_closed(self) -> None:
        self.heads[(self.pypi_repo, "main")] = fleet_verify.RemoteFailure("network unavailable")
        report = self._verify()
        self.assertFalse(report.ok)
        self.assertIn("network unavailable", report.adopters[0].errors)

    def test_non_exact_pypi_requirement_is_rejected(self) -> None:
        self.files[(self.pypi_repo, self.pypi_head, "requirements-repopact.txt")] = b"repopact>=2.2.0\n"
        report = self._verify()
        self.assertFalse(report.ok)
        self.assertTrue(any("exact repopact==" in error for error in report.adopters[0].errors))

    def test_vendored_marker_alone_cannot_hide_drift(self) -> None:
        self.files[(self.vendor_repo, self.vendor_head, "scripts/exact.py")] = b"drift\n"
        report = self._verify()
        vendor = next(result for result in report.adopters if result.adopter_id == "vendor")
        self.assertFalse(vendor.ok)
        self.assertTrue(any("checksum drift" in error for error in vendor.errors))

    def test_duplicate_local_checkouts_collapse_to_one_remote_identity(self) -> None:
        discovery = Path(self.temp.name) / "checkouts"
        discovery.mkdir()
        for name in ("one", "two"):
            checkout = discovery / name
            checkout.mkdir()
            subprocess.run(["git", "init", "-q"], cwd=checkout, check=True)
            subprocess.run(
                ["git", "remote", "add", "origin", "https://github.com/ForgeWireLabs/package-adopter.git"],
                cwd=checkout,
                check=True,
            )
            (checkout / "requirements-repopact.txt").write_text("repopact==2.2.0\n", encoding="utf-8")
        grouped, candidates, errors = fleet_verify.discover_local_consumers([discovery])
        self.assertEqual(2, len(grouped["forgewirelabs/package-adopter"]))
        self.assertEqual(1, len(candidates))
        self.assertEqual((), errors)

    def test_release_closeout_keeps_publication_and_rollout_separate(self) -> None:
        fleet = self._verify()
        incomplete = fleet_verify.release_closeout(self.root, fleet, None)
        self.assertEqual("incomplete", incomplete["package_publication"]["status"])
        self.assertEqual("complete", incomplete["ecosystem_rollout"]["status"])
        evidence = self.root / "publication.json"
        evidence.write_text(
            json.dumps(
                {
                    "result": "passed",
                    "environment": {"release": "2.2.0"},
                    "artifacts": ["https://pypi.org/project/repopact/2.2.0/"],
                }
            ),
            encoding="utf-8",
        )
        complete = fleet_verify.release_closeout(self.root, fleet, evidence)
        self.assertEqual("pass", complete["status"])
        self.files[(self.pypi_repo, self.pypi_head, "requirements-repopact.txt")] = b"repopact==2.1.0\n"
        stale = fleet_verify.release_closeout(self.root, self._verify(), evidence)
        self.assertEqual("complete", stale["package_publication"]["status"])
        self.assertEqual("incomplete", stale["ecosystem_rollout"]["status"])


if __name__ == "__main__":
    unittest.main()
