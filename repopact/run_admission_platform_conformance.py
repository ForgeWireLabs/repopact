"""Cross-platform WI050 semantic and protected-backend conformance harness.

Run ``python -m repopact.run_admission_platform_conformance --root .`` on any
OS.  The portable cases use an explicitly named testing backend so they never
masquerade as host-boundary evidence.  ``--require-installed`` turns an absent
or unhealthy native service into a failing platform proof.
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any

from .admission import Ed25519Signer, evaluate_action, issue_lease, issue_receipt, make_request, setup_admission
from .guard import ProtectedGuard
from .platform_backends import TestingBackend, current_backend


def _fixture(source: Path) -> tuple[tempfile.TemporaryDirectory[str], Path, Path, Ed25519Signer]:
    holder: tempfile.TemporaryDirectory[str] = tempfile.TemporaryDirectory(prefix="repopact-platform-conformance-")
    root = Path(holder.name) / "repo"
    shutil.copytree(source, root, ignore=shutil.ignore_patterns(".git", "__pycache__", "*.pyc", "build", "dist"))
    subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
    subprocess.run(["git", "config", "user.email", "conformance@example.invalid"], cwd=root, check=True)
    subprocess.run(["git", "config", "user.name", "RepoPact conformance"], cwd=root, check=True)
    subprocess.run(["git", "add", "."], cwd=root, check=True, capture_output=True)
    subprocess.run(["git", "commit", "-m", "platform conformance fixture"], cwd=root, check=True, capture_output=True)
    protected = Path(holder.name) / "protected"
    signer = Ed25519Signer.generate("platform-key", "platform-operator")
    setup_admission(root, protected, signer)
    return holder, root, protected, signer


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
        return {"result": "passed" if all(cases.values()) else "failed", "os": backend.os_name,
                "backend": backend.health(), "semantic_backend": guard.health().__dict__, "cases": cases,
                "fixture": "temporary Git repository; testing-only backend is not platform proof"}
    finally:
        holder.cleanup()


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
