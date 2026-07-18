"""Deterministic public-adopter verification and release closeout gates."""

from __future__ import annotations

import base64
import hashlib
import json
import re
import subprocess
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

import jsonschema


SEMVER = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")
EXACT_PIN = re.compile(r"(?m)^\s*repopact==([0-9]+\.[0-9]+\.[0-9]+)\s*(?:#.*)?$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
HUNK = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")


class RemoteFailure(RuntimeError):
    """A declared public remote or file could not be verified."""


class RemoteClient(Protocol):
    def resolve_head(self, repository: str, branch: str) -> str: ...

    def read_file(self, repository: str, revision: str, path: str) -> bytes: ...


class GitHubClient:
    """Read public GitHub state without requiring a signed-in session."""

    def __init__(self, timeout: float = 20.0) -> None:
        self.timeout = timeout

    def resolve_head(self, repository: str, branch: str) -> str:
        url = f"https://github.com/{repository}.git"
        try:
            proc = subprocess.run(
                ["git", "ls-remote", url, f"refs/heads/{branch}"],
                capture_output=True,
                text=True,
                timeout=self.timeout,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise RemoteFailure(f"cannot query {repository} branch {branch}: {exc}") from exc
        if proc.returncode != 0:
            detail = (proc.stderr or proc.stdout).strip() or f"git exited {proc.returncode}"
            raise RemoteFailure(f"cannot query {repository} branch {branch}: {detail}")
        fields = proc.stdout.strip().split()
        if len(fields) != 2 or not re.fullmatch(r"[0-9a-f]{40}", fields[0]):
            raise RemoteFailure(f"declared branch {repository}@{branch} is missing or returned no commit")
        return fields[0]

    def read_file(self, repository: str, revision: str, path: str) -> bytes:
        encoded_path = "/".join(urllib.parse.quote(part, safe="") for part in path.split("/"))
        url = f"https://raw.githubusercontent.com/{repository}/{revision}/{encoded_path}"
        request = urllib.request.Request(url, headers={"User-Agent": "RepoPact-fleet-verifier/1"})
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                return response.read()
        except (urllib.error.URLError, TimeoutError, OSError) as public_exc:
            # A declared adopter may be operator-visible but non-public (ForgeWire
            # is the current example). Reuse an existing gh login when available;
            # failure remains explicit and closed if neither route can read it.
            endpoint = f"repos/{repository}/contents/{encoded_path}?ref={revision}"
            try:
                proc = subprocess.run(
                    ["gh", "api", endpoint],
                    capture_output=True,
                    text=True,
                    timeout=self.timeout,
                    check=False,
                )
            except (OSError, subprocess.TimeoutExpired) as authenticated_exc:
                raise RemoteFailure(
                    f"cannot read {repository}@{revision}:{path}: public read failed ({public_exc}); "
                    f"authenticated GitHub read failed ({authenticated_exc})"
                ) from authenticated_exc
            if proc.returncode == 0:
                try:
                    payload = json.loads(proc.stdout)
                    if payload.get("encoding") != "base64":
                        raise ValueError("GitHub content response was not base64")
                    return base64.b64decode(str(payload["content"]), validate=False)
                except (KeyError, ValueError, json.JSONDecodeError) as exc:
                    raise RemoteFailure(
                        f"cannot decode authenticated GitHub content for {repository}@{revision}:{path}: {exc}"
                    ) from exc
            detail = proc.stderr.strip() or proc.stdout.strip() or f"gh exited {proc.returncode}"
            raise RemoteFailure(
                f"cannot read {repository}@{revision}:{path}: public read failed ({public_exc}); "
                f"authenticated GitHub read failed ({detail})"
            ) from public_exc


@dataclass(frozen=True)
class AdopterResult:
    adopter_id: str
    repository: str
    default_branch: str
    remote_head: str | None
    consumption: str
    declared_version: str | None
    current_release: str
    validation_commands: tuple[str, ...]
    local_checkouts: tuple[str, ...]
    checks: tuple[str, ...]
    errors: tuple[str, ...]

    @property
    def ok(self) -> bool:
        return not self.errors

    def as_dict(self) -> dict:
        return {
            "id": self.adopter_id,
            "repository": self.repository,
            "default_branch": self.default_branch,
            "remote_head": self.remote_head,
            "consumption": self.consumption,
            "declared_version": self.declared_version,
            "current_release": self.current_release,
            "validation_commands": list(self.validation_commands),
            "local_checkouts": list(self.local_checkouts),
            "checks": list(self.checks),
            "errors": list(self.errors),
            "status": "pass" if self.ok else "fail",
        }


@dataclass(frozen=True)
class FleetReport:
    release: str
    manifest: str
    adopters: tuple[AdopterResult, ...]
    unregistered_candidates: tuple[dict, ...]
    discovery_errors: tuple[str, ...]

    @property
    def ok(self) -> bool:
        return bool(self.adopters) and all(result.ok for result in self.adopters)

    def as_dict(self) -> dict:
        return {
            "release": self.release,
            "manifest": self.manifest,
            "status": "pass" if self.ok else "fail",
            "adopters": [result.as_dict() for result in self.adopters],
            "unregistered_candidates": list(self.unregistered_candidates),
            "discovery_errors": list(self.discovery_errors),
        }


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _repo_key(repository: str) -> str:
    return repository.removesuffix(".git").strip("/").lower()


def canonical_remote(url: str) -> str | None:
    """Return owner/repository for common public GitHub remote forms."""
    value = url.strip()
    if not value:
        return None
    if value.startswith("git@github.com:"):
        path = value.split(":", 1)[1]
    elif value.startswith("ssh://git@github.com/"):
        path = value.removeprefix("ssh://git@github.com/")
    else:
        parsed = urllib.parse.urlparse(value)
        if parsed.hostname != "github.com":
            return None
        path = parsed.path
    path = path.strip("/").removesuffix(".git")
    return path if path.count("/") == 1 else None


def _local_remote(path: Path) -> str | None:
    try:
        proc = subprocess.run(
            ["git", "-C", str(path), "remote", "get-url", "origin"],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    return canonical_remote(proc.stdout) if proc.returncode == 0 else None


def discover_local_consumers(discovery_roots: list[Path]) -> tuple[dict[str, tuple[str, ...]], tuple[dict, ...], tuple[str, ...]]:
    """Discover one-level local checkouts and collapse duplicates by remote identity."""
    grouped: dict[str, set[str]] = {}
    display: dict[str, str] = {}
    marked: set[str] = set()
    errors: list[str] = []
    seen_paths: set[Path] = set()
    for discovery_root in sorted({path.resolve() for path in discovery_roots}, key=lambda p: str(p).lower()):
        candidates = [discovery_root]
        try:
            candidates.extend(path for path in discovery_root.iterdir() if path.is_dir())
        except OSError as exc:
            errors.append(f"cannot scan {discovery_root}: {exc}")
            continue
        for candidate in sorted(candidates, key=lambda p: str(p).lower()):
            resolved = candidate.resolve()
            if resolved in seen_paths or not (candidate / ".git").exists():
                continue
            seen_paths.add(resolved)
            remote = _local_remote(candidate)
            if remote is None:
                continue
            key = _repo_key(remote)
            display.setdefault(key, remote)
            grouped.setdefault(key, set()).add(str(resolved))
            if (candidate / "requirements-repopact.txt").is_file() or (candidate / "scripts" / "REPOPACT_VERSION").is_file():
                marked.add(key)
    collapsed = {key: tuple(sorted(paths, key=str.lower)) for key, paths in sorted(grouped.items())}
    candidates = tuple(
        {"repository": display[key], "local_checkouts": list(collapsed[key])}
        for key in sorted(marked)
    )
    return collapsed, candidates, tuple(sorted(errors))


def load_manifest(root: Path, manifest_path: Path | None = None) -> tuple[Path, dict]:
    path = (manifest_path or root / "governance" / "adopters.json")
    if not path.is_absolute():
        path = root / path
    path = path.resolve()
    data = json.loads(path.read_text(encoding="utf-8"))
    schema = json.loads((root / "schemas" / "adopter-fleet.schema.json").read_text(encoding="utf-8"))
    jsonschema.Draft202012Validator(schema).validate(data)
    ids = [entry["id"] for entry in data["adopters"]]
    repos = [_repo_key(entry["repository"]) for entry in data["adopters"]]
    if len(ids) != len(set(ids)):
        raise ValueError("adopter manifest contains duplicate ids")
    if len(repos) != len(set(repos)):
        raise ValueError("adopter manifest contains duplicate remote identities")
    return path, data


def _apply_unified_patch(source: bytes, patch: bytes) -> bytes:
    """Apply the reviewable UTF-8 unified overlay stored by a vendored contract."""
    source_lines = source.decode("utf-8").splitlines(keepends=True)
    patch_lines = patch.decode("utf-8").splitlines(keepends=True)
    output: list[str] = []
    source_index = 0
    index = 0
    saw_hunk = False
    while index < len(patch_lines):
        match = HUNK.match(patch_lines[index])
        if not match:
            index += 1
            continue
        saw_hunk = True
        old_count = int(match.group(2) or "1")
        # Unified diffs encode a pure insertion as ``-N,0`` where N is the
        # source line *before* the insertion. Other hunks start at line N.
        target_index = int(match.group(1)) if old_count == 0 else int(match.group(1)) - 1
        if target_index < source_index:
            raise ValueError("overlay hunks are out of order")
        output.extend(source_lines[source_index:target_index])
        source_index = target_index
        index += 1
        while index < len(patch_lines) and not patch_lines[index].startswith("@@ "):
            line = patch_lines[index]
            if line.startswith(("--- ", "+++ ")):
                break
            if line.startswith("\\ No newline"):
                index += 1
                continue
            prefix = line[:1]
            body = line[1:]
            if prefix in {" ", "-"}:
                if source_index >= len(source_lines) or source_lines[source_index] != body:
                    raise ValueError("overlay context does not match the declared upstream bytes")
                if prefix == " ":
                    output.append(body)
                source_index += 1
            elif prefix == "+":
                output.append(body)
            else:
                break
            index += 1
    if not saw_hunk:
        raise ValueError("overlay contains no unified-diff hunks")
    output.extend(source_lines[source_index:])
    return "".join(output).encode("utf-8")


def _safe_repo_path(root: Path, value: str) -> Path:
    candidate = (root / value).resolve()
    try:
        candidate.relative_to(root.resolve())
    except ValueError as exc:
        raise ValueError(f"path escapes repository root: {value}") from exc
    return candidate


def _verify_vendored(
    root: Path,
    upstream_repository: str,
    release: str,
    adopter: dict,
    head: str,
    client: RemoteClient,
) -> tuple[list[str], list[str]]:
    consumption = adopter["consumption"]
    checks: list[str] = []
    errors: list[str] = []
    if consumption["upstream_version"] != release:
        errors.append(
            f"vendored upstream_version {consumption['upstream_version']} does not match current release {release}"
        )
    for contract in sorted(consumption["files"], key=lambda item: item["adopter_path"]):
        label = contract["adopter_path"]
        try:
            upstream = client.read_file(
                upstream_repository, consumption["upstream_revision"], contract["upstream_path"]
            )
            adopted = client.read_file(adopter["repository"], head, contract["adopter_path"])
            upstream_hash = _sha256(upstream)
            adopter_hash = _sha256(adopted)
            if upstream_hash != contract["upstream_sha256"]:
                errors.append(f"{label}: upstream checksum drift ({upstream_hash})")
            if adopter_hash != contract["adopter_sha256"]:
                errors.append(f"{label}: adopter checksum drift ({adopter_hash})")
            if contract["mode"] == "exact":
                if upstream != adopted:
                    errors.append(f"{label}: declared exact copy differs from upstream")
                elif upstream_hash == contract["upstream_sha256"]:
                    checks.append(f"exact checksum parity: {label}")
            else:
                overlay_path = _safe_repo_path(root, contract["overlay_path"])
                overlay = overlay_path.read_bytes()
                canonical_overlay = overlay.replace(b"\r\n", b"\n")
                if _sha256(canonical_overlay) != contract["overlay_sha256"]:
                    errors.append(f"{label}: overlay checksum drift")
                try:
                    reconstructed = _apply_unified_patch(upstream, canonical_overlay)
                except (UnicodeError, ValueError) as exc:
                    errors.append(f"{label}: overlay cannot be applied: {exc}")
                else:
                    if reconstructed != adopted:
                        errors.append(f"{label}: reviewable overlay does not reconstruct adopter bytes")
                    elif adopter_hash == contract["adopter_sha256"]:
                        checks.append(f"reviewable overlay parity: {label}")
        except (OSError, RemoteFailure, ValueError) as exc:
            errors.append(f"{label}: {exc}")
    return checks, errors


def _verify_adopter(
    root: Path,
    upstream_repository: str,
    release: str,
    adopter: dict,
    local_checkouts: tuple[str, ...],
    client: RemoteClient,
) -> AdopterResult:
    errors: list[str] = []
    checks: list[str] = []
    head: str | None = None
    declared_version: str | None = None
    consumption = adopter["consumption"]
    try:
        head = client.resolve_head(adopter["repository"], adopter["default_branch"])
        checks.append(f"public branch resolved: {head}")
        marker = client.read_file(adopter["repository"], head, consumption["version_file"])
        text = marker.decode("utf-8")
        if consumption["type"] == "pypi":
            pins = EXACT_PIN.findall(text)
            if len(pins) != 1:
                errors.append("version file must contain one exact repopact==X.Y.Z pin")
            else:
                declared_version = pins[0]
                checks.append(f"exact PyPI pin: repopact=={declared_version}")
        else:
            declared_version = text.strip()
            if not SEMVER.fullmatch(declared_version):
                errors.append("vendored version marker must contain one semantic version")
            vendored_checks, vendored_errors = _verify_vendored(
                root, upstream_repository, release, adopter, head, client
            )
            checks.extend(vendored_checks)
            errors.extend(vendored_errors)
        if declared_version and declared_version != release:
            errors.append(f"declared version {declared_version} is stale; current release is {release}")
    except (UnicodeError, RemoteFailure) as exc:
        errors.append(str(exc))
    return AdopterResult(
        adopter_id=adopter["id"],
        repository=adopter["repository"],
        default_branch=adopter["default_branch"],
        remote_head=head,
        consumption=consumption["type"],
        declared_version=declared_version,
        current_release=release,
        validation_commands=tuple(adopter["validation_commands"]),
        local_checkouts=local_checkouts,
        checks=tuple(checks),
        errors=tuple(errors),
    )


def verify_fleet(
    root: Path,
    manifest_path: Path | None = None,
    discovery_roots: list[Path] | None = None,
    client: RemoteClient | None = None,
) -> FleetReport:
    root = root.resolve()
    manifest_file, manifest = load_manifest(root, manifest_path)
    release = (root / manifest["upstream"]["version_file"]).read_text(encoding="utf-8").strip()
    if not SEMVER.fullmatch(release):
        raise ValueError(f"current release is not semantic: {release}")
    roots = discovery_roots if discovery_roots is not None else [root.parent]
    local, discovered, discovery_errors = discover_local_consumers(roots) if roots else ({}, (), ())
    registered = {_repo_key(entry["repository"]) for entry in manifest["adopters"]}
    unregistered = tuple(candidate for candidate in discovered if _repo_key(candidate["repository"]) not in registered)
    remote = client or GitHubClient()
    results = tuple(
        _verify_adopter(
            root,
            manifest["upstream"]["repository"],
            release,
            adopter,
            local.get(_repo_key(adopter["repository"]), ()),
            remote,
        )
        for adopter in sorted(manifest["adopters"], key=lambda entry: _repo_key(entry["repository"]))
    )
    try:
        manifest_display = str(manifest_file.relative_to(root)).replace("\\", "/")
    except ValueError:
        manifest_display = str(manifest_file)
    return FleetReport(release, manifest_display, results, unregistered, discovery_errors)


def release_closeout(root: Path, fleet: FleetReport, package_evidence: Path | None) -> dict:
    package_errors: list[str] = []
    evidence_display: str | None = None
    if package_evidence is None:
        package_errors.append("package publication evidence was not supplied")
    else:
        path = package_evidence if package_evidence.is_absolute() else root / package_evidence
        path = path.resolve()
        try:
            evidence = json.loads(path.read_text(encoding="utf-8"))
            try:
                evidence_display = str(path.relative_to(root.resolve())).replace("\\", "/")
            except ValueError:
                evidence_display = str(path)
            if evidence.get("result") != "passed":
                package_errors.append("package publication evidence result is not passed")
            if evidence.get("environment", {}).get("release") != fleet.release:
                package_errors.append("package publication evidence release does not match current release")
            expected_url = f"https://pypi.org/project/repopact/{fleet.release}/"
            artifacts = {str(value).rstrip("/") + "/" for value in evidence.get("artifacts", [])}
            if expected_url not in artifacts:
                package_errors.append(f"package publication evidence does not link {expected_url}")
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            package_errors.append(f"cannot read package publication evidence: {exc}")
    package_complete = not package_errors
    ecosystem_errors = [
        f"{result.repository}: {error}" for result in fleet.adopters for error in result.errors
    ]
    if not fleet.adopters:
        ecosystem_errors.append("fleet manifest contains no adopters")
    ecosystem_complete = fleet.ok
    complete = package_complete and ecosystem_complete
    return {
        "release": fleet.release,
        "status": "pass" if complete else "fail",
        "package_publication": {
            "status": "complete" if package_complete else "incomplete",
            "evidence": evidence_display,
            "errors": package_errors,
        },
        "ecosystem_rollout": {
            "status": "complete" if ecosystem_complete else "incomplete",
            "fleet_manifest": fleet.manifest,
            "errors": ecosystem_errors,
        },
    }


def render_fleet(report: FleetReport) -> str:
    lines = [f"Fleet verification {'PASS' if report.ok else 'FAIL'} (RepoPact {report.release})"]
    for result in report.adopters:
        status = "PASS" if result.ok else "FAIL"
        head = result.remote_head or "unresolved"
        version = result.declared_version or "unverified"
        lines.append(
            f"{status} {result.repository}@{result.default_branch} head={head} "
            f"consumption={result.consumption} version={version}"
        )
        lines.extend(f"  ERROR {error}" for error in result.errors)
    if report.unregistered_candidates:
        lines.append("Unregistered local RepoPact candidates (informational):")
        lines.extend(f"  {entry['repository']}" for entry in report.unregistered_candidates)
    lines.extend(f"DISCOVERY ERROR {error}" for error in report.discovery_errors)
    return "\n".join(lines)


def render_closeout(report: dict) -> str:
    lines = [f"Release closeout {report['status'].upper()} (RepoPact {report['release']})"]
    for phase in ("package_publication", "ecosystem_rollout"):
        value = report[phase]
        lines.append(f"{phase.replace('_', ' ')}: {value['status']}")
        lines.extend(f"  ERROR {error}" for error in value["errors"])
    return "\n".join(lines)


def render_json(value: FleetReport | dict) -> str:
    data = value.as_dict() if isinstance(value, FleetReport) else value
    return json.dumps(data, indent=2, sort_keys=True) + "\n"
