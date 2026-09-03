from __future__ import annotations

import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path
import sys

from repopact.admission import Ed25519Signer, issue_receipt, make_request, setup_admission
from repopact.guard import GuardService, ProtectedGuard
from repopact.guard_ipc import NativeGuardClient, local_peer_binding
from repopact.platform_backends import TestingBackend, WindowsBackend
import repopact.platform_backends as platform_backends


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

    def test_preflight_records_explicit_interpreter_and_isolated_service_command(self):
        backend = WindowsBackend()
        self_test = {
            "ok": True, "isolated": True, "user_site_enabled": False, "sys_path": [],
            "required_modules": ["cryptography", "cryptography_ed25519", "_cffi_backend"],
            "module_origins": {
                "cryptography": str(self.tmp / "system" / "cryptography.py"),
                "cryptography.hazmat.primitives.asymmetric.ed25519": str(self.tmp / "system" / "ed25519.py"),
                "_cffi_backend": str(self.tmp / "system" / "_cffi_backend.pyd"),
            }, "errors": [],
        }
        for origin in self_test["module_origins"].values():
            path = Path(origin); path.parent.mkdir(parents=True, exist_ok=True); path.touch()
        with patch.object(platform_backends, "_run_isolated_dependency_self_test", return_value=self_test):
            report = backend.preflight(self.root, interpreter=Path(sys.executable))
        self.assertEqual(report["interpreter"]["path"], str(Path(sys.executable).absolute()))
        self.assertTrue(report["interpreter"]["canonical_path"])
        self.assertIn(" -I ", f" {report['service_command']} ")
        self.assertIn(report["interpreter"]["canonical_path"], report["service_command"])
        self.assertIn("--state-root", report["service_command"])

    def test_user_site_dependency_is_rejected_even_outside_checkout_and_venv(self):
        backend = WindowsBackend()
        user_site = self.tmp / "user-site" / "site-packages"
        origin = user_site / "cryptography" / "__init__.py"
        origin.parent.mkdir(parents=True, exist_ok=True); origin.write_text("# fixture\n", encoding="utf-8")
        self_test = {
            "ok": True, "isolated": True, "user_site_enabled": False, "sys_path": [],
            "required_modules": ["cryptography"], "module_origins": {"cryptography": str(origin)}, "errors": [],
        }
        with patch.object(platform_backends, "_run_isolated_dependency_self_test", return_value=self_test):
            report = backend.preflight(self.root, interpreter=Path(sys.executable))
        record = report["dependencies"]["cryptography"]
        self.assertFalse(record["protected"])
        self.assertIn("user-writable", record["reason"])
        self.assertFalse(report["checks"]["required_dependency_closure"])

    def test_isolated_self_test_ignores_environment_and_checkout_injection(self):
        result = platform_backends._run_isolated_dependency_self_test(Path(sys.executable))
        self.assertTrue(result["ok"], result.get("errors"))
        self.assertFalse(result["user_site_enabled"])
        self.assertNotIn("", result["sys_path"])
        self.assertTrue(result["sys_path"])
        self.assertTrue(all(str(self.root).lower() not in str(path).lower() for path in result["sys_path"]))

    def test_acl_parser_allows_read_only_users_but_rejects_replacement_rights(self):
        read_only = r"C:\Program Files\RepoPact BUILTIN\Users:(OI)(CI)(RX)"
        writable = r"C:\Program Files\RepoPact BUILTIN\Users:(OI)(CI)(M)"
        authenticated = r"C:\Program Files\RepoPact NT AUTHORITY\Authenticated Users:(RX,W)"
        self.assertFalse(platform_backends._windows_acl_has_broad_write(read_only))
        self.assertTrue(platform_backends._windows_acl_has_broad_write(writable))
        self.assertTrue(platform_backends._windows_acl_has_broad_write(authenticated))

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
