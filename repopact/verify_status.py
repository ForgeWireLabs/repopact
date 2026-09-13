"""Unified local operator status view (WI046 AC-17).

Aggregates repository-native signals about local verification, hosted CI,
hosted CD, admission/enforcement, and release readiness into one command.
No remote API call is ever made: anything that would require observing an
actual GitHub repository variable, a branch-protection rule, or another
provider's remote state is reported as ``unknown`` / ``not evidenced``
rather than inferred from a local pass. Local success must never be
represented as proof of remote enforcement.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

from . import release_local
from .verification import VerificationConfigError, _candidate_identity, load_contract

_HOSTED_GATE = re.compile(r"vars\.([A-Z0-9_]+)\s*==\s*'true'")


def _load_json(path: Path) -> Any | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def _local_verification_state(root: Path) -> dict[str, Any]:
    config_path = root / "governance" / "verification.json"
    if not config_path.is_file():
        return {
            "contract_present": False,
            "default_profile": None,
            "profiles": [],
            "latest_evidence": None,
        }
    try:
        config = load_contract(root)
        default_profile = config.get("default_profile")
        profiles = sorted(config.get("profiles", {}))
    except VerificationConfigError:
        default_profile = None
        profiles = []
    return {
        "contract_present": True,
        "default_profile": default_profile,
        "profiles": profiles,
        "latest_evidence": _latest_verification_evidence(root),
    }


def _latest_verification_evidence(root: Path) -> dict[str, Any] | None:
    evidence_dir = root / "evidence" / "runs"
    if not evidence_dir.is_dir():
        return None
    best: tuple[str, Path, dict[str, Any], dict[str, Any]] | None = None
    for path in sorted(evidence_dir.glob("*.json")):
        data = _load_json(path)
        if not isinstance(data, dict):
            continue
        environment = data.get("environment")
        if not isinstance(environment, dict):
            continue
        verification = environment.get("verification")
        if not isinstance(verification, dict):
            continue
        timestamp = str(data.get("timestamp") or "")
        if best is None or timestamp >= best[0]:
            best = (timestamp, path, data, verification)
    if best is None:
        return None
    _, path, data, verification = best
    environment = data.get("environment", {})
    return {
        "evidence_id": data.get("id"),
        "path": path.relative_to(root).as_posix(),
        "timestamp": data.get("timestamp"),
        "profile": verification.get("profile"),
        "status": verification.get("status"),
        "candidate": environment.get("candidate"),
    }


def _hosted_adapter_state(root: Path, workflow_relpath: str, default_flag_key: str, execution_policy: dict[str, Any]) -> dict[str, Any]:
    default_enabled = bool(execution_policy.get(default_flag_key, False))
    path = root / workflow_relpath
    if not path.is_file():
        return {
            "policy_default_enabled": default_enabled,
            "adapter_workflow_present": False,
            "gate_variable": None,
            "actual_remote_state": "unknown; no hosted adapter workflow file found and no remote API call is made by this local status view",
        }
    text = path.read_text(encoding="utf-8", errors="replace")
    match = _HOSTED_GATE.search(text)
    return {
        "policy_default_enabled": default_enabled,
        "adapter_workflow_present": True,
        "gate_variable": match.group(1) if match else None,
        "actual_remote_state": "unknown; the gating repository variable's live value cannot be read without a remote API call, which this local status view never makes",
    }


def _admission_state(root: Path) -> dict[str, Any]:
    policy_path = root / "governance" / "admission-policy.json"
    policy = _load_json(policy_path)
    if not isinstance(policy, dict):
        return {
            "coverage": "not configured",
            "invocation": "unknown",
            "effectiveness": "not-proven",
            "enforcement_closure": "not-proven",
        }
    enabled = bool(policy.get("enabled"))
    return {
        "coverage": "configured" if enabled else "configured (disabled)",
        "invocation": "unknown; this local status view does not perform a merge or admission attempt to observe invocation",
        "effectiveness": "not-proven; effectiveness requires remote enforcement evidence (e.g. an observed rejected merge), which this view never infers from a local pass",
        "enforcement_closure": "not-proven; do not infer remote branch protection from a local pass",
    }


def _release_state(root: Path, evidence_paths: list[Path]) -> dict[str, Any]:
    required_platforms: list[str] = []
    config = _load_json(root / "governance" / "verification.json")
    if isinstance(config, dict):
        release_profile = config.get("profiles", {}).get("release")
        if isinstance(release_profile, dict):
            required_platforms = sorted(release_profile.get("required_platforms") or ())
    if not evidence_paths:
        return {
            "host_preparation": "not evidenced; no release-readiness.json supplied to this status view",
            "required_platforms": required_platforms,
            "platforms_with_evidence": [],
            "missing_platforms": required_platforms,
            "failed_platforms": [],
            "wrong_candidate_platforms": [],
            "aggregate_ready": False,
        }
    try:
        aggregate = release_local.aggregate_release_readiness(evidence_paths)
    except release_local.LocalReleaseError as exc:
        return {
            "host_preparation": "evidence supplied but rejected",
            "required_platforms": required_platforms,
            "error": str(exc),
            "aggregate_ready": False,
        }
    platform_states = aggregate.get("platform_states", {})
    return {
        "host_preparation": "evidenced",
        "required_platforms": aggregate.get("required_platforms", required_platforms),
        "platforms_with_evidence": sorted(
            platform for platform, state in platform_states.items() if state == "passed"
        ),
        "missing_platforms": aggregate.get("missing_platforms", []),
        "failed_platforms": aggregate.get("failed_platforms", []),
        "wrong_candidate_platforms": aggregate.get("wrong_candidate_platforms", []),
        "aggregate_ready": aggregate.get("aggregate_ready", False),
    }


def build_status(root: Path, *, release_evidence: list[Path] | None = None) -> dict[str, Any]:
    """Build the unified local operator status view.

    Every field is derived from repository-native files already on disk
    (governance/verification.json, evidence/runs/*.json, .github/workflows,
    governance/admission-policy.json, and any release-readiness.json files
    the caller explicitly supplies). No remote API call is made anywhere in
    this function.
    """
    root = root.resolve()
    config = _load_json(root / "governance" / "verification.json")
    execution_policy = config.get("execution_policy", {}) if isinstance(config, dict) else {}
    return {
        "format": "repopact-verify-status-v1",
        "candidate": _candidate_identity(root),
        "local_verification": _local_verification_state(root),
        "hosted_ci": _hosted_adapter_state(
            root, ".github/workflows/governance.yml", "hosted_ci_default", execution_policy
        ),
        "hosted_cd": _hosted_adapter_state(
            root, ".github/workflows/release.yml", "hosted_cd_default", execution_policy
        ),
        "admission_enforcement": _admission_state(root),
        "release": _release_state(root, release_evidence or []),
    }


def render_json(status: dict[str, Any]) -> str:
    return json.dumps(status, indent=2, sort_keys=True) + "\n"


def render_human(status: dict[str, Any]) -> str:
    candidate = status["candidate"]
    local = status["local_verification"]
    ci = status["hosted_ci"]
    cd = status["hosted_cd"]
    admission = status["admission_enforcement"]
    release = status["release"]
    latest = local.get("latest_evidence")
    lines = [
        "RepoPact unified verification status (local-only; no remote API calls)",
        f"candidate: commit={candidate.get('commit')} dirty={candidate.get('dirty')}",
        "",
        "local verification:",
        f"  contract present: {local['contract_present']}",
        f"  default profile: {local['default_profile']}",
        f"  latest evidence: {latest['profile'] + ' -> ' + latest['status'] if latest else 'none found'}",
        "",
        "hosted CI:",
        f"  policy default enabled: {ci['policy_default_enabled']}",
        f"  adapter workflow present: {ci['adapter_workflow_present']}",
        f"  actual remote state: {ci['actual_remote_state']}",
        "",
        "hosted CD:",
        f"  policy default enabled: {cd['policy_default_enabled']}",
        f"  adapter workflow present: {cd['adapter_workflow_present']}",
        f"  actual remote state: {cd['actual_remote_state']}",
        "",
        "admission / enforcement:",
        f"  coverage: {admission['coverage']}",
        f"  invocation: {admission['invocation']}",
        f"  effectiveness: {admission['effectiveness']}",
        f"  enforcement closure: {admission['enforcement_closure']}",
        "",
        "release:",
        f"  host preparation: {release['host_preparation']}",
        f"  required platforms: {', '.join(release['required_platforms']) or 'none declared'}",
        f"  platforms with evidence: {', '.join(release.get('platforms_with_evidence', [])) or 'none'}",
        f"  missing platforms: {', '.join(release.get('missing_platforms', [])) or 'none'}",
        f"  aggregate ready: {release['aggregate_ready']}",
    ]
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Print RepoPact's unified local operator status view (WI046 AC-17)"
    )
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")
    parser.add_argument(
        "--release-evidence",
        nargs="*",
        default=[],
        metavar="PATH",
        help="release-readiness.json files (or directories containing one) to fold into the release section",
    )
    args = parser.parse_args(argv)
    evidence_paths: list[Path] = []
    for raw in args.release_evidence:
        candidate = Path(raw)
        evidence_paths.append((candidate / release_local.READINESS_NAME) if candidate.is_dir() else candidate)
    status = build_status(args.root, release_evidence=evidence_paths)
    print(render_json(status) if args.json else render_human(status), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
