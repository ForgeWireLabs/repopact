"""Installed Windows RepoPact guard service host.

The SCM command line contains only the protected global state root.  The
module deliberately imports the RepoPact runtime only from ``ServiceMain`` so
the process reaches ``StartServiceCtrlDispatcherW`` before service-specific
initialization begins.
"""
from __future__ import annotations

import argparse
import ctypes
import json
import sys
import threading
import time
from ctypes import wintypes
from pathlib import Path

SERVICE_NAME = "RepoPactGuard"
ERROR_FAILED_SERVICE_CONTROLLER_CONNECT = 1063

# The installed entrypoint is executed as a script from
# ``runtime\repopact\``.  Add only its protected runtime parent; defer all
# RepoPact imports until ServiceMain has connected to SCM.
if __package__ in {None, ""}:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))


def _startup_diagnostic(state_root: Path, stage: str, error: BaseException | None = None,
                        win32_error: int | None = None) -> None:
    """Write bounded startup facts under the protected state root only."""
    record: dict[str, object] = {"timestamp": time.time_ns(), "stage": stage}
    if error is not None:
        record["error_type"] = type(error).__name__
        record["error_message"] = str(error)[:500]
    if win32_error is not None:
        record["win32_error"] = int(win32_error)
    try:
        state_root.mkdir(parents=True, exist_ok=True)
        with (state_root / "startup.jsonl").open("a", encoding="utf-8", newline="\n") as stream:
            stream.write(json.dumps(record, sort_keys=True) + "\n")
    except (OSError, TypeError, ValueError):
        pass


def _initialise_runtime(state_root: Path, runtime: dict[str, object]) -> tuple[object, object]:
    """Import the guard and create the endpoint after SCM handler setup."""
    try:
        if __package__ in {None, ""}:
            from repopact.guard import GuardService
            from repopact.guard_ipc import WINDOWS_PIPE, WindowsPipeListener
        else:
            from .guard import GuardService
            from .guard_ipc import WINDOWS_PIPE, WindowsPipeListener
        service = GuardService(registry_root=state_root / "registrations")
    except Exception as exc:
        _startup_diagnostic(state_root, "guard-imports-complete", exc)
        raise
    _startup_diagnostic(state_root, "guard-imports-complete")
    try:
        listener = WindowsPipeListener(WINDOWS_PIPE)
        listener.open()
    except Exception as exc:
        _startup_diagnostic(state_root, "pipe-listener-created", exc)
        raise
    runtime["listener"] = listener
    _startup_diagnostic(state_root, "pipe-listener-created")
    return service, listener


def _serve_loop(state_root: Path, service: object, listener: object,
                stop: threading.Event, runtime: dict[str, object]) -> None:
    if __package__ in {None, ""}:
        from repopact.guard_ipc import decode, encode, windows_peer_binding, windows_peer_identity
    else:
        from .guard_ipc import decode, encode, windows_peer_binding, windows_peer_identity

    _startup_diagnostic(state_root, "serve-loop-entered")
    try:
        while not stop.is_set():
            connection = listener.accept(stop)  # type: ignore[attr-defined]
            if connection is None:
                break
            runtime["connection"] = connection
            try:
                peer = windows_peer_identity(connection)
                request = decode(connection.recv_bytes())
                payload = dict(request.get("payload", {}))
                # The service derives this binding from the OS pipe/token. It
                # intentionally ignores all caller-supplied PID/session claims.
                response = service.dispatch(  # type: ignore[attr-defined]
                    {"op": request.get("op"), "payload": payload},
                    transport_binding=windows_peer_binding(connection),
                )
                connection.send_bytes(encode({
                    "protocol_version": "1", **response,
                    "transport": {"kind": peer.transport, "peer_pid": peer.peer_pid},
                }))
            except Exception as exc:
                if not stop.is_set():
                    try:
                        connection.send_bytes(encode({
                            "protocol_version": "1", "allowed": False,
                            "code": "GUARD_UNHEALTHY", "reason": str(exc),
                        }))
                    except OSError:
                        pass
            finally:
                runtime["connection"] = None
                connection.close()
    finally:
        listener.close()  # type: ignore[attr-defined]
        runtime["listener"] = None
        runtime["connection"] = None


def serve(state_root: Path, stop_event: threading.Event | None = None) -> None:
    """Run the guard loop outside SCM for diagnostics or direct invocation."""
    stop = stop_event or threading.Event()
    runtime: dict[str, object] = {}
    service, listener = _initialise_runtime(state_root, runtime)
    _serve_loop(state_root, service, listener, stop, runtime)


def _run_as_native_service(state_root: Path) -> int:
    if sys.platform != "win32":
        return 2

    advapi = ctypes.WinDLL("Advapi32", use_last_error=True)
    SERVICE_WIN32_OWN_PROCESS = 0x00000010
    SERVICE_STOPPED, SERVICE_START_PENDING = 0x00000001, 0x00000002
    SERVICE_RUNNING, SERVICE_STOP_PENDING = 0x00000004, 0x00000003
    SERVICE_ACCEPT_STOP, SERVICE_CONTROL_STOP = 0x00000001, 0x00000001

    class SERVICE_STATUS(ctypes.Structure):
        _fields_ = [
            ("service_type", ctypes.c_uint32), ("current_state", ctypes.c_uint32),
            ("controls_accepted", ctypes.c_uint32), ("win32_exit_code", ctypes.c_uint32),
            ("service_specific_exit_code", ctypes.c_uint32), ("check_point", ctypes.c_uint32),
            ("wait_hint", ctypes.c_uint32),
        ]

    SERVICE_MAIN = ctypes.WINFUNCTYPE(None, ctypes.c_uint32, ctypes.POINTER(ctypes.c_wchar_p))
    SERVICE_HANDLER = ctypes.WINFUNCTYPE(
        ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_void_p, ctypes.c_void_p,
    )

    class SERVICE_TABLE_ENTRY(ctypes.Structure):
        _fields_ = [("service_name", ctypes.c_wchar_p), ("service_proc", SERVICE_MAIN)]

    advapi.RegisterServiceCtrlHandlerExW.argtypes = [
        ctypes.c_wchar_p, SERVICE_HANDLER, ctypes.c_void_p,
    ]
    advapi.RegisterServiceCtrlHandlerExW.restype = ctypes.c_void_p
    advapi.SetServiceStatus.argtypes = [
        ctypes.c_void_p, ctypes.POINTER(SERVICE_STATUS),
    ]
    advapi.SetServiceStatus.restype = wintypes.BOOL
    advapi.StartServiceCtrlDispatcherW.argtypes = [ctypes.POINTER(SERVICE_TABLE_ENTRY)]
    advapi.StartServiceCtrlDispatcherW.restype = wintypes.BOOL

    stop = threading.Event()
    runtime: dict[str, object] = {"listener": None, "connection": None}
    status_handle = ctypes.c_void_p()
    status_lock = threading.RLock()
    stopped_reported = False
    service_failure: list[int] = []

    def report_status(state: int, controls: int, checkpoint: int, wait_hint: int,
                      win32_exit: int = 0, service_exit: int = 0) -> None:
        nonlocal stopped_reported
        with status_lock:
            if state == SERVICE_STOPPED and stopped_reported:
                return
            status = SERVICE_STATUS(
                SERVICE_WIN32_OWN_PROCESS, state, controls,
                int(win32_exit), int(service_exit), int(checkpoint), int(wait_hint),
            )
            if not advapi.SetServiceStatus(status_handle, ctypes.byref(status)):
                error = ctypes.get_last_error()
                raise ctypes.WinError(error)
            if state == SERVICE_STOPPED:
                stopped_reported = True

    @SERVICE_HANDLER
    def handler(control, _event_type, _event_data, _context):
        if control != SERVICE_CONTROL_STOP or stop.is_set():
            return 0
        try:
            report_status(SERVICE_STOP_PENDING, 0, 1, 5000)
            _startup_diagnostic(state_root, "stop-pending")
        except Exception as exc:
            _startup_diagnostic(state_root, "stop-pending", exc, ctypes.get_last_error())
        # Signal only after STOP_PENDING has been reported. Closing both
        # handles makes an in-flight accept/receive interruptible and bounded.
        stop.set()
        listener = runtime.get("listener")
        connection = runtime.get("connection")
        if listener is not None:
            listener.close()  # type: ignore[attr-defined]
        if connection is not None:
            connection.close()  # type: ignore[attr-defined]
        return 0

    @SERVICE_MAIN
    def service_main(_argc, _argv):
        nonlocal status_handle
        _startup_diagnostic(state_root, "service-main-entered")
        status_handle = advapi.RegisterServiceCtrlHandlerExW(SERVICE_NAME, handler, None)
        if not status_handle:
            error = ctypes.get_last_error()
            service_failure.append(error or 1)
            _startup_diagnostic(state_root, "handler-registered", win32_error=error)
            return
        _startup_diagnostic(state_root, "handler-registered")
        try:
            report_status(SERVICE_START_PENDING, 0, 1, 3000)
            _startup_diagnostic(state_root, "start-pending-reported")
            service, listener = _initialise_runtime(state_root, runtime)
            if stop.is_set():
                return
            report_status(SERVICE_RUNNING, SERVICE_ACCEPT_STOP, 0, 0)
            _startup_diagnostic(state_root, "running-reported")
            _serve_loop(state_root, service, listener, stop, runtime)
        except Exception as exc:
            error = ctypes.get_last_error()
            service_failure.append(error or 1)
            _startup_diagnostic(state_root, "startup-failure", exc, error)
            try:
                report_status(SERVICE_STOPPED, 0, 0, 0, error or 1, 1)
                _startup_diagnostic(state_root, "stopped")
            except Exception as status_exc:
                _startup_diagnostic(state_root, "stopped", status_exc, ctypes.get_last_error())
        finally:
            if not stopped_reported:
                try:
                    report_status(SERVICE_STOPPED, 0, 0, 0)
                    _startup_diagnostic(state_root, "stopped")
                except Exception as exc:
                    _startup_diagnostic(state_root, "stopped", exc, ctypes.get_last_error())

    dispatch = (SERVICE_TABLE_ENTRY * 2)()
    dispatch[0] = SERVICE_TABLE_ENTRY(SERVICE_NAME, service_main)
    dispatch[1] = SERVICE_TABLE_ENTRY(None, SERVICE_MAIN())
    _startup_diagnostic(state_root, "dispatcher-call")
    if not advapi.StartServiceCtrlDispatcherW(dispatch):
        error = ctypes.get_last_error()
        _startup_diagnostic(state_root, "dispatcher-return", win32_error=error)
        if error == ERROR_FAILED_SERVICE_CONTROLLER_CONNECT:
            print(
                "ERROR_FAILED_SERVICE_CONTROLLER_CONNECT (1063): "
                "this command was run outside the Windows Service Control Manager.",
                file=sys.stderr,
            )
        else:
            print(f"StartServiceCtrlDispatcherW failed: {ctypes.WinError(error)}", file=sys.stderr)
        return error or 1
    _startup_diagnostic(state_root, "dispatcher-return")
    return service_failure[0] if service_failure else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="repopact-guard-service")
    parser.add_argument("--state-root", type=Path, required=True)
    parser.add_argument("--service", action="store_true")
    args = parser.parse_args(argv)
    state_root = args.state_root.resolve()
    _startup_diagnostic(state_root, "arguments-parsed")
    if args.service:
        return _run_as_native_service(state_root)
    serve(state_root)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
