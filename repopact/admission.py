"""Vendor-neutral pre-execution admission policy and authorization primitives.

This module intentionally keeps policy decisions deterministic and independent
of a host or agent vendor.  The protected guard and adapters are thin callers
around these functions; a receipt is proof, while a lease is the short-lived
runtime capability derived from that proof.
"""
from __future__ import annotations

import base64
import hashlib
import hmac
import json
import os
import secrets
import subprocess
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Callable, Iterable, Mapping

try:  # optional until a signer is actually used
    from cryptography.hazmat.primitives import hashes, serialization
    from cryptography.hazmat.primitives.asymmetric.ed25519 import (
        Ed25519PrivateKey, Ed25519PublicKey,
    )
    from cryptography.hazmat.primitives.ciphers.aead import AESGCM
    from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
except ImportError:  # pragma: no cover - exercised by dependency/install tests
    Ed25519PrivateKey = Ed25519PublicKey = AESGCM = PBKDF2HMAC = None  # type: ignore


DENIAL_CODES = (
    "UNREGISTERED_REPO", "GUARD_UNHEALTHY", "NO_WORK_ITEM", "PROPOSED_WORK",
    "INVALID_PREFLIGHT", "WRONG_LIFECYCLE", "DEPENDENCY_AUTHORITY",
    "DEVELOPMENT_IDENTITY", "SCOPE_VIOLATION", "PATH_VIOLATION",
    "FROZEN_APPROVAL_REQUIRED", "WRONG_REPOSITORY", "WRONG_WORK_ITEM",
    "WRONG_SESSION", "EXPIRED_AUTHORIZATION", "REVOKED_AUTHORIZATION",
    "STATE_DRIFT", "AUTHORITY_DRIFT", "STALE_POLICY", "DELEGATION_ESCALATION",
    "PROFILE_ESCALATION", "BOOTSTRAP_SCOPE", "RECEIPT_INVALID",
    "RECEIPT_REPLAY", "NO_OPERATOR_PROOF", "NOT_COVERED",
)


def canonical_json(value: Any) -> bytes:
    """Return deterministic UTF-8 JSON bytes (no whitespace or NaN)."""
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
                      allow_nan=False).encode("utf-8")


canonicalize = canonical_json


def digest(value: Any) -> str:
    return hashlib.sha256(canonical_json(value)).hexdigest()


def _utc(value: str | datetime) -> datetime:
    if isinstance(value, datetime):
        dt = value
    else:
        dt = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.astimezone(timezone.utc)


def iso(value: datetime | None = None) -> str:
    return (value or datetime.now(timezone.utc)).astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _git(root: Path, *args: str) -> str:
    cp = subprocess.run(["git", "-C", str(root), *args], text=True, capture_output=True, check=False)
    if cp.returncode:
        return ""
    return cp.stdout.strip()


def normalize_path(path: str | Path) -> str:
    p = Path(path).expanduser().resolve(strict=False)
    text = os.path.normcase(str(p)).replace("\\", "/")
    if len(text) > 1 and text.endswith("/"):
        text = text.rstrip("/")
    return text


def canonical_identity(root: Path) -> dict[str, Any]:
    root = root.resolve()
    top = _git(root, "rev-parse", "--show-toplevel") or str(root)
    common = _git(root, "rev-parse", "--git-common-dir") or str(root / ".git")
    head = _git(root, "rev-parse", "HEAD")
    tree = _git(root, "rev-parse", "HEAD^{tree}")
    contract_parts = []
    for rel in ("AGENTS.md", "governance/owners.json", "governance/invariants.json", "governance/charter.md"):
        p = root / rel
        if p.is_file():
            contract_parts.append((rel, hashlib.sha256(p.read_bytes()).hexdigest()))
    return {
        "repository_root": normalize_path(top),
        "git_common_dir": normalize_path((root / common).resolve() if not Path(common).is_absolute() else common),
        "repopact_root": normalize_path(root), "head": head, "tree": tree,
        "contract_digest": digest(contract_parts),
    }


class SignerError(RuntimeError):
    pass


class UserPresence:
    """Small host/UI-neutral user-presence seam used by approval front ends."""
    def confirm(self, prompt: str) -> bool:
        import sys
        if not sys.stdin.isatty():
            raise SignerError("user presence requires an interactive terminal")
        return input(prompt + " [y/N] ").strip().lower() in {"y", "yes"}


class Ed25519Signer:
    """Ed25519 signer. Private keys are deliberately external to repositories."""
    algorithm = "ed25519"

    def __init__(self, private_key: Any, key_id: str = "operator-key", operator_id: str = "operator"):
        if Ed25519PrivateKey is None:
            raise SignerError("cryptography is required for Ed25519 signing")
        self.private_key, self.key_id, self.operator_id = private_key, key_id, operator_id

    @classmethod
    def generate(cls, key_id: str = "operator-key", operator_id: str = "operator") -> "Ed25519Signer":
        if Ed25519PrivateKey is None:
            raise SignerError("cryptography is required for Ed25519 signing")
        return cls(Ed25519PrivateKey.generate(), key_id, operator_id)

    @property
    def public_key_bytes(self) -> bytes:
        return self.private_key.public_key().public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)

    @property
    def public_key(self) -> str:
        return base64.b64encode(self.public_key_bytes).decode("ascii")

    def sign(self, value: Any) -> str:
        return base64.b64encode(self.private_key.sign(canonical_json(value))).decode("ascii")

    def save(self, path: Path, passphrase: str) -> None:
        if AESGCM is None or PBKDF2HMAC is None:
            raise SignerError("cryptography is required for encrypted key storage")
        salt, nonce = os.urandom(16), os.urandom(12)
        kdf = PBKDF2HMAC(algorithm=hashes.SHA256(), length=32, salt=salt, iterations=310000)
        key = kdf.derive(passphrase.encode("utf-8"))
        raw = self.private_key.private_bytes(serialization.Encoding.Raw, serialization.PrivateFormat.Raw, serialization.NoEncryption())
        ciphertext = AESGCM(key).encrypt(nonce, raw, self.key_id.encode())
        record = {"format": 1, "key_id": self.key_id, "operator_id": self.operator_id,
                  "salt": base64.b64encode(salt).decode(), "nonce": base64.b64encode(nonce).decode(),
                  "ciphertext": base64.b64encode(ciphertext).decode()}
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(canonical_json(record));
        try: os.chmod(path, 0o600)
        except OSError: pass

    @classmethod
    def load(cls, path: Path, passphrase: str) -> "Ed25519Signer":
        if AESGCM is None or PBKDF2HMAC is None:
            raise SignerError("cryptography is required for encrypted key storage")
        rec = json.loads(path.read_bytes())
        kdf = PBKDF2HMAC(algorithm=hashes.SHA256(), length=32, salt=base64.b64decode(rec["salt"]), iterations=310000)
        key = kdf.derive(passphrase.encode("utf-8"))
        try: raw = AESGCM(key).decrypt(base64.b64decode(rec["nonce"]), base64.b64decode(rec["ciphertext"]), rec["key_id"].encode())
        except Exception as exc: raise SignerError("invalid signing key or passphrase") from exc
        return cls(Ed25519PrivateKey.from_private_bytes(raw), rec["key_id"], rec["operator_id"])


def verify_signature(public_key: str, signature: str, value: Any) -> bool:
    if Ed25519PublicKey is None: return False
    try:
        Ed25519PublicKey.from_public_bytes(base64.b64decode(public_key)).verify(base64.b64decode(signature), canonical_json(value))
        return True
    except Exception:
        return False


@dataclass(frozen=True)
class AdmissionDecision:
    allowed: bool
    code: str = ""
    reason: str = ""
    enforcement: str = "pre-action"
    details: Mapping[str, Any] = field(default_factory=dict)

    @classmethod
    def allow(cls, enforcement: str = "pre-action", **details: Any) -> "AdmissionDecision":
        return cls(True, "", "allowed", enforcement, details)

    @classmethod
    def deny(cls, code: str, reason: str = "", **details: Any) -> "AdmissionDecision":
        if code not in DENIAL_CODES: code = "NOT_COVERED"
        return cls(False, code, reason or code.lower().replace("_", " "), "not-covered", details)


@dataclass(frozen=True)
class AdmissionRequest:
    """Typed wrapper used by API clients; fields remain JSON-contract compatible."""
    value: Mapping[str, Any]
    def to_dict(self) -> dict[str, Any]: return dict(self.value)
    @property
    def digest(self) -> str: return digest(self.value)


@dataclass(frozen=True)
class AuthorizationReceipt:
    value: Mapping[str, Any]
    def to_dict(self) -> dict[str, Any]: return dict(self.value)


@dataclass(frozen=True)
class Lease:
    value: Mapping[str, Any]
    def to_dict(self) -> dict[str, Any]: return dict(self.value)


@dataclass(frozen=True)
class Principal:
    principal_id: str
    kind: str = "session"
    parent: str | None = None


@dataclass(frozen=True)
class GuardHealth:
    healthy: bool
    security_level: str = "pre-action"
    reason: str = ""
    protected: bool = False
    integrity_checked: bool = False
    protected_from_gated_principal: bool = False
    process_confined: bool = False
    path_confined: bool = False
    service_identity_verified: bool = False
    host_configuration_protected: bool = False
    installed_code_path: str = ""
    protected_state_path: str = ""
    ipc_endpoint: str = ""
    backend_id: str = ""
    testing_only: bool = False


def default_policy() -> dict[str, Any]:
    profile = {"readable_scopes": ["*"], "writable_scopes": ["work", "src", "tests"], "writable_paths": ["work/**", "src/**", "tests/**"],
               "capabilities": {"process": False, "shell": False, "network": False, "frozen_surface": False}, "max_duration_seconds": 1800,
               "repair_allowed": False, "delegation_allowed": False, "max_delegation_depth": 0, "max_profile": "bounded"}
    repair = {**profile, "writable_scopes": ["governance", "work", "evidence"], "writable_paths": ["governance/**", "work/**", "evidence/**"], "max_duration_seconds": 900, "repair_allowed": True, "max_profile": "repair"}
    return {"version": 1, "policy_version": "1", "enabled": True, "minimum_enforcement": "pre-action", "failure_mode": "fail-closed",
            "approval_cadence": "per-session", "default_profile": "bounded", "profiles": {"observe": {**profile, "writable_scopes": [], "writable_paths": [], "max_duration_seconds": 3600, "max_profile": "observe"}, "bounded": profile, "repair": repair}, "protected_scopes": ["governance", "repopact/schemas"]}


def default_authority(public_key: str = "", operator_id: str = "operator", key_id: str = "operator-key") -> dict[str, Any]:
    return {"authority_version": "1", "operators": [{"operator_id": operator_id, "key_id": key_id, "algorithm": "ed25519", "public_key": public_key, "roles": ["operator"]}],
            "approval_classes": ["activate", "repair", "frozen", "scope", "revoke", "recovery"], "profiles": {"observe": {"approval_classes": ["activate"], "max_delegation_depth": 0}, "bounded": {"approval_classes": ["activate", "repair", "frozen", "scope"], "max_delegation_depth": 0}, "repair": {"approval_classes": ["repair"], "max_delegation_depth": 0}},
            "delegation": {"allowed": False, "max_depth": 0, "operator_approval_required": True}, "rotation": {"requires_existing_trust": True, "overlap_seconds": 86400}, "recovery": {"requires_existing_trust": True, "separate_approval_class": "recovery"}}


def _record(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _public_paths(root: Path) -> tuple[Path, Path, Path]:
    return root / "governance" / "admission-policy.json", root / "governance" / "operator-authority.json", root / "governance" / "repository-registration.json"


def frozen_surface_digest(root: Path) -> str:
    """Digest the repository's declared frozen surface, not caller assertions."""
    path = root / "governance" / "frozen-surface.json"
    if path.is_file():
        return hashlib.sha256(path.read_bytes()).hexdigest()
    return digest({"frozen_surface": []})


def setup_admission(root: Path, protected_dir: Path | None = None, signer: Ed25519Signer | None = None,
                    *, registry_key: str = "root") -> dict[str, Any]:
    """Explicit opt-in setup. Existing protected registration is never replaced."""
    root = root.resolve(); policy_path, authority_path, registration_path = _public_paths(root)
    base = protected_dir or (Path.home() / ".repopact" / "registrations")
    # Testing/reference installations historically use a root digest.  A
    # host-managed global guard uses the adoption id as the registry key so a
    # single service can hold independent registrations for many repositories.
    protected = base / digest(normalize_path(root)) if registry_key == "root" else base / "_pending"
    if protected.exists() and (protected / "registration.json").exists():
        raise RuntimeError("protected registration already exists; explicit rotation required")
    if signer is None:
        raise SignerError("explicit operator signer required; unattended setup cannot establish trust")
    policy, authority = default_policy(), default_authority(signer.public_key, signer.operator_id, signer.key_id)
    ident = canonical_identity(root)
    registration = {"registration_version": 1, "adoption_id": str(uuid.uuid4()), "repository_identity": {"root_digest": digest(ident["repository_root"]), "common_dir_digest": digest(ident["git_common_dir"]), "assurance": "git-common-dir"}, "repopact_root_digest": digest(ident["repopact_root"]), "git": {"common_dir_hint": ident["git_common_dir"], "worktree_hint": ident["repository_root"]}, "policy_version": policy["policy_version"], "authority_version": authority["authority_version"]}
    if registry_key != "root":
        protected = base / registration["adoption_id"]
        if protected.exists():
            raise RuntimeError("protected registration adoption id already exists")
    for p, data in ((policy_path, policy), (authority_path, authority), (registration_path, registration)):
        p.parent.mkdir(parents=True, exist_ok=True); p.write_bytes(canonical_json(data) + b"\n")
    protected.mkdir(parents=True, exist_ok=True)
    state = {"registration": registration, "registration_digest": digest(registration), "authority_digest": digest(authority), "policy_digest": digest(policy), "revocation_epoch": 0, "guard_version": "1", "guard_id": secrets.token_hex(8), "adoption_id": registration["adoption_id"], "common_dir_digest": digest(ident["git_common_dir"]), "registered_root_digest": digest(ident["repopact_root"]), "registered_root": ident["repopact_root"], "registry_key": registry_key}
    state_key = os.urandom(32)
    (protected / "registration.key").write_bytes(state_key)
    try: os.chmod(protected / "registration.key", 0o600)
    except OSError: pass
    state["state_mac"] = hmac.new(state_key, canonical_json(state), hashlib.sha256).hexdigest()
    (protected / "registration.json").write_bytes(canonical_json(state) + b"\n")
    return {"policy": policy_path, "authority": authority_path, "registration": registration_path, "protected": protected, "signer": signer}


def _read_protected_state(p: Path) -> dict[str, Any] | None:
    try:
        state = _record(p)
        key_path = p.with_name("registration.key")
        key = key_path.read_bytes() if key_path.is_file() else b""
        supplied = state.pop("state_mac", None)
        expected = hmac.new(key, canonical_json(state), hashlib.sha256).hexdigest() if key else None
        if not supplied or not expected or not hmac.compare_digest(str(supplied), expected): return None
        state["state_mac"] = supplied
        return state
    except (OSError, ValueError, json.JSONDecodeError): return None


def _protected_state(root: Path, protected_dir: Path | None) -> tuple[dict[str, Any] | None, Path]:
    base = protected_dir or (Path.home() / ".repopact" / "registrations")
    preferred = base / digest(normalize_path(root)) / "registration.json"
    if preferred.is_file():
        state = _read_protected_state(preferred)
        if state is not None:
            return state, preferred
    # Linked worktrees have a different checkout root but share git-common-dir.
    # Locate an authenticated registration by its common-dir binding.
    ident = canonical_identity(root)
    common_digest = digest(ident["git_common_dir"])
    if base.is_dir():
        # Global host guards keep registrations below ``state/registrations``;
        # reference backends keep them directly below the supplied base.
        for candidate in base.rglob("registration.json"):
            state = _read_protected_state(candidate)
            if state is not None and state.get("common_dir_digest") == common_digest:
                return state, candidate
    return None, preferred


def _known_worktree(root: Path, common_dir: str) -> bool:
    listing = _git(root, "worktree", "list", "--porcelain")
    if not listing:
        return False
    wanted = normalize_path(root)
    paths: list[str] = []
    for line in listing.splitlines():
        if line.startswith("worktree "):
            paths.append(normalize_path(line[9:]))
    return wanted in paths


def _save_protected_state(path: Path, state: dict[str, Any]) -> None:
    key = path.with_name("registration.key").read_bytes()
    state.pop("state_mac", None)
    state["state_mac"] = hmac.new(key, canonical_json(state), hashlib.sha256).hexdigest()
    path.write_bytes(canonical_json(state) + b"\n")


def verify_registration(root: Path, protected_dir: Path | None = None) -> AdmissionDecision:
    policy_path, authority_path, registration_path = _public_paths(root)
    state, p = _protected_state(root, protected_dir)
    if state is None:
        return AdmissionDecision.deny("AUTHORITY_DRIFT" if p.exists() else "UNREGISTERED_REPO", "protected registration is missing, tampered, or unreadable")
    if not registration_path.is_file(): return AdmissionDecision.deny("UNREGISTERED_REPO", "public registration is missing")
    try:
        reg, policy, authority = _record(registration_path), _record(policy_path), _record(authority_path)
    except Exception: return AdmissionDecision.deny("UNREGISTERED_REPO", "public admission records are invalid")
    if state.get("registration_digest") != digest(reg) or state.get("policy_digest") != digest(policy) or state.get("authority_digest") != digest(authority): return AdmissionDecision.deny("AUTHORITY_DRIFT", "protected trust pin does not match public records")
    ident = canonical_identity(root)
    same_checkout = reg.get("repopact_root_digest") == digest(ident["repopact_root"])
    linked_checkout = (state.get("common_dir_digest") == digest(ident["git_common_dir"])
                      and reg.get("repository_identity", {}).get("common_dir_digest") == digest(ident["git_common_dir"])
                      and _known_worktree(root, ident["git_common_dir"]))
    if not same_checkout and not linked_checkout:
        return AdmissionDecision.deny("WRONG_REPOSITORY", "repository root identity changed")
    if not linked_checkout and reg.get("repository_identity", {}).get("root_digest") != digest(ident["repository_root"]):
        return AdmissionDecision.deny("WRONG_REPOSITORY", "canonical repository identity changed")
    return AdmissionDecision.allow("pre-action", revocation_epoch=state.get("revocation_epoch", 0), policy=policy,
                                   authority=authority, registration=reg, protected_path=str(p),
                                   integrity_checked=True, protected_from_gated_principal=False)


def _find_work(root: Path, work_id: str) -> tuple[dict[str, Any] | None, Path | None]:
    for p in root.glob(f"work/*/{work_id}*/work-item.json"):
        try: return _record(p), p
        except Exception: return None, p
    return None, None


_NO_LEASE_KINDS = {"read/orient", "bootstrap-propose", "bootstrap-amend", "approval-request"}
_FROZEN_PREFIXES = ("governance/", "repopact/schemas/", ".github/workflows/")


def _action_kind(action: Mapping[str, Any]) -> str:
    value = str(action.get("kind", "mutation")).strip().lower().replace("_", "-")
    return "read/orient" if value in {"read", "read-only", "orient", "orientation"} else value


def _capability_names(value: Any) -> set[str]:
    if isinstance(value, Mapping):
        return {str(k) for k, enabled in value.items() if enabled}
    return {str(x) for x in (value or [])}


def _lease_path_allowed(path: str, lease_paths: Iterable[Any]) -> bool:
    return any(_match_path(path, str(pattern).replace("\\", "/")) for pattern in lease_paths)


def _frozen_path(path: str) -> bool:
    return path.startswith(_FROZEN_PREFIXES)


def _profile_rank(name: Any) -> int:
    return {"observe": 0, "bounded": 1, "repair": 2}.get(str(name), -1)


def evaluate_action(root: Path, action: Mapping[str, Any], lease: Mapping[str, Any] | None = None,
                    guard_health: GuardHealth | None = None, protected_dir: Path | None = None,
                    now: datetime | None = None) -> AdmissionDecision:
    health = guard_health or GuardHealth(True, protected=False, backend_id="policy-core-unbound")
    # A bare GuardHealth object is not an attestation.  Only a backend-owned
    # identity (or an explicitly marked testing backend) may assert protection.
    if not health.backend_id:
        health = GuardHealth(health.healthy, health.security_level, health.reason, False,
                             health.integrity_checked, False, health.process_confined,
                             health.path_confined, health.service_identity_verified,
                             health.host_configuration_protected, health.installed_code_path,
                             health.protected_state_path, health.ipc_endpoint, "unbound", False)
    elif not health.testing_only and health.backend_id != "policy-core-unbound":
        # A caller-supplied health structure is not trusted merely because it
        # contains a familiar backend id.  Re-attest the selected host backend
        # and fail closed if the claims do not match its current facts.
        from .platform_backends import current_backend
        actual = current_backend(root, protected_dir).attest(root, protected_dir)
        if (health.backend_id != actual.backend_id
                or health.protected != actual.protected_from_gated_principal
                or health.host_configuration_protected != actual.host_configuration_protected):
            health = GuardHealth(health.healthy, "not-covered", "backend attestation mismatch", False,
                                 health.integrity_checked, False, False, False, False, False,
                                 actual.installed_code_path, actual.protected_state_path,
                                 actual.ipc_endpoint, actual.backend_id, False)
    action_kind = _action_kind(action)
    if lease is None and action_kind not in _NO_LEASE_KINDS:
        # Make the missing operator proof explicit even when the host guard is
        # also unavailable.  Both paths fail closed; this stable code helps an
        # adapter request the correct operator workflow.
        return AdmissionDecision.deny("NO_OPERATOR_PROOF", "mutating actions require an operator-derived lease")
    if not health.healthy or (not health.protected and action_kind not in _NO_LEASE_KINDS):
        return AdmissionDecision.deny("GUARD_UNHEALTHY", health.reason or "protected guard is unhealthy")
    identity = verify_registration(root, protected_dir)
    if not identity.allowed:
        return identity
    policy = identity.details["policy"]
    if not policy.get("enabled"):
        return AdmissionDecision.allow("instruction-only")
    # Orientation and bounded bootstrap are intentionally non-mutating and do not
    # need a work item.  They can never be used as a source-code write escape.
    if lease is None:
        paths = [str(x).replace("\\", "/") for x in action.get("paths", [])]
        if action_kind == "approval-request" and paths:
            return AdmissionDecision.deny("BOOTSTRAP_SCOPE", "approval requests do not authorize mutation paths")
        if action_kind in {"bootstrap-propose", "bootstrap-amend"}:
            if any(_frozen_path(p) or not _match_path(p, "work/**") for p in paths):
                return AdmissionDecision.deny("BOOTSTRAP_SCOPE", "bootstrap is limited to proposed work records")
        return AdmissionDecision.allow(health.security_level, kind=action_kind)

    work_id = str(action.get("work_item", "")); item, path = _find_work(root, work_id)
    if not item:
        return AdmissionDecision.deny("NO_WORK_ITEM", "no governed work item")
    if item.get("status") == "proposed":
        return AdmissionDecision.deny("PROPOSED_WORK", "proposed work has no implementation authority")
    if item.get("status") != "active":
        return AdmissionDecision.deny("WRONG_LIFECYCLE", f"work item is {item.get('status')}")
    if not isinstance(item.get("preflight"), dict) or item["preflight"].get("created_before_work_started") is not True:
        return AdmissionDecision.deny("INVALID_PREFLIGHT", "mandatory preflight is missing or invalid")

    current_ident = canonical_identity(root)
    supplied_identity = action.get("repository_identity")
    actual_digest = digest(current_ident)
    if supplied_identity and supplied_identity != actual_digest and supplied_identity != current_ident:
        return AdmissionDecision.deny("WRONG_REPOSITORY", "request repository identity mismatch")
    if lease.get("repository_identity") != actual_digest:
        return AdmissionDecision.deny("WRONG_REPOSITORY", "lease repository identity mismatch")
    if lease.get("work_item") != work_id:
        return AdmissionDecision.deny("WRONG_WORK_ITEM", "lease work item mismatch")
    if action.get("principal") and lease.get("principal") != action.get("principal"):
        return AdmissionDecision.deny("NO_OPERATOR_PROOF", "lease principal mismatch")
    action_session = action.get("session_id", action.get("adapter_session"))
    if action_session and lease.get("session_id") != action_session:
        return AdmissionDecision.deny("WRONG_SESSION", "lease session mismatch")

    profile_name = str(action.get("profile") or policy.get("default_profile") or "bounded")
    profile = policy.get("profiles", {}).get(profile_name)
    if not profile:
        return AdmissionDecision.deny("PROFILE_ESCALATION", "unknown authorization profile")
    if _profile_rank(profile_name) < 0 or _profile_rank(profile_name) > _profile_rank(lease.get("profile")):
        return AdmissionDecision.deny("PROFILE_ESCALATION", "action profile exceeds lease profile")
    if lease.get("approval_class") == "repair" and profile_name != "repair":
        return AdmissionDecision.deny("PROFILE_ESCALATION", "repair approval cannot authorize normal actions")
    if profile_name == "observe" and action_kind not in {"read/orient", "approval-request"}:
        return AdmissionDecision.deny("PROFILE_ESCALATION", "observe profile is read-only")
    action_mode = str(action.get("mode", "normal"))
    if lease.get("mode", "normal") != action_mode:
        return AdmissionDecision.deny("PROFILE_ESCALATION", "action mode exceeds lease mode")
    action_scopes = set(str(x) for x in action.get("scopes", []))
    if action_scopes - set(str(x) for x in lease.get("scopes", [])):
        return AdmissionDecision.deny("SCOPE_VIOLATION", "requested scope exceeds lease")
    action_caps = _capability_names(action.get("capabilities", []))
    if action_caps - _capability_names(lease.get("capabilities", [])):
        return AdmissionDecision.deny("PROFILE_ESCALATION", "requested capability exceeds lease")
    required_capability = {"process": "process", "shell": "shell", "network": "network"}.get(action_kind)
    if required_capability and required_capability not in _capability_names(lease.get("capabilities", [])):
        return AdmissionDecision.deny("PROFILE_ESCALATION", f"{action_kind} capability is not in lease")
    if required_capability and not profile.get("capabilities", {}).get(required_capability):
        return AdmissionDecision.deny("PROFILE_ESCALATION", f"{action_kind} capability is disabled by policy")
    action_lineage = action.get("delegation_lineage")
    if action_lineage is not None and action_lineage != lease.get("delegation_lineage", []):
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "delegation lineage was changed")
    paths = [str(x).replace("\\", "/") for x in action.get("paths", [])]
    if any(not _lease_path_allowed(rel, lease.get("paths", [])) for rel in paths):
        return AdmissionDecision.deny("PATH_VIOLATION", "path exceeds lease allowlist")
    try:
        current = now or datetime.now(timezone.utc)
        if _utc(lease["expires_at"]) <= current.astimezone(timezone.utc):
            return AdmissionDecision.deny("EXPIRED_AUTHORIZATION", "lease expired")
    except Exception:
        return AdmissionDecision.deny("RECEIPT_INVALID", "lease expiry is invalid")
    if lease.get("revocation_epoch") != identity.details.get("revocation_epoch"):
        return AdmissionDecision.deny("REVOKED_AUTHORIZATION", "lease was revoked")
    base = lease.get("base", {})
    if base and (base.get("head") != current_ident.get("head") or base.get("tree") != current_ident.get("tree")):
        return AdmissionDecision.deny("STATE_DRIFT", "repository base changed since authorization")
    if lease.get("authority_state_digest") != digest(identity.details["authority"]) or lease.get("admission_policy_digest") != digest(identity.details["policy"]):
        return AdmissionDecision.deny("AUTHORITY_DRIFT", "authority or policy changed since authorization")

    repair = action_kind == "repair" or bool(action.get("repair"))
    frozen_approval = lease.get("approval_class") == "frozen" and lease.get("frozen_surface_digest") == frozen_surface_digest(root)
    allowed_scopes = set(profile.get("writable_scopes", []))
    if action_scopes and not (action_scopes <= allowed_scopes or "*" in allowed_scopes or frozen_approval):
        return AdmissionDecision.deny("SCOPE_VIOLATION", "requested scope exceeds profile")
    if repair:
        if profile_name != "repair" or lease.get("approval_class") != "repair" or lease.get("mode") != "repair" or lease.get("delegation_lineage"):
            return AdmissionDecision.deny("BOOTSTRAP_SCOPE", "repair requires a direct repair approval and lease")
        if not profile.get("repair_allowed"):
            return AdmissionDecision.deny("BOOTSTRAP_SCOPE", "repair requires explicit repair profile")
        diagnosed_raw = action.get("diagnosed_paths")
        if not isinstance(diagnosed_raw, (list, tuple, set)):
            return AdmissionDecision.deny("PATH_VIOLATION", "repair requires explicitly diagnosed paths")
        diagnosed = set(str(x).replace("\\", "/") for x in diagnosed_raw)
        if not diagnosed or not diagnosed.issubset(set(paths)):
            return AdmissionDecision.deny("PATH_VIOLATION", "repair paths must be diagnosed")
        return AdmissionDecision.allow(health.security_level, work_item=work_id, profile="repair", repair=True)

    if any(_frozen_path(rel) for rel in paths) and not frozen_approval and not profile.get("capabilities", {}).get("frozen_surface"):
        return AdmissionDecision.deny("FROZEN_APPROVAL_REQUIRED", "frozen/protected path requires approval")
    if (action.get("frozen") or action_kind == "frozen") and not frozen_approval:
        return AdmissionDecision.deny("FROZEN_APPROVAL_REQUIRED", "operator frozen receipt and matching lease required")
    for rel in paths:
        if rel.startswith("../") or "/../" in rel or rel.startswith("/"):
            return AdmissionDecision.deny("PATH_VIOLATION", "path escapes repository")
        if _frozen_path(rel) and not frozen_approval and not profile.get("capabilities", {}).get("frozen_surface"):
            return AdmissionDecision.deny("FROZEN_APPROVAL_REQUIRED", "frozen/protected path requires approval")
        patterns = profile.get("writable_paths", [])
        if patterns and not any(_match_path(rel, pat) for pat in patterns) and not frozen_approval:
            return AdmissionDecision.deny("PATH_VIOLATION", f"path is outside profile: {rel}")
    return AdmissionDecision.allow(health.security_level, work_item=work_id, profile=profile_name, work_path=str(path) if path else "")


def _match_path(path: str, pattern: str) -> bool:
    import fnmatch
    return fnmatch.fnmatch(path, pattern) or (pattern.endswith("/**") and path.startswith(pattern[:-3]))


def make_request(root: Path, work_item: str, session_id: str, principal: str = "agent", profile: str = "bounded", scopes: Iterable[str] = (), paths: Iterable[str] = (), capabilities: Mapping[str, bool] | None = None, approval_class: str = "activate", mode: str = "normal", expires_at: datetime | None = None, protected_dir: Path | None = None) -> dict[str, Any]:
    check = verify_registration(root, protected_dir)
    if not check.allowed: raise RuntimeError(check.reason)
    ident = canonical_identity(root); policy, authority = check.details["policy"], check.details["authority"]
    frozen = frozen_surface_digest(root)
    return {"protocol_version": "1", "request_id": str(uuid.uuid4()), "nonce": secrets.token_urlsafe(24), "repository_identity": digest(ident), "repopact_root": ident["repopact_root"], "work_item": work_item, "principal": principal, "adapter_session": session_id, "base": {"head": ident["head"], "tree": ident["tree"]}, "authority_state_digest": digest(authority), "admission_policy_digest": digest(policy), "frozen_surface_digest": frozen, "approval_class": approval_class, "profile": profile, "scopes": sorted(scopes), "paths": sorted(paths), "capabilities": sorted(k for k, v in (capabilities or {}).items() if v), "delegation_ceiling": 0, "mode": mode, "issued_at": iso(), "expires_at": iso(expires_at or (datetime.now(timezone.utc) + timedelta(minutes=30))), "revocation_epoch": check.details.get("revocation_epoch", 0)}


def issue_receipt(request: Mapping[str, Any], signer: Ed25519Signer, approval_class: str | None = None) -> dict[str, Any]:
    req = dict(request); req_digest = digest(req)
    return {"receipt_version": "1", "request_digest": req_digest, "operator_id": signer.operator_id, "key_id": signer.key_id, "signature_algorithm": "ed25519", "signature": signer.sign(req), "approval_class": approval_class or req.get("approval_class", "activate"), "issued_at": iso(), "expires_at": req.get("expires_at"), "authority_version": "1", "revocation_epoch": req.get("revocation_epoch", 0)}


def verify_receipt(request: Mapping[str, Any], receipt: Mapping[str, Any], authority: Mapping[str, Any], now: datetime | None = None) -> AdmissionDecision:
    if receipt.get("signature_algorithm") != "ed25519": return AdmissionDecision.deny("RECEIPT_INVALID", "unsupported receipt signature algorithm")
    if receipt.get("request_digest") != digest(request): return AdmissionDecision.deny("RECEIPT_INVALID", "receipt is not bound to this request")
    if receipt.get("approval_class") != request.get("approval_class"):
        return AdmissionDecision.deny("RECEIPT_INVALID", "receipt approval class does not match request")
    if receipt.get("expires_at") != request.get("expires_at"):
        return AdmissionDecision.deny("RECEIPT_INVALID", "receipt expiry does not match request")
    if receipt.get("revocation_epoch") != request.get("revocation_epoch"):
        return AdmissionDecision.deny("RECEIPT_INVALID", "receipt revocation epoch does not match request")
    op = next((x for x in authority.get("operators", []) if x.get("operator_id") == receipt.get("operator_id") and x.get("key_id") == receipt.get("key_id")), None)
    if not op or not verify_signature(op.get("public_key", ""), receipt.get("signature", ""), request): return AdmissionDecision.deny("RECEIPT_INVALID", "operator signature is invalid")
    n = now or datetime.now(timezone.utc)
    try:
        if _utc(request["expires_at"]) <= n.astimezone(timezone.utc): return AdmissionDecision.deny("EXPIRED_AUTHORIZATION", "request has expired")
    except Exception: return AdmissionDecision.deny("RECEIPT_INVALID", "invalid receipt expiry")
    allowed = set(authority.get("approval_classes", []))
    if receipt.get("approval_class") not in allowed: return AdmissionDecision.deny("NO_OPERATOR_PROOF", "approval class is not trusted")
    if receipt.get("authority_version") != authority.get("authority_version"):
        return AdmissionDecision.deny("RECEIPT_INVALID", "receipt authority version is stale")
    return AdmissionDecision.allow("pre-action", operator_id=receipt.get("operator_id"))


def issue_lease(request: Mapping[str, Any], receipt: Mapping[str, Any], root: Path, protected_dir: Path | None = None, now: datetime | None = None) -> tuple[AdmissionDecision, dict[str, Any] | None]:
    check = verify_registration(root, protected_dir)
    if not check.allowed: return check, None
    vr = verify_receipt(request, receipt, check.details["authority"], now)
    if not vr.allowed: return vr, None
    state, state_path = _protected_state(root, protected_dir)
    request_digest = digest(request)
    if state is None: return AdmissionDecision.deny("GUARD_UNHEALTHY", "protected state unavailable"), None
    if request_digest in set(state.get("used_request_digests", [])):
        return AdmissionDecision.deny("RECEIPT_REPLAY", "authorization request has already been consumed"), None
    state.setdefault("used_request_digests", []).append(request_digest)
    _save_protected_state(state_path, state)
    lease = {"lease_version": 1, "lease_id": str(uuid.uuid4()), "request_digest": request_digest,
             "repository_identity": request["repository_identity"], "repopact_root": request.get("repopact_root"),
             "work_item": request["work_item"], "principal": request["principal"], "session_id": request["adapter_session"],
             "approval_class": request.get("approval_class"), "profile": request["profile"], "mode": request.get("mode", "normal"),
             "scopes": request["scopes"], "paths": request["paths"], "capabilities": request.get("capabilities", []),
             "frozen_surface_digest": request.get("frozen_surface_digest"), "delegation_lineage": request.get("delegation_lineage", []),
             "delegation_ceiling": request.get("delegation_ceiling", 0), "base": request["base"],
             "authority_state_digest": request["authority_state_digest"], "admission_policy_digest": request["admission_policy_digest"],
             "issued_at": iso(now), "expires_at": request["expires_at"], "revocation_epoch": check.details.get("revocation_epoch", 0)}
    return AdmissionDecision.allow("pre-action"), lease


def revoke(root: Path, protected_dir: Path | None = None, *, request: Mapping[str, Any] | None = None,
           receipt: Mapping[str, Any] | None = None) -> int:
    """Apply a revocation transition only after a signed revoke receipt."""
    if request is None or receipt is None:
        raise SignerError("operator receipt required for revocation")
    identity = verify_registration(root, protected_dir)
    if not identity.allowed:
        raise RuntimeError(identity.reason)
    if request.get("approval_class") != "revoke" or receipt.get("approval_class") != "revoke":
        raise SignerError("revocation requires the revoke approval class")
    proof = verify_receipt(request, receipt, identity.details["authority"])
    if not proof.allowed:
        raise SignerError(proof.reason)
    state, p = _protected_state(root, protected_dir)
    if state is None: raise RuntimeError("protected registration missing")
    state["revocation_epoch"] = int(state.get("revocation_epoch", 0)) + 1
    _save_protected_state(p, state); return state["revocation_epoch"]


def operator_revoke(root: Path, signer: Ed25519Signer, protected_dir: Path | None = None) -> int:
    """Create and consume a direct operator-controlled revocation approval."""
    request = make_request(root, "000", "operator-revocation", principal=signer.operator_id,
                           profile="observe", scopes=(), paths=(), capabilities={},
                           approval_class="revoke", mode="normal", protected_dir=protected_dir)
    receipt = issue_receipt(request, signer, "revoke")
    return revoke(root, protected_dir, request=request, receipt=receipt)


def delegation_subset(parent: Mapping[str, Any], child: Mapping[str, Any]) -> AdmissionDecision:
    if child.get("repository_identity") != parent.get("repository_identity") or child.get("work_item") != parent.get("work_item"):
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "delegation changed repository or work item")
    if child.get("principal") in {None, ""} or child.get("principal") == parent.get("principal"):
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "child principal must be distinct")
    if parent.get("approval_class") in {"frozen", "revoke", "recovery"} or child.get("approval_class") in {"frozen", "revoke", "recovery"}:
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "sensitive approval classes cannot be delegated")
    if child.get("approval_class") != parent.get("approval_class") or child.get("mode", "normal") != parent.get("mode", "normal"):
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "delegation changed approval semantics")
    if int(child.get("delegation_ceiling", 0)) >= int(parent.get("delegation_ceiling", 1)):
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "delegation depth is not reduced")
    if set(child.get("scopes", [])) - set(parent.get("scopes", [])):
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "scope exceeds parent")
    if set(child.get("paths", [])) - set(parent.get("paths", [])):
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "paths exceed parent")
    if _capability_names(child.get("capabilities", [])) - _capability_names(parent.get("capabilities", [])):
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "capability exceeds parent")
    if _profile_rank(child.get("profile")) < 0 or _profile_rank(child.get("profile")) > _profile_rank(parent.get("profile")):
        return AdmissionDecision.deny("PROFILE_ESCALATION", "delegation changed profile")
    if _utc(child.get("expires_at")) > _utc(parent.get("expires_at")):
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "expiry exceeds parent")
    parent_lineage = list(parent.get("delegation_lineage", []))
    expected_lineage = parent_lineage + [parent.get("lease_id")]
    if child.get("parent_lease_id") != parent.get("lease_id") or child.get("delegation_lineage") != expected_lineage:
        return AdmissionDecision.deny("DELEGATION_ESCALATION", "delegation lineage is missing or altered")
    return AdmissionDecision.allow("pre-action")


def issue_child_lease(parent: Mapping[str, Any], child: Mapping[str, Any]) -> tuple[AdmissionDecision, dict[str, Any] | None]:
    """Derive a child lease as a strict, lineage-preserving subset of a parent."""
    check = delegation_subset(parent, child)
    if not check.allowed:
        return check, None
    lease = dict(child)
    lease.setdefault("lease_version", 1)
    lease.setdefault("lease_id", str(uuid.uuid4()))
    lease.setdefault("request_digest", digest(child))
    lease["parent_lease_id"] = parent.get("lease_id")
    lease["delegation_lineage"] = list(parent.get("delegation_lineage", [])) + [parent.get("lease_id")]
    lease["revocation_epoch"] = parent.get("revocation_epoch")
    return AdmissionDecision.allow("pre-action"), lease


def safe_audit(decision: AdmissionDecision, request: Mapping[str, Any] | None = None) -> dict[str, Any]:
    return {"event": "admission", "allowed": decision.allowed, "code": decision.code, "reason": decision.reason, "enforcement": decision.enforcement, "request_digest": digest(request) if request else None}


def diagnose(root: Path, protected_dir: Path | None = None) -> list[dict[str, Any]]:
    result = verify_registration(root, protected_dir)
    if result.allowed: return [{"severity": "info", "code": "ADMISSION_READY", "message": "protected admission registration is healthy"}]
    return [{"severity": "warn", "code": result.code, "message": result.reason}]
