"""Vendor-neutral guard IPC and host-derived peer identity helpers.

The JSON envelope is portable. Windows uses a native named pipe with an
explicit SDDL DACL; Unix implementations use a filesystem-owned Unix socket
and ``SO_PEERCRED``. ``WINDOWS_PROTOCOL_TAG`` is public framing metadata, never
a secret or an identity proof.
"""
from __future__ import annotations

import json
import os
import stat
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping

from .enforcement import EnforcementProvider

PROTOCOL_VERSION = "1"
WINDOWS_PIPE = r"\\.\pipe\RepoPactGuard"
WINDOWS_PROTOCOL_TAG = b"RepoPactGuard.protocol.v1"
# Compatibility name; callers must not treat this as authentication material.
WINDOWS_AUTHKEY = WINDOWS_PROTOCOL_TAG
WINDOWS_PIPE_SDDL = "D:P(A;;FA;;;SY)(A;;FA;;;BA)(A;;0x12019b;;;BU)"


@dataclass(frozen=True)
class IPCIdentity:
    transport: str
    peer_pid: int | None = None
    peer_uid: int | None = None
    peer_gid: int | None = None
    peer_sid: str = ""
    process_start: str = ""
    service_identity: str = ""


def local_peer_binding() -> dict[str, Any]:
    """Return facts the local process cannot choose for itself."""
    result: dict[str, Any] = {"transport": "local", "pid": os.getpid()}
    if hasattr(os, "getuid"):
        result["uid"] = os.getuid()
    if os.name == "nt":
        try:
            import ctypes
            from ctypes import wintypes
            kernel = ctypes.windll.kernel32
            handle = kernel.OpenProcess(0x1000, False, os.getpid())
            if handle:
                try:
                    size = wintypes.DWORD(32768)
                    buf = ctypes.create_unicode_buffer(size.value)
                    if kernel.QueryFullProcessImageNameW(handle, 0, buf, ctypes.byref(size)):
                        result["image"] = buf.value[:size.value].lower()
                finally:
                    kernel.CloseHandle(handle)
        except (AttributeError, OSError, TypeError, ValueError):
            pass
    return result


def envelope(op: str, payload: Mapping[str, Any] | None = None) -> dict[str, Any]:
    return {"protocol_version": PROTOCOL_VERSION, "op": op, "payload": dict(payload or {})}


def encode(message: Mapping[str, Any]) -> bytes:
    return (json.dumps(dict(message), sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def decode(data: bytes | str) -> dict[str, Any]:
    value = json.loads(data.decode("utf-8") if isinstance(data, bytes) else data)
    if not isinstance(value, dict) or value.get("protocol_version") != PROTOCOL_VERSION:
        raise ValueError("unsupported guard IPC protocol")
    return value


def _unix_endpoint_is_protected(endpoint: str | Path, *, owner_uid: int = 0) -> tuple[bool, str]:
    """Require a root-owned, non-writable Unix socket and parent chain."""
    if os.name == "nt":
        return False, "Unix socket protection is unavailable on Windows"
    path = Path(endpoint)
    try:
        current = path
        checked: list[str] = []
        while True:
            info = current.lstat()
            if stat.S_ISLNK(info.st_mode):
                return False, f"Unix IPC path contains a symlink: {current}"
            if current == path and not stat.S_ISSOCK(info.st_mode):
                return False, f"Unix IPC endpoint is not a socket: {current}"
            if info.st_uid != owner_uid:
                return False, f"Unix IPC path is not owned by uid {owner_uid}: {current}"
            # Group read/write on the socket is a deliberate access channel
            # for the configured gated principal. The parent chain itself may
            # never be group/world-writable, and the socket may never be
            # world-writable or user-owned.
            writable_bits = 0o002 if current == path else 0o022
            if info.st_mode & writable_bits:
                return False, f"Unix IPC path is group/world-writable: {current}"
            checked.append(str(current))
            if current.parent == current:
                break
            current = current.parent
        return True, "protected Unix IPC path: " + ", ".join(checked)
    except OSError:
        return False, f"Unix IPC endpoint is absent or unreadable: {path}"


class WindowsPipeConnection:
    """Small message-mode named-pipe wrapper using native Windows APIs."""
    def __init__(self, handle: int): self.handle = handle

    def send_bytes(self, data: bytes) -> None:
        import ctypes
        from ctypes import wintypes
        written = wintypes.DWORD()
        buf = ctypes.create_string_buffer(data)
        if not ctypes.windll.kernel32.WriteFile(self.handle, buf, len(data), ctypes.byref(written), None):
            raise OSError(ctypes.get_last_error(), "WriteFile failed")

    def recv_bytes(self) -> bytes:
        import ctypes
        from ctypes import wintypes
        chunks: list[bytes] = []
        while True:
            buf = ctypes.create_string_buffer(1024 * 1024)
            read = wintypes.DWORD()
            ok = ctypes.windll.kernel32.ReadFile(self.handle, buf, len(buf), ctypes.byref(read), None)
            if not ok and ctypes.get_last_error() not in (109, 234):  # broken pipe / more data
                raise OSError(ctypes.get_last_error(), "ReadFile failed")
            chunks.append(buf.raw[:read.value])
            if ok or not read.value or b"\n" in chunks[-1]:
                return b"".join(chunks)

    def close(self) -> None:
        if self.handle:
            try:
                import ctypes
                ctypes.windll.kernel32.CloseHandle(self.handle)
            finally:
                self.handle = 0


class WindowsPipeListener:
    """Named-pipe listener with an explicit DACL, not Listener's default ACL."""
    def __init__(self, name: str = WINDOWS_PIPE):
        if os.name != "nt": raise OSError("Windows named pipes require Windows")
        self.name = name
        self._handle = 0

    def _security_attributes(self):
        import ctypes
        from ctypes import wintypes
        advapi = ctypes.windll.advapi32
        convert = advapi.ConvertStringSecurityDescriptorToSecurityDescriptorW
        convert.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, ctypes.POINTER(wintypes.LPVOID), ctypes.POINTER(wintypes.DWORD)]
        convert.restype = wintypes.BOOL
        descriptor = wintypes.LPVOID()
        size = wintypes.DWORD()
        if not convert(WINDOWS_PIPE_SDDL, 1, ctypes.byref(descriptor), ctypes.byref(size)):
            raise OSError(ctypes.get_last_error(), "named-pipe SDDL conversion failed")
        class SECURITY_ATTRIBUTES(ctypes.Structure):
            _fields_ = [("nLength", wintypes.DWORD), ("lpSecurityDescriptor", wintypes.LPVOID), ("bInheritHandle", wintypes.BOOL)]
        return SECURITY_ATTRIBUTES(ctypes.sizeof(SECURITY_ATTRIBUTES), descriptor, False), descriptor

    def accept(self) -> WindowsPipeConnection:
        import ctypes
        from ctypes import wintypes
        attrs, descriptor = self._security_attributes()
        try:
            kernel = ctypes.windll.kernel32
            PIPE_ACCESS_DUPLEX = 0x00000003
            PIPE_TYPE_MESSAGE, PIPE_READMODE_MESSAGE, PIPE_WAIT = 4, 2, 0
            INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value
            self._handle = kernel.CreateNamedPipeW(self.name, PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT, 1, 1024 * 1024, 1024 * 1024, 3000, ctypes.byref(attrs))
            if self._handle == INVALID_HANDLE_VALUE:
                raise OSError(ctypes.get_last_error(), "CreateNamedPipeW failed")
            connected = kernel.ConnectNamedPipe(self._handle, None)
            if not connected and ctypes.get_last_error() != 535:  # ERROR_PIPE_CONNECTED
                raise OSError(ctypes.get_last_error(), "ConnectNamedPipe failed")
            handle = self._handle; self._handle = 0
            return WindowsPipeConnection(handle)
        finally:
            ctypes.windll.kernel32.LocalFree(descriptor)

    def close(self) -> None:
        if self._handle:
            import ctypes
            ctypes.windll.kernel32.CloseHandle(self._handle); self._handle = 0


def _windows_server_pid(service_name: str = "RepoPactGuard") -> int | None:
    try:
        cp = subprocess.run(["sc.exe", "queryex", service_name], text=True, capture_output=True, check=False)
        for line in (cp.stdout + cp.stderr).splitlines():
            if "PID" in line.upper() and ":" in line:
                value = line.split(":", 1)[1].strip()
                if value.isdigit(): return int(value)
    except OSError:
        pass
    return None


def windows_peer_identity(connection: Any, *, client: bool = True) -> IPCIdentity:
    if os.name != "nt": return IPCIdentity("unknown")
    try:
        import ctypes
        from ctypes import wintypes
        raw_handle = getattr(connection, "handle", None)
        if raw_handle is None: raw_handle = getattr(connection, "_handle")
        handle = wintypes.HANDLE(raw_handle)
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


def windows_peer_binding(connection: Any, *, client: bool = True) -> dict[str, Any]:
    """Derive a lease binding from the pipe peer, never from JSON claims."""
    identity = windows_peer_identity(connection, client=client)
    result: dict[str, Any] = {"transport": identity.transport, "pid": identity.peer_pid}
    if identity.peer_pid is None or os.name != "nt": return result
    result["image"] = windows_peer_image_path(connection) if not client else ""
    try:
        import ctypes
        from ctypes import wintypes
        kernel = ctypes.windll.kernel32
        PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
        handle = kernel.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, identity.peer_pid)
        if handle:
            try:
                creation, exit_time, kernel_time, user_time = (wintypes.FILETIME(), wintypes.FILETIME(), wintypes.FILETIME(), wintypes.FILETIME())
                if kernel.GetProcessTimes(handle, ctypes.byref(creation), ctypes.byref(exit_time), ctypes.byref(kernel_time), ctypes.byref(user_time)):
                    result["process_start"] = f"{creation.dwHighDateTime:08x}{creation.dwLowDateTime:08x}"
            finally: kernel.CloseHandle(handle)
    except (AttributeError, OSError, TypeError, ValueError):
        pass
    try:
        import ctypes
        from ctypes import wintypes
        kernel, advapi = ctypes.windll.kernel32, ctypes.windll.advapi32
        process = kernel.OpenProcess(0x1000, False, identity.peer_pid)
        if process:
            try:
                token = wintypes.HANDLE()
                if advapi.OpenProcessToken(process, 0x0008, ctypes.byref(token)):
                    try:
                        needed = wintypes.DWORD()
                        advapi.GetTokenInformation(token, 1, None, 0, ctypes.byref(needed))
                        buf = ctypes.create_string_buffer(needed.value)
                        if advapi.GetTokenInformation(token, 1, buf, needed, ctypes.byref(needed)):
                            sid_ptr = ctypes.cast(buf, ctypes.POINTER(ctypes.c_void_p))[0]
                            text = wintypes.LPWSTR()
                            if advapi.ConvertSidToStringSidW(sid_ptr, ctypes.byref(text)):
                                result["sid"] = text.value
                                kernel.LocalFree(text)
                    finally: kernel.CloseHandle(token)
            finally: kernel.CloseHandle(process)
    except (AttributeError, OSError, TypeError, ValueError):
        pass
    result.setdefault("sid", "")
    return result


def windows_peer_image_path(connection: Any) -> str:
    identity = windows_peer_identity(connection, client=False)
    if os.name != "nt" or identity.peer_pid is None: return ""
    try:
        import ctypes
        from ctypes import wintypes
        kernel = ctypes.windll.kernel32
        handle = kernel.OpenProcess(0x1000, False, identity.peer_pid)
        if not handle: return ""
        try:
            size = wintypes.DWORD(32768); buffer = ctypes.create_unicode_buffer(size.value)
            if not kernel.QueryFullProcessImageNameW(handle, 0, buffer, ctypes.byref(size)): return ""
            return buffer.value[:size.value]
        finally: kernel.CloseHandle(handle)
    except (AttributeError, OSError, TypeError, ValueError): return ""


def _windows_server_verified(connection: Any, expected_server_path: str | Path | None, service_name: str) -> bool:
    peer = windows_peer_identity(connection, client=False)
    server_pid = _windows_server_pid(service_name)
    if peer.peer_pid is None or server_pid is None or peer.peer_pid != server_pid: return False
    try:
        qc = subprocess.run(["sc.exe", "qc", service_name], text=True, capture_output=True, check=False)
        text = qc.stdout + qc.stderr
        upper = text.upper()
        if "LOCAL SYSTEM" not in upper and "NT AUTHORITY\\SYSTEM" not in upper and "NT AUTHORITY\\LOCALSYSTEM" not in upper:
            return False
        configured_line = next((line.split(":", 1)[1].strip() for line in text.splitlines()
                                if line.strip().upper().startswith("BINARY_PATH_NAME") and ":" in line), "")
        configured_executable = configured_line[1:configured_line.find('"', 1)] if configured_line.startswith('"') and '"' in configured_line[1:] else configured_line.split(None, 1)[0] if configured_line else ""
        if not configured_executable or "BINARY_PATH_NAME" not in upper: return False
        protected_runtime = str(Path(os.environ.get("ProgramData", r"C:\\ProgramData")) / "RepoPact" / "Guard" / "runtime")
        if protected_runtime.replace("/", "\\").upper() not in upper.replace("/", "\\"): return False
    except OSError:
        return False
    actual = windows_peer_image_path(connection)
    if not actual:
        return False
    configured_path = Path(configured_executable).resolve(strict=False)
    if os.path.normcase(actual) != os.path.normcase(str(configured_path)):
        return False
    if expected_server_path is not None and os.path.normcase(actual) != os.path.normcase(str(Path(expected_server_path).resolve())):
        return False
    try:
        from .platform_backends import _windows_protected_path_chain
        protected, _reason = _windows_protected_path_chain(Path(actual))
        return protected
    except (ImportError, OSError, ValueError):
        return False


def windows_request(message: Mapping[str, Any], timeout: float = 3.0, expected_server_path: str | Path | None = None,
                    service_name: str = "RepoPactGuard") -> dict[str, Any]:
    if os.name != "nt": raise OSError("Windows guard IPC requires Windows")
    import ctypes
    from ctypes import wintypes
    GENERIC_READ, GENERIC_WRITE, OPEN_EXISTING = 0x80000000, 0x40000000, 3
    handle = ctypes.windll.kernel32.CreateFileW(WINDOWS_PIPE, GENERIC_READ | GENERIC_WRITE, 0, None, OPEN_EXISTING, 0, None)
    if handle in (0, ctypes.c_void_p(-1).value): raise OSError(ctypes.get_last_error(), "guard named pipe unavailable")
    connection = WindowsPipeConnection(handle)
    try:
        if not _windows_server_verified(connection, expected_server_path, service_name):
            raise PermissionError("guard IPC server identity is not the installed LocalSystem service")
        outbound = dict(message); payload = dict(message.get("payload", {})); payload.pop("_transport_pid", None)
        outbound["payload"] = payload
        connection.send_bytes(encode(outbound))
        return decode(connection.recv_bytes())
    finally: connection.close()


def unix_request(endpoint: str | Path, message: Mapping[str, Any], *, expected_server_uid: int | None = 0) -> dict[str, Any]:
    import socket
    path = str(endpoint)
    protected, reason = _unix_endpoint_is_protected(path, owner_uid=expected_server_uid or 0)
    if expected_server_uid is not None and not protected:
        raise PermissionError(reason)
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
        sock.connect(path)
        if expected_server_uid is not None:
            identity = peer_identity(sock)
            if identity.peer_uid != expected_server_uid:
                raise PermissionError("Unix IPC peer is not the expected protected service identity")
        sock.sendall(encode(message)); chunks: list[bytes] = []
        while True:
            chunk = sock.recv(65536)
            if not chunk: break
            chunks.append(chunk)
            if b"\n" in chunk: break
    return decode(b"".join(chunks))


def peer_identity(sock: Any) -> IPCIdentity:
    if hasattr(sock, "getsockopt") and hasattr(os, "getuid"):
        try:
            import struct, socket
            if hasattr(socket, "SO_PEERCRED"):
                raw = sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i"))
                pid, uid, _gid = struct.unpack("3i", raw)
                return IPCIdentity("unix", peer_pid=pid, peer_uid=uid, peer_gid=_gid)
        except (OSError, AttributeError, ValueError): pass
    if hasattr(sock, "getpeereid"):
        try:
            uid, _gid = sock.getpeereid()
            return IPCIdentity("unix", peer_uid=int(uid))
        except (OSError, AttributeError, TypeError, ValueError):
            pass
    return IPCIdentity("unknown")


class UnixGuardListener:
    """Protected Unix-domain listener for a root-owned system service.

    The listener refuses to replace an existing endpoint.  An operator or
    service manager must remove a stale socket explicitly, preventing a
    user-writable path from being silently adopted as the guard endpoint.
    """

    def __init__(self, endpoint: str | Path, *, mode: int = 0o660):
        if os.name == "nt":
            raise OSError("Unix sockets are unavailable on Windows")
        self.endpoint = Path(endpoint)
        self.mode = mode
        self._socket: Any = None

    def bind(self) -> None:
        import socket
        if self.endpoint.exists() or self.endpoint.is_symlink():
            raise FileExistsError(f"refusing to replace existing Unix guard endpoint: {self.endpoint}")
        self.endpoint.parent.mkdir(parents=True, exist_ok=True)
        os.chmod(self.endpoint.parent, 0o750)
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            listener.bind(str(self.endpoint))
            os.chmod(self.endpoint, self.mode)
            owner_uid = os.geteuid() if hasattr(os, "geteuid") else os.getuid()
            protected, reason = _unix_endpoint_is_protected(self.endpoint, owner_uid=owner_uid)
            if not protected:
                raise PermissionError(reason)
            listener.listen(16)
            self._socket = listener
        except Exception:
            listener.close()
            try:
                self.endpoint.unlink()
            except OSError:
                pass
            raise

    def accept(self) -> tuple[Any, dict[str, Any]]:
        if self._socket is None:
            self.bind()
        while True:
            connection, _ = self._socket.accept()
            identity = peer_identity(connection)
            if identity.transport != "unknown" and identity.peer_uid is not None:
                return connection, {"transport": identity.transport, "pid": identity.peer_pid,
                                    "uid": identity.peer_uid, "gid": identity.peer_gid}
            connection.close()

    def close(self) -> None:
        if self._socket is not None:
            self._socket.close()
            self._socket = None
        try:
            self.endpoint.unlink()
        except OSError:
            pass


class NativeGuardClient(EnforcementProvider):
    """Production client; it never reads protected state directly."""
    def __init__(self, endpoint: str | Path | None = None, *, root: Path | None = None,
                 expected_server_path: str | Path | None = None, service_name: str = "RepoPactGuard"):
        self.endpoint = str(endpoint or (WINDOWS_PIPE if os.name == "nt" else "/run/repopact/guard.sock"))
        self.root, self.expected_server_path, self.service_name = root.resolve() if root else None, expected_server_path, service_name

    def _call(self, op: str, payload: Mapping[str, Any]) -> dict[str, Any]:
        message = envelope(op, payload)
        if os.name == "nt": return windows_request(message, expected_server_path=self.expected_server_path, service_name=self.service_name)
        return unix_request(self.endpoint, message)

    def health(self, root: Path | None = None) -> Any:
        from .admission import GuardHealth
        from dataclasses import fields
        selected = root or self.root
        if selected is None: return GuardHealth(False, security_level="not-covered", reason="native guard client has no repository root", backend_id="native-ipc")
        try: result = self._call("health", {"root": str(selected)})
        except Exception as exc: return GuardHealth(False, security_level="not-covered", reason=str(exc), backend_id="native-ipc")
        names = {f.name for f in fields(GuardHealth)}
        return GuardHealth(**{k: v for k, v in result.items() if k in names})

    def discover(self, root: Path | None = None) -> Any:
        from .guard import _decision
        selected = root or self.root
        if selected is None:
            from .admission import AdmissionDecision
            return AdmissionDecision.deny("GUARD_UNHEALTHY", "native guard client has no repository root")
        try: return _decision(self._call("discover", {"root": str(selected)}))
        except Exception as exc: return __import__("repopact.admission", fromlist=["AdmissionDecision"]).AdmissionDecision.deny("GUARD_UNHEALTHY", str(exc))

    def authorize(self, request: Mapping[str, Any], receipt: Mapping[str, Any], root: Path | None = None):
        from .guard import _decision
        selected = root or self.root or Path(str(request.get("repopact_root", "")))
        try:
            response = self._call("authorize", {"root": str(selected), "request": dict(request), "receipt": dict(receipt)})
            decision = _decision(response.get("decision", response))
            return decision, {k: response[k] for k in ("lease_token", "lease_metadata") if k in response} if decision.allowed else None
        except Exception as exc:
            from .admission import AdmissionDecision
            return AdmissionDecision.deny("GUARD_UNHEALTHY", str(exc)), None

    def check(self, action: Mapping[str, Any], lease: str | Mapping[str, Any] | None = None, root: Path | None = None):
        from .guard import _decision
        selected = root or self.root or Path(str(action.get("repopact_root", action.get("root", ""))))
        token = lease if isinstance(lease, str) else lease.get("lease_token") if isinstance(lease, Mapping) else None
        try:
            from .admission import canonical_identity, digest
            identity = digest(canonical_identity(selected))
            return _decision(self._call("check", {"root": str(selected), "repository_identity": identity,
                                                   "action": dict(action), "lease_token": token}))
        except Exception as exc:
            from .admission import AdmissionDecision
            return AdmissionDecision.deny("GUARD_UNHEALTHY", str(exc))

    def revoke(self, request: Mapping[str, Any], receipt: Mapping[str, Any], root: Path | None = None) -> dict[str, Any]:
        selected = root or self.root or Path(str(request.get("repopact_root", "")))
        return self._call("revoke", {"root": str(selected), "request": dict(request), "receipt": dict(receipt)})

    def delegate(self, parent_token: str, child: Mapping[str, Any], root: Path | None = None):
        from .guard import _decision
        selected = root or self.root or Path(str(child.get("repopact_root", "")))
        try:
            response = self._call("delegate", {"root": str(selected), "parent_token": parent_token, "child": dict(child)})
            decision = _decision(response.get("decision", response))
            return decision, {k: response[k] for k in ("lease_token", "lease_metadata") if k in response} if decision.allowed else None
        except Exception as exc:
            from .admission import AdmissionDecision
            return AdmissionDecision.deny("GUARD_UNHEALTHY", str(exc)), None
