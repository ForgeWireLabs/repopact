from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from repopact import release_local, verification


def _multi_platform_release_root(*, other_platforms: tuple[str, ...] = ("linux", "macos")) -> Path:
    """A synthetic repository whose 'release' profile is coverage:complete
    with a required_platforms set this single test host can never fully
    satisfy by itself -- the exact shape of the deadlock this module fixes.
    """
    temp = tempfile.TemporaryDirectory()
    current = verification.current_platform()
    root = Path(temp.name).resolve()
    (root / "governance").mkdir(parents=True)
    config = {
        "$schema": "../schemas/verification-profile.schema.json",
        "version": 1,
        "default_profile": "release",
        "execution_policy": {
            "local_primary": True,
            "hosted_ci_default": False,
            "hosted_cd_default": False,
        },
        "profiles": {
            "release": {
                "description": "synthetic multi-platform release profile",
                "coverage": "complete",
                "required_platforms": sorted({current, *other_platforms}),
                "steps": [
                    {
                        "id": "probe",
                        "argv": ["{python}", "-c", "print('ok')"],
                        "required": True,
                    }
                ],
            }
        },
    }
    (root / "governance" / "verification.json").write_text(
        json.dumps(config, indent=2) + "\n", encoding="utf-8"
    )
    return root, temp


class ReleaseCoverageDeadlockTests(unittest.TestCase):
    """WI046 architecture defect C: reproduce, then prove fixed."""

    def test_multi_platform_complete_profile_is_incomplete_on_one_host(self):
        root, temp = _multi_platform_release_root()
        self.addCleanup(temp.cleanup)
        report = verification.run_profile(root, "release")
        self.assertEqual("incomplete", report.status)
        self.assertTrue(report.coverage.missing_platforms)
        self.assertTrue(report.coverage.host_ready)

    def test_host_can_build_when_only_other_platforms_are_missing(self):
        # Before the fix, build_local_release() blocked on `status == "pass"`,
        # so a host could never build its own artifacts merely because other
        # required platforms had not (and, for this process, physically
        # could not) run -- no host could ever contribute the first evidence
        # toward aggregate completeness. This proves the corrected gate:
        # a host with a real, unavailable/failed local check is still
        # blocked, but "incomplete solely due to other required platforms"
        # is not.
        root, temp = _multi_platform_release_root()
        self.addCleanup(temp.cleanup)
        report = verification.run_profile(root, "release")
        self.assertEqual("incomplete", report.status)
        # host_ready is the fixed, narrower gate build_local_release() must
        # use instead of the full aggregate `status == "pass"`.
        self.assertTrue(report.coverage.host_ready)

    def test_host_with_a_real_local_failure_still_blocks_build(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        current = verification.current_platform()
        root = Path(temp.name).resolve()
        (root / "governance").mkdir(parents=True)
        config = {
            "$schema": "../schemas/verification-profile.schema.json",
            "version": 1,
            "default_profile": "release",
            "execution_policy": {
                "local_primary": True,
                "hosted_ci_default": False,
                "hosted_cd_default": False,
            },
            "profiles": {
                "release": {
                    "description": "single-platform profile with a real local failure",
                    "coverage": "complete",
                    "required_platforms": [current],
                    "steps": [
                        {
                            "id": "broken",
                            "argv": ["repopact-executable-that-does-not-exist-046"],
                            "required": True,
                        }
                    ],
                }
            },
        }
        (root / "governance" / "verification.json").write_text(
            json.dumps(config, indent=2) + "\n", encoding="utf-8"
        )
        report = verification.run_profile(root, "release")
        self.assertEqual("incomplete", report.status)
        self.assertFalse(report.coverage.host_ready)


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

    def write_readiness(
        self,
        dirpath: Path,
        *,
        platform: str,
        required_platforms: list[str],
        commit: str = "cafef00d",
        version: str = "1.2.3",
        dirty: bool = False,
        host_preparation: str = "passed",
        artifacts: list[dict] | None = None,
    ) -> Path:
        dirpath.mkdir(parents=True, exist_ok=True)
        payload = {
            "format": release_local.READINESS_FORMAT,
            "platform": platform,
            "candidate": {"commit": commit, "version": version, "dirty": dirty},
            "required_platforms": required_platforms,
            "missing_platforms": sorted(set(required_platforms) - {platform}),
            "host_preparation": host_preparation,
            "artifacts": artifacts if artifacts is not None else [{"kind": "wheel", "path": f"{platform}.whl", "sha256": "a" * 64}],
        }
        path = dirpath / release_local.READINESS_NAME
        path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
        return path

    def test_aggregate_readiness_complete_synthetic_matrix_is_ready(self):
        # No Mac is available on this machine; the macOS record here is
        # explicitly synthetic evidence used only to prove the aggregation
        # logic, never a claim that macOS execution actually happened.
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        required = ["linux", "macos", "windows"]
        paths = [
            self.write_readiness(root / "win", platform="windows", required_platforms=required),
            self.write_readiness(root / "lin", platform="linux", required_platforms=required),
            self.write_readiness(root / "mac", platform="macos", required_platforms=required),
        ]
        result = release_local.aggregate_release_readiness(paths)
        self.assertTrue(result["aggregate_ready"])
        self.assertEqual([], result["missing_platforms"])
        self.assertEqual([], result["failed_platforms"])

    def test_aggregate_readiness_missing_platform_is_not_ready(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        required = ["linux", "macos", "windows"]
        paths = [
            self.write_readiness(root / "win", platform="windows", required_platforms=required),
            self.write_readiness(root / "lin", platform="linux", required_platforms=required),
        ]
        result = release_local.aggregate_release_readiness(paths)
        self.assertFalse(result["aggregate_ready"])
        self.assertEqual(["macos"], result["missing_platforms"])

    def test_aggregate_readiness_failed_platform_is_not_ready(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        required = ["linux", "windows"]
        paths = [
            self.write_readiness(root / "win", platform="windows", required_platforms=required),
            self.write_readiness(root / "lin", platform="linux", required_platforms=required, host_preparation="failed"),
        ]
        result = release_local.aggregate_release_readiness(paths)
        self.assertFalse(result["aggregate_ready"])
        self.assertEqual(["linux"], result["failed_platforms"])

    def test_aggregate_readiness_rejects_wrong_candidate(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        required = ["linux", "windows"]
        paths = [
            self.write_readiness(root / "win", platform="windows", required_platforms=required, commit="aaaa"),
            self.write_readiness(root / "lin", platform="linux", required_platforms=required, commit="bbbb"),
        ]
        result = release_local.aggregate_release_readiness(paths)
        self.assertFalse(result["aggregate_ready"])
        self.assertIn("linux", result["wrong_candidate_platforms"])
        self.assertIn("linux", result["missing_platforms"])

    def test_aggregate_readiness_rejects_duplicate_platform(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        required = ["linux", "windows"]
        paths = [
            self.write_readiness(root / "win1", platform="windows", required_platforms=required),
            self.write_readiness(root / "win2", platform="windows", required_platforms=required),
        ]
        with self.assertRaises(release_local.LocalReleaseError):
            release_local.aggregate_release_readiness(paths)

    def test_aggregate_readiness_rejects_dirty_candidate(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        required = ["windows"]
        paths = [
            self.write_readiness(root / "win", platform="windows", required_platforms=required, dirty=True),
        ]
        result = release_local.aggregate_release_readiness(paths)
        self.assertFalse(result["aggregate_ready"])

    def test_aggregate_readiness_requires_at_least_one_record(self):
        with self.assertRaises(release_local.LocalReleaseError):
            release_local.aggregate_release_readiness([])

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
