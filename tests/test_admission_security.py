from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from repopact import cli
from repopact.admission import (
    Ed25519Signer, evaluate_action, issue_lease, issue_receipt, make_request,
    setup_admission, verify_receipt, frozen_surface_digest,
)
from repopact.adapters import LauncherAdapter
from repopact.guard import ProtectedGuard


class SecurityCorrectionTests(unittest.TestCase):
    """Focused regressions written before the WI050 correction pass."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="repopact-security-"))
        self.root = self.tmp / "repo"
        shutil.copytree(Path(__file__).parents[1], self.root, ignore=shutil.ignore_patterns(".git", "__pycache__", "*.pyc"))
        subprocess.run(["git", "init"], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=self.root, check=True)
        subprocess.run(["git", "config", "user.name", "RepoPact test"], cwd=self.root, check=True)
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)
        subprocess.run(["git", "commit", "-m", "fixture"], cwd=self.root, check=True, capture_output=True)
        self.protected = self.tmp / "protected"
        self.signer = Ed25519Signer.generate("key-1", "operator-1")
        setup_admission(self.root, self.protected, self.signer)

    def tearDown(self): shutil.rmtree(self.tmp, ignore_errors=True)

    def request(self, paths=("src/a.py",), profile="bounded", mode="normal", scopes=("src",), **kwargs):
        return make_request(self.root, "050", "session-1", profile=profile, scopes=list(scopes), paths=list(paths), mode=mode, protected_dir=self.protected, **kwargs)

    def test_mutation_without_lease_is_denied_before_callback(self):
        called = []
        decision = evaluate_action(self.root, {"kind": "mutation", "work_item": "050", "paths": ["src/a.py"], "scopes": ["src"]}, protected_dir=self.protected)
        self.assertFalse(decision.allowed)
        self.assertEqual(decision.code, "NO_OPERATOR_PROOF")
        self.assertEqual(called, [])

    def test_lease_cannot_widen_exact_path(self):
        req = self.request(paths=("src/a.py",)); receipt = issue_receipt(req, self.signer)
        decision, lease = issue_lease(req, receipt, self.root, self.protected)
        self.assertTrue(decision.allowed)
        denied = evaluate_action(self.root, {"kind": "mutation", "work_item": "050", "paths": ["src/b.py"], "scopes": ["src"], "session_id": "session-1", "principal": "agent"}, lease, protected_dir=self.protected)
        self.assertFalse(denied.allowed)
        self.assertEqual(denied.code, "PATH_VIOLATION")

    def test_repair_requires_operator_lease(self):
        denied = evaluate_action(self.root, {"kind": "repair", "repair": True, "profile": "repair", "work_item": "050", "paths": ["governance/owners.json"]}, protected_dir=self.protected)
        self.assertFalse(denied.allowed)
        self.assertIn(denied.code, {"NO_OPERATOR_PROOF", "BOOTSTRAP_SCOPE"})

    def test_frozen_flag_cannot_be_caller_asserted(self):
        denied = evaluate_action(self.root, {"kind": "frozen", "work_item": "050", "paths": ["src/a.py"], "scopes": ["src"], "frozen": True, "receipt_verified": True}, protected_dir=self.protected)
        self.assertFalse(denied.allowed)
        self.assertEqual(denied.code, "NO_OPERATOR_PROOF")

    def test_receipt_class_tamper_is_rejected(self):
        req = self.request(); receipt = issue_receipt(req, self.signer); receipt["approval_class"] = "frozen"
        authority = json.loads((self.root / "governance/operator-authority.json").read_text())
        denied = verify_receipt(req, receipt, authority)
        self.assertFalse(denied.allowed)
        self.assertEqual(denied.code, "RECEIPT_INVALID")

    def test_bootstrap_request_cannot_write_arbitrary_output(self):
        sentinel = self.tmp / "sentinel.txt"; sentinel.write_text("original")
        rc = cli.main(["approval", "request", "--root", str(self.root), "--work-item", "050", "--session", "s", "--output", str(sentinel), "--protected-dir", str(self.protected)])
        self.assertNotEqual(rc, 0)
        self.assertEqual(sentinel.read_text(), "original")

    def test_admission_setup_never_mints_unattended_ephemeral_trust(self):
        target = self.tmp / "new-admission-repo"
        rc = cli.main(["init", "--target", str(target), "--admission"])
        self.assertNotEqual(rc, 0)
        self.assertFalse(target.exists())

    def test_revocation_requires_operator_protocol(self):
        state = next(self.protected.rglob("registration.json")); before = json.loads(state.read_text())["revocation_epoch"]
        rc = cli.main(["admission", "revoke", "--root", str(self.root), "--protected-dir", str(self.protected)])
        self.assertNotEqual(rc, 0)
        self.assertEqual(json.loads(state.read_text())["revocation_epoch"], before)

    def test_same_principal_can_rewrite_hmac_key_is_not_protected(self):
        state = next(self.protected.rglob("registration.json")); key = state.with_name("registration.key")
        # The current reference HMAC is intentionally forgeable by this test
        # principal; the corrected health report must not call it protected.
        state.write_text(state.read_text()); key.write_bytes(key.read_bytes())
        health = ProtectedGuard(self.root, self.protected).health()
        self.assertFalse(health.protected)
        self.assertTrue(health.integrity_checked)
        self.assertFalse(health.protected_from_gated_principal)

    def test_repair_requires_matching_direct_repair_approval(self):
        req = self.request(paths=("governance/owners.json",), profile="repair", mode="repair", scopes=("governance",), approval_class="repair")
        rec = issue_receipt(req, self.signer, "repair")
        proof, lease = issue_lease(req, rec, self.root, self.protected)
        self.assertTrue(proof.allowed)
        action = {"kind": "repair", "repair": True, "profile": "repair", "mode": "repair", "work_item": "050",
                  "paths": ["governance/owners.json"], "diagnosed_paths": ["governance/owners.json"], "scopes": ["governance"]}
        self.assertTrue(evaluate_action(self.root, action, lease, protected_dir=self.protected).allowed)
        widened = {**action, "paths": ["governance/charter.md"], "diagnosed_paths": ["governance/charter.md"]}
        self.assertEqual(evaluate_action(self.root, widened, lease, protected_dir=self.protected).code, "PATH_VIOLATION")

    def test_frozen_approval_binds_declared_surface(self):
        req = self.request(paths=("governance/owners.json",), approval_class="frozen")
        rec = issue_receipt(req, self.signer, "frozen")
        proof, lease = issue_lease(req, rec, self.root, self.protected)
        self.assertTrue(proof.allowed)
        action = {"kind": "mutation", "work_item": "050", "paths": ["governance/owners.json"], "scopes": ["src"], "frozen": True}
        self.assertTrue(evaluate_action(self.root, action, lease, protected_dir=self.protected).allowed)
        lease["frozen_surface_digest"] = "0" * 64
        self.assertEqual(evaluate_action(self.root, action, lease, protected_dir=self.protected).code, "FROZEN_APPROVAL_REQUIRED")

    def test_launcher_denies_before_child_creation(self):
        marker = self.tmp / "child-created.txt"
        adapter = LauncherAdapter(ProtectedGuard(self.root, self.protected, protected_storage=True))
        denied, child = adapter.launch({"kind": "process", "work_item": "050", "paths": ["src/a.py"], "scopes": ["src"]},
                                       ["cmd", "/c", "echo", "bad", ">", str(marker)])
        self.assertFalse(denied.allowed); self.assertIsNone(child); self.assertFalse(marker.exists())

    def test_linked_worktree_uses_common_dir_registration(self):
        subprocess.run(["git", "add", "governance"], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "commit", "-m", "admission registration"], cwd=self.root, check=True, capture_output=True)
        linked = self.tmp / "linked checkout"
        subprocess.run(["git", "worktree", "add", "-b", "linked-test", str(linked), "HEAD"], cwd=self.root, check=True, capture_output=True)
        try:
            self.assertTrue(verify_receipt is not None)
            from repopact.admission import verify_registration
            self.assertTrue(verify_registration(linked, self.protected).allowed)
        finally:
            subprocess.run(["git", "worktree", "remove", "--force", str(linked)], cwd=self.root, check=False, capture_output=True)

    def test_new_session_cannot_reuse_lease(self):
        req = self.request(); rec = issue_receipt(req, self.signer); proof, lease = issue_lease(req, rec, self.root, self.protected)
        self.assertTrue(proof.allowed)
        denied = evaluate_action(self.root, {"kind": "mutation", "work_item": "050", "paths": ["src/a.py"], "scopes": ["src"], "session_id": "new-session"}, lease, protected_dir=self.protected)
        self.assertEqual(denied.code, "WRONG_SESSION")

    def test_nested_cwd_and_shell_attempts_still_gate_before_execution(self):
        nested = self.root / "src" / "nested"
        nested.mkdir(parents=True, exist_ok=True)
        marker = self.tmp / "shell-marker.txt"
        adapter = ProtectedGuard(self.root, self.protected, protected_storage=True)
        from repopact.adapters import LauncherAdapter
        gate = LauncherAdapter(adapter)
        for command in (["powershell", "-NoProfile", "-Command", f"Set-Content -LiteralPath '{marker}' bad"],
                        ["sh", "-c", f"printf bad > '{marker}'"]):
            if shutil.which(command[0]) is None:
                continue
            denied, child = gate.launch({"kind": "process", "work_item": "050", "paths": ["src/a.py"], "scopes": ["src"]}, command, cwd=nested)
            self.assertFalse(denied.allowed); self.assertIsNone(child)
        self.assertFalse(marker.exists())

    def test_check_frozen_ack_and_self_activation_are_not_authority(self):
        frozen = self.root / "governance" / "invariants.json"
        original = frozen.read_bytes()
        try:
            frozen.write_bytes(original + b"\n")
            self.assertNotEqual(cli.main(["check-frozen", "--root", str(self.root), "--base", "HEAD", "--ack"]), 0)
        finally:
            frozen.write_bytes(original)
        before = sorted(str(path) for path in self.root.glob("work/active/*/work-item.json"))
        self.assertNotEqual(cli.main(["new", "work-item", "self activation", "--root", str(self.root), "--status", "active"]), 0)
        self.assertEqual(before, sorted(str(path) for path in self.root.glob("work/active/*/work-item.json")))


if __name__ == "__main__": unittest.main()
