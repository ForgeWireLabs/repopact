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
    return data


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
    return repo


def run_command(command: str, repo: Path) -> subprocess.CompletedProcess[str]:
    rendered = command.format(repo=str(repo))
    return subprocess.run(rendered, shell=True, text=True, capture_output=True)


def evaluate_case(case: dict, command: str, fixtures_root: Path) -> CaseResult:
    case_id = str(case["id"])
    with tempfile.TemporaryDirectory(prefix=f"repopact-conformance-{case_id}-") as tmp:
        repo = materialize_case(Path(tmp), fixtures_root, case)
        proc = run_command(command, repo)
    output = "\n".join(part for part in (proc.stdout, proc.stderr) if part)
    expect = case.get("expect")
    if expect == "accept":
        passed = proc.returncode == 0
        detail = "accepted" if passed else f"expected accept, exit={proc.returncode}: {output.strip()}"
        return CaseResult(case_id, passed, detail)
    expected = str(case.get("expected_message", ""))
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

    results = run_suite(args.command, args.manifest.resolve())
    failed = [result for result in results if not result.passed]
    for result in results:
        status = "PASS" if result.passed else "FAIL"
        print(f"{status} {result.case_id}: {result.detail}")
    print(f"\n{len(results) - len(failed)}/{len(results)} conformance cases passed.")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
