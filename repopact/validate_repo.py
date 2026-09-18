from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from datetime import date, datetime, timedelta, timezone
from importlib.resources import files
from pathlib import Path

import jsonschema

from .frontmatter import FrontMatterError, parse_file
from . import generate_dashboard
from . import validate_research
from .package_version import RELEASE_LABEL_RE, PackageVersionError, package_version
from .repo_model import STATUSES, discover_evidence_ids, discover_work_items, iter_contracts, load_json


REQUIRED_WORK_FIELDS = {
    "id", "title", "status", "owner_scope", "affected_scopes", "depends_on",
    "acceptance_criteria", "created", "updated",
}

DECISION_STATUSES = ("proposed", "accepted", "rejected", "deferred", "superseded", "deprecated")
POLICY_STATUSES = ("active", "retired")

# An explicitly recording-backed evidence timestamp is compared with the
# commit that first recorded that file.  Five minutes covers ordinary clock
# drift and the write/commit gap without making validation depend on the
# validator's wall clock.  Legacy records without ``timestamp_basis`` retain
# structural-only timestamp validation: their historical timestamps were often
# backfilled before the file was imported, and completed history must not be
# rewritten to manufacture commit chronology.
FUTURE_TIMESTAMP_TOLERANCE = timedelta(minutes=5)
EVIDENCE_TIMESTAMP_BASIS = "git-recording"

# A work-item README may use the "- [ ] **CRIT-1** ..." checklist convention to
# mirror acceptance-criterion state. Where it does, the checkboxes must not
# contradict the manifest (decision 0014).
CHECKBOX_LINE = re.compile(r"-\s*\[([ xX])\]\s*\*\*([A-Za-z][A-Za-z0-9]*-\d+)\b")


@dataclass(frozen=True)
class Problem:
    path: Path
    message: str
    severity: str = "error"


def blocking_problems(problems: list[Problem]) -> list[Problem]:
    """Problems whose severity gates repository validity.

    Mirrors the Rust engine's ``valid = error_count == 0`` contract (Decision
    0055): 'warning' and 'info' diagnostics are advisory -- they surface
    potential issues or coverage limits but never make a repository invalid.
    Every caller that decides pass/fail from ``validate_repo.validate()``
    output must gate on this, not on the raw problem list, once any
    non-error-severity diagnostic exists.
    """
    return [p for p in problems if p.severity == "error"]


def validate_dates(value: object, field: str, path: Path, problems: list[Problem]) -> None:
    try:
        date.fromisoformat(str(value))
    except ValueError:
        problems.append(Problem(path, f"{field} must be an ISO date"))


# --- schema layer (decision 0003) ------------------------------------------

_SCHEMA_CACHE: dict[tuple[str, str], dict] = {}
_VALIDATOR_CACHE: dict[int, jsonschema.Draft202012Validator] = {}


def load_schema(root: Path, name: str) -> dict:
    local = root / "schemas" / name
    raw = local.read_bytes() if local.is_file() else files("repopact").joinpath("schemas", name).read_bytes()
    # Test fixtures and adopter checkouts normally carry byte-identical schemas
    # at different roots. Cache by content, not path, so a full validation suite
    # does not parse and compile the same contracts hundreds of times.
    key = (name, hashlib.sha256(raw).hexdigest())
    if key not in _SCHEMA_CACHE:
        _SCHEMA_CACHE[key] = json.loads(raw)
    return _SCHEMA_CACHE[key]


def check_schema(instance: object, schema: dict, path: Path, problems: list[Problem]) -> None:
    """Structural validation. Schemas are authoritative for shape; the validator
    functions below are authoritative for cross-record semantics (decision 0003)."""
    validator = _VALIDATOR_CACHE.get(id(schema))
    if validator is None:
        validator = jsonschema.Draft202012Validator(schema)
        _VALIDATOR_CACHE[id(schema)] = validator
    for error in sorted(validator.iter_errors(instance), key=lambda e: list(e.path)):
        location = "/".join(str(part) for part in error.path) or "<root>"
        problems.append(Problem(path, f"schema {location}: {error.message}"))


def registered_contracts(root: Path) -> set[Path]:
    try:
        data = load_json(root / "audits" / "registry.json")
    except (OSError, ValueError, json.JSONDecodeError):
        return set()
    result: set[Path] = set()
    for entry in data.get("scopes", []):
        contract = entry.get("contract")
        if contract:
            result.add((root / str(contract)).resolve().parent)
    return result


def validate_contracts(root: Path, problems: list[Problem]) -> None:
    contracts = iter_contracts(root)
    if root / "AGENTS.md" not in contracts:
        problems.append(Problem(root, "missing root AGENTS.md"))
    covered = registered_contracts(root)
    for contract in contracts:
        if contract.parent == root:
            continue
        if contract.parent.resolve() not in covered:
            problems.append(Problem(contract, "nested contract is not registered in audits/registry.json"))
        audit = contract.parent / "_audit"
        if audit.is_dir():
            for name in ("README.md", "inventory.md", "alignment-report.md"):
                if not (audit / name).is_file():
                    problems.append(Problem(contract, f"incomplete _audit companion, missing _audit/{name}"))


def validate_version(root: Path, problems: list[Problem]) -> None:
    path = root / "VERSION"
    if not path.is_file():
        problems.append(Problem(path, "missing VERSION file"))
        return
    version = path.read_text(encoding="utf-8").strip()
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        problems.append(Problem(path, f"VERSION '{version}' must be semantic (MAJOR.MINOR.PATCH)"))


# A SemVer pre-release/build suffix on the release line: the MAJOR.MINOR.PATCH core
# is captured so it can be pinned to VERSION, then the SemVer pre-release grammar
# (dot-separated identifiers, numeric identifiers with no leading zeros) and the
# optional build metadata (decision 0026).
def validate_release_label(root: Path, problems: list[Problem]) -> None:
    """Validate the optional ``RELEASE_LABEL`` product/maturity identity (decision 0026).

    ``VERSION`` stays a clean, totally-ordered ``MAJOR.MINOR.PATCH`` triple that
    adopter equality and overlay targeting key off. Maturity (``rc.1``, ``beta.2``)
    belongs on the git tag and, when a single shared pre-release string is wanted
    across a repo's surfaces, in this optional file. It is a full SemVer pre-release
    whose core is pinned to ``VERSION``, so it adds a label without ever letting the
    release line diverge. Absent means unconstrained; the rule is purely additive."""
    path = root / "RELEASE_LABEL"
    if not path.is_file():
        return
    label = path.read_text(encoding="utf-8").strip()
    match = RELEASE_LABEL_RE.fullmatch(label)
    if not match:
        problems.append(Problem(
            path,
            f"RELEASE_LABEL '{label}' must be a SemVer pre-release of VERSION "
            "(MAJOR.MINOR.PATCH-prerelease[+build], e.g. 2.3.0-rc.1)",
        ))
        return
    version_path = root / "VERSION"
    version = version_path.read_text(encoding="utf-8").strip() if version_path.is_file() else ""
    if match.group("base") != version:
        problems.append(Problem(
            path,
            f"RELEASE_LABEL base '{match.group('base')}' must equal VERSION '{version}'",
        ))


_PACKAGE_IDENTITY_PATHS = (
    "pyproject.toml",
    "README.md",
    "VERSION",
    "RELEASE_LABEL",
    "repopact/*.py",
    "repopact/schemas",
    "repopact/templates",
)


def _git(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(root), *args],
        text=True,
        capture_output=True,
        check=False,
        timeout=5,
        env={**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_OPTIONAL_LOCKS": "0"},
    )


def validate_source_artifact_identity(root: Path, problems: list[Problem]) -> None:
    """Distinguish post-tag package source from an already released artifact.

    Repositories without Git metadata, including portable conformance fixtures and
    exported source archives, remain valid. When the stable ``vVERSION`` tag exists,
    its exact package/runtime tree may remain unlabeled. Materially later source at
    the same compatibility core requires RELEASE_LABEL, whose package identity is
    independently derived and checked above (decision 0032).
    """
    version_path = root / "VERSION"
    label_path = root / "RELEASE_LABEL"
    if not version_path.is_file() or label_path.is_file():
        return
    version = version_path.read_text(encoding="utf-8").strip()
    tag = f"v{version}"
    resolved = _git(root, "rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}")
    if resolved.returncode != 0:
        return
    changed = _git(root, "diff", "--name-only", tag, "--", *_PACKAGE_IDENTITY_PATHS)
    untracked = _git(root, "ls-files", "--others", "--exclude-standard", "--", *_PACKAGE_IDENTITY_PATHS)
    if changed.returncode != 0 or untracked.returncode != 0:
        return
    paths = sorted({line for line in (changed.stdout + untracked.stdout).splitlines() if line})
    if paths:
        problems.append(Problem(
            label_path,
            f"package/runtime source differs from released {tag} but has no RELEASE_LABEL; "
            f"add a VERSION-pinned development identity (changed: {', '.join(paths)})",
        ))


def validate_package_version(root: Path, problems: list[Problem]) -> None:
    """Prove the governed identity can be rendered as deterministic PEP 440 metadata."""
    if not (root / "VERSION").is_file():
        return
    label_path = root / "RELEASE_LABEL"
    if label_path.is_file():
        label = label_path.read_text(encoding="utf-8").strip()
        version = (root / "VERSION").read_text(encoding="utf-8").strip()
        match = RELEASE_LABEL_RE.fullmatch(label)
        # The structural/core diagnostics belong to validate_release_label; do
        # not emit a second, less actionable error for the same malformed record.
        if not match or match.group("base") != version:
            return
    try:
        package_version(root)
    except (OSError, PackageVersionError) as exc:
        problems.append(Problem(root / "RELEASE_LABEL", f"invalid package identity: {exc}"))


# The README convention this rule is gated on: a release line naming the current
# version, optionally carrying a changelog link. Both groups are optional-friendly
# so a README that omits the line (any adopter's, typically) is simply unaffected.
_README_RELEASE_RE = re.compile(
    r"current release\s+\*\*(?P<version>[^*\s]+)\*\*"
    r"(?:[\s.]*\(\[[^\]]*\]\((?P<link>[^)]+)\)\))?"
)


def validate_release_surface(root: Path, problems: list[Problem]) -> None:
    """Pin the README's release claim to ``VERSION`` (decision 0028).

    The dashboard and SPEC are derived and diffed, but the README — the first
    document an evaluator reads — carried its release number by hand and drifted
    a full minor version behind ``VERSION`` while the newer release was already
    on PyPI. A project that sells drift detection cannot misreport its own
    version, so the one factual claim in that prose becomes a checked record.

    Gated on the convention being present, following README checkbox parity
    (decision 0014): a README without a ``current release **X.Y.Z**`` line is
    unaffected. A changelog link into ``decisions/`` must resolve and must name
    the same version in its front-matter title; a link anywhere else is checked
    for existence only, so RepoPact does not impose its changelog convention on
    adopters.
    """
    readme = root / "README.md"
    version_path = root / "VERSION"
    if not readme.is_file() or not version_path.is_file():
        return
    try:
        text = readme.read_text(encoding="utf-8")
        version = version_path.read_text(encoding="utf-8").strip()
    except OSError as exc:
        problems.append(Problem(readme, str(exc)))
        return
    match = _README_RELEASE_RE.search(text)
    if not match:
        return
    claimed = match.group("version")
    if claimed != version:
        problems.append(Problem(
            readme,
            f"README advertises release '{claimed}' but VERSION is '{version}'; "
            "update the release line together with VERSION",
        ))
    link = match.group("link")
    if not link:
        return
    if re.match(r"[A-Za-z][A-Za-z0-9+.-]*:", link):
        # An absolute URL (a hosted changelog, say). Out of the repository's
        # reach, so there is nothing to resolve; the version claim above still
        # applies.
        return
    target = (root / link.split("#", 1)[0]).resolve()
    try:
        target.relative_to(root.resolve())
    except ValueError:
        problems.append(Problem(readme, f"release changelog link escapes the repository: {link}"))
        return
    if not target.is_file():
        problems.append(Problem(readme, f"release changelog link does not resolve: {link}"))
        return
    if target.parent != (root / "decisions").resolve():
        return
    try:
        title = str(parse_file(target).get("title", ""))
    except FrontMatterError as exc:
        problems.append(Problem(readme, f"release changelog link has unreadable front matter: {exc}"))
        return
    if version not in title:
        problems.append(Problem(
            readme,
            f"release changelog link '{link}' documents '{title}', which does not "
            f"name the current release '{version}'",
        ))


def validate_invariants(root: Path, problems: list[Problem]) -> None:
    path = root / "governance" / "invariants.json"
    try:
        data = load_json(path)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(path, str(exc)))
        return
    check_schema(data, load_schema(root, "invariants.schema.json"), path, problems)
    seen: set[str] = set()
    for entry in data.get("invariants", []):
        inv_id = str(entry.get("id", ""))
        if inv_id in seen:
            problems.append(Problem(path, f"duplicate invariant id '{inv_id}'"))
        seen.add(inv_id)
        for field in ("statement", "rationale", "escalation"):
            if not str(entry.get(field, "")).strip():
                problems.append(Problem(path, f"invariant {inv_id} is missing {field}"))


def validate_frozen_surface(root: Path, problems: list[Problem]) -> None:
    path = root / "governance" / "frozen-surface.json"
    try:
        data = load_json(path)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(path, str(exc)))
        return
    check_schema(data, load_schema(root, "frozen-surface.schema.json"), path, problems)
    for entry in data.get("protected", []):
        if not str(entry.get("reason", "")).strip():
            problems.append(Problem(path, f"protected entry '{entry.get('glob')}' needs a reason"))


def validate_adopter_manifest(root: Path, problems: list[Problem]) -> None:
    """Validate the optional maintainer-only public adopter fleet declaration."""
    path = root / "governance" / "adopters.json"
    if not path.is_file():
        return
    try:
        data = load_json(path)
        schema = load_schema(root, "adopter-fleet.schema.json")
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(path, str(exc)))
        return
    check_schema(data, schema, path, problems)
    entries = [entry for entry in data.get("adopters", []) if isinstance(entry, dict)]
    ids = [str(entry.get("id", "")) for entry in entries]
    repositories = [str(entry.get("repository", "")).lower().removesuffix(".git") for entry in entries]
    if len(ids) != len(set(ids)):
        problems.append(Problem(path, "adopter ids must be unique"))
    if len(repositories) != len(set(repositories)):
        problems.append(Problem(path, "adopter remote identities must be unique"))
    for entry in entries:
        consumption = entry.get("consumption", {})
        if not isinstance(consumption, dict) or consumption.get("type") != "vendored":
            continue
        for contract in consumption.get("files", []):
            if not isinstance(contract, dict) or contract.get("mode") != "overlay":
                continue
            overlay = (root / str(contract.get("overlay_path", ""))).resolve()
            try:
                overlay.relative_to(root.resolve())
                actual = hashlib.sha256(overlay.read_bytes().replace(b"\r\n", b"\n")).hexdigest()
            except (OSError, ValueError) as exc:
                problems.append(Problem(path, f"vendored overlay is unreadable: {exc}"))
                continue
            if actual != contract.get("overlay_sha256"):
                problems.append(Problem(path, f"vendored overlay checksum drift: {contract.get('overlay_path')}"))


def discover_tracked_paths(root: Path) -> list[str] | None:
    """Return Git-tracked paths, or ``None`` when the target is not a checkout.

    Ownership is a source-control property: generated caches and installed seed
    trees are deliberately outside this rule. A checkout is validated against the
    index so every path that can be committed has exactly one declared owner.
    """
    # Ordinary checkouts use a `.git` directory and linked worktrees use a
    # `.git` file. Seeded/exported trees have neither, so avoid spawning Git on
    # every validation pass for those common targets.
    if not (root / ".git").exists():
        return None
    try:
        probe = subprocess.run(
            ["git", "rev-parse", "--is-inside-work-tree"],
            cwd=root,
            text=True,
            capture_output=True,
            check=False,
            timeout=5,
            env={**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_OPTIONAL_LOCKS": "0"},
        )
        if probe.returncode != 0 or probe.stdout.strip() != "true":
            return None
        result = subprocess.run(
            ["git", "ls-files", "--cached"],
            cwd=root,
            text=True,
            capture_output=True,
            check=False,
            timeout=5,
            env={**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_OPTIONAL_LOCKS": "0"},
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    return sorted(path.replace("\\", "/") for path in result.stdout.splitlines() if path)


def validate_tracked_path_ownership(
    root: Path, scopes: list[dict], owners_path: Path, problems: list[Problem]
) -> None:
    tracked = discover_tracked_paths(root)
    if tracked is None:
        return
    declared = [
        (str(scope.get("id", "")), str(pattern))
        for scope in scopes
        if isinstance(scope, dict)
        for pattern in scope.get("paths", [])
        if isinstance(pattern, str) and pattern
    ]
    for relative in tracked:
        matches = sorted({scope_id for scope_id, pattern in declared if fnmatch.fnmatchcase(relative, pattern)})
        if not matches:
            problems.append(Problem(owners_path, f"tracked path '{relative}' has no owner scope"))
        elif len(matches) > 1:
            problems.append(Problem(
                owners_path,
                f"tracked path '{relative}' has multiple owner scopes: {', '.join(matches)}",
            ))


def validate_owners(root: Path, problems: list[Problem]) -> tuple[set[str], bool]:
    path = root / "governance" / "owners.json"
    try:
        data = load_json(path)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(path, str(exc)))
        return set(), False
    scopes = data.get("scopes", [])
    ids = [scope.get("id") for scope in scopes if isinstance(scope, dict)]
    if len(ids) != len(set(ids)):
        problems.append(Problem(path, "scope IDs must be unique"))
    scope_ids = {str(value) for value in ids if value}
    for scope in scopes:
        if not isinstance(scope, dict):
            problems.append(Problem(path, "each scope must be an object"))
            continue
        patterns = scope.get("paths")
        if not isinstance(patterns, list) or not patterns or any(
            not isinstance(value, str) or not value for value in patterns
        ):
            problems.append(Problem(path, f"scope '{scope.get('id')}' must declare non-empty path patterns"))
    for role in data.get("roles", []):
        if not isinstance(role, dict):
            problems.append(Problem(path, "each role must be an object"))
            continue
        for scope in role.get("scopes", []):
            if scope not in scope_ids:
                problems.append(Problem(path, f"role '{role.get('id')}' references unknown scope '{scope}'"))
    if data.get("enforce_tracked_path_ownership", False):
        validate_tracked_path_ownership(root, scopes, path, problems)
    enforce_disjoint = bool(data.get("concurrency", {}).get("enforce_disjoint_active_scopes", False))
    return scope_ids, enforce_disjoint


def validate_findings(root: Path, owner_scopes: set[str], problems: list[Problem]) -> None:
    directory = root / "audits" / "findings"
    if not directory.is_dir():
        return
    schema = load_schema(root, "audit-finding.schema.json")
    seen: dict[str, Path] = {}
    for path in sorted(directory.glob("*.json")):
        try:
            data = load_json(path)
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            problems.append(Problem(path, str(exc)))
            continue
        check_schema(data, schema, path, problems)
        finding_id = str(data.get("id", ""))
        if finding_id and not path.name.startswith(f"{finding_id}-"):
            problems.append(Problem(path, "filename prefix must match finding id"))
        if finding_id in seen:
            problems.append(Problem(path, f"duplicate finding id also used by {seen[finding_id]}"))
        elif finding_id:
            seen[finding_id] = path
        scope = data.get("scope")
        if scope and scope not in owner_scopes:
            problems.append(Problem(path, f"unknown scope '{scope}'"))


def validate_readme_checkbox_parity(item: object, problems: list[Problem]) -> None:
    """When a work-item README uses the ``- [ ] **ID** ...`` checklist convention,
    every manifest criterion must have a checkbox whose state matches the manifest
    (satisfied -> [x], pending -> [ ]); waived is left flexible. Gated on the
    convention being present, so items that describe criteria in prose are
    unaffected. The manifest stays the source of truth; this only stops a README
    from silently disagreeing with it (decision 0014)."""
    readme = item.directory / "README.md"
    if not readme.is_file():
        return
    boxes = {
        match.group(2): match.group(1).strip().lower()
        for match in CHECKBOX_LINE.finditer(readme.read_text(encoding="utf-8"))
    }
    if not boxes:
        return
    for criterion in item.data.get("acceptance_criteria", []):
        criterion_id = str(criterion.get("id", ""))
        state = criterion.get("state")
        box = boxes.get(criterion_id)
        if box is None:
            problems.append(Problem(readme, f"criterion {criterion_id} has no checkbox in README"))
        elif state == "satisfied" and box != "x":
            problems.append(Problem(readme, f"criterion {criterion_id} is satisfied but its README checkbox is unchecked"))
        elif state == "pending" and box == "x":
            problems.append(Problem(readme, f"criterion {criterion_id} is pending but its README checkbox is checked"))


def _preflight_config(root: Path) -> dict:
    """Opt-in preflight settings from governance/owners.json (default: disabled).

        "preflight": {"enabled": true, "required_from_id": 10}
        "preflight": {"enabled": true, "required_from_date": "2026-06-17"}

    When enabled with neither threshold, a marker is required on every work item."""
    try:
        data = load_json(root / "governance" / "owners.json")
    except (OSError, ValueError):
        return {}
    cfg = data.get("preflight", {})
    return cfg if isinstance(cfg, dict) else {}


def _preflight_required(item, cfg: dict) -> bool:
    # Mandatory by default (decision 0021): absent/empty config means enabled. A repo
    # grandfathers its pre-2.0 items with required_from_id / required_from_date; adopt and
    # doctor set required_from_date to the adoption/upgrade date automatically.
    if not cfg.get("enabled", True):
        return False
    from_id = cfg.get("required_from_id")
    from_date = cfg.get("required_from_date")
    if from_id is None and from_date is None:
        return True
    if from_id is not None:
        try:
            if int(item.item_id) >= int(from_id):
                return True
        except (TypeError, ValueError):
            pass
    if from_date is not None:
        # Strict '>' so items created on/before the adoption/upgrade date are grandfathered;
        # only work created AFTER the epoch must carry a marker.
        created = str(item.data.get("created", ""))
        if created and created > str(from_date):
            return True
    return False


def _documentation_impact_config(root: Path) -> dict:
    """Opt-in documentation-closure settings from governance/owners.json (default: disabled).

        "documentation_impact": {"enabled": true, "required_from_id": 47}
        "documentation_impact": {"enabled": true, "required_from_date": "2026-09-02"}

    Mirrors _preflight_config (decision 0021's shape) for decision 0065 (WI047)."""
    try:
        data = load_json(root / "governance" / "owners.json")
    except (OSError, ValueError):
        return {}
    cfg = data.get("documentation_impact", {})
    return cfg if isinstance(cfg, dict) else {}


def _documentation_impact_required(item, cfg: dict) -> bool:
    if not cfg.get("enabled", False):
        return False
    from_id = cfg.get("required_from_id")
    from_date = cfg.get("required_from_date")
    if from_id is None and from_date is None:
        return True
    if from_id is not None:
        try:
            if int(item.item_id) >= int(from_id):
                return True
        except (TypeError, ValueError):
            pass
    if from_date is not None:
        created = str(item.data.get("created", ""))
        if created and created > str(from_date):
            return True
    return False


def validate_documentation_impact(item, manifest: Path, cfg: dict, evidence_ids: set[str], problems: list[Problem]) -> None:
    """Enforce documentation closure (decision 0065, WI047) at completion.

    A work item transitioning to `completed` must carry a resolved
    `documentation_impact`: either `state: affected` with named surfaces and
    linked evidence proving those surfaces were created/updated/regenerated,
    or `state: none` with a reviewable rationale. Silence does not satisfy
    closeout. Presence is required only when qualifying (governance/owners.json
    documentation_impact.enabled + the configured epoch); shape, when present,
    is always checked by the schema (check_schema), so this function only adds
    the conditional-presence and evidence-reference rules the schema cannot
    express on its own.
    """
    impact = item.data.get("documentation_impact")
    if item.status == "completed" and _documentation_impact_required(item, cfg):
        if not isinstance(impact, dict) or "state" not in impact:
            problems.append(Problem(
                manifest,
                "completed item requires a resolved documentation_impact "
                "(state 'affected' with surfaces+evidence, or 'none' with a rationale)",
            ))
            return
    if not isinstance(impact, dict):
        return
    if impact.get("state") == "affected":
        for evidence_id in impact.get("evidence", []):
            if evidence_id not in evidence_ids:
                problems.append(Problem(manifest, f"documentation_impact references unknown evidence '{evidence_id}'"))


_PROV_LEVEL = {"inferred": 0, "provisional": 1, "concrete": 2}


def _evidence_provenance(root: Path) -> dict:
    """Map evidence-run id -> provenance ('concrete' default). Decision 0021."""
    out: dict[str, str] = {}
    runs = root / "evidence" / "runs"
    if not runs.is_dir():
        return out
    for path in runs.glob("*.json"):
        try:
            data = load_json(path)
        except (OSError, ValueError):
            continue
        rid = data.get("id")
        if isinstance(rid, str):
            out[rid] = data.get("provenance", "concrete")
    return out


def validate_provenance(item, ev_prov: dict, manifest: Path, problems: list[Problem]) -> None:
    """Provenance rules (decision 0021), with the order inferred < provisional < concrete.

    P1 (admission): provisional/inferred work items are *valid* states - this is the
        trilemma escape that lets adopt be Closed and Faithful at once.
    P2 (completion requires concrete): a `completed` item must be concrete and every
        satisfied criterion must be backed only by concrete evidence (keeps INV-3 honest).
    P3 (consistency): a `concrete` item may not rest on non-concrete evidence - it must
        declare itself provisional/inferred until its evidence is ratcheted to concrete.
    """
    item_prov = item.data.get("provenance", "concrete")
    completed = item.status == "completed"
    rests_on_nonconcrete = False
    for criterion in item.data.get("acceptance_criteria", []):
        if criterion.get("state") != "satisfied":
            continue
        crit_prov = criterion.get("provenance", "concrete")
        ev_levels = [ev_prov.get(e, "concrete") for e in criterion.get("evidence", [])]
        nonconcrete = crit_prov != "concrete" or any(p != "concrete" for p in ev_levels)
        if nonconcrete:
            rests_on_nonconcrete = True
            if completed:
                problems.append(Problem(
                    manifest,
                    f"completed item criterion {criterion.get('id')} rests on non-concrete "
                    "evidence; ratchet it to concrete before completing (P2)"))
    if completed and item_prov != "concrete":
        problems.append(Problem(
            manifest, f"item provenance '{item_prov}' cannot be completed; ratchet to concrete first (P2)"))
    if item_prov == "concrete" and rests_on_nonconcrete and not completed:
        problems.append(Problem(
            manifest, "concrete item rests on non-concrete evidence; mark it provisional/inferred "
            "or ratchet the evidence (P3)"))


def validate_work_preflight(item, manifest: Path, cfg: dict, problems: list[Problem]) -> None:
    """Require a preflight marker on qualifying work items when preflight is enabled.

    The marker proves the item was recorded before implementation started. Its shape
    (created_before_work_started, created_at, note) is enforced by the work-item
    schema; this enforces only the *conditional requirement* the schema cannot
    express (required for items at/after a configured id or date). Disabled by
    default; see _preflight_config and governance/owners.json."""
    if not _preflight_required(item, cfg):
        return
    if not isinstance(item.data.get("preflight"), dict):
        problems.append(Problem(manifest, "work item requires a preflight marker (enabled via governance/owners.json preflight.enabled)"))


def validate_work(root: Path, owner_scopes: set[str], enforce_disjoint: bool, problems: list[Problem]) -> set[str]:
    try:
        items = discover_work_items(root)
        evidence_ids = discover_evidence_ids(root)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(root, str(exc)))
        return set()

    schema = load_schema(root, "work-item.schema.json")
    preflight_cfg = _preflight_config(root)
    doc_impact_cfg = _documentation_impact_config(root)
    ev_prov = _evidence_provenance(root)
    seen: dict[str, Path] = {}
    for item in items:
        manifest = item.directory / "work-item.json"
        missing = REQUIRED_WORK_FIELDS - item.data.keys()
        if missing:
            problems.append(Problem(manifest, f"missing fields: {', '.join(sorted(missing))}"))
            continue
        check_schema(item.data, schema, manifest, problems)

        expected_status = item.directory.parent.name
        if item.status != expected_status or item.status not in STATUSES:
            problems.append(Problem(manifest, f"status '{item.status}' does not match directory '{expected_status}'"))
        if not re.fullmatch(r"[0-9]{3,}", item.item_id):
            problems.append(Problem(manifest, "id must contain at least three digits"))
        elif item.item_id in seen:
            problems.append(Problem(manifest, f"duplicate id also used by {seen[item.item_id]}"))
        else:
            seen[item.item_id] = manifest

        if item.directory.name.split("-", 1)[0] != item.item_id:
            problems.append(Problem(manifest, "directory prefix must match work-item id"))
        if not (item.directory / "README.md").is_file():
            problems.append(Problem(item.directory, "missing README.md narrative"))
        if item.data["owner_scope"] not in owner_scopes:
            problems.append(Problem(manifest, f"unknown owner_scope '{item.data['owner_scope']}'"))
        for scope in item.data.get("affected_scopes", []):
            if scope not in owner_scopes:
                problems.append(Problem(manifest, f"unknown affected_scope '{scope}'"))
        validate_dates(item.data["created"], "created", manifest, problems)
        validate_dates(item.data["updated"], "updated", manifest, problems)

        criterion_ids: set[str] = set()
        for criterion in item.data.get("acceptance_criteria", []):
            criterion_id = str(criterion.get("id", ""))
            if not criterion_id or criterion_id in criterion_ids:
                problems.append(Problem(manifest, "acceptance criterion IDs must be present and unique"))
            criterion_ids.add(criterion_id)
            state = criterion.get("state")
            linked = criterion.get("evidence", [])
            if state == "satisfied" and not linked:
                problems.append(Problem(manifest, f"criterion {criterion_id} is satisfied without evidence"))
            for evidence_id in linked:
                if evidence_id not in evidence_ids:
                    problems.append(Problem(manifest, f"criterion {criterion_id} references unknown evidence '{evidence_id}'"))
            if item.status == "completed" and state == "pending":
                problems.append(Problem(manifest, f"completed item has pending criterion {criterion_id}"))

        validate_readme_checkbox_parity(item, problems)
        validate_work_preflight(item, manifest, preflight_cfg, problems)
        validate_provenance(item, ev_prov, manifest, problems)
        validate_documentation_impact(item, manifest, doc_impact_cfg, evidence_ids, problems)

    all_ids = set(seen)
    status_by_id = {item.item_id: item.status for item in items}
    for item in items:
        for dependency in item.data.get("depends_on", []):
            if dependency not in all_ids:
                problems.append(Problem(item.directory / "work-item.json", f"unknown dependency '{dependency}'"))
            elif item.status in ("active", "completed") and status_by_id.get(dependency) == "proposed":
                problems.append(Problem(
                    item.directory / "work-item.json",
                    f"{item.status} work item depends on proposed work item '{dependency}'; "
                    "proposed work is not accepted implementation authority",
                ))

    detect_dependency_cycles(items, all_ids, problems)
    if enforce_disjoint:
        validate_disjoint_scopes(items, problems)
    return all_ids


def detect_dependency_cycles(items: list, known_ids: set[str], problems: list[Problem]) -> None:
    graph: dict[str, list[str]] = {}
    location: dict[str, Path] = {}
    for item in items:
        graph[item.item_id] = [d for d in item.data.get("depends_on", []) if d in known_ids]
        location[item.item_id] = item.directory / "work-item.json"
    WHITE, GRAY, BLACK = 0, 1, 2
    color = {node: WHITE for node in graph}
    reported: set[frozenset[str]] = set()

    def visit(node: str, stack: list[str]) -> None:
        color[node] = GRAY
        stack.append(node)
        for nxt in graph.get(node, []):
            if color.get(nxt) == GRAY:
                cycle = stack[stack.index(nxt):]
                key = frozenset(cycle)
                if key not in reported:
                    reported.add(key)
                    problems.append(Problem(location[node], f"dependency cycle: {' -> '.join(cycle + [nxt])}"))
            elif color.get(nxt) == WHITE:
                visit(nxt, stack)
        stack.pop()
        color[node] = BLACK

    for node in graph:
        if color[node] == WHITE:
            visit(node, [])


def validate_disjoint_scopes(items: list, problems: list[Problem]) -> None:
    active = [item for item in items if item.status in ("active", "blocked")]
    for i, left in enumerate(active):
        left_scopes = {left.data.get("owner_scope")} | set(left.data.get("affected_scopes", []))
        for right in active[i + 1:]:
            right_scopes = {right.data.get("owner_scope")} | set(right.data.get("affected_scopes", []))
            overlap = left_scopes & right_scopes
            if overlap:
                problems.append(Problem(
                    left.directory / "work-item.json",
                    f"active scope conflict with {right.item_id} on {', '.join(sorted(map(str, overlap)))}",
                ))


def validate_evidence(root: Path, work_ids: set[str], problems: list[Problem]) -> None:
    schema = load_schema(root, "evidence-run.schema.json")
    seen: dict[str, Path] = {}
    for path in sorted((root / "evidence" / "runs").glob("*.json")):
        try:
            data = load_json(path)
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            problems.append(Problem(path, str(exc)))
            continue
        required = {"id", "timestamp", "work_item", "result", "commands", "artifacts", "environment"}
        missing = required - data.keys()
        if missing:
            problems.append(Problem(path, f"missing evidence fields: {', '.join(sorted(missing))}"))
            continue
        check_schema(data, schema, path, problems)
        evidence_id = str(data["id"])
        if evidence_id != path.stem:
            problems.append(Problem(path, "evidence id must match filename"))
        if evidence_id in seen:
            problems.append(Problem(path, f"duplicate evidence id also used by {seen[evidence_id]}"))
        seen[evidence_id] = path
        try:
            parsed_timestamp = datetime.fromisoformat(str(data["timestamp"]))
        except ValueError:
            problems.append(Problem(path, "timestamp must be ISO 8601"))
        else:
            if parsed_timestamp.tzinfo is None:
                # Naive timestamps are deliberately UTC, matching the
                # historical RepoPact interpretation and avoiding local-zone
                # dependence across exported checkouts.
                parsed_timestamp = parsed_timestamp.replace(tzinfo=timezone.utc)
            else:
                parsed_timestamp = parsed_timestamp.astimezone(timezone.utc)

            timestamp_basis = data.get("timestamp_basis")
            if timestamp_basis is not None and timestamp_basis != EVIDENCE_TIMESTAMP_BASIS:
                problems.append(
                    Problem(
                        path,
                        f"timestamp_basis must be '{EVIDENCE_TIMESTAMP_BASIS}' when present",
                    )
                )
            elif timestamp_basis == EVIDENCE_TIMESTAMP_BASIS:
                recording = _evidence_recording_commit(root, path)
                if recording is not None:
                    commit_timestamp, commit_sha = recording
                    if parsed_timestamp > commit_timestamp + FUTURE_TIMESTAMP_TOLERANCE:
                        problems.append(
                            Problem(
                                path,
                                f"timestamp {data['timestamp']} is later than its recording "
                                f"commit {commit_sha} ({commit_timestamp.isoformat()}) "
                                f"by more than {FUTURE_TIMESTAMP_TOLERANCE}; evidence "
                                "timestamps must describe execution no later than the "
                                "recording commit plus the allowed clock-skew tolerance",
                            )
                        )
        if data["work_item"] not in work_ids:
            problems.append(Problem(path, f"unknown work_item '{data['work_item']}'"))


def _evidence_recording_commit(root: Path, path: Path) -> tuple[datetime, str] | None:
    """Return the first Git commit that recorded *path*, when available.

    This is intentionally best-effort.  Exported trees, uncommitted evidence,
    and environments without usable Git metadata still receive schema and ISO
    validation, but cannot make a history-dependent claim.  The command uses
    only repository content and therefore returns the same result on every
    repeated validation of an unchanged tree.
    """
    try:
        relative = path.relative_to(root).as_posix()
    except ValueError:
        return None
    try:
        result = subprocess.run(
            [
                "git", "-C", str(root), "log", "--follow", "--diff-filter=A",
                "--format=%ct:%H", "--reverse", "--", relative,
            ],
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
            env={**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_OPTIONAL_LOCKS": "0"},
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode != 0:
        return None
    first = next((line.strip() for line in result.stdout.splitlines() if line.strip()), "")
    if not first or ":" not in first:
        return None
    seconds, commit_sha = first.split(":", 1)
    try:
        commit_timestamp = datetime.fromtimestamp(int(seconds), tz=timezone.utc)
    except (TypeError, ValueError, OverflowError, OSError):
        return None
    return commit_timestamp, commit_sha


def validate_audit_registry(root: Path, problems: list[Problem]) -> None:
    path = root / "audits" / "registry.json"
    try:
        data = load_json(path)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(Problem(path, str(exc)))
        return
    for entry in data.get("scopes", []):
        scope_path = root if entry.get("path") == "." else root / str(entry.get("path", ""))
        if not scope_path.exists():
            problems.append(Problem(path, f"audit scope does not exist: {entry.get('path')}"))
        validate_dates(entry.get("last_reviewed"), "last_reviewed", path, problems)
        validate_dates(entry.get("next_review"), "next_review", path, problems)
        try:
            last_reviewed = date.fromisoformat(str(entry.get("last_reviewed", "")))
            next_review = date.fromisoformat(str(entry.get("next_review", "")))
        except ValueError:
            continue
        if next_review < last_reviewed:
            problems.append(Problem(
                path,
                f"audit scope '{entry.get('path')}' review deadline precedes its last review",
            ))
        if next_review < date.today():
            problems.append(Problem(
                path,
                f"audit scope '{entry.get('path')}' freshness expired on {next_review.isoformat()}; "
                "re-review the scope and advance the registry dates",
            ))


def validate_dashboard(root: Path, problems: list[Problem]) -> None:
    """Reject a missing or stale committed dashboard projection.

    Source-record validation remains authoritative. If malformed sources prevent
    generation, their existing validators report the primary error and this
    secondary comparison stays silent instead of crashing validation.
    """
    path = root / "audits" / "reports" / "dashboard.md"
    if not path.is_file():
        problems.append(Problem(
            path,
            "missing generated dashboard; run `repopact dashboard --root .`",
        ))
        return
    try:
        expected = generate_dashboard.generate(root)
        actual = path.read_text(encoding="utf-8")
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError):
        return
    if actual != expected:
        problems.append(Problem(
            path,
            "generated dashboard is stale; run `repopact dashboard --root .` and commit the result",
        ))


def validate_research_records(root: Path, problems: list[Problem]) -> None:
    """Validate upstream research facts without imposing that surface on adopters."""
    research = root / "research"
    metadata = research / "metadata.json"
    upstream_record = (research / "paper.md").is_file() and (research / "protocol.md").is_file()
    if not metadata.is_file() and not upstream_record:
        return
    for problem in validate_research.validate(root):
        problems.append(Problem(problem.path, problem.message))


def _validate_records(root: Path, directory: Path, pattern: str, statuses: tuple[str, ...],
                      required: tuple[str, ...], problems: list[Problem]) -> dict[str, Path]:
    seen: dict[str, Path] = {}
    if not directory.is_dir():
        return seen
    for path in sorted(directory.glob("*.md")):
        if path.name.upper() == "README.MD":
            continue
        try:
            front = parse_file(path)
        except FrontMatterError as exc:
            problems.append(Problem(path, str(exc)))
            continue
        for field in required:
            if not str(front.get(field, "")).strip():
                problems.append(Problem(path, f"front matter missing '{field}'"))
        record_id = str(front.get("id", ""))
        if record_id and not re.fullmatch(pattern, record_id):
            problems.append(Problem(path, f"id '{record_id}' must match {pattern}"))
        if record_id and not path.name.startswith(f"{record_id}-"):
            problems.append(Problem(path, "filename prefix must match record id"))
        if record_id in seen:
            problems.append(Problem(path, f"duplicate id also used by {seen[record_id]}"))
        elif record_id:
            seen[record_id] = path
        if front.get("status") not in statuses:
            problems.append(Problem(path, f"status '{front.get('status')}' must be one of {', '.join(statuses)}"))
    return seen


def validate_decisions(root: Path, problems: list[Problem]) -> set[str]:
    directory = root / "decisions"
    ids = _validate_records(root, directory, r"[0-9]{4,}", DECISION_STATUSES,
                            ("id", "title", "status", "date"), problems)
    for path in sorted(directory.glob("*.md")) if directory.is_dir() else []:
        if path.name.upper() == "README.MD":
            continue
        try:
            front = parse_file(path)
        except FrontMatterError:
            continue
        validate_dates(front.get("date"), "date", path, problems)
        for target in front.get("supersedes", []) if isinstance(front.get("supersedes"), list) else []:
            if target not in ids:
                problems.append(Problem(path, f"supersedes unknown decision '{target}'"))
    return set(ids)


def validate_policies(root: Path, problems: list[Problem]) -> set[str]:
    ids = _validate_records(root, root / "governance" / "policies", r"[0-9]{3,}", POLICY_STATUSES,
                            ("id", "title", "status", "applies_to"), problems)
    return set(ids)


def _collect_invariant_ids(root: Path) -> set[str]:
    try:
        data = load_json(root / "governance" / "invariants.json")
    except (OSError, ValueError, json.JSONDecodeError):
        return set()
    return {str(entry.get("id", "")) for entry in data.get("invariants", []) if entry.get("id")}


# --- assurance/control mapping (WI051, Decision 0054) ----------------------

_ASSURANCE_APPLICABILITY_NEEDING_RATIONALE = {"applicable", "not_applicable", "conditional", "partial"}
_ASSURANCE_PATH_IMPLEMENTATION_KINDS = {"source", "configuration", "workflow", "test", "runtime_surface"}


def _resolve_repo_relative(root: Path, relative: str) -> Path | None:
    """Resolve *relative* against *root*, returning it only if provably
    contained within *root*. Mirrors the Rust ``resolve_within_root``/Python
    ``validate_research._resolve_local`` containment idiom (WI059): an
    absolute host path or an escaping ``../`` reference is never accepted,
    and the target's content is never read to make this decision."""
    if not relative or Path(relative).is_absolute():
        return None
    try:
        resolved = (root / relative).resolve()
        resolved.relative_to(root.resolve())
    except (OSError, ValueError):
        return None
    return resolved


def _validate_evidence_ref(
    ref: dict, path: Path, root: Path, evidence_ids: set[str], problems: list[Problem]
) -> None:
    kind = ref.get("kind")
    if kind == "evidence_run":
        evidence_run_id = str(ref.get("evidence_run_id", ""))
        if evidence_run_id not in evidence_ids:
            problems.append(Problem(path, f"assurance mapping references unknown evidence run '{evidence_run_id}'"))
    elif kind == "repository_artifact":
        relative = str(ref.get("path", ""))
        resolved = _resolve_repo_relative(root, relative)
        if resolved is None:
            problems.append(Problem(path, f"assurance evidence artifact path escapes the repository: {relative}"))
        elif not resolved.is_file():
            problems.append(Problem(path, f"assurance evidence artifact does not exist: {relative}"))


def validate_assurance_mappings(
    root: Path,
    decision_ids: set[str],
    policy_ids: set[str],
    invariant_ids: set[str],
    work_ids: set[str],
    problems: list[Problem],
) -> None:
    """Validate optional assurance/control mapping records (WI051).

    A repository with no ``assurance/mappings`` directory is fully valid and
    produces no problems. RepoPact validates mapping shape and that its
    canonical references resolve; it never derives compliance, certification,
    or audit conclusions from a mapping's presence (Decision 0054)."""
    directory = root / "assurance" / "mappings"
    if not directory.is_dir():
        return
    schema = load_schema(root, "assurance-mapping.schema.json")
    evidence_ids = discover_evidence_ids(root)
    seen: dict[str, Path] = {}
    for path in sorted(directory.glob("*.json")):
        try:
            data = load_json(path)
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            problems.append(Problem(path, str(exc)))
            continue
        check_schema(data, schema, path, problems)

        mapping_id = str(data.get("id", ""))
        if mapping_id != path.stem:
            problems.append(Problem(path, "assurance mapping id must match filename"))
        if mapping_id in seen:
            problems.append(Problem(path, f"duplicate assurance mapping id also used by {seen[mapping_id]}"))
        seen[mapping_id] = path

        applicability = data.get("applicability", {})
        if isinstance(applicability, dict) and applicability.get("status") in _ASSURANCE_APPLICABILITY_NEEDING_RATIONALE:
            if not str(applicability.get("rationale", "")).strip():
                problems.append(Problem(path, f"applicability '{applicability.get('status')}' requires a rationale"))
            if not str(applicability.get("determined_by", "")).strip():
                problems.append(Problem(path, f"applicability '{applicability.get('status')}' requires determined_by"))

        for control_ref in data.get("control_refs", []) if isinstance(data.get("control_refs"), list) else []:
            if not isinstance(control_ref, dict):
                continue
            kind = control_ref.get("kind")
            ref = str(control_ref.get("ref", ""))
            if kind == "decision" and ref not in decision_ids:
                problems.append(Problem(path, f"assurance mapping references unknown decision '{ref}'"))
            elif kind == "policy" and ref not in policy_ids:
                problems.append(Problem(path, f"assurance mapping references unknown policy '{ref}'"))
            elif kind == "invariant" and ref not in invariant_ids:
                problems.append(Problem(path, f"assurance mapping references unknown invariant '{ref}'"))
            elif kind == "work_item" and ref not in work_ids:
                problems.append(Problem(path, f"assurance mapping references unknown work item '{ref}'"))
            elif kind == "contract":
                resolved = _resolve_repo_relative(root, ref)
                if resolved is None:
                    problems.append(Problem(path, f"assurance mapping contract reference escapes the repository: {ref}"))
                elif not resolved.is_file():
                    problems.append(Problem(path, f"assurance mapping references unknown contract '{ref}'"))

        for impl_ref in data.get("implementation_refs", []) if isinstance(data.get("implementation_refs"), list) else []:
            if not isinstance(impl_ref, dict):
                continue
            kind = impl_ref.get("kind")
            ref = str(impl_ref.get("ref", ""))
            if kind == "decision" and ref not in decision_ids:
                problems.append(Problem(path, f"assurance mapping references unknown decision '{ref}'"))
            elif kind == "work_item" and ref not in work_ids:
                problems.append(Problem(path, f"assurance mapping references unknown work item '{ref}'"))
            elif kind in _ASSURANCE_PATH_IMPLEMENTATION_KINDS:
                resolved = _resolve_repo_relative(root, ref)
                if resolved is None:
                    problems.append(Problem(path, f"assurance implementation reference escapes the repository: {ref}"))
                elif not resolved.is_file():
                    problems.append(Problem(path, f"assurance implementation reference does not exist: {ref}"))

        for evidence_ref in data.get("evidence_refs", []) if isinstance(data.get("evidence_refs"), list) else []:
            if isinstance(evidence_ref, dict):
                _validate_evidence_ref(evidence_ref, path, root, evidence_ids, problems)

        for dependency in data.get("third_party_dependencies", []) if isinstance(data.get("third_party_dependencies"), list) else []:
            if not isinstance(dependency, dict):
                continue
            for evidence_ref in dependency.get("evidence_refs", []) if isinstance(dependency.get("evidence_refs"), list) else []:
                if isinstance(evidence_ref, dict):
                    _validate_evidence_ref(evidence_ref, path, root, evidence_ids, problems)


# --- assurance sensitive-evidence guardrails (WI051 phase 2, Decision 0055) -
#
# Bounded, false-positive-prone heuristics over content an assurance mapping
# already explicitly references -- never a whole-repository scan. Diagnostic
# wording names the pattern observed, never a legal/compliance conclusion
# (Decision 0055 section 2). This is the Python regression-comparator mirror
# of ``repopact-validation::assurance``; Rust is canonical.

MAX_ASSURANCE_ARTIFACT_BYTES = 262_144

_PRIVATE_KEY_MARKERS = (
    "-----BEGIN PRIVATE KEY-----",
    "-----BEGIN RSA PRIVATE KEY-----",
    "-----BEGIN OPENSSH PRIVATE KEY-----",
)
_SECRET_ASSIGNMENT_LABELS = ("password", "secret", "api_key", "access_token")
_IDENTITY_LABELS = (
    "patient_id", "medical_record_number", "mrn", "passport_number",
    "social_security_number", "ssn", "bank_account_number", "routing_number",
)
_SECRET_QUERY_PARAMS = ("token", "api_key", "password", "secret")
_STRONG_CLAIM_PHRASES = ("certified", "fully compliant", "complies with", "meets all requirements")
_NEGATION_CUES = ("not ", "n't ", "never ", "no longer ", "does not assert", "isn't", "aren't")
_NEGATION_WINDOW = 40


def _has_unnegated_strong_claim(lower_text: str) -> bool:
    """True if a strong-claim phrase appears without a nearby negation cue.

    A plain substring search flags disclaimers ("this does not assert X is
    certified") the same as an actual claim ("X is certified"). Requiring no
    negation cue in the preceding window keeps the heuristic advisory without
    firing on text that explicitly disclaims the very claim it mentions.
    """
    for phrase in _STRONG_CLAIM_PHRASES:
        start = 0
        while True:
            index = lower_text.find(phrase, start)
            if index == -1:
                break
            window = lower_text[max(0, index - _NEGATION_WINDOW):index]
            if not any(cue in window for cue in _NEGATION_CUES):
                return True
            start = index + len(phrase)
    return False


def _decode_text(raw: bytes) -> str | None:
    if b"\x00" in raw:
        return None
    try:
        return raw.decode("utf-8")
    except UnicodeDecodeError:
        return None


def _detect_private_key_marker(text: str) -> str | None:
    for marker in _PRIVATE_KEY_MARKERS:
        if marker in text:
            return marker
    return None


def _find_label_assignment(lower_text: str, label: str) -> bool:
    """True when *label* appears on a word boundary immediately followed by
    optional whitespace and ``=``/``:`` and then a non-blank value."""
    search_from = 0
    while True:
        offset = lower_text.find(label, search_from)
        if offset == -1:
            return False
        before = lower_text[offset - 1] if offset > 0 else ""
        boundary_ok = not (before.isalnum() or before == "_")
        after = lower_text[offset + len(label):]
        trimmed = after.lstrip(" \t")
        has_assignment = trimmed[:1] in ("=", ":")
        if boundary_ok and has_assignment:
            value = trimmed.lstrip(":=").lstrip(" ")
            if value and value[0] not in ("\n", "\r"):
                return True
        search_from = offset + len(label)
        if search_from >= len(lower_text):
            return False


def _detect_secret_assignment(text: str) -> bool:
    lower = text.lower()
    return any(_find_label_assignment(lower, label) for label in _SECRET_ASSIGNMENT_LABELS)


def _detect_identity_markers(text: str) -> bool:
    lower = text.lower()
    return any(_find_label_assignment(lower, label) for label in _IDENTITY_LABELS)


def _luhn_valid(digits: list[int]) -> bool:
    total = 0
    double = False
    for digit in reversed(digits):
        value = digit
        if double:
            value *= 2
            if value > 9:
                value -= 9
        total += value
        double = not double
    return total % 10 == 0


def _detect_luhn_valid_pan(text: str) -> bool:
    """Bounded scan for a 13-19 digit, Luhn-valid sequence (spaces/dashes
    tolerated as separators). A shape heuristic, never proof of cardholder
    data or PCI scope."""
    length = len(text)
    index = 0
    while index < length:
        if text[index].isdigit():
            start = index
            digits: list[int] = []
            cursor = index
            while cursor < length:
                ch = text[cursor]
                if ch.isdigit():
                    digits.append(int(ch))
                elif ch in (" ", "-"):
                    pass
                else:
                    break
                cursor += 1
                if len(digits) > 19:
                    break
            if 13 <= len(digits) <= 19 and _luhn_valid(digits):
                return True
            index = cursor if cursor > start else start + 1
        else:
            index += 1
    return False


def _has_credential_in_uri(uri: str) -> bool:
    if "://" not in uri:
        return False
    after_scheme = uri.split("://", 1)[1]
    authority = re.split(r"[/?#]", after_scheme, maxsplit=1)[0]
    if "@" not in authority:
        return False
    userinfo = authority.rsplit("@", 1)[0]
    return bool(userinfo) and ":" in userinfo


def _has_secret_query_param(uri: str) -> bool:
    if "?" not in uri:
        return False
    query = uri.split("?", 1)[1].split("#", 1)[0]
    for pair in query.split("&"):
        key, _, value = pair.partition("=")
        if key.lower() in _SECRET_QUERY_PARAMS and value:
            return True
    return False


def _scan_repository_artifact(resolved: Path, mapping_id: str, path: Path, problems: list[Problem]) -> None:
    try:
        size = resolved.stat().st_size
    except OSError:
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' evidence artifact could not be read for sensitive-evidence inspection",
            "info",
        ))
        return
    if size > MAX_ASSURANCE_ARTIFACT_BYTES:
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' evidence artifact exceeds the {MAX_ASSURANCE_ARTIFACT_BYTES}-byte "
            f"bounded inspection limit ({size} bytes); not inspected for sensitive-evidence indicators",
            "info",
        ))
        return
    try:
        raw = resolved.read_bytes()
    except OSError:
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' evidence artifact could not be read for sensitive-evidence inspection",
            "info",
        ))
        return
    text = _decode_text(raw)
    if text is None:
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' evidence artifact is binary or not valid UTF-8; "
            "not inspected for sensitive-evidence indicators",
            "info",
        ))
        return
    marker = _detect_private_key_marker(text)
    if marker:
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' evidence artifact contains a private-key marker ('{marker}'); "
            "private-key material must never be committed as evidence",
            "error",
        ))
    if _detect_secret_assignment(text):
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' evidence artifact contains a secret-assignment-shaped pattern "
            "(e.g. password=/secret=/api_key=/access_token=); potential secret/credential material, "
            "not a confirmed classification",
            "warning",
        ))
    if _detect_luhn_valid_pan(text):
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' evidence artifact contains a Luhn-valid, 13-19 digit sequence shaped "
            "like a primary account number; potential cardholder-data-shaped evidence, not a confirmed "
            "PCI scope determination",
            "warning",
        ))
    if _detect_identity_markers(text):
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' evidence artifact contains a high-signal health/identity/financial "
            "label (e.g. patient_id/ssn/passport_number/bank_account_number); potential restricted "
            "health/identity evidence, not a confirmed classification",
            "warning",
        ))


def _scan_external_reference(reference: str, mapping_id: str, path: Path, problems: list[Problem]) -> None:
    if _has_credential_in_uri(reference):
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' external evidence reference embeds userinfo credentials in its URI; "
            "use a credential-free reference",
            "error",
        ))
    if _has_secret_query_param(reference):
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' external evidence reference has a query parameter shaped like a "
            "secret/token/credential; verify it does not leak one",
            "warning",
        ))


def _scan_evidence_ref_for_hazards(
    root: Path, evidence_ref: object, mapping_id: str, path: Path, problems: list[Problem]
) -> None:
    if not isinstance(evidence_ref, dict):
        return
    kind = evidence_ref.get("kind")
    sensitivity = evidence_ref.get("sensitivity")
    if kind == "repository_artifact":
        if sensitivity == "restricted":
            problems.append(Problem(
                path,
                f"mapping '{mapping_id}' declares a 'restricted' evidence artifact stored directly as a "
                "repository file; restricted evidence should normally be represented by a hash, controlled "
                "external reference, redacted artifact, synthetic artifact, or bounded metadata instead of "
                "a raw repository artifact",
                "warning",
            ))
        relative = str(evidence_ref.get("path", ""))
        resolved = _resolve_repo_relative(root, relative)
        if resolved is not None:
            _scan_repository_artifact(resolved, mapping_id, path, problems)
    elif kind == "external_reference":
        external = evidence_ref.get("external")
        if isinstance(external, dict):
            reference = external.get("reference")
            if isinstance(reference, str):
                _scan_external_reference(reference, mapping_id, path, problems)


def validate_sensitive_evidence(root: Path, problems: list[Problem]) -> None:
    """Bounded sensitive-evidence guardrail diagnostics (ACM-004, Decision
    0055). Scoped only to evidence content an assurance mapping already
    explicitly references; never a whole-repository scan."""
    directory = root / "assurance" / "mappings"
    if not directory.is_dir():
        return
    for path in sorted(directory.glob("*.json")):
        try:
            data = load_json(path)
        except (OSError, ValueError, json.JSONDecodeError):
            continue
        mapping_id = str(data.get("id", ""))
        for evidence_ref in data.get("evidence_refs", []) if isinstance(data.get("evidence_refs"), list) else []:
            _scan_evidence_ref_for_hazards(root, evidence_ref, mapping_id, path, problems)
        for dependency in data.get("third_party_dependencies", []) if isinstance(data.get("third_party_dependencies"), list) else []:
            if not isinstance(dependency, dict):
                continue
            for evidence_ref in dependency.get("evidence_refs", []) if isinstance(dependency.get("evidence_refs"), list) else []:
                _scan_evidence_ref_for_hazards(root, evidence_ref, mapping_id, path, problems)


# --- assurance review snapshot, drift, and claim safety (ACM-005) ----------


def _find_markdown_record_path(directory: Path, record_id: str) -> Path | None:
    if not directory.is_dir():
        return None
    for path in sorted(directory.glob("*.md")):
        if path.name.upper() == "README.MD":
            continue
        try:
            front = parse_file(path)
        except FrontMatterError:
            continue
        if str(front.get("id", "")) == record_id:
            return path
    return None


def _find_work_item_path(root: Path, item_id: str) -> Path | None:
    for status in STATUSES:
        status_dir = root / "work" / status
        if not status_dir.is_dir():
            continue
        for manifest in sorted(status_dir.glob("*/work-item.json")):
            try:
                data = load_json(manifest)
            except (OSError, ValueError, json.JSONDecodeError):
                continue
            if str(data.get("id", "")) == item_id:
                return manifest
    return None


def _find_evidence_path(root: Path, evidence_id: str) -> Path | None:
    directory = root / "evidence" / "runs"
    if not directory.is_dir():
        return None
    for path in sorted(directory.glob("*.json")):
        try:
            data = load_json(path)
        except (OSError, ValueError, json.JSONDecodeError):
            continue
        if str(data.get("id", "")) == evidence_id:
            return path
    return None


def _digest_file(path: Path) -> str:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return "absent"


def _digest_repo_relative(root: Path, relative: str) -> str:
    resolved = _resolve_repo_relative(root, relative)
    if resolved is None:
        return "unresolvable"
    if not resolved.exists():
        return "absent"
    if not resolved.is_file():
        return "unresolvable"
    return _digest_file(resolved)


def _canonical_review_projection(mapping: dict) -> dict:
    """The mapping's canonical review projection (Decision 0055 section 5):
    the full record with ``review.snapshot``, ``created``, and ``updated``
    excluded. Everything else, including ``notes``, participates."""
    projection = json.loads(json.dumps(mapping))
    projection.pop("created", None)
    projection.pop("updated", None)
    review = projection.get("review")
    if isinstance(review, dict):
        review.pop("snapshot", None)
    return projection


def _canonical_bytes(value: object) -> bytes:
    """Deterministic, recursively key-sorted, compact JSON bytes.
    ``json.dumps(..., sort_keys=True)`` already sorts nested dict keys."""
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")


def _recompute_reference_digest(root: Path, category: str, kind: str, reference: str) -> str:
    if category == "control":
        if kind == "policy":
            found = _find_markdown_record_path(root / "governance" / "policies", reference)
            return _digest_file(found) if found else "absent"
        if kind == "decision":
            found = _find_markdown_record_path(root / "decisions", reference)
            return _digest_file(found) if found else "absent"
        if kind == "work_item":
            found = _find_work_item_path(root, reference)
            return _digest_file(found) if found else "absent"
        if kind == "invariant":
            return _digest_file(root / "governance" / "invariants.json")
        if kind == "contract":
            return _digest_repo_relative(root, reference)
        return "unresolvable"
    if category == "implementation":
        if kind == "decision":
            found = _find_markdown_record_path(root / "decisions", reference)
            return _digest_file(found) if found else "absent"
        if kind == "work_item":
            found = _find_work_item_path(root, reference)
            return _digest_file(found) if found else "absent"
        return _digest_repo_relative(root, reference)
    if category == "evidence":
        if kind == "evidence_run":
            found = _find_evidence_path(root, reference)
            return _digest_file(found) if found else "absent"
        return _digest_repo_relative(root, reference)
    if category == "documentation":
        return _digest_repo_relative(root, reference)
    return "unresolvable"


def compute_review_snapshot(root: Path, mapping_id: str) -> dict:
    """Compute the deterministic, read-only review snapshot for
    *mapping_id* (Decision 0055 section 5/9). Never mutates the mapping;
    mirrors ``repopact-validation::compute_review_snapshot`` (Rust
    canonical). Raises ``ValueError`` if the mapping does not exist."""
    path = root / "assurance" / "mappings" / f"{mapping_id}.json"
    if not path.is_file():
        raise ValueError(f"no assurance mapping with id '{mapping_id}'")
    data = load_json(path)
    projection = _canonical_review_projection(data)
    mapping_digest = hashlib.sha256(_canonical_bytes(projection)).hexdigest()

    references: list[dict] = []
    for control_ref in data.get("control_refs", []) if isinstance(data.get("control_refs"), list) else []:
        if not isinstance(control_ref, dict):
            continue
        kind = control_ref.get("kind")
        reference = control_ref.get("ref")
        if not kind or not reference:
            continue
        digest = _recompute_reference_digest(root, "control", kind, reference)
        references.append({"category": "control", "kind": kind, "ref": reference, "digest": digest})
    for impl_ref in data.get("implementation_refs", []) if isinstance(data.get("implementation_refs"), list) else []:
        if not isinstance(impl_ref, dict):
            continue
        kind = impl_ref.get("kind")
        reference = impl_ref.get("ref")
        if not kind or not reference:
            continue
        digest = _recompute_reference_digest(root, "implementation", kind, reference)
        references.append({"category": "implementation", "kind": kind, "ref": reference, "digest": digest})
    for evidence_ref in data.get("evidence_refs", []) if isinstance(data.get("evidence_refs"), list) else []:
        if not isinstance(evidence_ref, dict):
            continue
        kind = evidence_ref.get("kind")
        if kind == "evidence_run":
            reference = evidence_ref.get("evidence_run_id")
        elif kind == "repository_artifact":
            reference = evidence_ref.get("path")
        else:
            continue
        if not reference:
            continue
        digest = _recompute_reference_digest(root, "evidence", kind, reference)
        references.append({"category": "evidence", "kind": kind, "ref": reference, "digest": digest})
    for doc_ref in data.get("documentation_refs", []) if isinstance(data.get("documentation_refs"), list) else []:
        if not isinstance(doc_ref, dict):
            continue
        reference = doc_ref.get("path")
        if not reference:
            continue
        digest = _digest_repo_relative(root, reference)
        references.append({"category": "documentation", "kind": "documentation", "ref": reference, "digest": digest})

    return {"mapping_digest": mapping_digest, "references": references}


def _check_framework_version_stale(data: dict, mapping_id: str, path: Path, problems: list[Problem]) -> None:
    framework = data.get("framework")
    review = data.get("review")
    if not isinstance(framework, dict) or not isinstance(review, dict):
        return
    current = framework.get("version")
    reviewed = review.get("framework_version_reviewed")
    if isinstance(current, str) and isinstance(reviewed, str) and current != reviewed:
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' framework version '{current}' differs from the reviewed version "
            f"'{reviewed}'; the review has not accounted for this framework revision",
            "warning",
        ))


def _check_review_due(data: dict, mapping_id: str, path: Path, today_value: str, problems: list[Problem]) -> None:
    review = data.get("review")
    if not isinstance(review, dict):
        return
    due_at = review.get("review_due_at")
    reviewed_at = review.get("reviewed_at")
    interval_days = review.get("review_interval_days")
    due_date: str | None = None
    if isinstance(due_at, str) and len(due_at) >= 10:
        due_date = due_at[:10]
    elif isinstance(reviewed_at, str) and len(reviewed_at) >= 10 and isinstance(interval_days, int):
        try:
            base = date.fromisoformat(reviewed_at[:10])
            due_date = (base + timedelta(days=interval_days)).isoformat()
        except ValueError:
            due_date = None
    if due_date is None:
        return
    if due_date < today_value:
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' control review is overdue (due {due_date}, today {today_value})",
            "warning",
        ))


def _check_review_drift(root: Path, data: dict, mapping_id: str, snapshot: dict, path: Path, problems: list[Problem]) -> None:
    stored_digest = snapshot.get("mapping_digest")
    projection = _canonical_review_projection(data)
    current_digest = hashlib.sha256(_canonical_bytes(projection)).hexdigest()
    if isinstance(stored_digest, str) and stored_digest != current_digest:
        problems.append(Problem(
            path,
            f"mapping '{mapping_id}' has changed since its last review snapshot (reviewed digest "
            f"{stored_digest}, current digest {current_digest}); re-review and take a fresh snapshot",
            "warning",
        ))
    for entry in snapshot.get("references", []) if isinstance(snapshot.get("references"), list) else []:
        if not isinstance(entry, dict):
            continue
        category = str(entry.get("category", ""))
        kind = str(entry.get("kind", ""))
        reference = str(entry.get("ref", ""))
        stored = str(entry.get("digest", ""))
        if stored == "unresolvable":
            continue
        current = _recompute_reference_digest(root, category, kind, reference)
        if current in ("absent", "unresolvable"):
            if stored != current:
                problems.append(Problem(
                    path,
                    f"mapping '{mapping_id}' {category} reference '{reference}' ({kind}) was reviewed "
                    f"but is now {current}",
                    "warning",
                ))
        elif current != stored:
            problems.append(Problem(
                path,
                f"mapping '{mapping_id}' {category} reference '{reference}' ({kind}) has changed since "
                f"review (reviewed digest {stored}, current digest {current})",
                "warning",
            ))


def _non_empty_array(data: dict, key: str) -> bool:
    value = data.get(key)
    return isinstance(value, list) and len(value) > 0


def _check_documentation_claims(root: Path, data: dict, mapping_id: str, path: Path, problems: list[Problem]) -> None:
    has_control = _non_empty_array(data, "control_refs")
    has_impl = _non_empty_array(data, "implementation_refs")
    has_evidence = _non_empty_array(data, "evidence_refs")
    has_attestation = isinstance(data.get("attestation"), dict)
    has_gap = _non_empty_array(data, "gaps")
    basis_support = {
        "mapping": True,
        "control_reference": has_control,
        "implementation_reference": has_impl,
        "evidence_reference": has_evidence,
        "external_attestation_reference": has_attestation,
    }
    for doc_ref in data.get("documentation_refs", []) if isinstance(data.get("documentation_refs"), list) else []:
        if not isinstance(doc_ref, dict):
            continue
        claim_basis = doc_ref.get("claim_basis")
        doc_path = str(doc_ref.get("path", ""))
        supported = basis_support.get(claim_basis, True)
        if not supported:
            problems.append(Problem(
                path,
                f"mapping '{mapping_id}' documentation '{doc_path}' declares claim_basis '{claim_basis}' "
                "but the mapping has no corresponding data to support it",
                "error",
            ))
            continue
        resolved = _resolve_repo_relative(root, doc_path)
        if resolved is None or not resolved.is_file():
            continue
        try:
            size = resolved.stat().st_size
        except OSError:
            continue
        if size > MAX_ASSURANCE_ARTIFACT_BYTES:
            continue
        try:
            raw = resolved.read_bytes()
        except OSError:
            continue
        text = _decode_text(raw)
        if text is None:
            continue
        lower = text.lower()
        strong_claim = _has_unnegated_strong_claim(lower)
        weak_support = has_gap or (not has_evidence and not has_attestation)
        if strong_claim and weak_support:
            problems.append(Problem(
                path,
                f"mapping '{mapping_id}' documentation '{doc_path}' may claim stronger assurance than "
                "this record's evidence/attestation/gap state supports; this is an advisory heuristic "
                "over the declared documentation file only, not a legal or classification determination",
                "warning",
            ))


def validate_review_and_claims(root: Path, problems: list[Problem], today: str | None = None) -> None:
    """Review freshness, drift, and documentation claim-basis diagnostics
    (ACM-005, Decision 0055). All date comparisons accept an injectable
    *today* (``YYYY-MM-DD``) so tests never depend on the wall clock."""
    directory = root / "assurance" / "mappings"
    if not directory.is_dir():
        return
    today_value = today or date.today().isoformat()
    for path in sorted(directory.glob("*.json")):
        try:
            data = load_json(path)
        except (OSError, ValueError, json.JSONDecodeError):
            continue
        mapping_id = str(data.get("id", ""))
        _check_framework_version_stale(data, mapping_id, path, problems)
        _check_review_due(data, mapping_id, path, today_value, problems)
        review = data.get("review")
        if isinstance(review, dict) and isinstance(review.get("snapshot"), dict):
            _check_review_drift(root, data, mapping_id, review["snapshot"], path, problems)
        _check_documentation_claims(root, data, mapping_id, path, problems)


def validate_orphan_work_dirs(root: Path, problems: list[Problem]) -> None:
    """Fail when a directory under work/ holds planning content but no work-item.json.

    Work items are discovered only at ``work/<status>/<name>/work-item.json``
    (see repo_model.discover_work_items). A directory that carries a README.md,
    AGENTS.md, or an _audit/ companion but no manifest is therefore invisible to
    the validator, the dashboard, and dependency/scope checks while still holding
    load-bearing planning state. That silently breaks INV-1, so it is a hard
    error: record it with ``repopact new work-item`` / ``repopact import-plan``,
    or move it out of work/.
    """
    work = root / "work"
    if not work.is_dir():
        return
    candidates: list[Path] = []
    for child in sorted(work.iterdir()):
        if not child.is_dir() or child.name.startswith((".", "_")):
            continue
        if child.name in STATUSES:
            # Each subdirectory of a status container should be a tracked item.
            candidates.extend(
                sub for sub in sorted(child.iterdir())
                if sub.is_dir() and not sub.name.startswith((".", "_"))
            )
            continue
        candidates.append(child)
    for directory in candidates:
        if (directory / "work-item.json").is_file():
            continue
        has_planning = (
            (directory / "README.md").is_file()
            or (directory / "AGENTS.md").is_file()
            or (directory / "_audit").is_dir()
        )
        if has_planning:
            problems.append(Problem(
                directory,
                "work directory holds planning content (README/AGENTS/_audit) but no "
                "work-item.json; it is invisible to the ledger, validator, and dashboard "
                "(record it with `repopact new work-item` or `repopact import-plan`, or "
                "move it out of work/)",
            ))


def validate_admission_records(root: Path, problems: list[Problem]) -> None:
    """Additive validation for the opt-in WI050 public and audit records."""
    from pathlib import Path as _Path
    public = {
        "admission-policy.json": "admission-policy.schema.json",
        "operator-authority.json": "operator-authority.schema.json",
        "repository-registration.json": "repository-registration.schema.json",
    }
    gov = root / "governance"
    loaded: dict[str, dict] = {}
    for name, schema_name in public.items():
        path = gov / name
        if not path.is_file():
            continue
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            problems.append(Problem(path, f"admission record is not valid JSON: {exc}")); continue
        loaded[name] = value
        check_schema(value, load_schema(root, schema_name), path, problems)
    policy, authority, registration = loaded.get("admission-policy.json"), loaded.get("operator-authority.json"), loaded.get("repository-registration.json")
    if policy and policy.get("enabled"):
        default = policy.get("default_profile")
        if default and default not in policy.get("profiles", {}):
            problems.append(Problem(gov / "admission-policy.json", "default_profile must name a declared profile"))
        if authority:
            missing = set(policy.get("profiles", {})) - set(authority.get("profiles", {}))
            if missing: problems.append(Problem(gov / "operator-authority.json", f"missing authority profiles: {', '.join(sorted(missing))}"))
    if authority:
        classes = set(authority.get("approval_classes", []))
        for profile, cfg in authority.get("profiles", {}).items():
            unknown = set(cfg.get("approval_classes", [])) - classes
            if unknown: problems.append(Problem(gov / "operator-authority.json", f"profile {profile} references unknown approval classes: {', '.join(sorted(unknown))}"))
    for directory, schema_name in ((root / "evidence" / "admission" / "requests", "authorization-request.schema.json"), (root / "evidence" / "admission" / "receipts", "authorization-receipt.schema.json"), (root / "governance" / "adapters", "adapter-capabilities.schema.json")):
        if not directory.is_dir(): continue
        for path in sorted(directory.glob("*.json")):
            try: value = json.loads(path.read_text(encoding="utf-8"))
            except (OSError, ValueError, json.JSONDecodeError) as exc:
                problems.append(Problem(path, f"admission record is not valid JSON: {exc}")); continue
            check_schema(value, load_schema(root, schema_name), path, problems)


def validate(root: Path) -> list[Problem]:
    problems: list[Problem] = []
    validate_version(root, problems)
    validate_release_label(root, problems)
    validate_package_version(root, problems)
    validate_source_artifact_identity(root, problems)
    validate_release_surface(root, problems)
    validate_contracts(root, problems)
    validate_invariants(root, problems)
    validate_frozen_surface(root, problems)
    validate_adopter_manifest(root, problems)
    owner_scopes, enforce_disjoint = validate_owners(root, problems)
    validate_findings(root, owner_scopes, problems)
    work_ids = validate_work(root, owner_scopes, enforce_disjoint, problems)
    validate_orphan_work_dirs(root, problems)
    validate_evidence(root, work_ids, problems)
    validate_audit_registry(root, problems)
    decision_ids = validate_decisions(root, problems)
    policy_ids = validate_policies(root, problems)
    invariant_ids = _collect_invariant_ids(root)
    validate_assurance_mappings(root, decision_ids, policy_ids, invariant_ids, work_ids, problems)
    validate_sensitive_evidence(root, problems)
    validate_review_and_claims(root, problems)
    validate_dashboard(root, problems)
    validate_research_records(root, problems)
    validate_admission_records(root, problems)
    return sorted(problems, key=lambda problem: (str(problem.path), problem.message))


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate repository governance records")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    root = args.root.resolve()
    problems = validate(root)
    for problem in problems:
        print(f"{problem.severity.upper()} {problem.path.relative_to(root)}: {problem.message}")
    blocking = blocking_problems(problems)
    if blocking:
        print(f"\nValidation failed with {len(blocking)} error(s).")
        return 1
    print("Repository governance validation passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
