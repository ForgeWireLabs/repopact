"""Protected guard facade and server-side lease authority.

The policy functions in :mod:`repopact.admission` remain useful as a portable
reference implementation. An enforced guard never treats their returned lease
dictionary as client authority: ``LeaseStore`` keeps the authoritative record
in the protected service process and exposes only an opaque capability.
"""
from __future__ import annotations

import json
import secrets
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping

from .admission import (
    AdmissionDecision, GuardHealth, canonical_identity, delegation_subset,
    digest, evaluate_action, issue_lease, make_request, revoke, safe_audit,
    setup_admission, verify_registration,
)
from .platform_backends import PlatformBackend, current_backend


def _decision(value: Mapping[str, Any]) -> AdmissionDecision:
    details = value.get("details", {})
    return AdmissionDecision(bool(value.get("allowed")), str(value.get("code", "")),
                             str(value.get("reason", "")), str(value.get("enforcement", "not-covered")),
                             details if isinstance(details, Mapping) else {})


def _peer_binding() -> dict[str, Any]:
    from .guard_ipc import local_peer_binding
    return local_peer_binding()


def _safe_lease_metadata(lease: Mapping[str, Any], binding: Mapping[str, Any]) -> dict[str, Any]:
    fields = ("lease_version", "lease_id", "request_digest", "repository_identity", "repopact_root",
              "work_item", "principal", "session_id", "approval_class", "profile", "mode", "scopes",
              "paths", "capabilities", "frozen_surface_digest", "delegation_lineage", "delegation_ceiling",
              "base", "issued_at", "expires_at", "revocation_epoch")
    result = {key: lease[key] for key in fields if key in lease}
    result["peer_binding_digest"] = digest(dict(binding))
    return result


class LeaseStore:
    """Ephemeral guard-owned capability state; restart invalidates live leases."""

    def __init__(self) -> None:
        self._records: dict[str, tuple[dict[str, Any], dict[str, Any]]] = {}

    def authorize(self, guard: "ProtectedGuard", request: Mapping[str, Any], receipt: Mapping[str, Any],
                  binding: Mapping[str, Any] | None = None) -> tuple[AdmissionDecision, dict[str, Any] | None]:
        health = guard.health()
        if not health.healthy or (not health.protected and not health.testing_only):
            return AdmissionDecision.deny("GUARD_UNHEALTHY", health.reason or "protected guard is unavailable"), None
        proof, lease = issue_lease(request, receipt, guard.root, guard._effective_protected_dir())
        if not proof.allowed or lease is None:
            return proof, None
        peer = dict(binding or _peer_binding())
        token = secrets.token_urlsafe(48)
        self._records[token] = (dict(lease), peer)
        return AdmissionDecision.allow(health.security_level), {
            "lease_token": token, "lease_metadata": _safe_lease_metadata(lease, peer),
        }

    def check(self, guard: "ProtectedGuard", action: Mapping[str, Any], token: str,
              binding: Mapping[str, Any] | None = None) -> AdmissionDecision:
        if not isinstance(token, str) or len(token) < 32:
            return AdmissionDecision.deny("NO_OPERATOR_PROOF", "opaque guard lease token is missing or malformed")
        record = self._records.get(token)
        if record is None:
            return AdmissionDecision.deny("NO_OPERATOR_PROOF", "lease token is unknown to this guard instance")
        lease, expected_peer = record
        actual_peer = dict(binding or _peer_binding())
        if actual_peer != expected_peer:
            return AdmissionDecision.deny("WRONG_SESSION", "lease is bound to a different transport peer")
        return evaluate_action(guard.root, action, lease, guard.health(), guard._effective_protected_dir())

    def delegate(self, guard: "ProtectedGuard", parent_token: str, child: Mapping[str, Any],
                 binding: Mapping[str, Any] | None = None) -> tuple[AdmissionDecision, dict[str, Any] | None]:
        record = self._records.get(parent_token)
        if record is None:
            return AdmissionDecision.deny("NO_OPERATOR_PROOF", "parent lease token is unknown"), None
        parent, parent_peer = record
        actual_peer = dict(binding or _peer_binding())
        if actual_peer != parent_peer:
            return AdmissionDecision.deny("WRONG_SESSION", "parent lease peer binding mismatch"), None
        candidate = dict(parent)
        candidate.update({key: child[key] for key in (
            "principal", "profile", "scopes", "paths", "capabilities", "expires_at",
            "delegation_ceiling", "parent_lease_id", "delegation_lineage") if key in child})
        candidate["lease_id"] = secrets.token_hex(16)
        proof = delegation_subset(parent, candidate)
        if not proof.allowed:
            return proof, None
        token = secrets.token_urlsafe(48)
        # The child binding is the peer on this authenticated transport. A
        # JSON ``peer_binding`` field is deliberately ignored; a distinct
        # child process must perform an explicit service handshake of its own.
        child_peer = actual_peer
        self._records[token] = (candidate, child_peer)
        return AdmissionDecision.allow(guard.health().security_level), {
            "lease_token": token, "lease_metadata": _safe_lease_metadata(candidate, child_peer),
        }


@dataclass
class ProtectedGuard:
    root: Path
    protected_dir: Path | None = None
    backend: PlatformBackend | None = None
    lease_store: LeaseStore = field(default_factory=LeaseStore, init=False, repr=False)

    def __post_init__(self) -> None:
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
            healthy=bool(check.allowed), security_level=attestation.security_level,
            reason=check.reason if not check.allowed else attestation.reason,
            protected=protected, integrity_checked=bool(check.allowed),
            protected_from_gated_principal=protected,
            process_confined=attestation.process_confinement, path_confined=attestation.path_confinement,
            service_identity_verified=attestation.service_identity_verified,
            host_configuration_protected=attestation.host_configuration_protected,
            installed_code_path=attestation.installed_code_path,
            protected_state_path=attestation.protected_state_path,
            ipc_endpoint=attestation.ipc_endpoint, backend_id=attestation.backend_id,
            testing_only=attestation.testing_only,
        )

    def register(self, **kwargs: Any) -> dict[str, Any]:
        if self.backend is not None and not getattr(self.backend, "name", "").startswith("testing-only") and hasattr(self.backend, "register"):
            return self.backend.register(self.root, signer=kwargs.get("signer"))
        return setup_admission(self.root, self._effective_protected_dir(), kwargs.get("signer"))

    def discover(self) -> Any:
        return verify_registration(self.root, self._effective_protected_dir())

    def request(self, **kwargs: Any) -> dict[str, Any]:
        return make_request(self.root, protected_dir=self._effective_protected_dir(), **kwargs)

    def authorize(self, request: Mapping[str, Any], receipt: Mapping[str, Any],
                  *, peer_binding: Mapping[str, Any] | None = None) -> tuple[AdmissionDecision, dict[str, Any] | None]:
        return self.lease_store.authorize(self, request, receipt, peer_binding)

    def check(self, action: Mapping[str, Any], lease: Mapping[str, Any] | str | None = None,
              *, peer_binding: Mapping[str, Any] | None = None) -> AdmissionDecision:
        if isinstance(lease, str):
            return self.lease_store.check(self, action, lease, peer_binding)
        if isinstance(lease, Mapping) and isinstance(lease.get("lease_token"), str):
            return self.lease_store.check(self, action, str(lease["lease_token"]), peer_binding)
        # Legacy dictionaries remain available only to the explicitly marked
        # reference backend. Native service IPC never accepts them.
        if self.backend is not None and (getattr(self.backend, "testing_only", False)
                                         or getattr(self.backend, "name", "").startswith("testing-only")):
            return evaluate_action(self.root, action, lease, self.health(), self._effective_protected_dir())
        if lease is not None:
            return AdmissionDecision.deny("NO_OPERATOR_PROOF", "production guard accepts only an opaque lease token")
        return evaluate_action(self.root, action, None, self.health(), self._effective_protected_dir())

    def delegate(self, parent_token: str, child: Mapping[str, Any], *, peer_binding: Mapping[str, Any] | None = None):
        return self.lease_store.delegate(self, parent_token, child, peer_binding)

    def revoke(self, request: Mapping[str, Any], receipt: Mapping[str, Any]) -> int:
        return revoke(self.root, self._effective_protected_dir(), request=request, receipt=receipt)

    def audit(self, decision: AdmissionDecision, request: Mapping[str, Any] | None = None) -> dict[str, Any]:
        return safe_audit(decision, request)


class LocalTestingGuard:
    """Explicit reference client; never selected as a native backend."""
    def __init__(self, guard: ProtectedGuard): self.guard = guard
    def health(self) -> GuardHealth: return self.guard.health()
    def authorize(self, request: Mapping[str, Any], receipt: Mapping[str, Any]): return self.guard.authorize(request, receipt)
    def check(self, action: Mapping[str, Any], lease: Mapping[str, Any] | str | None = None): return self.guard.check(action, lease)


class GuardService:
    """JSON-friendly service API; installed instances govern many repositories."""
    def __init__(self, guard: ProtectedGuard | None = None, registry_root: Path | None = None):
        self.guard = guard
        self.registry_root = Path(registry_root).resolve() if registry_root else None
        self._guards: dict[str, ProtectedGuard] = {}

    def _resolve(self, payload: Mapping[str, Any], *, request: Mapping[str, Any] | None = None,
                 action: Mapping[str, Any] | None = None) -> ProtectedGuard:
        if self.guard is not None:
            return self.guard
        data = request or action or payload
        root_value = data.get("root") or data.get("repopact_root") or payload.get("root") or payload.get("repopact_root")
        if not root_value:
            raise PermissionError("registered repository root is required")
        root = Path(str(root_value)).resolve()
        if not root.is_dir():
            raise PermissionError("repository root is unavailable")
        supplied = data.get("repository_identity") or payload.get("repository_identity")
        actual = digest(canonical_identity(root))
        if supplied and supplied != actual:
            raise PermissionError("repository identity does not match canonical Git state")
        canonical = canonical_identity(root)
        # Share registration/trust by common-dir, but keep policy evaluation
        # bound to the concrete checkout so a linked worktree is not evaluated
        # against the primary worktree's filesystem.
        cache_key = digest((canonical.get("git_common_dir", str(root)), canonical.get("repopact_root", str(root))))
        guard = self._guards.get(cache_key)
        if guard is None:
            guard = ProtectedGuard(root, self.registry_root)
            self._guards[cache_key] = guard
        discovered = guard.discover()
        if not discovered.allowed:
            raise PermissionError(discovered.reason)
        return guard

    def dispatch(self, message: Mapping[str, Any], *, transport_binding: Mapping[str, Any] | None = None) -> dict[str, Any]:
        op = str(message.get("op", ""))
        payload = message.get("payload", message)
        if not isinstance(payload, Mapping):
            raise ValueError("guard payload must be an object")
        if op == "health" and self.guard is None and not (payload.get("root") or payload.get("repopact_root")):
            # A machine-wide service can report its own health before any
            # repository is selected. Repository ``discover`` still requires
            # an explicit canonical identity.
            return current_backend().health()
        if op == "health": return self._resolve(payload).health().__dict__
        if op == "discover": return self._resolve(payload).discover().__dict__
        if op == "authorize":
            guard = self._resolve(payload, request=payload.get("request", {}))
            decision, capability = guard.authorize(payload.get("request", {}), payload.get("receipt", {}), peer_binding=transport_binding or _peer_binding())
            return {**decision.__dict__, "decision": decision.__dict__, **(capability or {})}
        if op == "check":
            action = payload.get("action", {})
            guard = self._resolve(payload, action=action)
            return guard.check(action, payload.get("lease_token"), peer_binding=transport_binding or _peer_binding()).__dict__
        if op == "delegate":
            guard = self._resolve(payload, request=payload.get("child", {}))
            decision, capability = guard.delegate(str(payload.get("parent_token", "")), payload.get("child", {}), peer_binding=transport_binding or _peer_binding())
            return {**decision.__dict__, "decision": decision.__dict__, **(capability or {})}
        if op == "revoke":
            guard = self._resolve(payload, request=payload.get("request", {}))
            return {"revocation_epoch": guard.revoke(payload.get("request", {}), payload.get("receipt", {}))}
        raise ValueError(f"unknown guard operation: {op}")

    def serve_line(self, line: str) -> str:
        try: return json.dumps(self.dispatch(json.loads(line)), sort_keys=True, separators=(",", ":"))
        except Exception as exc: return json.dumps({"allowed": False, "code": "GUARD_UNHEALTHY", "reason": str(exc)}, separators=(",", ":"))
