"""Local-first release orchestration for WI046.

Release verification, artifact construction, artifact verification, and
publication are separate operations.  GitHub Actions may invoke these operations
when explicitly enabled, but they do not depend on GitHub Actions.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

from .release_build import ReleaseBuildError, build_release
from .verification import (
    VerificationConfigError,
    VerificationReport,
    render_human,
    render_json,
    run_profile,
)


MANIFEST_NAME = "release-manifest.json"
REPORT_NAME = "release-build-report.json"
READINESS_NAME = "release-readiness.json"
READINESS_FORMAT = "repopact-release-readiness-v1"


class LocalReleaseError(RuntimeError):
    pass


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_release_profile(root: Path, *, json_output: bool = False) -> int:
    try:
        report = run_profile(root, "release")
    except VerificationConfigError as exc:
        print(f"RepoPact release verification configuration error: {exc}", file=sys.stderr)
        return 2
    print(render_json(report) if json_output else render_human(report), end="")
    return report.exit_code


def write_manifest(outdir: Path, report: dict[str, Any]) -> Path:
    outdir = outdir.resolve()
    artifacts = []
    for kind in ("wheel", "sdist"):
        item = report[kind]
        path = outdir / item["path"]
        if not path.is_file():
            raise LocalReleaseError(f"release build report references missing artifact: {path}")
        actual = _sha256(path)
        if actual != item["sha256"]:
            raise LocalReleaseError(
                f"artifact hash changed before manifest generation: {path.name} ({actual} != {item['sha256']})"
            )
        artifacts.append({"kind": kind, "path": path.name, "sha256": actual})
    manifest = {
        "format": "repopact-local-release-manifest-v1",
        "commit": report["commit"],
        "version": report["version"],
        "artifact_version": report["artifact_version"],
        "reproducible": bool(report.get("reproducible")),
        "artifacts": artifacts,
        "publication": {"performed": False},
    }
    path = outdir / MANIFEST_NAME
    path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def verify_manifest(dist: Path) -> dict[str, Any]:
    dist = dist.resolve()
    path = dist / MANIFEST_NAME
    if not path.is_file():
        raise LocalReleaseError(f"missing {MANIFEST_NAME} in {dist}")
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise LocalReleaseError(f"invalid release manifest: {exc}") from exc
    if manifest.get("format") != "repopact-local-release-manifest-v1":
        raise LocalReleaseError("unsupported release manifest format")
    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        raise LocalReleaseError("release manifest has no artifacts")
    seen: set[str] = set()
    for item in artifacts:
        name = item.get("path")
        expected = item.get("sha256")
        if not isinstance(name, str) or not name or Path(name).name != name:
            raise LocalReleaseError(f"unsafe artifact path in manifest: {name!r}")
        if name in seen:
            raise LocalReleaseError(f"duplicate artifact in manifest: {name}")
        seen.add(name)
        artifact = dist / name
        if not artifact.is_file():
            raise LocalReleaseError(f"manifest artifact is missing: {name}")
        actual = _sha256(artifact)
        if actual != expected:
            raise LocalReleaseError(f"artifact hash mismatch for {name}: {actual} != {expected}")
    return manifest


def build_local_release(
    root: Path,
    outdir: Path,
    *,
    revision: str = "HEAD",
    verify_first: bool = True,
) -> dict[str, Any]:
    """Build this host's own release artifacts.

    This is deliberately a *host-scoped* operation: it proves the checks this
    host is itself responsible for, then builds what this host can build. It
    does not and cannot prove that other required release platforms have
    executed -- WI046 architecture defect C was conflating the two, which
    meant no single host could ever build a normal verified release once
    `required_platforms` named more than one platform (every individual run
    of a genuinely multi-platform `complete` profile is `incomplete` by
    construction). `coverage.host_ready` answers "did this host complete
    everything it owns", independent of `coverage.satisfied`, which
    additionally requires the full required_platforms set to be represented
    by this one invocation and can therefore never be true alone. See
    `aggregate_release_readiness` for the separate, explicit operation that
    combines multiple hosts' evidence into one release-readiness verdict.
    """
    root = root.resolve()
    outdir = outdir.resolve()
    report: VerificationReport | None = None
    if verify_first:
        report = run_profile(root, "release")
        if report.status in {"fail", "error"} or not report.coverage.host_ready:
            raise LocalReleaseError(
                f"release verification did not pass this host's own required checks "
                f"(status={report.status}, host_ready={report.coverage.host_ready}); "
                "artifact build was not started"
            )
    try:
        build_report = build_release(root, outdir, revision=revision)
    except ReleaseBuildError as exc:
        raise LocalReleaseError(str(exc)) from exc
    (outdir / REPORT_NAME).write_text(
        json.dumps(build_report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    manifest_path = write_manifest(outdir, build_report)
    verify_manifest(outdir)
    readiness_path = write_release_readiness(outdir, build_report, report)
    readiness = json.loads(readiness_path.read_text(encoding="utf-8"))
    return {
        "status": "ready",
        "outdir": str(outdir),
        "manifest": str(manifest_path),
        "build": build_report,
        "host_preparation": readiness["host_preparation"],
        "aggregate_release_readiness": "incomplete" if readiness["missing_platforms"] else "unknown-pending-aggregation",
        "missing_platforms": readiness["missing_platforms"],
    }


def write_release_readiness(
    outdir: Path,
    build_report: dict[str, Any],
    verify_report: VerificationReport | None,
) -> Path:
    """Record this host's own contribution toward aggregate release readiness.

    This is intentionally a narrow, host-scoped fact record: it never claims
    the overall release is ready by itself. `aggregate_release_readiness`
    combines one of these per required platform into that verdict.
    """
    outdir = outdir.resolve()
    manifest = json.loads((outdir / MANIFEST_NAME).read_text(encoding="utf-8"))
    if verify_report is not None:
        platform = verify_report.platform
        required_platforms = list(verify_report.coverage.required_platforms)
        missing_platforms = list(verify_report.coverage.missing_platforms)
        host_preparation = "passed" if verify_report.coverage.host_ready else "failed"
    else:
        # verify_first=False (development/debug only): the host's own state
        # was never actually checked, so it cannot be recorded as prepared.
        from .verification import current_platform

        platform = current_platform()
        required_platforms = []
        missing_platforms = []
        host_preparation = "unverified"
    payload = {
        "format": READINESS_FORMAT,
        "platform": platform,
        "candidate": {"commit": build_report["commit"], "version": build_report["version"], "dirty": False},
        "required_platforms": required_platforms,
        "missing_platforms": missing_platforms,
        "host_preparation": host_preparation,
        "artifacts": manifest["artifacts"],
    }
    path = outdir / READINESS_NAME
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def aggregate_release_readiness(readiness_paths: list[Path]) -> dict[str, Any]:
    """Combine per-host `release-readiness.json` records into one truthful
    cross-platform verdict.

    This never runs anything and never contacts a provider; it only reads
    repository-native evidence files the operator supplies (typically one
    per host that ran `release build`). No provider run ID is authoritative
    -- aggregation is keyed entirely on the recorded candidate identity.
    """
    if not readiness_paths:
        raise LocalReleaseError("no release-readiness records supplied for aggregation")
    records: list[dict[str, Any]] = []
    for path in readiness_paths:
        try:
            record = json.loads(Path(path).read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise LocalReleaseError(f"cannot read release-readiness record {path}: {exc}") from exc
        if record.get("format") != READINESS_FORMAT:
            raise LocalReleaseError(f"unsupported release-readiness format in {path}")
        records.append(record)

    candidate = records[0]["candidate"]
    required_platforms = sorted(records[0]["required_platforms"])
    if not required_platforms:
        raise LocalReleaseError("release-readiness records do not declare required_platforms")

    by_platform: dict[str, dict[str, Any]] = {}
    wrong_candidate: list[str] = []
    duplicate_platforms: list[str] = []
    for record in records:
        if sorted(record["required_platforms"]) != required_platforms:
            raise LocalReleaseError(
                "release-readiness records disagree on required_platforms; "
                "they do not describe the same release contract"
            )
        platform = record["platform"]
        if record["candidate"] != candidate:
            wrong_candidate.append(platform)
            continue
        if platform in by_platform:
            duplicate_platforms.append(platform)
            continue
        by_platform[platform] = record

    if duplicate_platforms:
        raise LocalReleaseError(
            f"duplicate release-readiness evidence for platform(s): {', '.join(sorted(set(duplicate_platforms)))}"
        )

    missing_platforms = sorted(set(required_platforms) - set(by_platform))
    failed_platforms = sorted(
        platform for platform, record in by_platform.items() if record["host_preparation"] != "passed"
    )
    platform_states = {
        platform: (
            "missing"
            if platform in missing_platforms
            else "failed"
            if platform in failed_platforms
            else "passed"
        )
        for platform in required_platforms
    }
    artifacts = {
        platform: record["artifacts"] for platform, record in by_platform.items()
    }
    candidate_dirty = bool(candidate.get("dirty"))
    ready = (
        not missing_platforms
        and not failed_platforms
        and not wrong_candidate
        and not candidate_dirty
    )
    return {
        "format": "repopact-release-aggregate-readiness-v1",
        "candidate": candidate,
        "required_platforms": required_platforms,
        "platform_states": platform_states,
        "missing_platforms": missing_platforms,
        "failed_platforms": failed_platforms,
        "wrong_candidate_platforms": sorted(set(wrong_candidate)),
        "artifacts": artifacts,
        "aggregate_ready": ready,
    }


def publish_local_release(
    dist: Path,
    *,
    confirm: bool,
    repository_url: str | None = None,
    dry_run: bool = False,
) -> dict[str, Any]:
    dist = dist.resolve()
    manifest = verify_manifest(dist)
    if not confirm:
        raise LocalReleaseError(
            "publication requires explicit --confirm-publish; verification/build never publish as a side effect"
        )
    artifact_names = [item["path"] for item in manifest["artifacts"]]
    command = [sys.executable, "-m", "twine", "upload"]
    if repository_url:
        command.extend(["--repository-url", repository_url])
    command.extend(str(dist / name) for name in artifact_names)
    if dry_run:
        return {
            "status": "dry-run",
            "command": command,
            "artifacts": artifact_names,
            "credential_source": "external operator environment/keyring",
        }
    env = dict(os.environ)
    env["PYTHONUNBUFFERED"] = "1"
    try:
        result = subprocess.run(
            command,
            cwd=dist,
            env=env,
            shell=False,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
    except OSError as exc:
        raise LocalReleaseError(f"unable to start Twine: {exc}") from exc
    if result.returncode != 0:
        raise LocalReleaseError(f"Twine publication failed with exit code {result.returncode}")
    return {
        "status": "published",
        "artifacts": artifact_names,
        "credential_source": "external operator environment/keyring",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="RepoPact local-first release operations")
    sub = parser.add_subparsers(dest="command", required=True)

    verify = sub.add_parser("verify", help="Run the repository release verification profile")
    verify.add_argument("--root", type=Path, default=Path.cwd())
    verify.add_argument("--json", action="store_true")

    build = sub.add_parser("build", help="Build reproducible release artifacts locally")
    build.add_argument("--root", type=Path, default=Path.cwd())
    build.add_argument("--outdir", type=Path, required=True)
    build.add_argument("--revision", default="HEAD")
    build.add_argument("--skip-verify", action="store_true", help="Development/debug only; do not use for release readiness evidence")
    build.add_argument("--json", action="store_true")

    inspect = sub.add_parser("inspect", help="Verify a local release manifest and artifact hashes")
    inspect.add_argument("--dist", type=Path, required=True)
    inspect.add_argument("--json", action="store_true")

    publish = sub.add_parser("publish", help="Explicitly publish already verified local artifacts")
    publish.add_argument("--dist", type=Path, required=True)
    publish.add_argument("--repository-url")
    publish.add_argument("--confirm-publish", action="store_true")
    publish.add_argument("--dry-run", action="store_true")
    publish.add_argument("--json", action="store_true")

    readiness = sub.add_parser(
        "readiness",
        help="Aggregate per-host release-readiness records into one cross-platform verdict",
    )
    readiness.add_argument(
        "--evidence",
        type=Path,
        nargs="+",
        required=True,
        help=f"One or more {READINESS_NAME} files (or directories containing one) from `release build` runs",
    )
    readiness.add_argument("--json", action="store_true")

    args = parser.parse_args(argv)
    try:
        if args.command == "verify":
            return verify_release_profile(args.root, json_output=args.json)
        if args.command == "build":
            result = build_local_release(
                args.root,
                args.outdir,
                revision=args.revision,
                verify_first=not args.skip_verify,
            )
        elif args.command == "inspect":
            result = {"status": "verified", "manifest": verify_manifest(args.dist)}
        elif args.command == "readiness":
            paths = [
                (p / READINESS_NAME) if p.is_dir() else p
                for p in args.evidence
            ]
            result = aggregate_release_readiness(paths)
        else:
            result = publish_local_release(
                args.dist,
                confirm=args.confirm_publish,
                repository_url=args.repository_url,
                dry_run=args.dry_run,
            )
    except (LocalReleaseError, VerificationConfigError) as exc:
        print(f"RepoPact local release error: {exc}", file=sys.stderr)
        return 2
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        if args.command == "readiness":
            print(f"RepoPact aggregate release readiness: {'ready' if result['aggregate_ready'] else 'not ready'}")
            if result["missing_platforms"]:
                print(f"missing platforms: {', '.join(result['missing_platforms'])}")
            if result["failed_platforms"]:
                print(f"failed platforms: {', '.join(result['failed_platforms'])}")
            if result["wrong_candidate_platforms"]:
                print(f"wrong-candidate platforms: {', '.join(result['wrong_candidate_platforms'])}")
        else:
            print(f"RepoPact local release: {result['status']}")
            if "manifest" in result and isinstance(result["manifest"], str):
                print(f"manifest: {result['manifest']}")
            if "aggregate_release_readiness" in result:
                print(f"host_preparation: {result['host_preparation']}")
                print(f"aggregate_release_readiness: {result['aggregate_release_readiness']}")
                if result["missing_platforms"]:
                    print(f"missing platforms: {', '.join(result['missing_platforms'])}")
    return 0 if args.command != "readiness" or result["aggregate_ready"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
