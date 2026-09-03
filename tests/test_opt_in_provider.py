"""WI050 opt-in semantics and adopter-neutral provider contract coverage."""
from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from typing import Any, Mapping

from repopact.admission import (
    AdmissionDecision,
    Ed25519Signer,
    GuardHealth,
    default_policy,
    evaluate_action,
    issue_lease,
    setup_admission,
    verify_registration,
)
from repopact.adapters import AdapterCapabilities, PreActionAdapter
from repopact.enforcement import EnforcementProvider, resolve_enforcement_requirement
from repopact.guard_ipc import NativeGuardClient


class ExternalProviderFixture:
    """A provider implemented without ProtectedGuard or NativeGuardClient.

    It deliberately narrows the RepoPact lease to ``src/**`` and proves that a
    downstream implementation can consume the public contract without joining
    RepoPact's semantic kernel.
    """

    def __init__(self, root: Path, protected: Path, *, security_level: str = "pre-action"):
        self.root, self.protected, self.security_level = root, protected, security_level
        self.allowed_paths = ("src/**",)
        self.available = True

    def health(self, root: Path | None = None) -> GuardHealth:
        if not self.available:
            return GuardHealth(False, security_level="not-covered", reason="external provider unavailable", backend_id="external-fixture", testing_only=True)
        return GuardHealth(True, security_level=self.security_level, reason="external fixture", protected=True,
                           integrity_checked=True, protected_from_gated_principal=True,
                           backend_id="external-fixture", testing_only=True)

    def discover(self, root: Path | None = None) -> AdmissionDecision:
        return verify_registration(root or self.root, self.protected)

    def authorize(self, request: Mapping[str, Any], receipt: Mapping[str, Any], root: Path | None = None):
        paths = [str(value).replace("\\", "/") for value in request.get("paths", [])]
        if any(not self._allowed(path) for path in paths):
            return AdmissionDecision.deny("PATH_VIOLATION", "external provider narrowed the requested paths"), None
        return issue_lease(request, receipt, root or self.root, self.protected)

    def check(self, action: Mapping[str, Any], lease: Mapping[str, Any] | None = None, root: Path | None = None):
        if not self.available:
            return AdmissionDecision.deny("GUARD_UNHEALTHY", "external provider unavailable")
        paths = [str(value).replace("\\", "/") for value in action.get("paths", [])]
        if any(not self._allowed(path) for path in paths):
            return AdmissionDecision.deny("PATH_VIOLATION", "external provider path ceiling")
        return evaluate_action(root or self.root, action, lease, self.health(root), self.protected)

    def delegate(self, parent_token: str, child: Mapping[str, Any], root: Path | None = None):
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "fixture does not delegate opaque tokens"), None

    def revoke(self, request: Mapping[str, Any], receipt: Mapping[str, Any], root: Path | None = None):
        return {"revoked": True}

    def _allowed(self, path: str) -> bool:
        return path.startswith("src/") or path == "src"


class OptInProviderTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="repopact-opt-in-"))
        self.root = self.tmp / "repo"
        shutil.copytree(Path(__file__).parents[1], self.root, ignore=shutil.ignore_patterns(".git", "__pycache__", "*.pyc"))
        subprocess.run(["git", "init"], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "config", "user.name", "RepoPact test"], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "add", "."], cwd=self.root, check=True, capture_output=True)
        subprocess.run(["git", "commit", "-m", "fixture"], cwd=self.root, check=True, capture_output=True)
        self.protected = self.tmp / "protected"

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_no_policy_is_valid_standalone_and_has_no_lease_prerequisite(self):
        decision = evaluate_action(self.root, {"kind": "mutation", "paths": ["src/a.py"]}, guard_health=GuardHealth(False, reason="no provider"))
        self.assertTrue(decision.allowed)
        self.assertEqual(decision.enforcement, "instruction-only")
        self.assertFalse(decision.details["enforcement_required"])
        self.assertEqual(decision.details["effective_class"], "instruction-only")

    def test_disabled_policy_is_explicitly_not_required(self):
        policy = default_policy()
        policy["enabled"] = False
        path = self.root / "governance" / "admission-policy.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(policy), encoding="utf-8")
        decision = evaluate_action(self.root, {"kind": "mutation", "paths": ["src/a.py"]})
        self.assertTrue(decision.allowed)
        self.assertEqual(decision.details["reason_code"], "not-required")

    def test_required_policy_without_provider_fails_closed(self):
        policy = default_policy()
        policy["minimum_enforcement"] = "pre-action"
        path = self.root / "governance" / "admission-policy.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(policy), encoding="utf-8")
        decision = evaluate_action(self.root, {"kind": "mutation", "paths": ["src/a.py"]})
        self.assertFalse(decision.allowed)
        self.assertEqual(decision.code, "UNREGISTERED_REPO")
        resolution = resolve_enforcement_requirement(policy, None, "pre-action")
        self.assertFalse(resolution.satisfied)
        self.assertEqual(resolution.effective_class, "not-covered")

    def test_external_provider_satisfies_and_narrows_without_widening(self):
        signer = Ed25519Signer.generate("external-key", "external-operator")
        setup_admission(self.root, self.protected, signer)
        provider = ExternalProviderFixture(self.root, self.protected)
        self.assertIsInstance(provider, EnforcementProvider)
        adapter = PreActionAdapter(provider, AdapterCapabilities("external", pre_action_interception=True))
        resolution = adapter.enforcement_resolution(default_policy(), root=self.root)
        self.assertTrue(resolution.satisfied)
        self.assertEqual(resolution.effective_class, "pre-action")
        self.assertEqual(resolution.adapter_class, "pre-action")
        self.assertEqual(resolution.provider_class, "pre-action")
        provider.security_level = "session-start"
        self.assertEqual(adapter.enforcement_class(), "session-start")
        provider.security_level = "pre-action"

        # A request inside the downstream ceiling is authorized.
        from repopact.admission import issue_receipt, make_request
        request = make_request(self.root, "050", "external-session", scopes=["src"], paths=["src/allowed.py"], protected_dir=self.protected)
        receipt = issue_receipt(request, signer)
        proof, lease = provider.authorize(request, receipt)
        self.assertTrue(proof.allowed)
        self.assertTrue(provider.check({"work_item": "050", "paths": ["src/allowed.py"], "scopes": ["src"]}, lease).allowed)
        self.assertEqual(provider.check({"work_item": "050", "paths": ["tests/escape.py"], "scopes": ["tests"]}, lease).code, "PATH_VIOLATION")

        provider.available = False
        lost = resolve_enforcement_requirement(default_policy(), provider, adapter.capabilities, root=self.root)
        self.assertFalse(lost.satisfied)
        self.assertEqual(lost.effective_class, "not-covered")

    def test_native_client_implements_public_provider_shape(self):
        self.assertIsInstance(NativeGuardClient(root=self.root), EnforcementProvider)


if __name__ == "__main__":
    unittest.main()
