from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from repopact.admission import GuardHealth
from repopact.adapters import AdapterCapabilities, LandlockSandboxAdapter
from repopact.confinement import LandlockConfinementProvider
from repopact.enforcement import resolve_enforcement_requirement


class _FixtureGuard:
    endpoint = "/run/repopact/guard.sock"

    def __init__(self, health: GuardHealth):
        self._health = health

    def health(self, root=None):
        return self._health

    def discover(self, root=None):
        return None

    def authorize(self, request, receipt, root=None):
        return None

    def check(self, action, lease=None, root=None):
        return None

    def delegate(self, parent_token, child, root=None):
        return None

    def revoke(self, request, receipt, root=None):
        return None


class LandlockProviderTests(unittest.TestCase):
    def test_non_linux_provider_is_not_covered(self):
        guard = _FixtureGuard(GuardHealth(True, security_level="pre-action", protected=True, backend_id="fixture"))
        with tempfile.TemporaryDirectory() as directory:
            helper = Path(directory) / "repopact-sandbox"
            helper.write_bytes(b"fixture")
            provider = LandlockConfinementProvider(guard, helper)
            health = provider.health(Path(directory))
        self.assertEqual(health.security_level, "not-covered")
        self.assertFalse(health.process_confined)

    def test_testing_probe_can_resolve_higher_class_without_production_helper_claim(self):
        guard = _FixtureGuard(GuardHealth(True, security_level="pre-action", protected=True,
                                         backend_id="fixture", testing_only=True))
        with tempfile.TemporaryDirectory() as directory:
            helper = Path(directory) / "repopact-sandbox"
            helper.write_bytes(b"fixture")
            provider = LandlockConfinementProvider(
                guard, helper, allow_unprotected_testing_helper=True,
            )
            with patch("repopact.confinement.os.name", "posix"), patch("repopact.confinement.sys.platform", "linux"), patch(
                "repopact.confinement._native_probe",
                return_value=(True, "fixture probe", {"abi": 3, "probe": "passed"}),
            ):
                health = provider.health(Path(directory))
                adapter = LandlockSandboxAdapter(provider)
                resolution = resolve_enforcement_requirement(
                    {"enabled": True, "minimum_enforcement": "sandbox/process-enforced"},
                    provider,
                    adapter.capabilities,
                    root=Path(directory),
                )
        self.assertEqual(health.security_level, "sandbox/process-enforced")
        self.assertTrue(health.path_confined and health.process_confined)
        self.assertTrue(resolution.satisfied)

    def test_adapter_capability_vector_alone_does_not_claim_sandbox(self):
        caps = AdapterCapabilities("fixture", os="linux", path_confinement=True, process_confinement=True)
        self.assertEqual(caps.security_level, "not-covered")
        self.assertEqual(caps.enforcement_class("sandbox/process-enforced"), "sandbox/process-enforced")


if __name__ == "__main__":
    unittest.main()
