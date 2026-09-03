from __future__ import annotations

import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path

from repopact.admission import Ed25519Signer, issue_receipt, make_request, setup_admission
from repopact.guard import GuardService, ProtectedGuard
from repopact.guard_ipc import NativeGuardClient, local_peer_binding
from repopact.platform_backends import TestingBackend, WindowsBackend


class GuardAuthorityTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="repopact-lease-authority-"))
        self.root = self.tmp / "repo"
        shutil.copytree(Path(__file__).parents[1], self.root,
                        ignore=shutil.ignore_patterns(".git", "__pycache__", "*.pyc", "build", "dist", "*.egg-info"))
        subprocess.run(["git", "init"], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "config", "user.name", "RepoPact test"], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "add", "."], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "commit", "-m", "fixture"], cwd=self.root, check=True, capture_output=True)
        self.protected = self.tmp / "protected"
        self.signer = Ed25519Signer.generate("key", "operator")
        setup_admission(self.root, self.protected, self.signer)
        self.guard = ProtectedGuard(self.root, self.protected, backend=TestingBackend(self.protected))

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _request(self):
        request = make_request(self.root, "050", "session-a", scopes=["src"], paths=["src/a.py"], protected_dir=self.protected)
        return request, issue_receipt(request, self.signer)

    def test_authorize_returns_opaque_token_and_restart_invalidates(self):
        request, receipt = self._request()
        decision, capability = self.guard.authorize(request, receipt)
        self.assertTrue(decision.allowed)
        self.assertIsInstance(capability["lease_token"], str)
        self.assertNotIn("authority_state_digest", capability["lease_metadata"])
        action = {"kind": "mutation", "work_item": "050", "paths": ["src/a.py"], "scopes": ["src"], "session_id": "session-a", "principal": "agent"}
        self.assertTrue(self.guard.check(action, capability).allowed)
        restarted = ProtectedGuard(self.root, self.protected, backend=TestingBackend(self.protected))
        self.assertFalse(restarted.check(action, capability).allowed)

    def test_forged_metadata_and_wrong_peer_are_denied(self):
        request, receipt = self._request()
        _, capability = self.guard.authorize(request, receipt)
        forged = {**capability["lease_metadata"], "lease_token": capability["lease_token"], "paths": ["governance/owners.json"]}
        action = {"kind": "mutation", "work_item": "050", "paths": ["src/a.py"], "scopes": ["src"], "session_id": "session-a", "principal": "agent"}
        self.assertEqual(self.guard.check(action, forged, peer_binding={"pid": 999, "transport": "fake"}).code, "WRONG_SESSION")
        self.assertEqual(self.guard.check(action, {"lease_token": "x" * 64}).code, "NO_OPERATOR_PROOF")

    def test_service_exposes_authorize_check_revoke_and_delegate(self):
        service = GuardService(self.guard)
        request, receipt = self._request()
        response = service.dispatch({"op": "authorize", "payload": {"request": request, "receipt": receipt}})
        self.assertIn("lease_token", response)
        action = {"kind": "mutation", "work_item": "050", "paths": ["src/a.py"], "scopes": ["src"], "session_id": "session-a", "principal": "agent"}
        checked = service.dispatch({"op": "check", "payload": {"action": action, "lease_token": response["lease_token"]}})
        self.assertTrue(checked["allowed"])
        self.assertFalse(service.dispatch({"op": "check", "payload": {"action": action, "lease_token": "z" * 64}})["allowed"])

    def test_native_client_fails_closed_without_service(self):
        client = NativeGuardClient(self.tmp / "missing.sock", root=self.root)
        self.assertFalse(client.health().healthy)
        self.assertFalse(client.check({}, None).allowed)

    def test_install_preflight_is_non_mutating_and_rejects_dirty_source(self):
        backend = WindowsBackend()
        before = backend.install_root.exists()
        report = backend.preflight(self.root)
        self.assertFalse(report["checks"]["source_tree_clean"])
        self.assertEqual(report["mutations"], [])
        self.assertEqual(backend.install_root.exists(), before)

    def test_binding_has_host_pid_not_claimed_session(self):
        binding = local_peer_binding()
        self.assertEqual(binding["pid"], __import__("os").getpid())
        self.assertNotIn("session", binding)

    def test_global_adoption_registry_keeps_independent_repositories_separate(self):
        second = self.tmp / "second-repo"
        shutil.copytree(self.root, second, ignore=shutil.ignore_patterns(".git", "__pycache__", "*.pyc", "build", "dist", "*.egg-info"))
        subprocess.run(["git", "init"], cwd=second, check=True, capture_output=True)
        subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=second, check=True, capture_output=True)
        subprocess.run(["git", "config", "user.name", "RepoPact test"], cwd=second, check=True, capture_output=True)
        subprocess.run(["git", "add", "."], cwd=second, check=True, capture_output=True)
        subprocess.run(["git", "commit", "-m", "fixture"], cwd=second, check=True, capture_output=True)
        registry = self.tmp / "global-registrations"
        setup_admission(self.root, registry, self.signer, registry_key="adoption")
        setup_admission(second, registry, self.signer, registry_key="adoption")
        from repopact.admission import verify_registration
        self.assertTrue(verify_registration(self.root, registry).allowed)
        self.assertTrue(verify_registration(second, registry).allowed)
        self.assertNotEqual((self.root / "governance/repository-registration.json").read_bytes(),
                            (second / "governance/repository-registration.json").read_bytes())
        request = make_request(self.root, "050", "global-a", scopes=["src"], paths=["src/a.py"], protected_dir=registry)
        receipt = issue_receipt(request, self.signer)
        with patch("repopact.guard.current_backend", lambda *_args, **_kwargs: TestingBackend(registry)):
            service = GuardService(registry_root=registry)
            authorized = service.dispatch({"op": "authorize", "payload": {"root": str(self.root), "request": request, "receipt": receipt}})
            self.assertTrue(authorized["allowed"])
            action = {"kind": "mutation", "work_item": "050", "paths": ["src/a.py"], "scopes": ["src"], "session_id": "global-a", "principal": "agent"}
            checked = service.dispatch({"op": "check", "payload": {"root": str(self.root), "repository_identity": request["repository_identity"], "action": action, "lease_token": authorized["lease_token"]}})
            self.assertTrue(checked["allowed"])
            other = service.dispatch({"op": "check", "payload": {"root": str(second), "action": action, "lease_token": authorized["lease_token"]}})
            self.assertFalse(other["allowed"])


if __name__ == "__main__":
    unittest.main()
