from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from repopact import release_local


class LocalReleaseTests(unittest.TestCase):
    def make_dist(self) -> Path:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name).resolve()
        wheel = root / "repopact-test.whl"
        sdist = root / "repopact-test.tar.gz"
        wheel.write_bytes(b"wheel")
        sdist.write_bytes(b"sdist")
        manifest = {
            "format": "repopact-local-release-manifest-v1",
            "commit": "deadbeef",
            "version": "1.2.3",
            "artifact_version": "1.2.3",
            "reproducible": True,
            "artifacts": [
                {"kind": "wheel", "path": wheel.name, "sha256": hashlib.sha256(wheel.read_bytes()).hexdigest()},
                {"kind": "sdist", "path": sdist.name, "sha256": hashlib.sha256(sdist.read_bytes()).hexdigest()},
            ],
            "publication": {"performed": False},
        }
        (root / release_local.MANIFEST_NAME).write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        return root

    def test_manifest_hashes_are_verified(self):
        dist = self.make_dist()
        manifest = release_local.verify_manifest(dist)
        self.assertEqual("1.2.3", manifest["version"])

    def test_manifest_rejects_tampered_artifact(self):
        dist = self.make_dist()
        (dist / "repopact-test.whl").write_bytes(b"tampered")
        with self.assertRaises(release_local.LocalReleaseError):
            release_local.verify_manifest(dist)

    def test_manifest_rejects_path_escape(self):
        dist = self.make_dist()
        path = dist / release_local.MANIFEST_NAME
        manifest = json.loads(path.read_text(encoding="utf-8"))
        manifest["artifacts"][0]["path"] = "../escape.whl"
        path.write_text(json.dumps(manifest), encoding="utf-8")
        with self.assertRaises(release_local.LocalReleaseError):
            release_local.verify_manifest(dist)

    def test_manifest_missing_fails_closed(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        dist = Path(temp.name).resolve()
        with self.assertRaises(release_local.LocalReleaseError):
            release_local.verify_manifest(dist)

    def test_manifest_missing_artifact_file_fails_closed(self):
        dist = self.make_dist()
        (dist / "repopact-test.whl").unlink()
        with self.assertRaises(release_local.LocalReleaseError):
            release_local.verify_manifest(dist)

    def test_manifest_rejects_duplicate_artifact_record(self):
        dist = self.make_dist()
        path = dist / release_local.MANIFEST_NAME
        manifest = json.loads(path.read_text(encoding="utf-8"))
        manifest["artifacts"].append(dict(manifest["artifacts"][0]))
        path.write_text(json.dumps(manifest), encoding="utf-8")
        with self.assertRaises(release_local.LocalReleaseError):
            release_local.verify_manifest(dist)

    def test_manifest_rejects_unsupported_format(self):
        dist = self.make_dist()
        path = dist / release_local.MANIFEST_NAME
        manifest = json.loads(path.read_text(encoding="utf-8"))
        manifest["format"] = "some-other-format-v2"
        path.write_text(json.dumps(manifest), encoding="utf-8")
        with self.assertRaises(release_local.LocalReleaseError):
            release_local.verify_manifest(dist)

    def test_publish_requires_explicit_confirmation(self):
        dist = self.make_dist()
        with self.assertRaises(release_local.LocalReleaseError):
            release_local.publish_local_release(dist, confirm=False, dry_run=True)

    def test_publish_dry_run_never_contacts_provider(self):
        dist = self.make_dist()
        result = release_local.publish_local_release(dist, confirm=True, dry_run=True)
        self.assertEqual("dry-run", result["status"])
        self.assertIn("twine", result["command"])
        self.assertIn("external operator environment/keyring", result["credential_source"])


if __name__ == "__main__":
    unittest.main()
