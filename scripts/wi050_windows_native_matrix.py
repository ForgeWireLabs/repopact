"""Run the explicit WI050 native Windows destructive proof.

This harness is deliberately opt-in.  It creates disposable Git fixtures,
registers one through the installed Windows service, and exercises the native
pipe/lease contract.  It must never be folded into portable conformance: the
service is stopped and restarted during this proof and the fixture registration
is machine state.

The operator supplies an encrypted Ed25519 key outside the repository.  The
key is loaded only in memory; neither it nor opaque lease tokens are written to
the evidence record.
"""
from __future__ import annotations

import argparse
import ctypes
import getpass
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import uuid
from ctypes import wintypes
from pathlib import Path
from typing import Any, Mapping

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from repopact.admission import (  # noqa: E402
    Ed25519Signer,
    canonical_identity,
    digest,
    issue_receipt,
    make_request,
)
from repopact.dev_fixtures import FixtureRepo  # noqa: E402
from repopact.guard_ipc import (  # noqa: E402
    NativeGuardClient,
    WINDOWS_PIPE,
    WindowsPipeConnection,
    _windows_server_verified,
    windows_peer_identity,
    windows_peer_image_path,
)
from repopact.platform_backends import WindowsBackend  # noqa: E402


def _json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def _hash(path: Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def _run(command: list[str], *, check: bool = False) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(command, text=True, capture_output=True, check=False)
    if check and result.returncode:
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(command)}: {result.stderr.strip()}")
    return result


def _decision(value: Any) -> dict[str, Any]:
    if hasattr(value, "__dict__"):
        value = value.__dict__
    if isinstance(value, Mapping):
        return {key: value[key] for key in ("allowed", "code", "reason", "enforcement") if key in value}
    return {"allowed": False, "code": "HARNESS_ERROR", "reason": repr(value)}


def _service_facts(backend: WindowsBackend) -> dict[str, Any]:
    qc = _run(["sc.exe", "qc", backend.service_name])
    query = _run(["sc.exe", "queryex", backend.service_name])
    identity, image = backend._service_configuration()
    return {
        "service_name": backend.service_name,
        "configured_identity": identity,
        "configured_image": image,
        "query_exit_code": query.returncode,
        "running": "RUNNING" in query.stdout.upper(),
        "service_pid": backend._service_pid(),
        "sc_qc_exit_code": qc.returncode,
        "scm_config_contains_runtime": str(backend.runtime_path).upper() in qc.stdout.upper().replace("/", "\\"),
    }


def _pipe_identity(backend: WindowsBackend) -> dict[str, Any]:
    kernel = ctypes.WinDLL("Kernel32", use_last_error=True)
    kernel.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, wintypes.LPVOID,
                                   wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
    kernel.CreateFileW.restype = wintypes.HANDLE
    handle = kernel.CreateFileW(WINDOWS_PIPE, 0xC0000000, 0, None, 3, 0, None)
    if handle == wintypes.HANDLE(-1).value:
        raise ctypes.WinError(ctypes.get_last_error())
    connection = WindowsPipeConnection(handle)
    try:
        peer = windows_peer_identity(connection, client=False)
        image = windows_peer_image_path(connection)
        configured_identity, configured_image = backend._service_configuration()
        executable = configured_image[1:configured_image.find('"', 1)] if configured_image.startswith('"') else configured_image.split(None, 1)[0]
        service_pid = backend._service_pid()
        return {
            "peer_pid": peer.peer_pid,
            "service_pid": service_pid,
            "peer_pid_matches_service": peer.peer_pid is not None and peer.peer_pid == service_pid,
            "peer_image": image,
            "configured_executable": executable,
            "peer_image_matches_service": bool(image and executable and os.path.normcase(image) == os.path.normcase(executable)),
            "configured_identity": configured_identity,
            "server_verified": _windows_server_verified(connection, None, backend.service_name),
        }
    finally:
        connection.close()


def _wait_healthy(client: NativeGuardClient, timeout: float = 8.0) -> Any:
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        last = client.health()
        if last.healthy:
            return last
        time.sleep(0.15)
    raise RuntimeError(f"native service did not become healthy: {last}")


def _restart_service(client: NativeGuardClient) -> Any:
    _run(["powershell.exe", "-NoProfile", "-NonInteractive", "-Command",
          "Restart-Service -Name RepoPactGuard -Force"], check=True)
    return _wait_healthy(client)


def _action(request: Mapping[str, Any], root: Path, paths: list[str], *, kind: str = "mutation") -> dict[str, Any]:
    return {
        "kind": kind,
        "work_item": request["work_item"],
        "principal": request["principal"],
        "session_id": request["adapter_session"],
        "repository_identity": digest(canonical_identity(root)),
        "repopact_root": request["repopact_root"],
        "profile": request["profile"],
        "mode": request["mode"],
        "scopes": request["scopes"],
        "paths": paths,
        "capabilities": request["capabilities"],
    }


def _auth(root: Path, signer: Ed25519Signer, protected: Path, paths: list[str], session: str) -> tuple[dict[str, Any], dict[str, Any], str, dict[str, Any]]:
    request = make_request(root, "050", session, principal="wi050-windows-proof", profile="bounded",
                           scopes=["src"], paths=paths, capabilities={"process": True},
                           approval_class="activate", protected_dir=protected)
    receipt = issue_receipt(request, signer)
    decision, capability = NativeGuardClient().authorize(request, receipt, root=root)
    result = _decision(decision)
    if not result.get("allowed") or not capability or not isinstance(capability.get("lease_token"), str):
        raise RuntimeError(f"native authorization failed: {result}")
    token = capability["lease_token"]
    metadata = capability.get("lease_metadata", {})
    return request, receipt, token, {"decision": result, "metadata": metadata, "token_sha256": hashlib.sha256(token.encode()).hexdigest()}


def _child_peer_check(root: Path, action: Mapping[str, Any], token: str) -> dict[str, Any]:
    child = (
        "import json,sys; from pathlib import Path; "
        "from repopact.guard_ipc import NativeGuardClient; "
        "a=json.loads(sys.argv[1]); r=Path(sys.argv[2]); "
        "d=NativeGuardClient().check(a,sys.argv[3],root=r); print(json.dumps(d.__dict__,sort_keys=True))"
    )
    cp = subprocess.run([sys.executable, "-c", child, _json(action), str(root), token],
                        text=True, capture_output=True, check=False)
    if not cp.stdout.strip():
        return {"allowed": False, "code": "CHILD_NO_RESPONSE", "reason": cp.stderr.strip()}
    try:
        return json.loads(cp.stdout.strip().splitlines()[-1])
    except json.JSONDecodeError:
        return {"allowed": False, "code": "CHILD_BAD_RESPONSE", "reason": cp.stdout.strip()[-300:]}


class _SidAndAttributes(ctypes.Structure):
    _fields_ = [("Sid", ctypes.c_void_p), ("Attributes", ctypes.c_uint32)]


class _StartupInfo(ctypes.Structure):
    _fields_ = [("cb", ctypes.c_uint32), ("lpReserved", ctypes.c_wchar_p), ("lpDesktop", ctypes.c_wchar_p),
                ("lpTitle", ctypes.c_wchar_p), ("dwX", ctypes.c_uint32), ("dwY", ctypes.c_uint32),
                ("dwXSize", ctypes.c_uint32), ("dwYSize", ctypes.c_uint32), ("dwXCountChars", ctypes.c_uint32),
                ("dwYCountChars", ctypes.c_uint32), ("dwFillAttribute", ctypes.c_uint32), ("dwFlags", ctypes.c_uint32),
                ("wShowWindow", ctypes.c_uint16), ("cbReserved2", ctypes.c_uint16), ("lpReserved2", ctypes.c_void_p),
                ("hStdInput", ctypes.c_void_p), ("hStdOutput", ctypes.c_void_p), ("hStdError", ctypes.c_void_p)]


class _ProcessInfo(ctypes.Structure):
    _fields_ = [("hProcess", ctypes.c_void_p), ("hThread", ctypes.c_void_p),
                ("dwProcessId", ctypes.c_uint32), ("dwThreadId", ctypes.c_uint32)]


def _restricted_attempt(paths: list[Path], service_name: str) -> dict[str, Any]:
    """Run bounded write/config attempts without the Administrators SID."""
    if os.name != "nt":
        raise RuntimeError("restricted-token proof requires Windows")
    output_handle = tempfile.NamedTemporaryFile(prefix="wi050-restricted-", suffix=".json", delete=False)
    output = Path(output_handle.name)
    output_handle.close()
    child = (
        "import json, pathlib, subprocess, sys; "
        "paths=[pathlib.Path(x) for x in json.loads(sys.argv[1])]; out=pathlib.Path(sys.argv[2]); "
        "items=[]; "
        "\nfor p in paths:\n"
        "  try:\n    b=p.read_bytes(); p.write_bytes(b); items.append({'target':str(p),'result':'write-succeeded'})\n"
        "  except Exception as e: items.append({'target':str(p),'result':'denied','error':type(e).__name__})\n"
        "cp=subprocess.run(['sc.exe','config',sys.argv[3],'start=','auto'],text=True,capture_output=True); "
        "items.append({'target':'service-configuration','result':'write-succeeded' if cp.returncode == 0 else 'denied','exit_code':cp.returncode}); "
        "out.write_text(json.dumps(items,sort_keys=True))"
    )
    kernel = ctypes.WinDLL("Kernel32", use_last_error=True)
    advapi = ctypes.WinDLL("Advapi32", use_last_error=True)
    kernel.GetCurrentProcess.restype = ctypes.c_void_p
    advapi.OpenProcessToken.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.POINTER(ctypes.c_void_p)]
    advapi.OpenProcessToken.restype = ctypes.c_int
    advapi.CreateRestrictedToken.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32,
                                              ctypes.POINTER(_SidAndAttributes), ctypes.c_uint32, ctypes.c_void_p,
                                              ctypes.c_uint32, ctypes.c_void_p, ctypes.POINTER(ctypes.c_void_p)]
    advapi.CreateRestrictedToken.restype = ctypes.c_int
    advapi.ConvertStringSidToSidW.argtypes = [ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_void_p)]
    advapi.ConvertStringSidToSidW.restype = ctypes.c_int
    advapi.CreateProcessAsUserW.argtypes = [ctypes.c_void_p, ctypes.c_wchar_p, ctypes.c_wchar_p, ctypes.c_void_p,
                                             ctypes.c_void_p, ctypes.c_int, ctypes.c_uint32, ctypes.c_void_p,
                                             ctypes.c_wchar_p, ctypes.POINTER(_StartupInfo), ctypes.POINTER(_ProcessInfo)]
    advapi.CreateProcessAsUserW.restype = ctypes.c_int
    kernel.CloseHandle.argtypes = [ctypes.c_void_p]
    kernel.LocalFree.argtypes = [ctypes.c_void_p]
    token = ctypes.c_void_p()
    new_token = ctypes.c_void_p()
    sid = ctypes.c_void_p()
    token_access = 0x0002 | 0x0008 | 0x0001 | 0x0080 | 0x0100 | 0x0200
    if not advapi.OpenProcessToken(kernel.GetCurrentProcess(), token_access, ctypes.byref(token)):
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        if not advapi.ConvertStringSidToSidW("S-1-5-32-544", ctypes.byref(sid)):
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            disabled = _SidAndAttributes(sid.value, 0)
            if not advapi.CreateRestrictedToken(token, 0x1, 1, ctypes.byref(disabled), 0, None, 0, None, ctypes.byref(new_token)):
                raise ctypes.WinError(ctypes.get_last_error())
        finally:
            kernel.LocalFree(sid)
    finally:
        kernel.CloseHandle(token)
    try:
        command = subprocess.list2cmdline([sys.executable, "-c", child, _json([str(p) for p in paths]), str(output), service_name])
        command_buffer = ctypes.create_unicode_buffer(command)
        startup = _StartupInfo(); startup.cb = ctypes.sizeof(_StartupInfo)
        process = _ProcessInfo()
        CREATE_NO_WINDOW = 0x08000000
        if not advapi.CreateProcessAsUserW(new_token, None, command_buffer, None, None, False, CREATE_NO_WINDOW,
                                           None, str(Path.cwd()), ctypes.byref(startup), ctypes.byref(process)):
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            kernel.WaitForSingleObject(process.hProcess, 15000)
        finally:
            kernel.CloseHandle(process.hThread); kernel.CloseHandle(process.hProcess)
    finally:
        kernel.CloseHandle(new_token)
    try:
        return json.loads(output.read_text())
    finally:
        output.unlink(missing_ok=True)


def _case(results: list[dict[str, Any]], name: str, expected: str, **checks: Any) -> None:
    results.append({"case": name, "expected": expected, "checks": checks,
                    "pass": all(value is True for value in checks.values())})


def _commit_fixture(root: Path) -> None:
    _run(["git", "-C", str(root), "add", "governance/admission-policy.json",
          "governance/operator-authority.json", "governance/repository-registration.json"], check=True)
    _run(["git", "-C", str(root), "commit", "-m", "fixture: record native admission registration"], check=True)


def run(args: argparse.Namespace) -> dict[str, Any]:
    if os.name != "nt":
        raise SystemExit("WI050 Windows native proof requires Windows; no fallback is permitted")
    if not args.native_destructive:
        raise SystemExit("refusing destructive proof without --native-destructive")
    source = Path(args.source_root).resolve()
    key_file = Path(args.key_file).resolve()
    if not (source / ".git").exists():
        raise SystemExit(f"source root is not a Git checkout: {source}")
    signer = Ed25519Signer.load(key_file, getpass.getpass("External operator key passphrase: "))
    backend = WindowsBackend()
    before_attestation = backend.attest()
    if not before_attestation.healthy or not before_attestation.protected_from_gated_principal:
        raise SystemExit("installed protected Windows guard is not healthy; refusing native destructive proof")
    client = NativeGuardClient()
    root_fixture: FixtureRepo | None = None
    other_fixture: FixtureRepo | None = None
    linked: Path | None = None
    registration_path: Path | None = None
    results: list[dict[str, Any]] = []
    cleanup: dict[str, Any] = {}
    try:
        root_fixture = FixtureRepo(source_root=source, prefix="wi050-windows-native-primary-").open()
        other_fixture = FixtureRepo(source_root=source, prefix="wi050-windows-native-independent-").open()
        root = root_fixture.root; other = other_fixture.root
        assert root is not None and other is not None
        sentinel = root / "src" / "wi050-native-sentinel.txt"
        sentinel.parent.mkdir(parents=True, exist_ok=True); sentinel.write_text("before\n", encoding="utf-8")
        _run(["git", "-C", str(root), "add", "src/wi050-native-sentinel.txt"], check=True)
        _run(["git", "-C", str(root), "commit", "-m", "fixture: add native sentinel"], check=True)
        registration = backend.register(root, signer=signer)
        registration_path = Path(str(registration["protected"]))
        _commit_fixture(root)
        public_registration = json.loads((root / "governance/repository-registration.json").read_text())
        service_facts = _service_facts(backend)
        health = _wait_healthy(client)
        pipe_identity = _pipe_identity(backend)
        # The rootless call is the real installed service health path; the
        # identity proof below uses the same AF_PIPE connection semantics as
        # NativeGuardClient, without recording any handle or credential.
        _case(results, "connect_real_service", "healthy native service", healthy=health.healthy,
              backend_id=health.backend_id == "windows-service")
        _case(results, "verify_server_identity", "LocalSystem and protected installed image",
              service_identity=pipe_identity["configured_identity"].casefold().replace(" ", "") in {"localsystem", "ntauthority\\system"},
              protected_health=health.protected and health.service_identity_verified,
              runtime_in_scm=service_facts["scm_config_contains_runtime"],
              peer_pid_matches_service=pipe_identity["peer_pid_matches_service"],
              peer_image_matches_service=pipe_identity["peer_image_matches_service"],
              server_verified=pipe_identity["server_verified"])

        # Unregistered repository denial, before any authorization for it.
        other_sentinel = other / "src" / "wi050-independent-sentinel.txt"
        other_sentinel.write_text("unchanged\n", encoding="utf-8")
        unregistered_before = _hash(other_sentinel)
        unregistered_action = {"kind": "mutation", "work_item": "050", "paths": ["src/wi050-independent-sentinel.txt"],
                               "scopes": ["src"], "repopact_root": str(other)}
        unregistered = client.check(unregistered_action, None, root=other)
        _case(results, "unregistered_repo_denied", "native denial", allowed=not unregistered.allowed,
              denial_code=bool(unregistered.code), sentinel_unchanged=_hash(other_sentinel) == unregistered_before)

        allowed_rel = "src/wi050-native-sentinel.txt"
        request, receipt, token, auth_info = _auth(root, signer, registration_path, [allowed_rel], "wi050-native-authorize")
        metadata = auth_info["metadata"]
        _case(results, "authorize_opaque_token", "approved opaque lease and in-scope mutation",
              authorized=auth_info["decision"].get("allowed") is True,
              opaque=bool(token) and "signature" not in metadata and "private" not in metadata,
              token_hash_len=len(auth_info["token_sha256"]) == 64)
        before = _hash(sentinel)
        allowed = client.check(_action(request, root, [allowed_rel]), token, root=root)
        if allowed.allowed:
            sentinel.write_text("authorized\n", encoding="utf-8")
        after = _hash(sentinel)
        _case(results, "positive_authorized_mutation", "allowed mutation changes declared sentinel",
              check_allowed=allowed.allowed, before_hash=bool(before), after_hash=bool(after),
              changed=before != after)

        out_request, _, out_token, _ = _auth(root, signer, registration_path, [allowed_rel], "wi050-native-out-of-scope")
        out_before = _hash(sentinel)
        out_scope = client.check(_action(out_request, root, ["governance/wi050-out-of-scope.txt"]), out_token, root=root)
        _case(results, "out_of_scope_mutation_denied", "frozen/out-of-scope mutation denied before write",
              denied=not out_scope.allowed, denial_code=out_scope.code == "FROZEN_APPROVAL_REQUIRED",
              sentinel_unchanged=_hash(sentinel) == out_before)

        forged = token[:-1] + ("A" if token[-1] != "A" else "B")
        forged_before = _hash(sentinel)
        forged_decision = client.check(_action(request, root, [allowed_rel]), forged, root=root)
        _case(results, "token_mutation_forgery_denied", "opaque token forgery denied",
              denied=not forged_decision.allowed, denial_code=forged_decision.code == "NO_OPERATOR_PROOF",
              sentinel_unchanged=_hash(sentinel) == forged_before)

        request, receipt, token, _ = _auth(root, signer, registration_path, [allowed_rel], "wi050-native-peer")
        peer_decision = _child_peer_check(root, _action(request, root, [allowed_rel]), token)
        _case(results, "cross_process_peer_binding_denied", "different process peer denied",
              denied=not peer_decision.get("allowed", False), denial_code=peer_decision.get("code") == "WRONG_SESSION")

        request, receipt, old_token, _ = _auth(root, signer, registration_path, [allowed_rel], "wi050-native-restart-old")
        _restart_service(client)
        stale = client.check(_action(request, root, [allowed_rel]), old_token, root=root)
        fresh_request, _, fresh_token, _ = _auth(root, signer, registration_path, [allowed_rel], "wi050-native-restart-new")
        fresh = client.check(_action(fresh_request, root, [allowed_rel]), fresh_token, root=root)
        _case(results, "restart_invalidates_token", "old lease denied after service restart; fresh lease works",
              old_denied=not stale.allowed, old_code=stale.code == "NO_OPERATOR_PROOF", fresh_allowed=fresh.allowed)

        linked = root.parent / "linked-worktree"
        _run(["git", "-C", str(root), "worktree", "add", "--detach", str(linked), "HEAD"], check=True)
        linked_rel = "src/wi050-linked-sentinel.txt"
        linked_sentinel = linked / linked_rel
        linked_sentinel.write_text("before\n", encoding="utf-8")
        _run(["git", "-C", str(linked), "add", linked_rel], check=True)
        _run(["git", "-C", str(linked), "commit", "-m", "fixture: add linked sentinel"], check=True)
        linked_request, _, linked_token, _ = _auth(linked, signer, registration_path, [linked_rel], "wi050-native-linked")
        linked_decision = client.check(_action(linked_request, linked, [linked_rel]), linked_token, root=linked)
        if linked_decision.allowed:
            linked_sentinel.write_text("authorized-linked\n", encoding="utf-8")
        _case(results, "linked_worktree_same_registration", "linked worktree uses common registration",
              allowed=linked_decision.allowed, common_registration=canonical_identity(linked)["git_common_dir"] == canonical_identity(root)["git_common_dir"])

        _case(results, "independent_repo_unregistered", "independent repository remains unregistered",
              denied=not unregistered.allowed, distinct_identity=digest(canonical_identity(root)) != digest(canonical_identity(other)))

        request, _, stop_token, _ = _auth(root, signer, registration_path, [allowed_rel], "wi050-native-stop")
        stop_before = _hash(sentinel)
        _run(["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", "Stop-Service -Name RepoPactGuard -Force"], check=True)
        stopped = client.check(_action(request, root, [allowed_rel]), stop_token, root=root)
        _restart_service(client)
        _case(results, "service_stop_fail_closed", "service loss denies before mutation",
              denied=not stopped.allowed, no_advisory_downgrade=stopped.code in {"GUARD_UNHEALTHY", "NO_OPERATOR_PROOF"},
              sentinel_unchanged=_hash(sentinel) == stop_before)

        runtime_before = backend._runtime_digest()
        checkout_guard = root / "repopact" / "guard.py"
        checkout_original = checkout_guard.read_bytes()
        checkout_guard.write_bytes(checkout_original + b"\n# disposable checkout modification\n")
        checkout_health = client.health()
        runtime_after = backend._runtime_digest()
        _case(results, "checkout_runtime_modification_irrelevant", "checkout copy cannot alter installed runtime",
              healthy=checkout_health.healthy, installed_digest_unchanged=runtime_before == runtime_after)

        protected_targets = [backend.runtime_path / "repopact" / "guard.py"]
        if registration_path.exists():
            protected_targets.append(registration_path / "registration.json")
        tamper = _restricted_attempt(protected_targets, backend.service_name)
        _case(results, "protected_state_and_service_config_denied", "restricted non-service principal denied",
              all_denied=all(item.get("result") == "denied" for item in tamper),
              service_config_denied=next((item.get("result") == "denied" for item in tamper if item.get("target") == "service-configuration"), False),
              health_after_tamper=client.health().healthy)

        evidence = {
            "id": args.evidence_id,
            "timestamp": __import__("datetime").datetime.now(__import__("datetime").timezone.utc).isoformat().replace("+00:00", "Z"),
            "work_item": "050", "result": "passed" if all(item["pass"] for item in results) else "failed",
            "provenance": "concrete",
            "commands": [{"command": "python scripts/wi050_windows_native_matrix.py --native-destructive --key-file <external>", "exit_code": 0 if all(item["pass"] for item in results) else 1}],
            "artifacts": ["evidence/runs/20260916-050-windows-native-destructive-proof.json", "evidence/runs/20260916-050-linux-landlock-native-proof.json"],
            "environment": {"platform": "windows", "source_revision": _run(["git", "-C", str(source), "rev-parse", "HEAD"]).stdout.strip(),
                            "service": service_facts, "health": health.__dict__,
                            "fixture_repository_identity": digest(canonical_identity(root)),
                            "registration_adoption_id": public_registration.get("adoption_id"),
                            "external_signer": {"key_id": signer.key_id, "operator_id": signer.operator_id, "key_path_external": str(key_file)},
                            "operator_approval": "make_request + Ed25519Signer.issue_receipt + NativeGuardClient.authorize",
                            "cases": results, "cleanup": cleanup,
                            "ac18": "pending: macOS native proof remains outstanding"},
        }
        return evidence
    finally:
        if linked is not None and linked.exists() and root_fixture is not None and root_fixture.root is not None:
            _run(["git", "-C", str(root_fixture.root), "worktree", "remove", "--force", str(linked)], check=False)
        if registration_path is not None:
            try:
                shutil.rmtree(registration_path)
                cleanup["registration_removed"] = True
            except OSError as exc:
                cleanup["registration_removed"] = False
                cleanup["registration_cleanup_error"] = type(exc).__name__
        if other_fixture is not None:
            try:
                other_fixture.close()
                cleanup["independent_fixture_removed"] = True
            except OSError as exc:
                cleanup["independent_fixture_removed"] = False
                cleanup["independent_fixture_cleanup_error"] = type(exc).__name__
        if root_fixture is not None:
            try:
                root_fixture.close()
                cleanup["primary_fixture_removed"] = True
            except OSError as exc:
                cleanup["primary_fixture_removed"] = False
                cleanup["primary_fixture_cleanup_error"] = type(exc).__name__
        if args.delete_key:
            try:
                key_file.unlink()
                cleanup["external_key_removed"] = True
            except OSError as exc:
                cleanup["external_key_removed"] = False
                cleanup["external_key_cleanup_error"] = type(exc).__name__
        else:
            cleanup["external_key_removed"] = False
            cleanup["external_key_disposition"] = "retained; operator must remove the test-only external key"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native-destructive", action="store_true", help="required explicit opt-in")
    parser.add_argument("--source-root", type=Path, default=Path.cwd(), help="clean source checkout used only to build fixtures")
    parser.add_argument("--key-file", type=Path, required=True, help="external encrypted Ed25519 operator key")
    parser.add_argument("--evidence-output", type=Path, default=Path("evidence/runs/20260916-050-windows-native-destructive-proof.json"))
    parser.add_argument("--evidence-id", default="20260916-050-windows-native-destructive-proof")
    parser.add_argument("--delete-key", action="store_true", help="remove the supplied external test key after the run")
    args = parser.parse_args()
    evidence = run(args)
    cleanup = evidence["environment"]["cleanup"]
    cleanup_ok = cleanup.get("registration_removed") is True and cleanup.get("primary_fixture_removed") is True and cleanup.get("independent_fixture_removed") is True
    if args.delete_key:
        cleanup_ok = cleanup_ok and cleanup.get("external_key_removed") is True
    evidence["result"] = "passed" if evidence["result"] == "passed" and cleanup_ok else "failed"
    evidence["commands"][0]["exit_code"] = 0 if evidence["result"] == "passed" else 1
    output = Path(args.evidence_output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(_json(evidence) + "\n", encoding="utf-8", newline="\n")
    print(_json({"result": evidence["result"], "evidence_id": evidence["id"],
                 "passed": sum(item["pass"] for item in evidence["environment"]["cases"]),
                 "total": len(evidence["environment"]["cases"]), "ac18": evidence["environment"]["ac18"]}))
    return 0 if evidence["result"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
