"""Protected local guard facade for RepoPact admission.

The reference guard stores its registration outside the checkout and exposes a
small synchronous service API.  It is deliberately not described as a kernel
or sandbox: process/path confinement is only reported when a host backend proves
those capabilities.
"""
from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping

from .admission import (
    AdmissionDecision, GuardHealth, evaluate_action, issue_lease, make_request,
    operator_revoke, revoke, safe_audit, setup_admission, verify_receipt, verify_registration,
)
from .platform_backends import PlatformBackend, current_backend


@dataclass
class ProtectedGuard:
    root: Path
    protected_dir: Path | None = None
    backend: PlatformBackend | None = None

    def __post_init__(self) -> None:
        # Backend attestation is host-owned.  There is intentionally no
        # protected_storage/security_level boolean accepted from callers.
        self.root = Path(self.root).resolve()
        if self.backend is None:
            self.backend = current_backend(self.root, self.protected_dir)

    def _effective_protected_dir(self) -> Path | None:
        if self.protected_dir is not None:
            return self.protected_dir
        attestation = self.backend.attest(self.root, None) if self.backend else None
        if attestation and attestation.installed:
            return Path(attestation.protected_state_path)
        return None

    def health(self) -> GuardHealth:
        assert self.backend is not None
        attestation = self.backend.attest(self.root, self.protected_dir)
        check = verify_registration(self.root, self._effective_protected_dir())
        protected = bool(attestation.protected_from_gated_principal and attestation.healthy)
        return GuardHealth(
            healthy=bool(check.allowed),
            security_level=attestation.security_level,
            reason=check.reason if not check.allowed else attestation.reason,
            protected=protected,
            # The reference registration HMAC can verify even when the same
            # principal can rewrite its key.  Keep integrity detection
            # separate from the host-owned protection claim.
            integrity_checked=bool(check.allowed),
            protected_from_gated_principal=protected,
            process_confined=attestation.process_confinement,
            path_confined=attestation.path_confinement,
            service_identity_verified=attestation.service_identity_verified,
            host_configuration_protected=attestation.host_configuration_protected,
            installed_code_path=attestation.installed_code_path,
            protected_state_path=attestation.protected_state_path,
            ipc_endpoint=attestation.ipc_endpoint,
            backend_id=attestation.backend_id,
            testing_only=attestation.testing_only,
        )

    def register(self, **kwargs: Any) -> dict[str, Any]:
        if self.backend is not None and not getattr(self.backend, "name", "").startswith("testing-only") and hasattr(self.backend, "register"):
            return self.backend.register(self.root, signer=kwargs.get("signer"))
        return setup_admission(self.root, self._effective_protected_dir(), kwargs.get("signer"))

    def discover(self) -> AdmissionDecision:
        return verify_registration(self.root, self._effective_protected_dir())

    def request(self, **kwargs: Any) -> dict[str, Any]:
        return make_request(self.root, protected_dir=self._effective_protected_dir(), **kwargs)

    def authorize(self, request: Mapping[str, Any], receipt: Mapping[str, Any]) -> tuple[AdmissionDecision, dict[str, Any] | None]:
        return issue_lease(request, receipt, self.root, self._effective_protected_dir())

    def check(self, action: Mapping[str, Any], lease: Mapping[str, Any] | None = None) -> AdmissionDecision:
        return evaluate_action(self.root, action, lease, self.health(), self._effective_protected_dir())

    def revoke(self, request: Mapping[str, Any], receipt: Mapping[str, Any]) -> int:
        return revoke(self.root, self._effective_protected_dir(), request=request, receipt=receipt)

    def audit(self, decision: AdmissionDecision, request: Mapping[str, Any] | None = None) -> dict[str, Any]:
        return safe_audit(decision, request)


class GuardService:
    """JSON-lines friendly service adapter; transport is intentionally replaceable."""
    def __init__(self, guard: ProtectedGuard): self.guard = guard

    def dispatch(self, message: Mapping[str, Any]) -> dict[str, Any]:
        op = message.get("op")
        if op == "health": return self.guard.health().__dict__
        if op == "discover": return self.guard.discover().__dict__
        if op == "check": return self.guard.check(message.get("action", {}), message.get("lease")).__dict__
        if op == "revoke": return {"revocation_epoch": self.guard.revoke(message.get("request", {}), message.get("receipt", {}))}
        raise ValueError(f"unknown guard operation: {op}")

    def serve_line(self, line: str) -> str:
        try: return json.dumps(self.dispatch(json.loads(line)), sort_keys=True, separators=(",", ":"))
        except Exception as exc: return json.dumps({"allowed": False, "code": "GUARD_UNHEALTHY", "reason": str(exc)}, separators=(",", ":"))
