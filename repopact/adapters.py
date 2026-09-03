"""Vendor-neutral adapter SPI and two reference integration families."""
from __future__ import annotations

from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Any, Callable, Mapping
import subprocess

from .admission import AdmissionDecision, iso
from .enforcement import (
    EnforcementProvider,
    adapter_enforcement_class,
    intersect_enforcement,
    provider_enforcement_class,
    resolve_enforcement_requirement,
)


@dataclass(frozen=True)
class AdapterCapabilities:
    adapter_id: str
    adapter_version: str = "1"
    host: str = "generic"
    os: str = "unknown"
    repo_discovery: bool = True
    session_identity: bool = True
    session_start_gate: bool = True
    pre_action_interception: bool = True
    path_reporting: bool = True
    path_confinement: bool = False
    process_confinement: bool = False
    network_confinement: bool = False
    protected_host_config: bool = False
    operator_handoff: bool = False
    subprincipal_propagation: bool = False
    fail_closed_health: bool = True
    audit_receipt: bool = True

    def enforcement_class(self, required: str = "pre-action") -> str:
        if not self.fail_closed_health or not self.repo_discovery or not self.session_identity: return "not-covered"
        if required == "sandbox/process-enforced" and self.path_confinement and self.process_confinement: return "sandbox/process-enforced"
        if self.pre_action_interception: return "pre-action"
        if self.session_start_gate: return "session-start"
        return "instruction-only"

    @property
    def security_level(self) -> str:
        # A capability vector alone is not an enforcement witness.  The bound
        # adapter's ``enforcement_class`` method combines it with backend-owned
        # guard health before exposing an effective class.
        return "not-covered"

    def record(self) -> dict[str, Any]:
        raw = asdict(self)
        caps = {name: raw.pop(name) for name in ("repo_discovery", "session_identity", "session_start_gate", "pre_action_interception", "path_reporting", "path_confinement", "process_confinement", "network_confinement", "protected_host_config", "operator_handoff", "subprincipal_propagation", "fail_closed_health", "audit_receipt")}
        raw["spi_version"] = "1"
        raw["capabilities"] = caps
        raw["guard"] = {"endpoint_id": "protected-local", "protocol_version": "1", "required_health": "fail-closed"}
        raw["action_families"] = ["mutation", "process", "read-only"]
        raw["health"] = {"status": "healthy", "checked_at": iso()}
        raw["os"] = raw["os"] if raw["os"] in {"windows", "linux", "macos"} else "other"
        return raw


class PreActionAdapter:
    """Reference pre-action adapter over any adopter-owned provider.

    ``guard`` remains the parameter name for source compatibility, but it is
    deliberately typed and treated as the generic provider SPI.  No adapter
    imports or requires the built-in ``ProtectedGuard`` implementation.
    """
    def __init__(self, provider: EnforcementProvider | Any, capabilities: AdapterCapabilities | None = None):
        self.provider = provider
        # Compatibility for callers that used the old attribute; semantics are
        # provider-based and do not depend on the concrete guard class.
        self.guard = provider
        self.capabilities = capabilities or AdapterCapabilities("repopact-reference")

    def before(self, action: Mapping[str, Any], callback: Callable[[], Any], lease: Mapping[str, Any] | None = None) -> tuple[AdmissionDecision, Any | None]:
        decision = self.provider.check(action, lease)
        if not decision.allowed: return decision, None
        return decision, callback()

    def start(self, action: Mapping[str, Any]) -> AdmissionDecision:
        return self.provider.check(action)

    def enforcement_class(self, required: str = "pre-action") -> str:
        """Return adapter/provider assurance intersection, never an adapter claim."""
        return intersect_enforcement(
            adapter_enforcement_class(self.capabilities, required),
            provider_enforcement_class(self.provider),
        )

    def enforcement_resolution(self, policy: Mapping[str, Any] | None, *, root: Path | None = None):
        return resolve_enforcement_requirement(policy, self.provider, self.capabilities, root=root)

    def capability_record(self) -> dict[str, Any]:
        record = self.capabilities.record()
        health = self.provider.health()
        record["health"] = {"status": "healthy" if health.healthy and health.protected else "failed", "checked_at": iso()}
        return record


class LauncherAdapter(PreActionAdapter):
    """Independent launcher gate; child creation happens only after admission."""
    def __init__(self, provider: EnforcementProvider | Any, capabilities: AdapterCapabilities | None = None):
        super().__init__(provider, capabilities or AdapterCapabilities(
            "repopact-launcher-reference", pre_action_interception=False,
            path_reporting=False, session_start_gate=True))

    def launch(self, action: Mapping[str, Any], launch: Callable[[], Any] | list[str] | tuple[str, ...], lease: Mapping[str, Any] | None = None, *, cwd: str | Path | None = None, env: Mapping[str, str] | None = None) -> tuple[AdmissionDecision, Any | None]:
        decision = self.provider.check(action, lease)
        if not decision.allowed:
            return decision, None
        if callable(launch):
            return decision, launch()
        if not launch:
            return AdmissionDecision.deny("NOT_COVERED", "launcher command is empty"), None
        # No shell interpolation: this is a launcher/proxy integration, not a
        # claim that arbitrary children are confined after creation.
        return decision, subprocess.Popen(list(launch), cwd=str(cwd) if cwd else None, env=dict(env) if env else None)


class CodexReferenceAdapter(PreActionAdapter):
    def __init__(self, provider: EnforcementProvider | Any): super().__init__(provider, AdapterCapabilities("repopact-codex-reference", host="coding-surface"))


class ClaudeReferenceAdapter(PreActionAdapter):
    def __init__(self, provider: EnforcementProvider | Any): super().__init__(provider, AdapterCapabilities("repopact-claude-reference", host="coding-surface"))
