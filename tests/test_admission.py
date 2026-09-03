from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

from repopact.admission import (
    Ed25519Signer, canonical_json, delegation_subset, digest, evaluate_action,
    issue_lease, issue_receipt, make_request, operator_revoke, setup_admission, verify_receipt,
    verify_registration,
)
from repopact.adapters import AdapterCapabilities, PreActionAdapter, LauncherAdapter
from repopact.guard import ProtectedGuard
from repopact.platform_backends import LinuxBackend, MacOSBackend, WindowsBackend


class AdmissionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="repopact-admission-"))
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

    def request(self, **kwargs):
        return make_request(self.root, "050", "session-1", scopes=["src"], paths=["src/example.py"], protected_dir=self.protected, **kwargs)

    def test_canonicalization_and_signature(self):
        self.assertEqual(canonical_json({"b": 2, "a": 1}), canonical_json({"a": 1, "b": 2}))
        self.assertNotEqual(digest({"a": 1}), digest({"a": 2}))
        req = self.request(); receipt = issue_receipt(req, self.signer)
        authority = json.loads((self.root / "governance/operator-authority.json").read_text())
        self.assertTrue(verify_receipt(req, receipt, authority).allowed)
        req["paths"].append("src/other.py")
        self.assertFalse(verify_receipt(req, receipt, authority).allowed)

    def test_invalid_lifecycle_and_scope_denied(self):
        denied = evaluate_action(self.root, {"work_item": "050", "paths": ["governance/owners.json"], "scopes": ["governance"]}, protected_dir=self.protected)
        self.assertEqual(denied.code, "NO_OPERATOR_PROOF")
        item = self.root / "work/active/050-pre-execution-agent-work-admission-and-preflight-enforcement/work-item.json"
        data = json.loads(item.read_text()); data["status"] = "proposed"; item.write_text(json.dumps(data))
        self.assertEqual(evaluate_action(self.root, {"work_item": "050"}, protected_dir=self.protected).code, "NO_OPERATOR_PROOF")

    def test_receipt_lease_and_revocation(self):
        req = self.request(); rec = issue_receipt(req, self.signer)
        d, lease = issue_lease(req, rec, self.root, self.protected)
        self.assertTrue(d.allowed); self.assertTrue(lease)
        replay, _ = issue_lease(req, rec, self.root, self.protected)
        self.assertEqual(replay.code, "RECEIPT_REPLAY")
        operator_revoke(self.root, self.signer, self.protected)
        self.assertEqual(evaluate_action(self.root, {"work_item": "050"}, lease, protected_dir=self.protected).code, "REVOKED_AUTHORIZATION")

    def test_pre_action_callback_never_runs_on_denial(self):
        called = []
        adapter = PreActionAdapter(ProtectedGuard(self.root, self.protected, protected_storage=True))
        decision, result = adapter.before({"work_item": "050", "paths": ["outside.txt"]}, lambda: called.append(1))
        self.assertFalse(decision.allowed); self.assertIsNone(result); self.assertEqual(called, [])
        target = self.root / "src" / "admission-sentinel.txt"
        target.parent.mkdir(exist_ok=True)
        req = make_request(self.root, "050", "session-1", scopes=["src"], paths=["src/admission-sentinel.txt"], protected_dir=self.protected)
        rec = issue_receipt(req, self.signer); proof, lease = issue_lease(req, rec, self.root, self.protected)
        self.assertTrue(proof.allowed)
        allowed, _ = adapter.before({"work_item": "050", "paths": ["src/admission-sentinel.txt"], "scopes": ["src"]}, lambda: target.write_text("authorized"), lease)
        self.assertTrue(allowed.allowed); self.assertEqual(target.read_text(), "authorized")

    def test_reference_adapters_truthful(self):
        caps = AdapterCapabilities("x", path_confinement=False, process_confinement=False)
        self.assertEqual(caps.enforcement_class("sandbox/process-enforced"), "pre-action")
        self.assertFalse(LauncherAdapter(ProtectedGuard(self.root, self.protected), caps).capabilities.path_confinement)
        self.assertEqual(WindowsBackend().security_level, "pre-action")
        self.assertEqual(LinuxBackend().os_name, "linux"); self.assertEqual(MacOSBackend().os_name, "macos")

    def test_protected_tamper_fails_closed(self):
        state = next(self.protected.rglob("registration.json"))
        state.write_text("{}")
        self.assertEqual(verify_registration(self.root, self.protected).code, "AUTHORITY_DRIFT")

    def test_delegation_only_subsets(self):
        parent = {"lease_id": "parent", "repository_identity": "r", "work_item": "050", "principal": "operator", "approval_class": "activate", "profile": "bounded", "mode": "normal", "delegation_ceiling": 2, "scopes": ["src"], "paths": ["src/a.py"], "capabilities": [], "delegation_lineage": [], "expires_at": "2030-01-01T00:00:00Z"}
        child = {**parent, "lease_id": "child", "principal": "subagent", "parent_lease_id": "parent", "delegation_lineage": ["parent"], "delegation_ceiling": 1, "scopes": ["src"], "paths": ["src/a.py"], "expires_at": "2029-01-01T00:00:00Z"}
        self.assertTrue(delegation_subset(parent, child).allowed)
        self.assertFalse(delegation_subset(parent, {**child, "paths": ["src/a.py", "src/b.py"]}).allowed)


if __name__ == "__main__": unittest.main()
