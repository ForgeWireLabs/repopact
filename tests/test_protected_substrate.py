from __future__ import annotations

import contextlib
import io
import json
import os
import unittest
from unittest.mock import patch

from repopact import cli
from repopact.dev_fixtures import open_fixture_repo
from repopact.guard import ProtectedGuard
from repopact.guard_ipc import decode, encode, envelope
from repopact.platform_backends import PrivilegeRequired, TestingBackend, WindowsBackend
from repopact import unix_guard_service


class ProtectedSubstrateTests(unittest.TestCase):
    def setUp(self):
        self.root = open_fixture_repo(self, prefix="repopact-substrate-")
        self.tmp = self.root.parent

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
        self.assertEqual(testing.security_level, "pre-action")

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

    @unittest.skipUnless(hasattr(os, "geteuid"), "unix_guard_service targets POSIX; os.geteuid is not available on Windows")
    def test_unix_service_returns_protocol_error_before_closing_connection(self):
        class Connection:
            def __init__(self):
                self.responses = []

            def __enter__(self):
                return self

            def __exit__(self, *_args):
                return False

            def recv(self, _size):
                return b'{}\n'

            def sendall(self, payload):
                self.responses.append(payload)

        class Listener:
            def __init__(self, _endpoint):
                self.connection = Connection()
                self.served = None

            def bind(self):
                return None

            def accept(self):
                if self.connection is not None:
                    connection, self.connection = self.connection, None
                    self.served = connection
                    return connection, {"transport": "unix", "uid": 1000}
                raise KeyboardInterrupt

            def close(self):
                return None

        listener = Listener(unix_guard_service.Path("/tmp/guard.sock"))
        with patch.object(unix_guard_service.os, "geteuid", return_value=0), patch.object(
            unix_guard_service, "UnixGuardListener", return_value=listener
        ):
            unix_guard_service.serve(unix_guard_service.Path("/tmp/guard.sock"), self.tmp / "state")
        self.assertIsNotNone(listener.served)
        response = decode(listener.served.responses[0])
        self.assertEqual(response["code"], "GUARD_UNHEALTHY")


if __name__ == "__main__":
    unittest.main()
