"""Installed Windows guard service host.

This file is copied into the protected runtime during ``guard install`` and is
never imported from the checkout by the service.  The SCM bridge uses ctypes
from the standard library so the installed service has no editable-package or
optional pywin32 dependency.
"""
from __future__ import annotations

import argparse
import ctypes
from ctypes import wintypes
import sys
import threading
from multiprocessing.connection import Listener
from pathlib import Path

if __package__ in {None, ""}:  # installed service entrypoint, never checkout imports
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from repopact.guard import GuardService, ProtectedGuard
    from repopact.guard_ipc import WINDOWS_AUTHKEY, WINDOWS_PIPE, decode, encode, windows_peer_identity
else:
    from .guard import GuardService, ProtectedGuard
    from .guard_ipc import WINDOWS_AUTHKEY, WINDOWS_PIPE, decode, encode, windows_peer_identity


def serve(root: Path, protected_dir: Path) -> None:
    guard = ProtectedGuard(root, protected_dir)
    service = GuardService(guard)
    listener = Listener(WINDOWS_PIPE, family="AF_PIPE", authkey=WINDOWS_AUTHKEY)
    try:
        while True:
            connection = listener.accept()
            try:
                peer = windows_peer_identity(connection)
                request = decode(connection.recv_bytes())
                payload = request.get("payload", {})
                claimed_pid = payload.pop("_transport_pid", None)
                if peer.peer_pid is None or claimed_pid != peer.peer_pid:
                    raise PermissionError("named-pipe caller identity could not be authenticated")
                op = request.get("op")
                response = service.dispatch({"op": op, **payload})
                connection.send_bytes(encode({"protocol_version": "1", "op": op, "result": response,
                                               "transport": {"kind": peer.transport, "peer_pid": peer.peer_pid}}))
            except Exception as exc:
                connection.send_bytes(encode({"protocol_version": "1", "allowed": False, "code": "GUARD_UNHEALTHY", "reason": str(exc)}))
            finally:
                connection.close()
    finally:
        listener.close()


def _run_as_service(root: Path, protected_dir: Path) -> int:
    # Keep the protected runtime dependency-free.  This ctypes bridge is
    # sufficient for SCM lifecycle registration; all policy work remains in
    # the installed RepoPact package.
    return _run_as_native_service(root, protected_dir)


def _run_as_native_service(root: Path, protected_dir: Path) -> int:
    if sys.platform != "win32":
        return 2
    advapi = ctypes.WinDLL("Advapi32", use_last_error=True)
    SERVICE_WIN32_OWN_PROCESS = 0x00000010
    SERVICE_RUNNING = 0x00000004
    SERVICE_STOPPED = 0x00000001
    SERVICE_START_PENDING = 0x00000002
    SERVICE_ACCEPT_STOP = 0x00000001
    SERVICE_CONTROL_STOP = 0x00000001

    class SERVICE_STATUS(ctypes.Structure):
        _fields_ = [("service_type", wintypes.DWORD), ("current_state", wintypes.DWORD),
                    ("controls_accepted", wintypes.DWORD), ("win32_exit_code", wintypes.DWORD),
                    ("service_specific_exit_code", wintypes.DWORD), ("check_point", wintypes.DWORD),
                    ("wait_hint", wintypes.DWORD)]

    SERVICE_MAIN = ctypes.WINFUNCTYPE(None, wintypes.DWORD, ctypes.POINTER(wintypes.LPWSTR))
    SERVICE_HANDLER = ctypes.WINFUNCTYPE(wintypes.DWORD, wintypes.DWORD, wintypes.DWORD,
                                         wintypes.LPVOID, wintypes.LPVOID)
    stop = threading.Event()
    status_handle = wintypes.HANDLE()
    status = SERVICE_STATUS(SERVICE_WIN32_OWN_PROCESS, SERVICE_START_PENDING, SERVICE_ACCEPT_STOP, 0, 0, 1, 3000)

    @SERVICE_HANDLER
    def handler(control, _event_type, _event_data, _context):
        if control == SERVICE_CONTROL_STOP:
            stop.set()
            status.current_state = SERVICE_STOPPED
            advapi.SetServiceStatus(status_handle, ctypes.byref(status))
        return 0

    @SERVICE_MAIN
    def service_main(_argc, _argv):
        nonlocal status_handle
        advapi.RegisterServiceCtrlHandlerExW.argtypes = [wintypes.LPCWSTR, SERVICE_HANDLER, wintypes.LPVOID]
        advapi.RegisterServiceCtrlHandlerExW.restype = wintypes.HANDLE
        status_handle = advapi.RegisterServiceCtrlHandlerExW("RepoPactGuard", handler, None)
        status.current_state = SERVICE_RUNNING
        status.controls_accepted = SERVICE_ACCEPT_STOP
        advapi.SetServiceStatus(status_handle, ctypes.byref(status))
        try:
            serve(root, protected_dir)
        finally:
            status.current_state = SERVICE_STOPPED
            advapi.SetServiceStatus(status_handle, ctypes.byref(status))

    class SERVICE_TABLE_ENTRY(ctypes.Structure):
        _fields_ = [("service_name", wintypes.LPWSTR), ("service_proc", SERVICE_MAIN)]

    dispatch = (SERVICE_TABLE_ENTRY * 2)()
    dispatch[0] = SERVICE_TABLE_ENTRY("RepoPactGuard", service_main)
    dispatch[1] = SERVICE_TABLE_ENTRY(None, None)
    advapi.StartServiceCtrlDispatcherW.argtypes = [ctypes.POINTER(SERVICE_TABLE_ENTRY)]
    advapi.StartServiceCtrlDispatcherW(dispatch)
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="repopact-guard-service")
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--protected-dir", type=Path, required=True)
    parser.add_argument("--service", action="store_true")
    args = parser.parse_args(argv)
    if args.service:
        return _run_as_service(args.root.resolve(), args.protected_dir.resolve())
    serve(args.root.resolve(), args.protected_dir.resolve())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
