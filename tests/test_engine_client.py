from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest import mock

from repopact import cli, engine_client, validate_repo


class EngineClientContractTests(unittest.TestCase):
    def test_malformed_and_multiple_responses_fail_closed(self) -> None:
        client = object.__new__(engine_client.EngineClient)
        with self.assertRaises(engine_client.EngineProtocolError):
            client._decode_response("not json", "", "request-1")
        with self.assertRaisesRegex(engine_client.EngineProtocolError, "more than one"):
            client._decode_response('{"ok": true} {}', "", "request-1")

    def test_protocol_and_product_mismatch_fail_closed(self) -> None:
        client = object.__new__(engine_client.EngineClient)
        base = {
            "protocol": engine_client.PROTOCOL,
            "protocol_version": engine_client.PROTOCOL_VERSION,
            "request_id": "request-1",
            "engine_version": engine_client.expected_product_version(),
        }
        with self.assertRaisesRegex(engine_client.EngineProtocolError, "protocol major mismatch"):
            client._verify_response({**base, "protocol_version": 99}, "validate", "request-1")
        with self.assertRaisesRegex(engine_client.EngineProtocolError, "product version mismatch"):
            client._verify_response({**base, "engine_version": "0.0.0"}, "validate", "request-1")
        with self.assertRaisesRegex(engine_client.EngineProtocolError, "identity mismatch"):
            client._verify_response(base, "validate", "request-2")

    def test_public_validate_uses_engine_without_legacy_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            response = {
                "ok": True,
                "result": {"valid": True, "error_count": 0, "warning_count": 0, "diagnostics": []},
                "diagnostics": [],
            }
            with (
                mock.patch.object(engine_client, "EngineClient") as client_type,
                mock.patch.object(validate_repo, "validate", side_effect=AssertionError("legacy fallback")),
            ):
                client_type.return_value.call.return_value = response
                self.assertEqual(0, cli.main(["validate", "--root", temp]))
                client_type.return_value.call.assert_called_once_with(
                    "validate", root=Path(temp).resolve()
                )

    def test_missing_engine_does_not_fallback_to_python(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            with (
                mock.patch.object(
                    engine_client,
                    "locate_engine",
                    side_effect=engine_client.EngineUnavailableError("missing test engine"),
                ),
                mock.patch.object(validate_repo, "validate", side_effect=AssertionError("legacy fallback")),
            ):
                self.assertEqual(1, cli.main(["validate", "--root", temp]))

    def test_work_commands_send_typed_intents_to_engine(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            response = {
                "ok": True,
                "result": {
                    "success": True,
                    "changed_paths": ["work/proposed/001-probe/work-item.json"],
                    "diagnostics": [],
                },
                "diagnostics": [],
            }
            with mock.patch.object(engine_client, "EngineClient") as client_type:
                client_type.return_value.call.return_value = response
                self.assertEqual(0, cli.main(["work", "propose", "Probe", "--root", str(root)]))
                call = client_type.return_value.call.call_args
                operation = call.args[0]
                kwargs = call.kwargs
                self.assertEqual("work.propose", operation)
                self.assertEqual(root.resolve(), kwargs["root"])
                self.assertEqual("Probe", kwargs["params"]["title"])

    def test_engine_client_does_not_parse_human_output(self) -> None:
        client = object.__new__(engine_client.EngineClient)
        response = client._decode_response(
            '{"protocol":"repopact-engine","protocol_version":1,"request_id":"r",'
            + f'"engine_version":"{engine_client.expected_product_version()}","ok":true}}',
            "human stderr is not semantic output",
            "r",
        )
        self.assertTrue(response["ok"])


if __name__ == "__main__":
    unittest.main()
