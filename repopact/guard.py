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
    revoke, safe_audit, setup_admission, verify_receipt, verify_registration,
)


@dataclass
class ProtectedGuard:
    root: Path
    protected_dir: Path | None = None
    security_level: str = "pre-action"

    def health(self) -> GuardHealth:
        check = verify_registration(self.root, self.protected_dir)
        return GuardHealth(check.allowed, self.security_level if check.allowed else "not-covered", check.reason, True)

    def register(self, **kwargs: Any) -> dict[str, Any]:
        return setup_admission(self.root, self.protected_dir, kwargs.get("signer"))

    def discover(self) -> AdmissionDecision:
        return verify_registration(self.root, self.protected_dir)

    def request(self, **kwargs: Any) -> dict[str, Any]:
        return make_request(self.root, protected_dir=self.protected_dir, **kwargs)

    def authorize(self, request: Mapping[str, Any], receipt: Mapping[str, Any]) -> tuple[AdmissionDecision, dict[str, Any] | None]:
        return issue_lease(request, receipt, self.root, self.protected_dir)

    def check(self, action: Mapping[str, Any], lease: Mapping[str, Any] | None = None) -> AdmissionDecision:
        return evaluate_action(self.root, action, lease, self.health(), self.protected_dir)

    def revoke(self) -> int:
        return revoke(self.root, self.protected_dir)

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
        if op == "revoke": return {"revocation_epoch": self.guard.revoke()}
        raise ValueError(f"unknown guard operation: {op}")

    def serve_line(self, line: str) -> str:
        try: return json.dumps(self.dispatch(json.loads(line)), sort_keys=True, separators=(",", ":"))
        except Exception as exc: return json.dumps({"allowed": False, "code": "GUARD_UNHEALTHY", "reason": str(exc)}, separators=(",", ":"))
