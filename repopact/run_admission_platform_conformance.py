"""Cross-platform WI050 semantic and protected-backend conformance harness.

Run ``python -m repopact.run_admission_platform_conformance --root .`` on any
OS.  The portable cases use an explicitly named testing backend so they never
masquerade as host-boundary evidence.  ``--require-installed`` turns an absent
or unhealthy native service into a failing platform proof.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import sys
from pathlib import Path
from typing import Any

from .admission import Ed25519Signer, evaluate_action, issue_lease, issue_receipt, make_request, setup_admission
from .adapters import LauncherAdapter, PreActionAdapter
from .dev_fixtures import FixtureRepo
from .guard import ProtectedGuard
from .guard_ipc import NativeGuardClient
from .platform_backends import TestingBackend, current_backend


def _fixture(source: Path) -> tuple[FixtureRepo, Path, Path, Ed25519Signer]:
    holder = FixtureRepo(source_root=source, prefix="repopact-platform-conformance-")
    holder.open()
    root = holder.root
    protected = root.parent / "protected"
    signer = Ed25519Signer.generate("platform-key", "platform-operator")
    setup_admission(root, protected, signer)
    return holder, root, protected, signer


def _reference_process_matrix(fixture: Path, guard: ProtectedGuard) -> dict[str, Any]:
    """Exercise real process-shaped attempts through the pre-action seam.

    These cases intentionally use the explicit testing backend. They prove
    that the adapter refuses to create a child or invoke a write callback
    before admission; they do not prove arbitrary-process confinement.
    """
    sentinel = fixture / "src" / "pre-action-sentinel.txt"
    sentinel.parent.mkdir(parents=True, exist_ok=True)
    sentinel.write_bytes(b"stable\n")
    before = hashlib.sha256(sentinel.read_bytes()).hexdigest()
    nested = fixture / "src" / "nested"
    nested.mkdir(parents=True, exist_ok=True)
    action = {"kind": "process", "work_item": "050", "paths": ["src/pre-action-sentinel.txt"], "scopes": ["src"]}
    launcher = LauncherAdapter(guard)
    commands: dict[str, list[str]] = {
        "python-filesystem": [sys.executable, "-c", f"open(r'{sentinel}', 'wb').write(b'changed')"],
        "python-child-process": [sys.executable, "-c", f"import subprocess,sys; subprocess.run([sys.executable, '-c', \"open(r'{sentinel}', 'wb').write(b'child')\"], check=True)"],
    }
    powershell = shutil.which("pwsh") or shutil.which("powershell")
    if powershell:
        commands["powershell"] = [powershell, "-NoProfile", "-Command", f"Set-Content -LiteralPath '{sentinel}' -Value changed"]
    command_shell = shutil.which("cmd.exe") or shutil.which("cmd")
    if command_shell:
        commands["cmd"] = [command_shell, "/c", f'echo changed > "{sentinel}"']
    posix_shell = shutil.which("sh")
    if posix_shell:
        commands["posix-shell"] = [posix_shell, "-c", f"printf changed > '{sentinel}'"]

    command_results: dict[str, bool] = {}
    denial_codes: dict[str, str] = {}
    for name, command in commands.items():
        decision, child = launcher.launch(action, command, cwd=nested)
        command_results[name] = (not decision.allowed and child is None)
        denial_codes[name] = decision.code

    callback_called = False
    pre_action = PreActionAdapter(guard)

    def write_callback() -> None:
        nonlocal callback_called
        callback_called = True
        sentinel.write_bytes(b"callback\n")

    direct_decision, _ = pre_action.before(
        {"kind": "mutation", "work_item": "050", "paths": ["src/pre-action-sentinel.txt"], "scopes": ["src"]},
        write_callback,
    )
    return {
        "assurance": "pre-action",
        "backend": "testing-only-attested-backend",
        "commands": command_results,
        "denial_codes": denial_codes,
        "nested_working_directory": all(command_results.values()),
        "child_creation_denied": all(command_results.values()),
        "direct_callback_denied": not direct_decision.allowed and not callback_called,
        "sentinel_sha256_before": before,
        "sentinel_sha256_after": hashlib.sha256(sentinel.read_bytes()).hexdigest(),
        "sentinel_unchanged": hashlib.sha256(sentinel.read_bytes()).hexdigest() == before,
        "native_process_confinement_proven": False,
    }


def run(root: Path) -> dict[str, Any]:
    backend = current_backend(root)
    holder, fixture, protected, signer = _fixture(root)
    try:
        guard = ProtectedGuard(fixture, protected, backend=TestingBackend(protected))
        request = make_request(fixture, "050", "platform-session", scopes=["src"], paths=["src/a.py"], protected_dir=protected)
        receipt = issue_receipt(request, signer)
        proof, lease = issue_lease(request, receipt, fixture, protected)
        cases: dict[str, bool] = {
            "service_attestation_is_explicit": backend.attest(root).record().get("testing_only") is False,
            "no_lease_mutation_denied": evaluate_action(fixture, {"kind": "mutation", "work_item": "050", "paths": ["src/a.py"]}, protected_dir=protected).code == "NO_OPERATOR_PROOF",
            "valid_lease_allowed": bool(proof.allowed and lease and guard.check({"kind": "mutation", "work_item": "050", "paths": ["src/a.py"], "scopes": ["src"], "session_id": "platform-session", "principal": "agent"}, lease).allowed),
            "wrong_lease_denied": bool(lease and guard.check({"kind": "mutation", "work_item": "050", "paths": ["src/a.py"], "scopes": ["src"], "session_id": "other-session"}, lease).code == "WRONG_SESSION"),
            "expiry_or_revocation_semantics_present": bool(lease and "expires_at" in lease and "revocation_epoch" in lease),
            "guard_health_is_backend_owned": guard.health().backend_id == "testing-only-attested-backend" and guard.health().testing_only,
        }
        process_matrix = _reference_process_matrix(fixture, guard)
        cases["real_subprocess_pre_action_matrix"] = bool(
            process_matrix["nested_working_directory"]
            and process_matrix["child_creation_denied"]
            and process_matrix["direct_callback_denied"]
            and process_matrix["sentinel_unchanged"]
        )
        native_cases = [
            "connect_real_service", "verify_server_identity", "unregistered_repo_denied", "authorize_opaque_token",
            "token_mutation_forgery_denied", "cross_process_peer_binding_denied", "restart_invalidates_token",
            "linked_worktree_same_registration", "independent_repo_unregistered", "service_stop_fail_closed",
            "checkout_runtime_modification_irrelevant", "protected_state_and_service_config_denied",
        ]
        native = {"executed": False, "cases": {name: "not-run" for name in native_cases},
                  "reason": "native protected service is not installed"}
        if backend.attest(root).healthy:
            client = NativeGuardClient(root=root)
            health = client.health(root)
            native["executed"] = bool(health.healthy)
            native["cases"]["connect_real_service"] = "passed" if health.healthy else "failed"
            native["cases"]["verify_server_identity"] = "passed" if health.service_identity_verified else "failed"
            native["reason"] = "service health and identity reached through NativeGuardClient; full destructive matrix requires registered fixtures"
        return {"result": "passed" if all(cases.values()) else "failed", "os": backend.os_name,
                "backend": backend.health(), "semantic_backend": guard.health().__dict__, "cases": cases,
                "native": native,
                "process_path": process_matrix,
                "fixture": "temporary Git repository; testing-only backend is not platform proof"}
    finally:
        holder.close()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="repopact-platform-conformance")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--require-installed", action="store_true", help="Fail unless the native protected backend is installed and healthy")
    args = parser.parse_args(argv)
    report = run(args.root.resolve())
    print(json.dumps(report, indent=2, sort_keys=True))
    if report["result"] != "passed":
        return 1
    if args.require_installed and not report["backend"].get("healthy"):
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
