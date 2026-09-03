"""Adopter-neutral enforcement contracts and capability resolution.

The policy core intentionally has no dependency on a particular guard, host,
agent, operating system, or cryptographic implementation.  A provider is a
runtime capability boundary; an adapter is the host integration that presents
that boundary.  RepoPact reports the intersection of both capabilities and
never upgrades a weaker participant.
"""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping, Protocol, runtime_checkable


ENFORCEMENT_CLASSES = (
    "instruction-only",
    "session-start",
    "pre-action",
    "sandbox/process-enforced",
)
_RANK = {name: index for index, name in enumerate(ENFORCEMENT_CLASSES)}


def enforcement_rank(value: Any) -> int:
    return _RANK.get(str(value), -1)


def normalize_enforcement_class(value: Any) -> str:
    value = str(value or "not-covered")
    return value if value in _RANK else "not-covered"


def intersect_enforcement(*classes: Any) -> str:
    """Return the effective assurance class, never stronger than any input."""
    values = [normalize_enforcement_class(item) for item in classes]
    if not values or "not-covered" in values:
        return "not-covered"
    return min(values, key=enforcement_rank)


@runtime_checkable
class EnforcementProvider(Protocol):
    """Runtime SPI implemented by a guard or an adopter-owned provider.

    Implementations may return the normal RepoPact decision/lease shapes or a
    transport-specific equivalent understood by their adapter.  ``root`` is
    optional so one provider can serve multiple repositories without making a
    repository-local object the authority root.
    """

    def health(self, root: Path | None = None) -> Any: ...
    def discover(self, root: Path | None = None) -> Any: ...
    def authorize(self, request: Mapping[str, Any], receipt: Mapping[str, Any], root: Path | None = None) -> Any: ...
    def check(self, action: Mapping[str, Any], lease: Any = None, root: Path | None = None) -> Any: ...
    def delegate(self, parent_token: str, child: Mapping[str, Any], root: Path | None = None) -> Any: ...
    def revoke(self, request: Mapping[str, Any], receipt: Mapping[str, Any], root: Path | None = None) -> Any: ...


@dataclass(frozen=True)
class EnforcementRequirement:
    required: bool
    minimum_class: str = "instruction-only"
    failure_mode: str = "degraded"
    source: str = "no-policy"


@dataclass(frozen=True)
class EnforcementResolution:
    requirement: EnforcementRequirement
    adapter_class: str
    provider_class: str
    effective_class: str
    satisfied: bool
    reason: str

    @property
    def required_class(self) -> str:
        return self.requirement.minimum_class

    @property
    def failure_mode(self) -> str:
        return self.requirement.failure_mode

    def record(self) -> dict[str, Any]:
        return {
            "required": self.requirement.required,
            "required_class": self.required_class,
            "failure_mode": self.failure_mode,
            "source": self.requirement.source,
            "adapter_class": self.adapter_class,
            "provider_class": self.provider_class,
            "effective_class": self.effective_class,
            "satisfied": self.satisfied,
            "reason": self.reason,
        }


def policy_requirement(policy: Mapping[str, Any] | None) -> EnforcementRequirement:
    """Translate adopter policy into an explicit, truthful requirement.

    Missing policy and ``enabled=false`` are both ordinary standalone RepoPact
    operation.  They do not create a hidden lease or guard prerequisite.
    """
    if not policy:
        return EnforcementRequirement(False, "instruction-only", "degraded", "no-policy")
    minimum = normalize_enforcement_class(policy.get("minimum_enforcement", "instruction-only"))
    enabled = bool(policy.get("enabled", False))
    return EnforcementRequirement(
        enabled,
        minimum,
        str(policy.get("failure_mode", "degraded")),
        "admission-policy",
    )


def _health(provider: Any, root: Path | None) -> Any:
    if provider is None:
        return None
    health = getattr(provider, "health", None)
    if not callable(health):
        return None
    try:
        return health(root)
    except TypeError:
        try:
            return health()
        except Exception:
            return None
    except Exception:
        return None


def provider_enforcement_class(provider: Any, root: Path | None = None) -> str:
    """Derive provider assurance from backend-owned health/capability facts."""
    if provider is None:
        return "not-covered"
    advertised = getattr(provider, "enforcement_class", None)
    if callable(advertised):
        try:
            advertised = advertised()
        except TypeError:
            advertised = None
        except Exception:
            advertised = "not-covered"
    health = _health(provider, root)
    if health is not None:
        if not bool(getattr(health, "healthy", False)) or not bool(getattr(health, "protected", False)):
            return "not-covered"
        advertised = getattr(health, "security_level", advertised)
    return normalize_enforcement_class(advertised)


def adapter_enforcement_class(adapter: Any, required: str = "pre-action") -> str:
    """Derive adapter assurance independently from its provider."""
    if adapter is None:
        return "not-covered"
    if isinstance(adapter, str):
        return normalize_enforcement_class(adapter)
    capabilities = getattr(adapter, "capabilities", adapter)
    method = getattr(adapter, "adapter_enforcement_class", None)
    if callable(method):
        try:
            return normalize_enforcement_class(method(required))
        except TypeError:
            return normalize_enforcement_class(method())
    method = getattr(capabilities, "enforcement_class", None)
    if callable(method):
        try:
            return normalize_enforcement_class(method(required))
        except Exception:
            return "not-covered"
    return normalize_enforcement_class(getattr(capabilities, "security_level", "not-covered"))


def resolve_enforcement_requirement(
    policy: Mapping[str, Any] | None,
    provider: EnforcementProvider | Any = None,
    adapter: Any = None,
    *,
    root: Path | None = None,
    adapter_class: Any = None,
) -> EnforcementResolution:
    """Resolve policy, adapter, and provider into an effective class.

    A required class is never satisfied by ``failure_mode=degraded`` when the
    effective class is below that requirement.  Degraded mode controls the
    diagnostic/operational response, not an assurance upgrade.
    """
    requirement = policy_requirement(policy)
    if not requirement.required:
        return EnforcementResolution(
            requirement, "instruction-only", "not-covered", "instruction-only", True,
            "admission is not required; standalone RepoPact mode",
        )
    # ``adapter_class`` is a convenience for policy/doctor callers that have
    # already materialized a capability vector; adapter objects remain the
    # preferred path for integrations.
    resolved_adapter_class = normalize_enforcement_class(adapter_class) if adapter_class is not None else adapter_enforcement_class(adapter, requirement.minimum_class)
    provider_class = provider_enforcement_class(provider, root)
    effective = ("instruction-only" if requirement.minimum_class == "instruction-only"
                 else intersect_enforcement(resolved_adapter_class, provider_class))
    satisfied = (requirement.minimum_class != "not-covered"
                 and enforcement_rank(effective) >= enforcement_rank(requirement.minimum_class))
    if requirement.minimum_class == "instruction-only":
        satisfied = True
    if satisfied:
        reason = f"provider and adapter satisfy required {requirement.minimum_class} enforcement"
    else:
        reason = (f"required {requirement.minimum_class} enforcement is unsatisfied: "
                  f"adapter={resolved_adapter_class}, provider={provider_class}, effective={effective}")
    return EnforcementResolution(requirement, resolved_adapter_class, provider_class, effective, satisfied, reason)


# Short aliases make the contract convenient for downstream adapters while
# retaining one canonical resolver implementation.
resolve_requirement = resolve_enforcement_requirement
resolve_capabilities = resolve_enforcement_requirement
