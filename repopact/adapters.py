"""Vendor-neutral adapter SPI and two reference integration families."""
from __future__ import annotations

from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Any, Callable, Mapping
import subprocess

from .admission import AdmissionDecision, iso
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
        raw["health"] = {"status": "healthy", "checked_at": iso()}
        raw["os"] = raw["os"] if raw["os"] in {"windows", "linux", "macos"} else "other"
        return raw


class PreActionAdapter:
    def __init__(self, guard: ProtectedGuard, capabilities: AdapterCapabilities | None = None):
        self.guard = guard; self.capabilities = capabilities or AdapterCapabilities("repopact-reference")

    def before(self, action: Mapping[str, Any], callback: Callable[[], Any], lease: Mapping[str, Any] | None = None) -> tuple[AdmissionDecision, Any | None]:
        decision = self.guard.check(action, lease)
        if not decision.allowed: return decision, None
        return decision, callback()

    def start(self, action: Mapping[str, Any]) -> AdmissionDecision:
        return self.guard.check(action)

    def capability_record(self) -> dict[str, Any]: return self.capabilities.record()


class LauncherAdapter(PreActionAdapter):
    """Independent launcher gate; child creation happens only after admission."""
    def __init__(self, guard: ProtectedGuard, capabilities: AdapterCapabilities | None = None):
        super().__init__(guard, capabilities or AdapterCapabilities(
            "repopact-launcher-reference", pre_action_interception=False,
            path_reporting=False, session_start_gate=True))

    def launch(self, action: Mapping[str, Any], launch: Callable[[], Any] | list[str] | tuple[str, ...], lease: Mapping[str, Any] | None = None, *, cwd: str | Path | None = None, env: Mapping[str, str] | None = None) -> tuple[AdmissionDecision, Any | None]:
        decision = self.guard.check(action, lease)
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
    def __init__(self, guard: ProtectedGuard): super().__init__(guard, AdapterCapabilities("repopact-codex-reference", host="coding-surface"))


class ClaudeReferenceAdapter(PreActionAdapter):
    def __init__(self, guard: ProtectedGuard): super().__init__(guard, AdapterCapabilities("repopact-claude-reference", host="coding-surface"))
