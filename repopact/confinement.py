"""Optional Linux Landlock provider for the RepoPact launcher reference.

The provider composes with the existing protected guard.  It never creates a
lease, interprets caller paths as authority, or changes the portable
``pre-action`` result.  Its higher capability is reported only when a
host-protected helper passes its own native Landlock probe.
"""
from __future__ import annotations

from dataclasses import replace
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
from typing import Any, Mapping, Sequence

from .admission import AdmissionDecision, GuardHealth
from .enforcement import EnforcementProvider


MINIMUM_LANDLOCK_ABI = 3
DEFAULT_LANDLOCK_HELPER = Path("/usr/local/lib/repopact/guard/repopact-sandbox")


def _protected_helper(path: Path) -> tuple[bool, str]:
    """Require a regular root-owned helper and protected parent chain."""
    if os.name != "posix" or not sys.platform.startswith("linux"):
        return False, "Landlock helper attestation is Linux-only"
    try:
        current = path.absolute()
        leaf = current.lstat()
        if stat.S_ISLNK(leaf.st_mode) or not stat.S_ISREG(leaf.st_mode):
            return False, "Landlock helper is absent, non-regular, or a symlink"
        if leaf.st_uid != 0 or leaf.st_mode & 0o022:
            return False, "Landlock helper is not root-owned and non-writable"
        while True:
            info = current.lstat()
            if stat.S_ISLNK(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
                return False, f"Landlock helper parent is not protected: {current}"
            if current.parent == current:
                break
            current = current.parent
        return True, "root-owned helper and protected parent chain"
    except OSError as exc:
        return False, f"Landlock helper attestation failed: {exc}"


def _native_probe(path: Path) -> tuple[bool, str, dict[str, Any]]:
    try:
        result = subprocess.run(
            [str(path), "--probe"],
            text=True,
            capture_output=True,
            check=False,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return False, f"Landlock helper probe failed: {exc}", {}
    try:
        payload = json.loads(result.stdout)
    except (TypeError, ValueError):
        return False, "Landlock helper probe did not return JSON", {}
    if result.returncode != 0:
        return False, str(payload.get("reason", "Landlock helper probe failed")), payload
    abi = payload.get("abi")
    if payload.get("probe") != "passed" or not isinstance(abi, int) or abi < MINIMUM_LANDLOCK_ABI:
        return False, f"Landlock helper probe did not prove ABI {MINIMUM_LANDLOCK_ABI}", payload
    return True, "native Landlock mutation probe passed", payload


def _helper_sha256(path: Path) -> str:
    try:
        digest = hashlib.sha256()
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
        return digest.hexdigest()
    except OSError:
        return ""


class LandlockConfinementProvider(EnforcementProvider):
    """Guard provider composed with a host-controlled ``repopact-sandbox``."""

    def __init__(
        self,
        guard: EnforcementProvider | Any,
        helper: str | Path = DEFAULT_LANDLOCK_HELPER,
        *,
        endpoint: str | Path | None = None,
        allow_unprotected_testing_helper: bool = False,
    ) -> None:
        self.guard = guard
        self.helper = Path(helper)
        self.endpoint = str(endpoint) if endpoint is not None else getattr(guard, "endpoint", None)
        self.allow_unprotected_testing_helper = allow_unprotected_testing_helper

    def attestation(self, root: Path | None = None) -> dict[str, Any]:
        """Return privacy-safe host/helper facts for evidence manifests."""
        protected, protection_reason = _protected_helper(self.helper)
        passed, probe_reason, probe = _native_probe(self.helper) if protected or self.allow_unprotected_testing_helper else (False, "helper is not protected", {})
        try:
            mode = oct(self.helper.lstat().st_mode & 0o7777)
            owner = self.helper.lstat().st_uid
        except OSError:
            mode, owner = "", None
        return {
            "helper_path": str(self.helper),
            "owner_uid": owner,
            "mode": mode,
            "protected": protected,
            "protection_reason": protection_reason,
            "sha256": _helper_sha256(self.helper),
            "probe_passed": passed,
            "probe_reason": probe_reason,
            "probe": probe,
        }

    def health(self, root: Path | None = None) -> GuardHealth:
        try:
            base = self.guard.health(root)
        except TypeError:
            # ProtectedGuard is the in-process reference implementation and
            # predates the provider root parameter; native IPC clients accept
            # it.  Preserve both call shapes without changing authority flow.
            base = self.guard.health()
        if not os.name == "posix" or not sys.platform.startswith("linux"):
            return replace(base, healthy=False, security_level="not-covered", process_confined=False,
                           path_confined=False, reason="Landlock confinement is Linux-only")
        if not base.healthy or not base.protected:
            return replace(base, security_level="not-covered", process_confined=False,
                           path_confined=False, reason=base.reason or "protected guard is unavailable")
        protected, protection_reason = _protected_helper(self.helper)
        if not protected and not (base.testing_only and self.allow_unprotected_testing_helper):
            return replace(base, healthy=False, security_level="not-covered", process_confined=False,
                           path_confined=False, reason=protection_reason)
        passed, probe_reason, _ = _native_probe(self.helper)
        if not passed:
            return replace(base, healthy=False, security_level="not-covered", process_confined=False,
                           path_confined=False, reason=probe_reason)
        backend = f"{base.backend_id}+landlock" if base.backend_id else "linux-landlock"
        return replace(
            base,
            security_level="sandbox/process-enforced",
            process_confined=True,
            path_confined=True,
            backend_id=backend,
            installed_code_path=str(self.helper),
            reason="protected guard and native Landlock process-tree probe are healthy",
        )

    def discover(self, root: Path | None = None) -> Any:
        return self.guard.discover(root)

    def authorize(self, request: Mapping[str, Any], receipt: Mapping[str, Any], root: Path | None = None) -> Any:
        return self.guard.authorize(request, receipt, root)

    def check(self, action: Mapping[str, Any], lease: Any = None, root: Path | None = None) -> Any:
        return self.guard.check(action, lease, root)

    def delegate(self, parent_token: str, child: Mapping[str, Any], root: Path | None = None) -> Any:
        return self.guard.delegate(parent_token, child, root)

    def revoke(self, request: Mapping[str, Any], receipt: Mapping[str, Any], root: Path | None = None) -> Any:
        return self.guard.revoke(request, receipt, root)

    def launch_authorized(
        self,
        request: Mapping[str, Any],
        receipt: Mapping[str, Any],
        command: Sequence[str],
        *,
        root: Path | None = None,
        cwd: str | Path | None = None,
        env: Mapping[str, str] | None = None,
    ) -> tuple[AdmissionDecision, subprocess.Popen[str] | None]:
        """Start the host helper with explicit argv after capability attestation.

        The request and receipt are signed/verified by the helper through the
        protected guard.  They are not treated as an allowlist by this Python
        adapter, and no opaque lease is handed from this process to another
        peer.
        """
        selected = Path(root or getattr(self.guard, "root", None) or request.get("repopact_root", "")).resolve()
        if not command or any(not isinstance(value, str) or not value for value in command):
            return AdmissionDecision.deny("NOT_COVERED", "sandbox target argv is empty or malformed"), None
        health = self.health(selected)
        if not health.healthy or health.security_level != "sandbox/process-enforced":
            return AdmissionDecision.deny("NOT_COVERED", health.reason or "sandbox/process-enforced is unavailable"), None
        endpoint = self.endpoint or "/run/repopact/guard.sock"
        argv = [
            str(self.helper), "--endpoint", endpoint, "--root", str(selected),
            "--request-json", json.dumps(dict(request), sort_keys=True, separators=(",", ":")),
            "--receipt-json", json.dumps(dict(receipt), sort_keys=True, separators=(",", ":")), "--",
            *command,
        ]
        try:
            child = subprocess.Popen(argv, cwd=str(cwd) if cwd else None,
                                     env=dict(env) if env is not None else None)
        except OSError as exc:
            return AdmissionDecision.deny("NOT_COVERED", f"sandbox helper could not start: {exc}"), None
        return AdmissionDecision.allow("sandbox/process-enforced"), child


__all__ = ["DEFAULT_LANDLOCK_HELPER", "LandlockConfinementProvider", "MINIMUM_LANDLOCK_ABI"]
