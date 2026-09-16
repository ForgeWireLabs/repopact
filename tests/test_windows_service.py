from __future__ import annotations

import ctypes
import os
import tempfile
import threading
import unittest
from pathlib import Path
from unittest.mock import patch

import repopact.windows_guard_service as service_host


@unittest.skipUnless(os.name == "nt", "native Windows service lifecycle")
class WindowsServiceLifecycleTests(unittest.TestCase):
    class _Call:
        def __init__(self, function):
            self.function = function
            self.argtypes = None
            self.restype = None

        def __call__(self, *args):
            return self.function(*args)

    class _Listener:
        def __init__(self):
            self.closed = False

        def close(self):
            self.closed = True

    class _Advapi:
        def __init__(self, *, start=True, register=True, set_status=True):
            self.statuses = []
            self.registered_handler = None
            self.start_enabled = start
            self.register_enabled = register
            self.set_status_enabled = set_status
            self.RegisterServiceCtrlHandlerExW = WindowsServiceLifecycleTests._Call(self._register)
            self.SetServiceStatus = WindowsServiceLifecycleTests._Call(self._set_status)
            self.StartServiceCtrlDispatcherW = WindowsServiceLifecycleTests._Call(self._start)

        def _register(self, _name, handler, _context):
            self.registered_handler = handler
            return ctypes.c_void_p(1) if self.register_enabled else ctypes.c_void_p()

        def _set_status(self, _handle, status):
            value = status._obj
            self.statuses.append({
                "state": value.current_state,
                "controls": value.controls_accepted,
                "checkpoint": value.check_point,
                "wait_hint": value.wait_hint,
                "win32_exit": value.win32_exit_code,
                "service_exit": value.service_specific_exit_code,
            })
            return self.set_status_enabled

        def _start(self, dispatch):
            if not self.start_enabled:
                return 0
            dispatch[0].service_proc(0, None)
            return 1

    def _run(self, api, *, initialise=None, loop=None, last_error=0):
        initialise = initialise or (lambda _root, _runtime: (object(), self._Listener()))
        loop = loop or (lambda _root, _service, _listener, _stop, _runtime: None)
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(service_host.ctypes, "WinDLL", return_value=api), \
                    patch.object(service_host.ctypes, "get_last_error", return_value=last_error), \
                    patch.object(service_host, "_startup_diagnostic"), \
                    patch.object(service_host, "_initialise_runtime", side_effect=initialise), \
                    patch.object(service_host, "_serve_loop", side_effect=loop):
                result = service_host._run_as_native_service(Path(directory))
        return result

    def test_dispatcher_failure_reports_interactive_1063(self):
        api = self._Advapi(start=False)
        self.assertEqual(self._run(api, last_error=1063), 1063)

    def test_handler_registration_failure_returns_error(self):
        api = self._Advapi(register=False)
        self.assertEqual(self._run(api, last_error=5), 5)
        self.assertEqual(api.statuses, [])

    def test_startup_and_shutdown_status_sequence(self):
        api = self._Advapi()
        entered = threading.Event()

        def loop(_root, _service, _listener, stop, _runtime):
            entered.set()
            stop.wait(2)

        result_holder = []
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(service_host.ctypes, "WinDLL", return_value=api), \
                    patch.object(service_host, "_startup_diagnostic"), \
                    patch.object(service_host, "_initialise_runtime", return_value=(object(), self._Listener())), \
                    patch.object(service_host, "_serve_loop", side_effect=loop):
                thread = threading.Thread(
                    target=lambda: result_holder.append(service_host._run_as_native_service(Path(directory))),
                    daemon=True,
                )
                thread.start()
                self.assertTrue(entered.wait(2))
                self.assertIsNotNone(api.registered_handler)
                api.registered_handler(1, 0, None, None)
                thread.join(2)

        self.assertFalse(thread.is_alive())
        self.assertEqual(result_holder, [0])
        self.assertEqual([item["state"] for item in api.statuses], [2, 4, 3, 1])
        self.assertEqual(api.statuses[0]["controls"], 0)
        self.assertEqual(api.statuses[1]["controls"], 1)
        self.assertEqual(api.statuses[1]["checkpoint"], 0)
        self.assertEqual(api.statuses[1]["wait_hint"], 0)
        self.assertEqual(api.statuses[-1]["state"], 1)

    def test_initialisation_failure_reports_stopped_with_failure(self):
        api = self._Advapi()

        def fail(_root, _runtime):
            raise RuntimeError("listener setup failed")

        self.assertEqual(self._run(api, initialise=fail), 1)
        self.assertEqual([item["state"] for item in api.statuses], [2, 1])
        self.assertNotEqual(api.statuses[-1]["win32_exit"], 0)

    def test_status_api_failure_returns_nonzero(self):
        api = self._Advapi(set_status=False)
        self.assertEqual(self._run(api, last_error=87), 87)


if __name__ == "__main__":
    unittest.main()
