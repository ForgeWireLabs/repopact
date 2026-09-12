"""Provider-neutral local verification profiles for WI046.

The repository owns the verification contract in ``governance/verification.json``.
This module is the reference local executor for that contract.  It deliberately
uses argv arrays with ``shell=False`` and does not interpret provider YAML,
secrets, or remote admission state.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from importlib.resources import files
from pathlib import Path
from typing import Any

import jsonschema


CONFIG_REL = Path("governance/verification.json")
SCHEMA_NAME = "verification-profile.schema.json"
KNOWN_PLATFORMS = {"windows", "linux", "macos", "android", "ios"}


class VerificationConfigError(RuntimeError):
    """Raised when the repository verification contract is unusable."""


@dataclass(frozen=True)
class StepResult:
    id: str
    status: str
    required: bool
    command: list[str] | None
    builtin: str | None
    cwd: str
    exit_code: int | None
    duration_seconds: float
    summary: str
    stdout: str = ""
    stderr: str = ""


@dataclass(frozen=True)
class VerificationReport:
    profile: str
    status: str
    executor: str
    platform: str
    root: str
    duration_seconds: float
    steps: list[StepResult]

    @property
    def exit_code(self) -> int:
        if self.status == "pass":
            return 0
        if self.status == "fail":
            return 1
        return 2

    def to_dict(self) -> dict[str, Any]:
        return {
            "profile": self.profile,
            "status": self.status,
            "executor": self.executor,
            "platform": self.platform,
            "root": self.root,
            "duration_seconds": round(self.duration_seconds, 6),
            "exit_code": self.exit_code,
            "steps": [asdict(step) for step in self.steps],
        }


def current_platform() -> str:
    """Return RepoPact's normalized host platform name."""
    if os.environ.get("ANDROID_ROOT") and os.environ.get("ANDROID_DATA"):
        return "android"
    if sys.platform.startswith("win"):
        return "windows"
    if sys.platform == "darwin":
        return "macos"
    if sys.platform.startswith("linux"):
        return "linux"
    # The schema intentionally has a closed platform vocabulary.  Unknown
    # hosts cannot truthfully impersonate one of the supported capability sets.
    return sys.platform.lower()


def default_verification_config(*, schema_ref: str = "../schemas/verification-profile.schema.json") -> dict[str, Any]:
    """Return the minimal provider-neutral contract seeded into adopters."""
    return {
        "$schema": schema_ref,
        "version": 1,
        "default_profile": "governance",
        "execution_policy": {
            "local_primary": True,
            "hosted_ci_default": False,
            "hosted_cd_default": False,
        },
        "profiles": {
            "governance": {
                "description": "Validate the repository with the installed canonical RepoPact engine.",
                "steps": [
                    {
                        "id": "validate",
                        "argv": ["{repopact}", "validate", "--root", "{root}"],
                        "required": True,
                        "timeout_seconds": 300,
                    }
                ],
            }
        },
    }


def _schema_bytes(root: Path) -> bytes:
    local = root / "schemas" / SCHEMA_NAME
    if local.is_file():
        return local.read_bytes()
    return files("repopact").joinpath("schemas", SCHEMA_NAME).read_bytes()


def load_contract(root: Path) -> dict[str, Any]:
    root = root.resolve()
    path = root / CONFIG_REL
    if not path.is_file():
        raise VerificationConfigError(
            f"missing {CONFIG_REL.as_posix()}; run `repopact init/adopt` with a current RepoPact or add a typed verification contract"
        )
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise VerificationConfigError(f"cannot read {CONFIG_REL.as_posix()}: {exc}") from exc
    try:
        schema = json.loads(_schema_bytes(root))
        jsonschema.Draft202012Validator(schema).validate(value)
    except (OSError, json.JSONDecodeError, jsonschema.ValidationError) as exc:
        raise VerificationConfigError(f"invalid {CONFIG_REL.as_posix()}: {exc}") from exc
    _validate_semantics(root, value)
    return value


def _validate_semantics(root: Path, value: dict[str, Any]) -> None:
    profiles = value.get("profiles") or {}
    default_profile = value.get("default_profile")
    if default_profile is not None and default_profile not in profiles:
        raise VerificationConfigError(
            f"default_profile {default_profile!r} does not name a declared verification profile"
        )
    for profile_name, profile in profiles.items():
        seen: set[str] = set()
        for step in profile.get("steps", []):
            step_id = step.get("id", "")
            if step_id in seen:
                raise VerificationConfigError(
                    f"profile {profile_name!r} contains duplicate step id {step_id!r}"
                )
            seen.add(step_id)
            cwd = step.get("cwd", ".")
            _safe_cwd(root, cwd)
            for platform in step.get("platforms", []):
                if platform not in KNOWN_PLATFORMS:
                    raise VerificationConfigError(
                        f"profile {profile_name!r} step {step_id!r} names unsupported platform {platform!r}"
                    )
            for token in step.get("argv", []):
                if token.startswith("{") and token.endswith("}") and token not in {
                    "{python}",
                    "{repopact}",
                    "{root}",
                }:
                    raise VerificationConfigError(
                        f"profile {profile_name!r} step {step_id!r} uses unknown placeholder {token!r}"
                    )


def _safe_cwd(root: Path, value: str) -> Path:
    relative = Path(value)
    if relative.is_absolute():
        raise VerificationConfigError(f"verification step cwd must be repository-relative: {value!r}")
    resolved = (root / relative).resolve()
    try:
        resolved.relative_to(root)
    except ValueError as exc:
        raise VerificationConfigError(f"verification step cwd escapes repository: {value!r}") from exc
    return resolved


def _expand_argv(argv: list[str], root: Path) -> list[str]:
    expanded: list[str] = []
    for token in argv:
        if token == "{python}":
            expanded.append(sys.executable)
        elif token == "{repopact}":
            expanded.extend([sys.executable, "-m", "repopact.cli"])
        elif token == "{root}":
            expanded.append(str(root))
        else:
            expanded.append(token)
    return expanded


def _tail(text: str | None, limit: int = 8000) -> str:
    value = text or ""
    if len(value) <= limit:
        return value
    return "...<truncated>...\n" + value[-limit:]


def _executable_available(executable: str, cwd: Path) -> bool:
    candidate = Path(executable)
    if candidate.is_absolute() or candidate.parent != Path("."):
        path = candidate if candidate.is_absolute() else cwd / candidate
        return path.is_file()
    return shutil.which(executable) is not None


def _command_step(root: Path, step: dict[str, Any], platform: str) -> StepResult:
    step_id = str(step["id"])
    required = bool(step.get("required", True))
    platforms = step.get("platforms") or []
    if platforms and platform not in platforms:
        return StepResult(
            id=step_id,
            status="skipped",
            required=required,
            command=None,
            builtin=None,
            cwd=str(step.get("cwd", ".")),
            exit_code=None,
            duration_seconds=0.0,
            summary=f"not applicable on {platform}; declared for {', '.join(platforms)}",
        )

    cwd = _safe_cwd(root, str(step.get("cwd", ".")))
    argv = _expand_argv(list(step["argv"]), root)
    capability = step.get("capability")
    if capability and shutil.which(str(capability)) is None:
        return StepResult(
            id=step_id,
            status="unavailable",
            required=required,
            command=argv,
            builtin=None,
            cwd=str(cwd.relative_to(root)) or ".",
            exit_code=None,
            duration_seconds=0.0,
            summary=f"required capability/executable {capability!r} is unavailable",
        )
    if not _executable_available(argv[0], cwd):
        return StepResult(
            id=step_id,
            status="unavailable",
            required=required,
            command=argv,
            builtin=None,
            cwd=str(cwd.relative_to(root)) or ".",
            exit_code=None,
            duration_seconds=0.0,
            summary=f"executable {argv[0]!r} is unavailable",
        )

    env = dict(os.environ)
    env["GIT_TERMINAL_PROMPT"] = "0"
    env["GIT_OPTIONAL_LOCKS"] = "0"
    started = time.monotonic()
    timeout = int(step.get("timeout_seconds", 900))
    try:
        result = subprocess.run(
            argv,
            cwd=cwd,
            env=env,
            shell=False,
            text=True,
            encoding="utf-8",
            errors="replace",
            capture_output=True,
            check=False,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        return StepResult(
            id=step_id,
            status="error",
            required=required,
            command=argv,
            builtin=None,
            cwd=str(cwd.relative_to(root)) or ".",
            exit_code=None,
            duration_seconds=time.monotonic() - started,
            summary=f"timed out after {timeout}s",
            stdout=_tail(exc.stdout if isinstance(exc.stdout, str) else None),
            stderr=_tail(exc.stderr if isinstance(exc.stderr, str) else None),
        )
    except OSError as exc:
        return StepResult(
            id=step_id,
            status="error",
            required=required,
            command=argv,
            builtin=None,
            cwd=str(cwd.relative_to(root)) or ".",
            exit_code=None,
            duration_seconds=time.monotonic() - started,
            summary=f"runner error: {exc}",
        )
    duration = time.monotonic() - started
    return StepResult(
        id=step_id,
        status="passed" if result.returncode == 0 else "failed",
        required=required,
        command=argv,
        builtin=None,
        cwd=str(cwd.relative_to(root)) or ".",
        exit_code=result.returncode,
        duration_seconds=duration,
        summary="command passed" if result.returncode == 0 else f"command exited {result.returncode}",
        stdout=_tail(result.stdout),
        stderr=_tail(result.stderr),
    )


def _builtin_step(root: Path, step: dict[str, Any], platform: str) -> StepResult:
    step_id = str(step["id"])
    required = bool(step.get("required", True))
    platforms = step.get("platforms") or []
    if platforms and platform not in platforms:
        return StepResult(
            id=step_id,
            status="skipped",
            required=required,
            command=None,
            builtin=str(step["builtin"]),
            cwd=str(step.get("cwd", ".")),
            exit_code=None,
            duration_seconds=0.0,
            summary=f"not applicable on {platform}; declared for {', '.join(platforms)}",
        )
    started = time.monotonic()
    builtin = str(step["builtin"])
    if builtin == "spec_freshness":
        spec = root / "SPEC.md"
        if not spec.is_file():
            return StepResult(
                id=step_id,
                status="unavailable",
                required=required,
                command=None,
                builtin=builtin,
                cwd=".",
                exit_code=None,
                duration_seconds=time.monotonic() - started,
                summary="SPEC.md is not present in this repository",
            )
        try:
            from . import generate_spec

            current = spec.read_text(encoding="utf-8")
            expected = generate_spec.render(current, root)
        except Exception as exc:
            return StepResult(
                id=step_id,
                status="error",
                required=required,
                command=None,
                builtin=builtin,
                cwd=".",
                exit_code=None,
                duration_seconds=time.monotonic() - started,
                summary=f"unable to render SPEC.md: {exc}",
            )
        fresh = current == expected
        return StepResult(
            id=step_id,
            status="passed" if fresh else "failed",
            required=required,
            command=None,
            builtin=builtin,
            cwd=".",
            exit_code=0 if fresh else 1,
            duration_seconds=time.monotonic() - started,
            summary="SPEC.md derived blocks are current" if fresh else "SPEC.md derived blocks are stale; run `repopact spec`",
        )
    return StepResult(
        id=step_id,
        status="error",
        required=required,
        command=None,
        builtin=builtin,
        cwd=".",
        exit_code=None,
        duration_seconds=time.monotonic() - started,
        summary=f"unsupported verification builtin {builtin!r}",
    )


def run_profile(root: Path, profile_name: str | None = None) -> VerificationReport:
    root = root.resolve()
    contract = load_contract(root)
    name = profile_name or contract.get("default_profile")
    if not name:
        raise VerificationConfigError("no verification profile supplied and no default_profile is declared")
    profiles = contract["profiles"]
    if name not in profiles:
        raise VerificationConfigError(f"unknown verification profile {name!r}")
    platform = current_platform()
    started = time.monotonic()
    results: list[StepResult] = []
    for step in profiles[name]["steps"]:
        if "argv" in step:
            results.append(_command_step(root, step, platform))
        else:
            results.append(_builtin_step(root, step, platform))

    required = [step for step in results if step.required]
    if any(step.status == "error" for step in required):
        status = "error"
    elif any(step.status == "failed" for step in required):
        status = "fail"
    elif any(step.status == "unavailable" for step in required):
        status = "incomplete"
    else:
        status = "pass"
    return VerificationReport(
        profile=name,
        status=status,
        executor="local",
        platform=platform,
        root=str(root),
        duration_seconds=time.monotonic() - started,
        steps=results,
    )


def render_json(report: VerificationReport) -> str:
    return json.dumps(report.to_dict(), indent=2, sort_keys=True) + "\n"


def render_human(report: VerificationReport) -> str:
    lines = [
        f"RepoPact verification profile: {report.profile}",
        f"executor: {report.executor} | platform: {report.platform}",
    ]
    for step in report.steps:
        marker = {
            "passed": "PASS",
            "failed": "FAIL",
            "unavailable": "UNAVAILABLE",
            "skipped": "SKIP",
            "error": "ERROR",
        }.get(step.status, step.status.upper())
        requirement = "required" if step.required else "optional"
        lines.append(f"  {marker:11} {step.id} ({requirement}) - {step.summary}")
        if step.status in {"failed", "error"}:
            if step.stdout.strip():
                lines.append("    stdout: " + _tail(step.stdout, 1200).strip().replace("\n", "\n    "))
            if step.stderr.strip():
                lines.append("    stderr: " + _tail(step.stderr, 1200).strip().replace("\n", "\n    "))
    lines.append(f"result: {report.status.upper()} ({report.duration_seconds:.2f}s)")
    if report.status == "pass":
        lines.append("This proves the local profile invocation only; it does not prove remote admission enforcement.")
    return "\n".join(lines) + "\n"
