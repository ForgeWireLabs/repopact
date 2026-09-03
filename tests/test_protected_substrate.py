from __future__ import annotations

import contextlib
import io
import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from repopact import cli
from repopact.guard import ProtectedGuard
from repopact.guard_ipc import decode, encode, envelope
from repopact.platform_backends import PrivilegeRequired, TestingBackend, WindowsBackend


class ProtectedSubstrateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="repopact-substrate-"))
        self.root = self.tmp / "repo"
        shutil.copytree(Path(__file__).parents[1], self.root, ignore=shutil.ignore_patterns(".git", "__pycache__", "*.pyc", "build", "dist"))
        subprocess.run(["git", "init"], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "config", "user.email", "substrate@example.invalid"], cwd=self.root, check=True)
        subprocess.run(["git", "config", "user.name", "RepoPact substrate"], cwd=self.root, check=True)
        subprocess.run(["git", "add", "."], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "commit", "-m", "substrate fixture"], cwd=self.root, check=True, capture_output=True)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_caller_cannot_assert_protected_storage(self):
        with self.assertRaises(TypeError):
            ProtectedGuard(self.root, self.tmp / "protected", protected_storage=True)  # type: ignore[call-arg]

    def test_backend_attestation_controls_health(self):
        protected = self.tmp / "protected"
        from repopact.admission import Ed25519Signer, setup_admission
        setup_admission(self.root, protected, Ed25519Signer.generate("k", "operator"))
        reference = ProtectedGuard(self.root, protected).health()
        self.assertFalse(reference.protected)
        testing = ProtectedGuard(self.root, protected, backend=TestingBackend(protected)).health()
        self.assertTrue(testing.protected)
        self.assertTrue(testing.testing_only)
        self.assertEqual(testing.backend_id, "testing-only-attested-backend")

    def test_windows_backend_is_not_covered_before_install(self):
        attestation = WindowsBackend().attest(self.root)
        self.assertFalse(attestation.installed)
        self.assertFalse(attestation.protected_from_gated_principal)
        self.assertEqual(attestation.security_level, "not-covered")

    def test_status_reports_backend_owned_health(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            rc = cli.main(["guard", "status", "--root", str(self.root), "--json"])
        self.assertEqual(rc, 1)
        data = json.loads(output.getvalue())
        self.assertIn("protected_from_gated_principal", data)
        self.assertFalse(data["protected_from_gated_principal"])

    def test_windows_install_requires_real_elevation(self):
        backend = WindowsBackend()
        try:
            import ctypes
            elevated = bool(ctypes.windll.shell32.IsUserAnAdmin())
        except (AttributeError, OSError):
            elevated = False
        if elevated:
            self.skipTest("elevated operator context; installation is intentionally not run by unit tests")
        with self.assertRaises(PrivilegeRequired):
            backend.install(self.root)

    def test_ipc_envelope_is_canonical_and_versioned(self):
        message = envelope("health", {"root": str(self.root)})
        self.assertEqual(decode(encode(message)), message)
        with self.assertRaises(ValueError):
            decode(b'{"protocol_version":"unsupported"}')


if __name__ == "__main__":
    unittest.main()
