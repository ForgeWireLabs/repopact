"""Run the published RepoPact conformance suite.

The runner materializes each fixture as an isolated temporary repository, injects
the canonical schemas from this checkout, runs a RepoPact implementation, and
compares the result with conformance/manifest.json.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

import jsonschema

import generate_dashboard
import validate_repo


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "conformance" / "manifest.json"


@dataclass(frozen=True)
class CaseResult:
    case_id: str
    passed: bool
    detail: str


def load_manifest(path: Path = MANIFEST) -> dict:
    with path.open(encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise ValueError(f"manifest must be a JSON object: {path}")
    schema = json.loads((ROOT / "schemas" / "conformance-manifest.schema.json").read_text(encoding="utf-8"))
    jsonschema.Draft202012Validator(schema).validate(data)
    validate_manifest_coverage(data)
    return data


def validate_manifest_coverage(manifest: dict) -> None:
    """Enforce bidirectional rule/case coverage with deterministic diagnostics."""
    errors: list[str] = []
    rule_ids = [str(rule.get("id", "")) for rule in manifest.get("rules", []) if isinstance(rule, dict)]
    case_ids = [str(case.get("id", "")) for case in manifest.get("cases", []) if isinstance(case, dict)]
    if len(rule_ids) != len(set(rule_ids)):
        errors.append("rule ids must be unique")
    if len(case_ids) != len(set(case_ids)):
        errors.append("case ids must be unique")
    known = set(rule_ids)
    referenced = {
        str(rule_id)
        for case in manifest.get("cases", []) if isinstance(case, dict)
        for rule_id in case.get("rules", [])
    }
    uncovered = sorted(known - referenced)
    unknown = sorted(referenced - known)
    if uncovered:
        errors.append(f"rules without conformance cases: {', '.join(uncovered)}")
    if unknown:
        errors.append(f"cases reference unknown rules: {', '.join(unknown)}")
    if errors:
        raise ValueError("; ".join(errors))


def _copy_overlay(src: Path, dst: Path) -> None:
    for path in src.rglob("*"):
        if path.is_dir() or path.name == "meta.json":
            continue
        target = dst / path.relative_to(src)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, target)


def materialize_case(root: Path, fixtures_root: Path, case: dict) -> Path:
    repo = root / "repo"
    overlay_on = case.get("overlay_on")
    if overlay_on:
        shutil.copytree(fixtures_root / str(overlay_on), repo)
        _copy_overlay(fixtures_root / str(case["path"]), repo)
    else:
        shutil.copytree(fixtures_root / str(case["path"]), repo)
    shutil.copytree(ROOT / "schemas", repo / "schemas")
    # Overlays intentionally mutate source records. Materialize their canonical
    # projection unless the dashboard itself is the isolated signal under test.
    dashboard_mode = case.get("dashboard", "canonical")
    if dashboard_mode == "canonical":
        generate_dashboard.write_dashboard(repo)
    elif dashboard_mode == "remove":
        (repo / "audits" / "reports" / "dashboard.md").unlink(missing_ok=True)
    return repo


def run_command(command: str, repo: Path) -> subprocess.CompletedProcess[str]:
    rendered = command.format(repo=str(repo))
    return subprocess.run(rendered, shell=True, text=True, capture_output=True)


def evaluate_case(case: dict, command: str, fixtures_root: Path) -> CaseResult:
    case_id = str(case["id"])
    with tempfile.TemporaryDirectory(prefix=f"repopact-conformance-{case_id}-") as tmp:
        repo = materialize_case(Path(tmp), fixtures_root, case)
        reference_problems = validate_repo.validate(repo)
        proc = run_command(command, repo)
    output = "\n".join(part for part in (proc.stdout, proc.stderr) if part)
    expect = case.get("expect")
    if expect == "accept":
        if reference_problems:
            observed = "; ".join(problem.message for problem in reference_problems)
            return CaseResult(case_id, False, f"fixture isolation failed: unexpected violations: {observed}")
        passed = proc.returncode == 0
        detail = "accepted" if passed else f"expected accept, exit={proc.returncode}: {output.strip()}"
        return CaseResult(case_id, passed, detail)
    expected = str(case.get("expected_message", ""))
    matching = [problem for problem in reference_problems if expected in problem.message]
    unexpected = [problem for problem in reference_problems if expected not in problem.message]
    if len(matching) != 1 or unexpected:
        observed = "; ".join(problem.message for problem in reference_problems) or "none"
        extra = "; ".join(problem.message for problem in unexpected) or "none"
        return CaseResult(
            case_id,
            False,
            f"fixture isolation failed: expected one '{expected}' violation; observed: {observed}; "
            f"unexpected violations: {extra}",
        )
    passed = proc.returncode != 0 and expected in output
    detail = (
        f"rejected with '{expected}'"
        if passed
        else f"expected reject containing '{expected}', exit={proc.returncode}: {output.strip()}"
    )
    return CaseResult(case_id, passed, detail)


def run_suite(command: str, manifest_path: Path = MANIFEST) -> list[CaseResult]:
    manifest = load_manifest(manifest_path)
    fixtures_root = manifest_path.parent / str(manifest.get("fixtures_root", "fixtures"))
    return [evaluate_case(case, command, fixtures_root) for case in manifest.get("cases", [])]


def main() -> int:
    parser = argparse.ArgumentParser(description="Run the RepoPact conformance suite")
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument(
        "--command",
        default=f'"{sys.executable}" "{ROOT / "scripts" / "validate_repo.py"}" --root "{{repo}}"',
        help="Implementation command template; {repo} is replaced with the fixture repo path.",
    )
    args = parser.parse_args()

    try:
        results = run_suite(args.command, args.manifest.resolve())
    except (OSError, ValueError, json.JSONDecodeError, jsonschema.ValidationError) as exc:
        print(f"CONFORMANCE MANIFEST ERROR: {exc}", file=sys.stderr)
        return 2
    failed = [result for result in results if not result.passed]
    for result in results:
        status = "PASS" if result.passed else "FAIL"
        print(f"{status} {result.case_id}: {result.detail}")
    print(f"\n{len(results) - len(failed)}/{len(results)} conformance cases passed.")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
