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
from .verification import VerificationConfigError, render_human, render_json, run_profile


MANIFEST_NAME = "release-manifest.json"
REPORT_NAME = "release-build-report.json"


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
    root = root.resolve()
    outdir = outdir.resolve()
    if verify_first:
        report = run_profile(root, "release")
        if report.status != "pass":
            raise LocalReleaseError(
                f"release verification profile did not pass ({report.status}); artifact build was not started"
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
    return {
        "status": "ready",
        "outdir": str(outdir),
        "manifest": str(manifest_path),
        "build": build_report,
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
        print(f"RepoPact local release: {result['status']}")
        if "manifest" in result and isinstance(result["manifest"], str):
            print(f"manifest: {result['manifest']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
