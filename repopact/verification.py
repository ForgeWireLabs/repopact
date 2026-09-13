"""Provider-neutral local verification profiles for WI046.

The repository owns the verification contract in ``governance/verification.json``.
This module is the reference local executor for that contract. It deliberately
uses argv arrays with ``shell=False`` and does not interpret provider YAML,
secrets, or remote admission state.
"""

from __future__ import annotations

import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import time
from dataclasses import asdict, dataclass, replace
from datetime import datetime, timezone
from importlib.resources import files
from pathlib import Path
from typing import Any

import jsonschema


CONFIG_REL = Path("governance/verification.json")
SCHEMA_NAME = "verification-profile.schema.json"
KNOWN_PLATFORMS = {"windows", "linux", "macos", "android", "ios"}
WORK_STATUSES = ("proposed", "active", "blocked", "deferred", "completed")
_WORK_ITEM_ID = re.compile(r"^[0-9]{3,}$")


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
    capability: str | None = None
    platforms: tuple[str, ...] = ()
    applicable: bool = True


@dataclass(frozen=True)
class CoverageSummary:
    mode: str
    satisfied: bool
    all_declared_executed: bool
    required_total: int
    required_executed: int
    required_passed: int
    required_failed: int
    required_errors: int
    required_unavailable: int
    required_not_applicable: int
    declared_platforms: tuple[str, ...]
    required_platforms: tuple[str, ...]
    missing_platforms: tuple[str, ...]
    capabilities: dict[str, str]


@dataclass(frozen=True)
class VerificationReport:
    profile: str
    status: str
    executor: str
    platform: str
    root: str
    duration_seconds: float
    coverage: CoverageSummary
    steps: list[StepResult]
    evidence_path: str | None = None

    @property
    def exit_code(self) -> int:
        if self.status == "pass":
            return 0
        if self.status == "fail":
            return 1
        return 2

    def to_dict(self) -> dict[str, Any]:
        payload = {
            "profile": self.profile,
            "status": self.status,
            "executor": self.executor,
            "platform": self.platform,
            "root": self.root,
            "duration_seconds": round(self.duration_seconds, 6),
            "exit_code": self.exit_code,
            "coverage": asdict(self.coverage),
            "steps": [asdict(step) for step in self.steps],
        }
        if self.evidence_path is not None:
            payload["evidence_path"] = self.evidence_path
        return payload

    def with_evidence(self, path: str) -> "VerificationReport":
        return replace(self, evidence_path=path)


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
                "coverage": "host",
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
        if profile.get("coverage") == "complete":
            required_platforms = profile.get("required_platforms")
            if not required_platforms:
                raise VerificationConfigError(
                    f"profile {profile_name!r} declares coverage 'complete' but does not declare "
                    "required_platforms; a complete profile must state which platforms are "
                    "required for the contract to be honestly satisfied"
                )
            for platform in required_platforms:
                if platform not in KNOWN_PLATFORMS:
                    raise VerificationConfigError(
                        f"profile {profile_name!r} required_platforms names unsupported platform {platform!r}"
                    )
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


def _step_meta(step: dict[str, Any]) -> tuple[bool, str | None, tuple[str, ...]]:
    return (
        bool(step.get("required", True)),
        str(step["capability"]) if step.get("capability") else None,
        tuple(str(item) for item in (step.get("platforms") or [])),
    )


def _command_step(root: Path, step: dict[str, Any], platform: str) -> StepResult:
    step_id = str(step["id"])
    required, capability, platforms = _step_meta(step)
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
            capability=capability,
            platforms=platforms,
            applicable=False,
        )

    cwd = _safe_cwd(root, str(step.get("cwd", ".")))
    argv = _expand_argv(list(step["argv"]), root)
    if capability and shutil.which(capability) is None:
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
            capability=capability,
            platforms=platforms,
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
            capability=capability,
            platforms=platforms,
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
            capability=capability,
            platforms=platforms,
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
            capability=capability,
            platforms=platforms,
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
        capability=capability,
        platforms=platforms,
    )


def _builtin_step(root: Path, step: dict[str, Any], platform: str) -> StepResult:
    step_id = str(step["id"])
    required, capability, platforms = _step_meta(step)
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
            capability=capability,
            platforms=platforms,
            applicable=False,
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
                capability=capability,
                platforms=platforms,
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
                capability=capability,
                platforms=platforms,
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
            capability=capability,
            platforms=platforms,
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
        capability=capability,
        platforms=platforms,
    )


def _coverage(profile: dict[str, Any], results: list[StepResult], platform: str) -> CoverageSummary:
    mode = str(profile.get("coverage", "host"))
    required = [step for step in results if step.required]
    required_executed = [
        step for step in required if step.status in {"passed", "failed", "error"}
    ]
    required_not_applicable = [step for step in required if not step.applicable]
    required_unavailable = [step for step in required if step.status == "unavailable"]
    capabilities: dict[str, str] = {}
    for step in results:
        if not step.capability:
            continue
        previous = capabilities.get(step.capability)
        state = (
            "not-applicable"
            if not step.applicable
            else "unavailable"
            if step.status == "unavailable"
            else "available"
        )
        if previous == "unavailable" or state == previous:
            continue
        if state == "unavailable" or previous is None or previous == "not-applicable":
            capabilities[step.capability] = state
    all_declared_executed = not required_unavailable and not required_not_applicable
    declared_platforms = tuple(
        sorted({platform_name for step in required for platform_name in step.platforms})
    )
    required_platforms = tuple(sorted(profile.get("required_platforms") or ()))
    if mode == "complete":
        # A single local invocation can only ever attest to the host it ran on.
        # "complete" is honestly satisfied only when this run's platform covers
        # the *entire* declared required-platform set; any other required
        # platform is reported as missing rather than assumed executed
        # elsewhere. Aggregating separate hosts' evidence into one complete
        # verdict is a deliberately separate, not-yet-built concern.
        missing_platforms = tuple(sorted(set(required_platforms) - {platform}))
        satisfied = (
            not required_unavailable
            and not required_not_applicable
            and not missing_platforms
        )
    else:
        missing_platforms = ()
        satisfied = not required_unavailable
    return CoverageSummary(
        mode=mode,
        satisfied=satisfied,
        all_declared_executed=all_declared_executed,
        required_total=len(required),
        required_executed=len(required_executed),
        required_passed=sum(step.status == "passed" for step in required),
        required_failed=sum(step.status == "failed" for step in required),
        required_errors=sum(step.status == "error" for step in required),
        required_unavailable=len(required_unavailable),
        required_not_applicable=len(required_not_applicable),
        declared_platforms=declared_platforms,
        required_platforms=required_platforms,
        missing_platforms=missing_platforms,
        capabilities=capabilities,
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
    profile = profiles[name]
    for step in profile["steps"]:
        if "argv" in step:
            results.append(_command_step(root, step, platform))
        else:
            results.append(_builtin_step(root, step, platform))

    required = [step for step in results if step.required]
    coverage = _coverage(profile, results, platform)
    if any(step.status == "error" for step in required):
        status = "error"
    elif any(step.status == "failed" for step in required):
        status = "fail"
    elif not coverage.satisfied:
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
        coverage=coverage,
        steps=results,
    )


def _git_value(root: Path, args: list[str]) -> str | None:
    try:
        result = subprocess.run(
            ["git", *args],
            cwd=root,
            shell=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
            timeout=5,
            env={**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_OPTIONAL_LOCKS": "0"},
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    return result.stdout.strip() if result.returncode == 0 else None


def _candidate_identity(root: Path) -> dict[str, Any]:
    commit = _git_value(root, ["rev-parse", "--verify", "HEAD"])
    tree = _git_value(root, ["rev-parse", "HEAD^{tree}"])
    dirty_output = _git_value(root, ["status", "--porcelain", "--untracked-files=all"])
    return {
        "commit": commit,
        "tree": tree,
        "dirty": None if dirty_output is None else bool(dirty_output),
    }


def _work_item_exists(root: Path, work_item: str) -> bool:
    if not _WORK_ITEM_ID.fullmatch(work_item):
        return False
    for status in WORK_STATUSES:
        directory = root / "work" / status
        if not directory.is_dir():
            continue
        for candidate in directory.glob(f"{work_item}-*"):
            if (candidate / "work-item.json").is_file():
                return True
    return False


def build_evidence(
    root: Path,
    report: VerificationReport,
    work_item: str,
    *,
    evidence_id: str | None = None,
    timestamp: datetime | None = None,
) -> dict[str, Any]:
    """Build an evidence-run record for an actual local profile invocation."""
    root = root.resolve()
    if not _WORK_ITEM_ID.fullmatch(work_item):
        raise VerificationConfigError(
            f"evidence work item must be a numeric RepoPact id with at least three digits: {work_item!r}"
        )
    if not _work_item_exists(root, work_item):
        raise VerificationConfigError(
            f"cannot record verification evidence for missing work item {work_item!r}"
        )
    now = (timestamp or datetime.now(timezone.utc)).astimezone(timezone.utc)
    stamp = now.strftime("%Y%m%d-%H%M%S-%f")
    eid = evidence_id or f"{stamp}-verify-{report.profile}"
    if not eid or "/" in eid or "\\" in eid:
        raise VerificationConfigError(f"invalid evidence id {eid!r}")
    result = {
        "pass": "passed",
        "fail": "failed",
        "incomplete": "partial",
        "error": "failed",
    }[report.status]
    commands: list[dict[str, Any]] = []
    for step in report.steps:
        if step.exit_code is None:
            continue
        if step.command:
            command = shlex.join(step.command)
        else:
            command = f"builtin:{step.builtin or step.id}"
        commands.append(
            {
                "command": command,
                "exit_code": step.exit_code,
                "summary": f"{step.id}: {step.status} - {step.summary}",
            }
        )
    timestamp_text = now.replace(microsecond=0).isoformat().replace("+00:00", "Z")
    return {
        "$schema": "../../repopact/schemas/evidence-run.schema.json"
        if (root / "repopact" / "schemas").is_dir()
        else "../../schemas/evidence-run.schema.json",
        "id": eid,
        "timestamp": timestamp_text,
        "timestamp_basis": "git-recording",
        "work_item": work_item,
        "result": result,
        "provenance": "concrete",
        "commands": commands,
        "artifacts": [],
        "environment": {
            "platform": report.platform,
            "executor": report.executor,
            "candidate": _candidate_identity(root),
            "verification": {
                "profile": report.profile,
                "status": report.status,
                "exit_code": report.exit_code,
                "duration_seconds": round(report.duration_seconds, 6),
                "coverage": asdict(report.coverage),
                "steps": [
                    {
                        "id": step.id,
                        "status": step.status,
                        "required": step.required,
                        "applicable": step.applicable,
                        "capability": step.capability,
                        "platforms": list(step.platforms),
                        "exit_code": step.exit_code,
                        "summary": step.summary,
                    }
                    for step in report.steps
                ],
            },
        },
    }


def record_evidence(
    root: Path,
    report: VerificationReport,
    work_item: str,
    *,
    evidence_id: str | None = None,
    refresh_dashboard: bool = True,
) -> str:
    """Persist one immutable verification evidence run and refresh derived state.

    The write is fail-closed: an existing id is never overwritten, and a dashboard
    refresh failure restores the prior dashboard and removes the new evidence file.
    """
    root = root.resolve()
    record = build_evidence(root, report, work_item, evidence_id=evidence_id)
    evidence_dir = root / "evidence" / "runs"
    evidence_dir.mkdir(parents=True, exist_ok=True)
    path = evidence_dir / f"{record['id']}.json"
    if path.exists():
        raise VerificationConfigError(f"evidence record already exists: {path.relative_to(root)}")
    dashboard = root / "audits" / "reports" / "dashboard.md"
    previous_dashboard = dashboard.read_bytes() if dashboard.is_file() else None
    try:
        with path.open("x", encoding="utf-8", newline="\n") as handle:
            json.dump(record, handle, indent=2)
            handle.write("\n")
        if refresh_dashboard:
            from . import generate_dashboard

            generate_dashboard.write_dashboard(root)
    except Exception as exc:
        path.unlink(missing_ok=True)
        if previous_dashboard is None:
            dashboard.unlink(missing_ok=True)
        else:
            dashboard.parent.mkdir(parents=True, exist_ok=True)
            dashboard.write_bytes(previous_dashboard)
        if isinstance(exc, VerificationConfigError):
            raise
        raise VerificationConfigError(f"unable to record verification evidence: {exc}") from exc
    return path.relative_to(root).as_posix()


def render_json(report: VerificationReport) -> str:
    return json.dumps(report.to_dict(), indent=2, sort_keys=True) + "\n"


def render_human(report: VerificationReport) -> str:
    lines = [
        f"RepoPact verification profile: {report.profile}",
        f"executor: {report.executor} | platform: {report.platform}",
        (
            "coverage: "
            f"{report.coverage.mode} | satisfied={str(report.coverage.satisfied).lower()} | "
            f"executed={report.coverage.required_executed}/{report.coverage.required_total} required"
        ),
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
    if report.coverage.capabilities:
        rendered = ", ".join(
            f"{name}={state}" for name, state in sorted(report.coverage.capabilities.items())
        )
        lines.append(f"capabilities: {rendered}")
    if report.coverage.required_not_applicable:
        lines.append(
            f"coverage note: {report.coverage.required_not_applicable} required step(s) were not applicable on this host"
        )
    if report.coverage.mode == "complete" and report.coverage.missing_platforms:
        lines.append(
            "coverage note: required_platforms "
            f"{', '.join(report.coverage.required_platforms)} declared complete; "
            f"this host ({report.platform}) did not execute or represent: "
            f"{', '.join(report.coverage.missing_platforms)}"
        )
    lines.append(f"result: {report.status.upper()} ({report.duration_seconds:.2f}s)")
    if report.evidence_path:
        lines.append(f"evidence: {report.evidence_path}")
    if report.status == "pass":
        lines.append("This proves the local profile invocation only; it does not prove remote admission enforcement.")
    return "\n".join(lines) + "\n"
