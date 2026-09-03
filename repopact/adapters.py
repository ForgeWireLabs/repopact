"""Vendor-neutral adapter SPI and two reference integration families."""
from __future__ import annotations

from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Any, Callable, Mapping

from .admission import AdmissionDecision
from .guard import ProtectedGuard


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
    def security_level(self) -> str: return self.enforcement_class()

    def record(self) -> dict[str, Any]:
        raw = asdict(self)
        caps = {name: raw.pop(name) for name in ("repo_discovery", "session_identity", "session_start_gate", "pre_action_interception", "path_reporting", "path_confinement", "process_confinement", "network_confinement", "protected_host_config", "operator_handoff", "subprincipal_propagation", "fail_closed_health", "audit_receipt")}
        raw["spi_version"] = "1"
        raw["capabilities"] = caps
        raw["guard"] = {"endpoint_id": "protected-local", "protocol_version": "1", "required_health": "fail-closed"}
        raw["action_families"] = ["mutation", "process", "read-only"]
        raw["health"] = {"status": "healthy", "checked_at": "1970-01-01T00:00:00Z"}
        raw["os"] = raw["os"] if raw["os"] in {"windows", "linux", "macos"} else "other"
        return raw


class PreActionAdapter:
    def __init__(self, guard: ProtectedGuard, capabilities: AdapterCapabilities | None = None):
        self.guard = guard; self.capabilities = capabilities or AdapterCapabilities("repopact-reference")

    def before(self, action: Mapping[str, Any], callback: Callable[[], Any]) -> tuple[AdmissionDecision, Any | None]:
        decision = self.guard.check(action)
        if not decision.allowed: return decision, None
        return decision, callback()

    def start(self, action: Mapping[str, Any]) -> AdmissionDecision:
        return self.guard.check(action)

    def capability_record(self) -> dict[str, Any]: return self.capabilities.record()


class LauncherAdapter(PreActionAdapter):
    """Reference launcher gate. It does not claim arbitrary child confinement."""
    def launch(self, action: Mapping[str, Any], launch: Callable[[], Any]) -> tuple[AdmissionDecision, Any | None]:
        return self.before(action, launch)


class CodexReferenceAdapter(PreActionAdapter):
    def __init__(self, guard: ProtectedGuard): super().__init__(guard, AdapterCapabilities("repopact-codex-reference", host="coding-surface"))


class ClaudeReferenceAdapter(PreActionAdapter):
    def __init__(self, guard: ProtectedGuard): super().__init__(guard, AdapterCapabilities("repopact-claude-reference", host="coding-surface"))
