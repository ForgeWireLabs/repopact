"""Protected Unix guard service host for a root-owned system service.

The service manager owns installation and identity.  This module only owns the
socket accept/dispatch loop once a protected runtime has been installed.  It
never treats a checkout-local module, socket, or configuration file as the
protection root.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path

from .guard import GuardService
from .guard_ipc import UnixGuardListener, decode, encode


def serve(endpoint: Path, state_root: Path) -> None:
    if not hasattr(os, "geteuid") or os.geteuid() != 0:
        raise PermissionError("the Unix RepoPact guard service must run as root")
    service = GuardService(registry_root=state_root / "registrations")
    listener = UnixGuardListener(endpoint)
    try:
        listener.bind()
        while True:
            connection, binding = listener.accept()
            try:
                with connection:
                    try:
                        chunks: list[bytes] = []
                        while True:
                            chunk = connection.recv(1024 * 1024)
                            if not chunk:
                                break
                            chunks.append(chunk)
                            if b"\n" in chunk:
                                break
                        request = decode(b"".join(chunks))
                        payload = request.get("payload", {})
                        if not isinstance(payload, dict):
                            raise ValueError("guard payload must be an object")
                        response = service.dispatch(
                            {"op": request.get("op"), "payload": payload},
                            transport_binding=binding,
                        )
                    except Exception as exc:
                        response = {
                            "allowed": False, "code": "GUARD_UNHEALTHY", "reason": str(exc),
                        }
                    connection.sendall(encode({"protocol_version": "1", **response}))
            except Exception as exc:
                try:
                    connection.sendall(encode({
                        "protocol_version": "1", "allowed": False,
                        "code": "GUARD_UNHEALTHY", "reason": str(exc),
                    }))
                except OSError:
                    pass
    except KeyboardInterrupt:
        return
    finally:
        listener.close()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="repopact-unix-guard-service")
    parser.add_argument("--endpoint", type=Path, default=Path("/run/repopact/guard.sock"))
    parser.add_argument("--state-root", type=Path, required=True)
    args = parser.parse_args(argv)
    serve(args.endpoint.resolve(), args.state_root.resolve())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
