"""OS-neutral guard IPC envelope with platform transports.

The wire format is JSON and the transport is replaceable.  Windows uses an
authenticated named pipe when the protected service is installed; POSIX
backends use a filesystem-owned Unix socket.  Transport authentication is a
necessary channel property, not a substitute for lease and repository checks.
"""
from __future__ import annotations

import json
import os
from dataclasses import dataclass
from multiprocessing.connection import Client
from pathlib import Path
from typing import Any, Mapping


PROTOCOL_VERSION = "1"
WINDOWS_PIPE = r"\\.\pipe\RepoPactGuard"
WINDOWS_AUTHKEY = b"RepoPactGuard.protocol.v1"


@dataclass(frozen=True)
class IPCIdentity:
    """Identity claims supplied by the transport, not arbitrary action data."""

    transport: str
    peer_pid: int | None = None
    peer_uid: int | None = None
    service_identity: str = ""


def envelope(op: str, payload: Mapping[str, Any] | None = None) -> dict[str, Any]:
    return {"protocol_version": PROTOCOL_VERSION, "op": op, "payload": dict(payload or {})}


def encode(message: Mapping[str, Any]) -> bytes:
    return (json.dumps(dict(message), sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def decode(data: bytes | str) -> dict[str, Any]:
    value = json.loads(data.decode("utf-8") if isinstance(data, bytes) else data)
    if not isinstance(value, dict) or value.get("protocol_version") != PROTOCOL_VERSION:
        raise ValueError("unsupported guard IPC protocol")
    return value


def windows_request(message: Mapping[str, Any], timeout: float = 3.0, expected_server_path: str | Path | None = None) -> dict[str, Any]:
    """Send one request to the installed Windows named-pipe guard."""
    # ``multiprocessing.connection`` uses the native AF_PIPE transport on
    # Windows.  The service additionally checks the pipe ACL and caller token;
    # an unauthenticated local listener is not accepted as the guard.
    connection = Client(WINDOWS_PIPE, family="AF_PIPE", authkey=WINDOWS_AUTHKEY)
    try:
        if expected_server_path is not None:
            actual = windows_peer_image_path(connection)
            if not actual or os.path.normcase(actual) != os.path.normcase(str(Path(expected_server_path).resolve())):
                raise PermissionError("guard IPC peer is not the installed RepoPact service")
        payload = dict(message.get("payload", {}))
        payload["_transport_pid"] = os.getpid()
        outbound = dict(message)
        outbound["payload"] = payload
        connection.send_bytes(encode(outbound))
        return decode(connection.recv_bytes())
    finally:
        connection.close()


def unix_request(endpoint: str | Path, message: Mapping[str, Any]) -> dict[str, Any]:
    """Send one request to a protected Unix-domain socket."""
    import socket

    path = str(endpoint)
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
        sock.connect(path)
        sock.sendall(encode(message))
        chunks: list[bytes] = []
        while True:
            chunk = sock.recv(65536)
            if not chunk:
                break
            chunks.append(chunk)
            if b"\n" in chunk:
                break
    return decode(b"".join(chunks))


def peer_identity(sock: Any) -> IPCIdentity:
    """Return best-effort peer identity for a Unix socket connection."""
    if hasattr(sock, "getsockopt") and hasattr(os, "getuid"):
        try:
            import struct
            import socket
            raw = sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i"))
            _pid, uid, _gid = struct.unpack("3i", raw)
            return IPCIdentity("unix", peer_pid=_pid, peer_uid=uid)
        except (OSError, AttributeError, ValueError):
            pass
    return IPCIdentity("unknown")


def windows_peer_identity(connection: Any, *, client: bool = True) -> IPCIdentity:
    """Inspect the native named-pipe peer process identity when possible."""
    if os.name != "nt":
        return IPCIdentity("unknown")
    try:
        import ctypes
        from ctypes import wintypes
        handle = wintypes.HANDLE(getattr(connection, "_handle"))
        pid = wintypes.ULONG()
        function = "GetNamedPipeClientProcessId" if client else "GetNamedPipeServerProcessId"
        get_pid = getattr(ctypes.windll.kernel32, function)
        get_pid.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.ULONG)]
        get_pid.restype = wintypes.BOOL
        if get_pid(handle, ctypes.byref(pid)):
            return IPCIdentity("windows-named-pipe", peer_pid=int(pid.value))
    except (AttributeError, OSError, TypeError, ValueError):
        pass
    return IPCIdentity("windows-named-pipe")


def windows_peer_image_path(connection: Any) -> str:
    """Return the authenticated named-pipe peer executable path, if queryable."""
    identity = windows_peer_identity(connection, client=False)
    if os.name != "nt" or identity.peer_pid is None:
        return ""
    try:
        import ctypes
        from ctypes import wintypes
        kernel = ctypes.windll.kernel32
        handle = kernel.OpenProcess(0x1000, False, identity.peer_pid)  # QUERY_LIMITED_INFORMATION
        if not handle:
            return ""
        try:
            size = wintypes.DWORD(32768)
            buffer = ctypes.create_unicode_buffer(size.value)
            if not kernel.QueryFullProcessImageNameW(handle, 0, buffer, ctypes.byref(size)):
                return ""
            return buffer.value[:size.value]
        finally:
            kernel.CloseHandle(handle)
    except (AttributeError, OSError, TypeError, ValueError):
        return ""
