"""Fail-closed client for RepoPact's versioned Rust engine protocol."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import sysconfig
import uuid
from pathlib import Path
from typing import Any

from .package_version import package_version


PROTOCOL = "repopact-engine"
PROTOCOL_VERSION = 1
ENGINE_OVERRIDE = "REPOPACT_ENGINE"
ENGINE_TIMEOUT_SECONDS = 30.0


class EngineError(RuntimeError):
    """Base class for compatibility-engine failures."""


class EngineUnavailableError(EngineError):
    """No trusted, same-environment engine executable was found."""


class EngineProtocolError(EngineError):
    """The engine could not satisfy the transport/version contract."""


class EngineSemanticError(EngineError):
    """The engine returned a structured transport error."""


def expected_product_version() -> str:
    """Return the installed/check-out package identity expected from the engine."""
    value = package_version(Path(__file__).resolve().parents[1])
    # The historical source-side helper emits ``3.0.2dev1`` while installed
    # PEP 440 metadata and Maturin emit the canonical
    # ``3.0.2.dev1`` spelling. Normalize only this representation detail.
    for suffix in ("alpha", "a", "beta", "b", "rc", "dev"):
        marker = suffix
        if marker in value and value.rsplit(marker, 1)[1].isdigit():
            prefix, number = value.rsplit(marker, 1)
            if prefix.endswith("."):
                return value
            return f"{prefix}.{marker}{number}"
    return value


def _candidate_scripts_directories() -> list[Path]:
    candidates = [Path(sysconfig.get_path("scripts")), Path(sys.executable).resolve().parent]
    result: list[Path] = []
    for candidate in candidates:
        if candidate and candidate not in result:
            result.append(candidate)
    return result


def locate_engine(*, allow_checkout: bool = False) -> Path:
    """Locate the explicit override or engine installed beside this Python.

    Checkout target directories are considered only by callers that explicitly
    opt into development discovery (the conformance runner); normal CLI use
    never searches PATH or unrelated directories.
    """
    override = os.environ.get(ENGINE_OVERRIDE)
    if override:
        path = Path(override).expanduser().resolve()
        if not path.is_file():
            raise EngineUnavailableError(f"{ENGINE_OVERRIDE} does not name an engine executable: {path}")
        return path

    names = ("repopact-engine.exe", "repopact-engine")
    for directory in _candidate_scripts_directories():
        for name in names:
            path = directory / name
            if path.is_file():
                return path

    if allow_checkout:
        root = Path(__file__).resolve().parents[1]
        target_directories: list[Path] = []
        configured = os.environ.get("CARGO_TARGET_DIR")
        if configured:
            target_directories.append(Path(configured).expanduser())
        target_directories.extend([root / "target", root / "rust" / "target"])
        for target in target_directories:
            for profile in ("debug", "release"):
                for name in names:
                    path = target / profile / name
                    if path.is_file():
                        return path.resolve()

    raise EngineUnavailableError(
        "repopact-engine is not installed in this Python environment; "
        "install a RepoPact wheel or set REPOPACT_ENGINE explicitly for development"
    )


def canonical_command_template() -> str:
    """Return the published conformance command for the canonical engine."""
    path = locate_engine(allow_checkout=True)
    return (
        f'"{sys.executable}" -m repopact.engine_client validate '
        f'--engine "{path}" --root "{{repo}}"'
    )


class EngineClient:
    def __init__(self, *, engine: Path | None = None, timeout: float = ENGINE_TIMEOUT_SECONDS):
        self.engine = engine.resolve() if engine is not None else locate_engine()
        self.timeout = timeout
        self._handshaken = False

    def call(
        self,
        operation: str,
        *,
        root: Path | None = None,
        params: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        self._handshake()
        response = self._invoke(
            {
                "protocol": PROTOCOL,
                "protocol_version": PROTOCOL_VERSION,
                "request_id": uuid.uuid4().hex,
                "operation": operation,
                "root": str(root.resolve()) if root is not None else None,
                "params": params or {},
            }
        )
        if not response.get("ok", False):
            error = response.get("error") or {}
            raise EngineSemanticError(
                f"Rust engine rejected {operation}: {error.get('code', 'engine.error')}: "
                f"{error.get('message', 'unknown engine error')}"
            )
        return response

    def _handshake(self) -> None:
        if self._handshaken:
            return
        request = {
            "protocol": PROTOCOL,
            "protocol_version": PROTOCOL_VERSION,
            "request_id": uuid.uuid4().hex,
            "operation": "handshake",
            "root": None,
            "params": {},
        }
        response = self._invoke(request)
        self._verify_response(response, "handshake", request["request_id"])
        if not response.get("ok", False):
            error = response.get("error") or {}
            raise EngineProtocolError(
                f"Rust engine handshake failed: {error.get('code', 'engine.error')}: "
                f"{error.get('message', 'unknown engine error')}"
            )
        capabilities = response.get("capabilities")
        if not isinstance(capabilities, dict) or not isinstance(capabilities.get("operations"), list):
            raise EngineProtocolError("Rust engine handshake omitted capabilities")
        self._handshaken = True

    def _invoke(self, request: dict[str, Any]) -> dict[str, Any]:
        encoded = json.dumps(request, sort_keys=True, separators=(",", ":")) + "\n"
        creationflags = getattr(subprocess, "CREATE_NO_WINDOW", 0) if os.name == "nt" else 0
        try:
            process = subprocess.Popen(
                [str(self.engine)],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                errors="strict",
                shell=False,
                creationflags=creationflags,
            )
            stdout, stderr = process.communicate(encoded, timeout=self.timeout)
        except FileNotFoundError as exc:
            raise EngineUnavailableError(f"unable to start Rust engine {self.engine}: {exc}") from exc
        except subprocess.TimeoutExpired as exc:
            process.kill()
            process.communicate()
            raise EngineProtocolError(
                f"Rust engine timed out after {self.timeout:g}s during {request['operation']}"
            ) from exc
        except OSError as exc:
            raise EngineProtocolError(f"unable to start Rust engine {self.engine}: {exc}") from exc

        response = self._decode_response(stdout, stderr, request["request_id"])
        self._verify_response(response, request["operation"], request["request_id"])
        if process.returncode != 0 and response.get("ok"):
            detail = stderr.strip() or "engine exited unsuccessfully"
            raise EngineProtocolError(f"Rust engine failed after a successful response: {detail}")
        return response

    def _decode_response(self, stdout: str, stderr: str, request_id: str) -> dict[str, Any]:
        text = stdout.strip()
        if not text:
            detail = stderr.strip()
            raise EngineProtocolError(
                f"Rust engine returned no JSON response for request {request_id}"
                + (f": {detail}" if detail else "")
            )
        try:
            decoder = json.JSONDecoder()
            response, end = decoder.raw_decode(text)
        except json.JSONDecodeError as exc:
            raise EngineProtocolError(f"Rust engine returned malformed JSON: {exc.msg}") from exc
        if text[end:].strip():
            raise EngineProtocolError("Rust engine returned more than one JSON response")
        if not isinstance(response, dict):
            raise EngineProtocolError("Rust engine response must be a JSON object")
        return response

    def _verify_response(self, response: dict[str, Any], operation: str, request_id: Any) -> None:
        if response.get("protocol") != PROTOCOL:
            raise EngineProtocolError("Rust engine protocol name mismatch")
        if response.get("protocol_version") != PROTOCOL_VERSION:
            raise EngineProtocolError(
                f"Rust engine protocol major mismatch: {response.get('protocol_version')!r}"
            )
        if response.get("request_id") != request_id:
            raise EngineProtocolError(f"Rust engine request identity mismatch during {operation}")
        if response.get("engine_version") != expected_product_version():
            raise EngineProtocolError(
                "Rust engine product version mismatch: "
                f"{response.get('engine_version')!r} != {expected_product_version()!r}"
            )


def client() -> EngineClient:
    return EngineClient()


def render_validation(response: dict[str, Any]) -> int:
    """Render the structured validation result using the historical CLI shape."""
    result = response.get("result") or {}
    diagnostics = response.get("diagnostics") or result.get("diagnostics") or []
    for diagnostic in diagnostics:
        code = diagnostic.get("code", "engine.diagnostic")
        location = diagnostic.get("path") or diagnostic.get("record") or "<repository>"
        print(f"ERROR [{code}] {location}: {diagnostic.get('message', '')}")
    valid = bool(result.get("valid"))
    if valid:
        print("Repository governance validation passed.")
        return 0
    print(f"\nValidation failed with {result.get('error_count', len(diagnostics))} error(s).")
    return 1


def main(argv: list[str] | None = None) -> int:
    """Small internal conformance adapter; it never implements validation."""
    import argparse

    parser = argparse.ArgumentParser(description="Run canonical RepoPact engine validation")
    parser.add_argument("validate", choices=["validate"])
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--engine", type=Path)
    args = parser.parse_args(argv)
    try:
        response = EngineClient(engine=args.engine).call("validate", root=args.root)
        return render_validation(response)
    except EngineError as exc:
        print(f"Rust engine compatibility error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
