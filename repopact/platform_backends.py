"""Platform-owned protected guard backends.

The policy core is portable, but an enforced guard needs a host boundary that
the gated process cannot rewrite.  This module owns the attestation used by
``ProtectedGuard``; callers cannot turn a reference filesystem backend into an
enforced backend by passing a boolean.
"""
from __future__ import annotations

import ctypes
import hashlib
import json
import os
import platform
import re
import shutil
import stat
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping


class PrivilegeRequired(RuntimeError):
    """Raised when installation or maintenance needs operator elevation."""


@dataclass(frozen=True)
class BackendAttestation:
    """Host-owned facts consumed by the protected guard."""

    backend_id: str
    os_name: str
    installed: bool
    healthy: bool
    integrity_checked: bool
    protected_from_gated_principal: bool
    service_identity_verified: bool
    path_confinement: bool
    process_confinement: bool
    host_configuration_protected: bool
    service_identity: str = ""
    installed_code_path: str = ""
    protected_state_path: str = ""
    ipc_endpoint: str = ""
    reason: str = ""
    assumptions: tuple[str, ...] = ()
    testing_only: bool = False

    @property
    def security_level(self) -> str:
        if not self.healthy or not self.installed or not self.protected_from_gated_principal:
            return "not-covered"
        if self.path_confinement and self.process_confinement:
            return "sandbox/process-enforced"
        if self.service_identity_verified:
            return "session-start"
        return "pre-action"

    def record(self) -> dict[str, Any]:
        return {
            "backend_id": self.backend_id,
            "os": self.os_name,
            "installed": self.installed,
            "healthy": self.healthy,
            "integrity_checked": self.integrity_checked,
            "protected_from_gated_principal": self.protected_from_gated_principal,
            "service_identity_verified": self.service_identity_verified,
            "path_confinement": self.path_confinement,
            "process_confinement": self.process_confinement,
            "host_configuration_protected": self.host_configuration_protected,
            "service_identity": self.service_identity,
            "installed_code_path": self.installed_code_path,
            "protected_state_path": self.protected_state_path,
            "ipc_endpoint": self.ipc_endpoint,
            "security_level": self.security_level,
            "reason": self.reason,
            "assumptions": list(self.assumptions),
            "testing_only": self.testing_only,
        }


def _normalise(path: str | Path) -> str:
    return os.path.normcase(str(Path(path).expanduser().resolve(strict=False))).replace("\\", "/").rstrip("/")


def _is_windows_admin() -> bool:
    if os.name != "nt":
        return False
    try:
        return bool(ctypes.windll.shell32.IsUserAnAdmin())
    except (AttributeError, OSError):
        return False


def _command_output(command: list[str]) -> tuple[int, str]:
    try:
        completed = subprocess.run(command, text=True, capture_output=True, check=False)
    except OSError as exc:
        return 127, str(exc)
    return completed.returncode, (completed.stdout or "") + (completed.stderr or "")


def _windows_acl(path: Path) -> tuple[bool, str]:
    """Return whether an installed path has a verifiable protected ACL."""
    if os.name != "nt" or not path.exists():
        return False, "path is absent or Windows ACL inspection is unavailable"
    code, output = _command_output(["icacls", str(path)])
    if code != 0:
        return False, "icacls could not inspect the installed path"
    upper = output.upper()
    owner_protected = "NT AUTHORITY\\SYSTEM" in upper or "BUILTIN\\ADMINISTRATORS" in upper
    deny_present = "(DENY)" in upper or "DENY" in upper
    users_present = "BUILTIN\\USERS" in upper or "NT AUTHORITY\\AUTHENTICATED USERS" in upper
    return bool(owner_protected and deny_present and users_present), output.strip()


def _windows_runtime_is_protected(path: Path) -> bool:
    """Conservatively reject an interpreter replaceable through its path."""
    protected, _ = _windows_path_chain_is_protected(path)
    return protected


def _windows_reparse_point(path: Path) -> bool:
    """Return whether *path* is a symlink or Windows reparse point."""
    try:
        if path.is_symlink():
            return True
        attributes = getattr(path.stat(follow_symlinks=False), "st_file_attributes", 0)
        return bool(attributes & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400))
    except (OSError, ValueError):
        return True


def _windows_user_writable_roots() -> tuple[Path, ...]:
    """Known per-user roots that must never feed a LocalSystem trust chain."""
    values = [
        os.environ.get("USERPROFILE"),
        os.environ.get("LOCALAPPDATA"),
        os.environ.get("APPDATA"),
        os.environ.get("TEMP"),
        os.environ.get("TMP"),
        str(Path.home()),
    ]
    user_profile = values[0] or str(Path.home())
    values.append(str(Path(user_profile) / "Downloads"))
    roots: list[Path] = []
    for value in values:
        if value:
            try:
                roots.append(Path(value).expanduser().resolve(strict=False))
            except (OSError, ValueError):
                continue
    # Preserve order while removing duplicate paths.
    unique: dict[str, Path] = {}
    for root in roots:
        unique.setdefault(_normalise(root), root)
    return tuple(unique.values())


def _is_under(path: Path, roots: tuple[Path, ...]) -> bool:
    try:
        candidate = path.resolve(strict=False)
    except (OSError, ValueError):
        return True
    return any(candidate == root or candidate.is_relative_to(root) for root in roots)


def _windows_acl_has_broad_write(output: str) -> bool:
    """Detect write/delete rights granted to ordinary broad principals."""
    user_principals = ["(?:BUILTIN\\\\)?USERS:", "AUTHENTICATEDUSERS:", "EVERYONE:", "INTERACTIVE:"]
    for username in (os.environ.get("USERNAME"), os.environ.get("USER")):
        if username:
            user_principals.append(re.escape(username.upper()) + ":")
    principals = re.compile("|".join(user_principals))
    dangerous = {"F", "M", "W", "D", "DC", "WDAC", "WO", "DELETE", "AD", "WEA", "WA"}
    for line in output.upper().splitlines():
        compact = line.replace(" ", "")
        match = principals.search(compact)
        if not match:
            continue
        permissions = compact[match.end():]
        for token in re.findall(r"\(([^)]*)\)", permissions):
            token = token.strip()
            # OI/CI/I/NP/IO are inheritance flags, not access rights.
            if token in {"OI", "CI", "I", "NP", "IO"}:
                continue
            if token in dangerous or any(right in token for right in dangerous):
                return True
    return False


def _windows_path_chain_is_protected(path: Path) -> tuple[bool, str]:
    """Check a file and every replaceable parent against user write rights."""
    if os.name != "nt":
        return False, "Windows ACL inspection is unavailable on this host"
    try:
        candidate = Path(path).expanduser()
        # Inspect the supplied hierarchy before resolving it. A junction or
        # symlink in a parent directory can redirect an otherwise protected
        # canonical target to a user-replaceable location.
        supplied = candidate.absolute()
        supplied_parts: list[Path] = []
        current_supplied = supplied
        while True:
            supplied_parts.append(current_supplied)
            if current_supplied.parent == current_supplied:
                break
            current_supplied = current_supplied.parent
        if any(_windows_reparse_point(item) for item in supplied_parts):
            return False, "interpreter/dependency path hierarchy contains a symlink or reparse point"
        if not candidate.is_file():
            return False, "path is absent or not a regular file"
        canonical = candidate.resolve(strict=True)
        if _windows_reparse_point(candidate) or canonical != supplied.resolve(strict=False):
            return False, "interpreter/dependency path contains a symlink or reparse point"
        if _is_under(canonical, _windows_user_writable_roots()):
            return False, "path is under a known user-writable root"
    except (OSError, ValueError):
        return False, "path could not be resolved canonically"

    checked: list[str] = []
    current = canonical
    while True:
        if _windows_reparse_point(current):
            return False, f"path hierarchy contains a symlink or reparse point: {current}"
        code, output = _command_output(["icacls", str(current)])
        if code != 0:
            return False, f"icacls could not inspect {current}"
        if _windows_acl_has_broad_write(output):
            return False, f"ordinary users can modify or replace {current}"
        checked.append(str(current))
        if current.parent == current:
            break
        current = current.parent
    return True, "ACL/path chain protected: " + ", ".join(checked)


_ISOLATED_DEPENDENCY_SELF_TEST = r'''
import importlib
import json
import site
import sys

result = {
    "ok": False,
    "isolated": False,
    "user_site_enabled": bool(getattr(site, "ENABLE_USER_SITE", False)),
    "sys_path": list(sys.path),
    "required_modules": [],
    "module_origins": {},
    "errors": [],
}

def add_module(logical, name):
    module = importlib.import_module(name)
    origin = getattr(module, "__file__", None)
    if not origin:
        raise RuntimeError(f"{name} has no file origin")
    result["required_modules"].append(logical)
    result["module_origins"][name] = origin
    return module

try:
    add_module("cryptography", "cryptography")
    add_module("cryptography_hashes", "cryptography.hazmat.primitives.hashes")
    add_module("cryptography_serialization", "cryptography.hazmat.primitives.serialization")
    ed25519 = add_module("cryptography_ed25519", "cryptography.hazmat.primitives.asymmetric.ed25519")
    add_module("cryptography_aesgcm", "cryptography.hazmat.primitives.ciphers.aead")
    add_module("cryptography_pbkdf2", "cryptography.hazmat.primitives.kdf.pbkdf2")
    private = ed25519.Ed25519PrivateKey.generate()
    signature = private.sign(b"RepoPact isolated dependency self-test")
    private.public_key().verify(signature, b"RepoPact isolated dependency self-test")
    # The modern cryptography build uses cryptography.hazmat.bindings._rust;
    # older supported builds may load _openssl through cffi instead. Record
    # whichever native binding actually entered this process.
    for name, module in sorted(sys.modules.items()):
        if not module or not (name.startswith("cryptography") or name.startswith("cffi") or name == "_cffi_backend"):
            continue
        origin = getattr(module, "__file__", None)
        if origin:
            result["module_origins"][name] = origin
    for name in ("cffi", "_cffi_backend"):
        if name in result["module_origins"]:
            result["required_modules"].append(name)
    result["isolated"] = (not result["user_site_enabled"] and "" not in result["sys_path"] and
                           all(".venv" not in str(item).lower() for item in result["sys_path"]))
    result["ok"] = bool(result["isolated"] and signature and not result["errors"])
except Exception as exc:
    result["errors"].append(f"{type(exc).__name__}: {exc}")

print("REPOPACT_SELF_TEST=" + json.dumps(result, sort_keys=True))
'''


def _run_isolated_dependency_self_test(interpreter: Path) -> dict[str, Any]:
    """Run the selected interpreter with the exact ``-I`` isolation mode."""
    base: dict[str, Any] = {
        "ok": False, "isolated": False, "user_site_enabled": None,
        "sys_path": [], "required_modules": [], "module_origins": {}, "errors": [],
    }
    if not interpreter.is_file():
        base["errors"] = ["selected interpreter is absent or not a regular file"]
        return base
    env = os.environ.copy()
    # Deliberately hostile values make the executable test prove that -I wins.
    env["PYTHONPATH"] = str(Path.cwd())
    env["PYTHONHOME"] = str(Path.cwd())
    env["PYTHONUSERBASE"] = str(Path.cwd())
    try:
        completed = subprocess.run(
            [str(interpreter), "-I", "-c", _ISOLATED_DEPENDENCY_SELF_TEST],
            cwd=tempfile.gettempdir(), env=env, text=True, capture_output=True,
            check=False, timeout=30,
        )
    except (OSError, subprocess.SubprocessError) as exc:
        base["errors"] = [f"isolated self-test failed to execute: {exc}"]
        return base
    marker = "REPOPACT_SELF_TEST="
    payload = next((line[len(marker):] for line in reversed((completed.stdout or "").splitlines())
                    if line.startswith(marker)), "")
    if not payload:
        base["errors"] = [f"isolated self-test produced no result (exit {completed.returncode})"]
        if completed.stderr:
            base["errors"].append(completed.stderr.strip()[-500:])
        return base
    try:
        result = json.loads(payload)
    except (TypeError, ValueError, json.JSONDecodeError) as exc:
        base["errors"] = [f"isolated self-test returned invalid JSON: {exc}"]
        return base
    if completed.returncode != 0:
        result.setdefault("errors", []).append(f"isolated self-test exit code {completed.returncode}")
        result["ok"] = False
    return result


def _dependency_origin_record(origin: str, root: Path | None) -> dict[str, Any]:
    """Return a conservative, explainable trust result for one module origin."""
    try:
        path = Path(origin).expanduser()
        canonical = path.resolve(strict=True)
    except (OSError, ValueError):
        return {"origin": str(origin), "canonical_path": "", "protected": False,
                "reason": "dependency origin is absent or cannot be resolved"}
    if not canonical.is_file():
        return {"origin": str(origin), "canonical_path": str(canonical), "protected": False,
                "reason": "dependency origin is not a regular file"}
    if root is not None:
        try:
            if canonical.is_relative_to(root.resolve()):
                return {"origin": str(origin), "canonical_path": str(canonical), "protected": False,
                        "reason": "dependency resolves inside the checkout"}
        except (OSError, ValueError):
            return {"origin": str(origin), "canonical_path": str(canonical), "protected": False,
                    "reason": "checkout boundary could not be resolved"}
    if any(".venv" in part.lower() for part in canonical.parts):
        return {"origin": str(origin), "canonical_path": str(canonical), "protected": False,
                "reason": "dependency resolves inside a repository virtual environment"}
    if _is_under(canonical, _windows_user_writable_roots()):
        return {"origin": str(origin), "canonical_path": str(canonical), "protected": False,
                "reason": "dependency resolves under a known user-writable root"}
    protected, reason = _windows_path_chain_is_protected(canonical)
    return {"origin": str(origin), "canonical_path": str(canonical), "protected": protected,
            "reason": reason}


def _isolated_sys_path_is_safe(values: Any, root: Path | None) -> tuple[bool, str]:
    """Ensure isolated startup did not regain checkout/user path injection."""
    if not isinstance(values, list):
        return False, "isolated self-test did not report sys.path"
    checkout = root.resolve() if root is not None else None
    source_package = Path(__file__).resolve().parent
    for value in values:
        if not isinstance(value, str) or not value:
            return False, "isolated sys.path contains an empty or invalid entry"
        try:
            path = Path(value).resolve(strict=False)
        except (OSError, ValueError):
            return False, f"isolated sys.path entry could not be resolved: {value}"
        if checkout is not None and (path == checkout or checkout in path.parents):
            return False, f"isolated sys.path points into the checkout: {path}"
        if path == source_package or source_package in path.parents:
            return False, f"isolated sys.path points into the source checkout: {path}"
        if any(".venv" in part.lower() for part in path.parts):
            return False, f"isolated sys.path points into a virtual environment: {path}"
        if _is_under(path, _windows_user_writable_roots()):
            return False, f"isolated sys.path points into a user-writable root: {path}"
    return True, "isolated sys.path contains no checkout, virtualenv, or user-writable path"


@dataclass(frozen=True)
class PlatformBackend:
    name: str
    os_name: str
    protected_state_location: Path
    process_boundary: bool = False
    path_boundary: bool = False

    @property
    def security_level(self) -> str:
        return self.attest().security_level

    def attest(self, root: Path | None = None, protected_dir: Path | None = None) -> BackendAttestation:
        return BackendAttestation(
            backend_id=self.name,
            os_name=self.os_name,
            installed=False,
            healthy=False,
            integrity_checked=False,
            protected_from_gated_principal=False,
            service_identity_verified=False,
            path_confinement=False,
            process_confinement=False,
            host_configuration_protected=False,
            protected_state_path=str(self.protected_state_location),
            reason="no protected platform guard is installed",
            assumptions=("reference filesystem state is not a security boundary",),
        )

    def capabilities(self) -> dict[str, object]:
        attestation = self.attest()
        return {
            "os": self.os_name,
            "backend": self.name,
            "protected_state": str(self.protected_state_location),
            "path_confinement": attestation.path_confinement,
            "process_confinement": attestation.process_confinement,
            "protected_from_gated_principal": attestation.protected_from_gated_principal,
            "integrity_checked": attestation.integrity_checked,
            "service_identity_verified": attestation.service_identity_verified,
            "host_configuration_protected": attestation.host_configuration_protected,
            "security_level": attestation.security_level,
        }

    def health(self) -> dict[str, object]:
        return self.attest().record()

    def normalize(self, path: str | Path) -> str:
        return _normalise(path)

    def install(self, root: Path | None = None, **_: Any) -> dict[str, Any]:
        raise PrivilegeRequired(f"{self.os_name} protected guard installation is not available on this host")

    def register(self, root: Path, **_: Any) -> dict[str, Any]:
        raise PrivilegeRequired(f"{self.name} registration requires its protected install flow")

    def uninstall(self, **_: Any) -> dict[str, Any]:
        raise PrivilegeRequired(f"{self.os_name} protected guard uninstall requires operator elevation")


class WindowsBackend(PlatformBackend):
    """Windows service backend using protected installed code/state and a pipe."""

    service_name = "RepoPactGuard"

    def __init__(self, location: Path | None = None):
        program_data = Path(os.environ.get("ProgramData", r"C:\ProgramData"))
        install_root = program_data / "RepoPact" / "Guard"
        super().__init__("windows-service", "windows", location or (install_root / "state"))
        object.__setattr__(self, "install_root", install_root)
        object.__setattr__(self, "manifest_path", install_root / "install.json")
        object.__setattr__(self, "runtime_path", install_root / "runtime")
        object.__setattr__(self, "ipc_endpoint", r"\\.\pipe\RepoPactGuard")
        object.__setattr__(self, "registrations_path", self.protected_state_location / "registrations")

    def _service_running(self) -> bool:
        code, output = _command_output(["sc.exe", "query", self.service_name])
        return code == 0 and "RUNNING" in output.upper()

    def _service_configuration(self) -> tuple[str, str]:
        code, output = _command_output(["sc.exe", "qc", self.service_name])
        if code != 0:
            return "", ""
        identity, image = "", ""
        for line in output.splitlines():
            key, _, value = line.partition(":")
            key = key.strip().upper()
            if key == "SERVICE_START_NAME":
                identity = value.strip()
            elif key == "BINARY_PATH_NAME":
                image = value.strip()
        return identity, image

    def _manifest(self) -> Mapping[str, Any] | None:
        try:
            return json.loads(self.manifest_path.read_text(encoding="utf-8"))
        except (OSError, ValueError, json.JSONDecodeError):
            return None

    def _runtime_digest(self) -> str:
        files = sorted(self.runtime_path.rglob("*.py")) if self.runtime_path.is_dir() else []
        h = hashlib.sha256()
        for path in files:
            h.update(_normalise(path).encode("utf-8"))
            h.update(path.read_bytes())
        return h.hexdigest() if files else ""

    def attest(self, root: Path | None = None, protected_dir: Path | None = None) -> BackendAttestation:
        manifest = self._manifest()
        installed = bool(manifest and self.runtime_path.is_dir() and self.protected_state_location.is_dir())
        digest_ok = bool(installed and manifest.get("runtime_digest") == self._runtime_digest())
        acl_results = [_windows_acl(path) for path in (self.install_root, self.runtime_path, self.protected_state_location)]
        acl_ok = bool(acl_results) and all(result[0] for result in acl_results)
        acl_detail = "; ".join(result[1] for result in acl_results if result[1])
        service_running = self._service_running() if installed else False
        configured_identity, configured_image = self._service_configuration() if installed else ("", "")
        service_identity = configured_identity or (str(manifest.get("service_identity", "")) if manifest else "")
        identity_ok = service_identity.lower() in {"nt authority\\system", "local system"}
        image_ok = not configured_image or str(self.runtime_path).lower() in configured_image.lower()
        healthy = bool(installed and digest_ok and acl_ok and service_running and identity_ok and image_ok)
        reason = "healthy protected Windows service" if healthy else (
            "protected Windows service is absent, stopped, tampered, or ACL protection is unproven"
        )
        assumptions = (
            "service runs as LocalSystem",
            "install root and state deny write/ACL changes to the ordinary Users principal",
            "named pipe ACL and caller identity are checked by the service",
        )
        if not acl_ok and acl_detail:
            assumptions += ("ACL attestation detail: " + acl_detail[:300],)
        return BackendAttestation(
            backend_id=self.name,
            os_name=self.os_name,
            installed=installed,
            healthy=healthy,
            integrity_checked=digest_ok,
            protected_from_gated_principal=acl_ok,
            service_identity_verified=identity_ok,
            path_confinement=False,
            process_confinement=False,
            host_configuration_protected=acl_ok,
            service_identity=service_identity,
            installed_code_path=str(self.runtime_path),
            protected_state_path=str(self.protected_state_location),
            ipc_endpoint=self.ipc_endpoint,
            reason=reason,
            assumptions=assumptions,
        )

    def _require_admin(self) -> None:
        if not _is_windows_admin():
            raise PrivilegeRequired(
                "operator elevation required; run `Start-Process pwsh -Verb RunAs` and then "
                "`python -m repopact.cli guard install --root <repo>` in the elevated shell"
            )

    def preflight(self, root: Path | None = None, *, interpreter: Path | None = None) -> dict[str, Any]:
        """Run all non-mutating install checks and return a deterministic report."""
        checks: dict[str, Any] = {}
        checks["platform"] = os.name == "nt"
        checks["operator_elevated"] = _is_windows_admin()
        source_package = Path(__file__).resolve().parent
        checks["source_package"] = source_package.is_dir() and (source_package / "windows_guard_service.py").is_file()
        revision = ""
        if root is not None:
            code, revision_output = _command_output(["git", "-C", str(root.resolve()), "rev-parse", "HEAD"])
            revision = revision_output.strip() if code == 0 else ""
            status_code, status = _command_output(["git", "-C", str(root.resolve()), "status", "--porcelain"])
            checks["source_revision"] = bool(revision)
            checks["source_tree_clean"] = status_code == 0 and not status.strip()
        else:
            checks["source_revision"] = False
            checks["source_tree_clean"] = False
        checks["scm_api"] = shutil.which("sc.exe") is not None if os.name == "nt" else False
        checks["acl_api"] = shutil.which("icacls.exe") is not None if os.name == "nt" else False
        checks["runtime_target_absent"] = not self.install_root.exists()
        checks["service_name_available"] = not bool(self._service_configuration()[0] or self._service_running())

        # An omitted path is only a convenience default. It is still made
        # explicit, canonicalized, and subjected to exactly the same checks as
        # an operator-selected interpreter.
        requested_interpreter = Path(interpreter if interpreter is not None else sys.executable).expanduser()
        try:
            requested_absolute = requested_interpreter.absolute()
            canonical_interpreter = requested_interpreter.resolve(strict=True)
        except (OSError, ValueError):
            requested_absolute = requested_interpreter.absolute()
            canonical_interpreter = Path()
        interpreter_path = str(requested_absolute)
        canonical_path = str(canonical_interpreter) if canonical_interpreter != Path() else ""
        interpreter_exists = bool(canonical_path and canonical_interpreter.is_file())
        interpreter_stable = bool(interpreter_exists and not _windows_reparse_point(canonical_interpreter))
        interpreter_protected = bool(interpreter_stable and _windows_runtime_is_protected(canonical_interpreter)) if os.name == "nt" else False
        self_test = _run_isolated_dependency_self_test(canonical_interpreter if interpreter_exists else requested_absolute)
        checks["interpreter_path"] = interpreter_path
        checks["interpreter_stable"] = interpreter_stable
        checks["interpreter_protected"] = interpreter_protected
        checks["isolated_self_test"] = bool(self_test.get("ok"))
        isolated_paths_safe, isolated_paths_reason = _isolated_sys_path_is_safe(self_test.get("sys_path", []), root)
        self_test["sys_path_safe"] = isolated_paths_safe
        self_test["sys_path_reason"] = isolated_paths_reason
        checks["isolated_paths_safe"] = isolated_paths_safe

        module_origins = self_test.get("module_origins", {})
        if not isinstance(module_origins, Mapping):
            module_origins = {}
        dependency_records: dict[str, dict[str, Any]] = {}
        for module_name, origin in sorted(module_origins.items()):
            if isinstance(module_name, str) and isinstance(origin, str):
                dependency_records[module_name] = _dependency_origin_record(origin, root)
        required_modules = [name for name in self_test.get("required_modules", []) if isinstance(name, str)]
        required_origins = [dependency_records[name] for name in required_modules if name in dependency_records]
        cryptography_records = [record for name, record in dependency_records.items() if name.startswith("cryptography")]
        native_records = [record for name, record in dependency_records.items()
                          if name.startswith("cryptography") and Path(record["canonical_path"] or record["origin"]).suffix.lower() in {".pyd", ".dll", ".so", ".dylib"}]
        cffi_required = any(name in {"cffi", "_cffi_backend"} for name in required_modules)
        cffi_records = [record for name, record in dependency_records.items() if name == "cffi" or name == "_cffi_backend"]
        checks["dependency_cryptography"] = bool(cryptography_records) and all(record["protected"] for record in cryptography_records)
        checks["dependency_cryptography_native"] = bool(native_records) and all(record["protected"] for record in native_records)
        checks["dependency_cffi"] = (not cffi_required) or (bool(cffi_records) and all(record["protected"] for record in cffi_records))
        checks["required_dependency_closure"] = bool(self_test.get("ok") and isolated_paths_safe and required_origins and
                                                     all(record["protected"] for record in required_origins))
        checks["windows_api"] = os.name == "nt"

        service_command_argv = [str(canonical_interpreter or requested_absolute), "-I",
                                str(self.runtime_path / "repopact" / "windows_guard_service.py"),
                                "--service", "--state-root", str(self.protected_state_location)]
        service_command = subprocess.list2cmdline(service_command_argv)
        checks["service_command_isolated"] = " -I " in f" {service_command} "
        # Only boolean checks participate in readiness; descriptive paths and
        # lists must never accidentally make a report truthy.
        checks["ready"] = all(value for key, value in checks.items() if key != "ready" and isinstance(value, bool))
        dependency_origins = sorted({record["canonical_path"] for record in dependency_records.values()
                                     if record.get("canonical_path")})
        interpreter_info = {
            "path": interpreter_path,
            "canonical_path": canonical_path,
            "protected": interpreter_protected,
            "stable": interpreter_stable,
            "isolated_self_test": bool(self_test.get("ok")),
        }
        return {
            "backend": self.name, "service_name": self.service_name, "install_root": str(self.install_root),
            "runtime_path": str(self.runtime_path), "state_path": str(self.protected_state_location),
            "interpreter": interpreter_info, "interpreter_path": interpreter_path, "source_revision": revision,
            "service_command": service_command, "service_command_argv": service_command_argv,
            "isolated_self_test": self_test, "required_dependency_modules": required_modules,
            "dependency_origins": dependency_origins, "dependencies": dependency_records,
            "checks": checks, "mutations": [],
            "rollback": "not-needed" if checks["ready"] else "no machine mutation performed",
        }

    def install(self, root: Path | None = None, *, preflight: bool = False,
                interpreter: Path | None = None, **_: Any) -> dict[str, Any]:
        report = self.preflight(root, interpreter=interpreter)
        if preflight:
            return report
        self._require_admin()
        if os.name != "nt":
            raise PrivilegeRequired("Windows guard installation requires Windows")
        if not report["checks"].get("ready"):
            failures = [key for key, value in report["checks"].items() if key not in {"interpreter_path"} and not value]
            raise RuntimeError("guard install preflight failed (zero machine mutation): " + ", ".join(failures))
        source_package = Path(__file__).resolve().parent
        parent = self.install_root.parent
        staging = parent / f".RepoPactGuard.install-{os.getpid()}-{os.urandom(6).hex()}"
        created_service = False
        try:
            staging.mkdir(parents=True, exist_ok=False)
            stage_runtime = staging / "runtime"; stage_state = staging / "state"
            for source in source_package.rglob("*.py"):
                relative = source.relative_to(source_package); target = stage_runtime / "repopact" / relative
                target.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(source, target)
            stage_state.mkdir(parents=True, exist_ok=True)
            service_entry = stage_runtime / "repopact" / "windows_guard_service.py"
            if not service_entry.exists(): raise RuntimeError("installed guard runtime is missing windows_guard_service.py")
            stage_digest = hashlib.sha256()
            for path in sorted(stage_runtime.rglob("*.py")):
                stage_digest.update(_normalise(path).encode()); stage_digest.update(path.read_bytes())
            code, revision = _command_output(["git", "-C", str(root or Path.cwd()), "rev-parse", "HEAD"])
            selected_interpreter = str(report["interpreter"]["canonical_path"] or report["interpreter"]["path"])
            manifest = {"protocol_version": "1", "service_name": self.service_name, "service_identity": "NT AUTHORITY\\SYSTEM",
                        "installed_code_path": str(self.runtime_path), "protected_state_path": str(self.protected_state_location),
                        "registrations_path": str(self.registrations_path), "ipc_endpoint": self.ipc_endpoint,
                        "runtime_digest": stage_digest.hexdigest(), "source_revision": revision.strip() if code == 0 else "",
                        "interpreter": selected_interpreter, "interpreter_requested": report["interpreter"]["path"],
                        "dependency_closure": report["dependency_origins"],
                        "dependency_modules": report["dependencies"], "service_command": report["service_command"],
                        "isolation": ["-I"]}
            (staging / "install.json").write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n", encoding="utf-8", newline="\n")
            if self.install_root.exists(): raise RuntimeError("install root appeared after preflight; refusing overwrite")
            staging.replace(self.install_root)
            commands = [["icacls", str(self.install_root), "/inheritance:r"],
                        ["icacls", str(self.install_root), "/grant:r", "SYSTEM:(OI)(CI)(F)", "Administrators:(OI)(CI)(F)", "Users:(OI)(CI)(RX)"],
                        ["icacls", str(self.install_root), "/deny", "Users:(OI)(CI)(W,D,DC,WDAC,WO)"],
                        ["icacls", str(self.install_root), "/setowner", "SYSTEM"]]
            for command in commands:
                code, output = _command_output(command)
                if code != 0: raise RuntimeError(f"protected ACL setup failed: {' '.join(command)}: {output.strip()}")
            code, output = _command_output(["sc.exe", "create", self.service_name, "binPath=",
                report["service_command"],
                "start=", "auto", "obj=", "LocalSystem"])
            if code != 0: raise RuntimeError(f"Windows service registration failed: {output.strip()}")
            created_service = True
            start_code, start_output = _command_output(["sc.exe", "start", self.service_name])
            if start_code != 0: raise RuntimeError(f"Windows service start failed: {start_output.strip()}")
            return self.attest(root).record()
        except Exception:
            if created_service: _command_output(["sc.exe", "stop", self.service_name]); _command_output(["sc.exe", "delete", self.service_name])
            if self.install_root.exists(): shutil.rmtree(self.install_root, ignore_errors=True)
            if staging.exists(): shutil.rmtree(staging, ignore_errors=True)
            raise

    def register(self, root: Path, **kwargs: Any) -> dict[str, Any]:
        self._require_admin()
        signer = kwargs.get("signer")
        if signer is None:
            raise PrivilegeRequired("guard register requires an explicit external operator signer")
        from .admission import setup_admission, verify_registration
        if not self.install_root.exists():
            raise PrivilegeRequired("guard must be installed before repository registration")
        if verify_registration(root.resolve(), self.registrations_path).allowed:
            raise RuntimeError("repository is already registered; explicit rotation/unregister is required")
        self.registrations_path.mkdir(parents=True, exist_ok=True)
        result = setup_admission(root.resolve(), self.registrations_path, signer, registry_key="adoption")
        return {"backend": self.name, "root": str(root.resolve()), "status": "registered", **{k: str(v) for k, v in result.items() if k != "signer"}}

    def uninstall(self, **_: Any) -> dict[str, Any]:
        self._require_admin()
        _command_output(["sc.exe", "stop", self.service_name])
        code, output = _command_output(["sc.exe", "delete", self.service_name])
        if code != 0 and "DOES NOT EXIST" not in output.upper():
            raise RuntimeError(output.strip())
        return {"backend": self.name, "status": "service-removed", "state": str(self.protected_state_location)}


class LinuxBackend(PlatformBackend):
    def __init__(self, location: Path | None = None):
        super().__init__("linux-system-service", "linux", location or Path("/var/lib/repopact/registrations"))
        object.__setattr__(self, "service_name", "repopact-guard.service")
        object.__setattr__(self, "ipc_endpoint", "/run/repopact/guard.sock")
        object.__setattr__(self, "runtime_path", Path("/usr/local/lib/repopact/guard"))

    def attest(self, root: Path | None = None, protected_dir: Path | None = None) -> BackendAttestation:
        unit = Path("/etc/systemd/system") / self.service_name
        socket = Path(self.ipc_endpoint)
        installed = unit.is_file() and self.runtime_path.is_dir() and self.protected_state_location.is_dir()
        service_running = False
        if installed:
            code, output = _command_output(["systemctl", "is-active", self.service_name])
            service_running = code == 0 and output.strip() == "active"
        state_ok = self.protected_state_location.exists() and (self.protected_state_location.stat().st_mode & 0o077) == 0
        socket_ok = socket.exists() and (socket.stat().st_mode & 0o077) == 0
        healthy = bool(installed and service_running and state_ok and socket_ok)
        return BackendAttestation(self.name, self.os_name, installed, healthy, installed, healthy, healthy,
                                  False, False, healthy, "repopact-guard.service", str(self.runtime_path),
                                  str(self.protected_state_location), str(socket),
                                  "healthy Linux system service" if healthy else "protected Linux system service is unavailable",
                                  ("system service runs under a distinct service identity", "Unix socket uses peer credentials and restrictive mode"))

    def install(self, root: Path | None = None, **_: Any) -> dict[str, Any]:
        if os.name != "posix" or os.geteuid() != 0:
            raise PrivilegeRequired("Linux guard installation requires root; install a system service and 0600 Unix socket")
        raise RuntimeError("Linux installer is intentionally explicit: package the protected runtime and systemd unit before enabling it")


class MacOSBackend(PlatformBackend):
    def __init__(self, location: Path | None = None):
        super().__init__("macos-launch-daemon", "macos", location or Path("/Library/Application Support/RepoPact/registrations"))
        object.__setattr__(self, "service_name", "com.repopact.guard")
        object.__setattr__(self, "ipc_endpoint", "/var/run/repopact-guard.sock")
        object.__setattr__(self, "runtime_path", Path("/Library/PrivilegedHelperTools/com.repopact.guard"))

    def attest(self, root: Path | None = None, protected_dir: Path | None = None) -> BackendAttestation:
        plist = Path("/Library/LaunchDaemons") / f"{self.service_name}.plist"
        installed = plist.is_file() and self.runtime_path.is_file() and self.protected_state_location.is_dir()
        return BackendAttestation(self.name, self.os_name, installed, False, installed, False, False, False, False, False,
                                  "launchd-daemon", str(self.runtime_path), str(self.protected_state_location), str(self.ipc_endpoint),
                                  "macOS launch daemon is not installed or attested", ("code identity and daemon ACL must be verified by the operator",))

    def install(self, root: Path | None = None, **_: Any) -> dict[str, Any]:
        if platform.system().lower() != "darwin" or os.geteuid() != 0:
            raise PrivilegeRequired("macOS guard installation requires an operator-authorized launch daemon install")
        raise RuntimeError("macOS installer is intentionally explicit: install a protected launch daemon and authenticated IPC endpoint")


class TestingBackend(PlatformBackend):
    """Testing-only backend; never selected by :func:`current_backend`."""

    def __init__(self, location: Path | None = None):
        super().__init__("testing-only-attested-backend", "test", location or Path("testing-protected-state"))

    def attest(self, root: Path | None = None, protected_dir: Path | None = None) -> BackendAttestation:
        return BackendAttestation(self.name, self.os_name, True, True, True, True, True, False, False, True,
                                  "testing-service", "<testing-only>", str(self.protected_state_location), "testing://guard",
                                  "explicit test backend; not production evidence", ("test code supplies the boundary",), True)


def current_backend(root: Path | None = None, protected_dir: Path | None = None) -> PlatformBackend:
    system = platform.system().lower()
    if system == "windows":
        return WindowsBackend()
    if system == "linux":
        return LinuxBackend()
    if system == "darwin":
        return MacOSBackend()
    return PlatformBackend("generic", system, Path.home() / ".repopact" / "registrations")
