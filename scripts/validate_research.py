from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from repo_model import STATUSES


@dataclass(frozen=True)
class ResearchProblem:
    path: Path
    message: str


def _load_metadata(root: Path, problems: list[ResearchProblem]) -> dict[str, Any] | None:
    path = root / "research" / "metadata.json"
    if not path.is_file():
        problems.append(ResearchProblem(path, "missing canonical research metadata"))
        return None
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        problems.append(ResearchProblem(path, f"cannot load canonical research metadata: {exc}"))
        return None
    if not isinstance(data, dict) or data.get("version") != 1:
        problems.append(ResearchProblem(path, "canonical research metadata version must be 1"))
        return None
    return data


def _read(root: Path, relative: str, problems: list[ResearchProblem]) -> str | None:
    path = root / relative
    if not path.is_file():
        problems.append(ResearchProblem(path, "missing research fact source"))
        return None
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        problems.append(ResearchProblem(path, f"cannot read research fact source: {exc}"))
        return None


def _configured_list(
    data: dict[str, Any], key: str, path: Path, problems: list[ResearchProblem]
) -> list[Any]:
    value = data.get(key)
    if not isinstance(value, list) or not value:
        problems.append(ResearchProblem(path, f"metadata field '{key}' must be a non-empty list"))
        return []
    return value


def _section(text: str, title: str) -> str | None:
    match = re.search(
        rf"^##\s+{re.escape(title)}\b(?P<body>.*?)(?=^##\s+|\Z)",
        text,
        flags=re.MULTILINE | re.DOTALL,
    )
    return match.group("body") if match else None


def _validate_lifecycle(
    root: Path, metadata: dict[str, Any], metadata_path: Path, problems: list[ResearchProblem]
) -> None:
    lifecycle = metadata.get("lifecycle")
    if not isinstance(lifecycle, dict):
        problems.append(ResearchProblem(metadata_path, "metadata field 'lifecycle' must be an object"))
        return
    states = lifecycle.get("states")
    if states != list(STATUSES):
        problems.append(ResearchProblem(
            metadata_path,
            f"canonical lifecycle states must match scripts/repo_model.py: {', '.join(STATUSES)}",
        ))
        return

    for entry in _configured_list(lifecycle, "set_documents", metadata_path, problems):
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str) or not isinstance(entry.get("pattern"), str):
            problems.append(ResearchProblem(metadata_path, "lifecycle set_documents entries require path and pattern"))
            continue
        relative = entry["path"]
        text = _read(root, relative, problems)
        if text is None:
            continue
        match = re.search(entry["pattern"], text)
        if not match:
            problems.append(ResearchProblem(root / relative, "canonical lifecycle set is missing"))
            continue
        observed = [token.strip().strip("`") for token in match.group(1).split(",")]
        if observed != states:
            problems.append(ResearchProblem(
                root / relative,
                f"lifecycle states contradict metadata: expected {', '.join(states)}; observed {', '.join(observed)}",
            ))

    for entry in _configured_list(lifecycle, "figure_documents", metadata_path, problems):
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str) or not isinstance(entry.get("section"), str):
            problems.append(ResearchProblem(metadata_path, "lifecycle figure_documents entries require path and section"))
            continue
        relative = entry["path"]
        text = _read(root, relative, problems)
        if text is None:
            continue
        body = _section(text, entry["section"])
        if body is None:
            problems.append(ResearchProblem(root / relative, f"missing lifecycle section '{entry['section']}'"))
            continue
        diagram_match = re.search(r"```(?P<diagram>.*?)```", body, flags=re.DOTALL)
        diagram = diagram_match.group("diagram") if diagram_match else body
        observed = [state for state in states if re.search(rf"\b{re.escape(state)}\b", diagram)]
        if observed != states:
            missing = [state for state in states if state not in observed]
            problems.append(ResearchProblem(
                root / relative,
                f"lifecycle figure contradicts metadata; missing state(s): {', '.join(missing)}",
            ))

    figures = _read(root, "research/figures.md", problems)
    release = lifecycle.get("provenance_typing_release")
    if not isinstance(release, str) or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", release):
        problems.append(ResearchProblem(metadata_path, "lifecycle provenance_typing_release must be semantic"))
    elif figures is not None:
        body = _section(figures, "Figure 3") or ""
        release_core = ".".join(release.split(".")[:2])
        if "future escape" in body.lower() or f"RepoPact {release_core}" not in body:
            problems.append(ResearchProblem(
                root / "research/figures.md",
                f"provenance figure must describe provenance typing as shipped in RepoPact {release_core}, not future work",
            ))


def _validate_benchmark(
    root: Path, metadata: dict[str, Any], metadata_path: Path, problems: list[ResearchProblem]
) -> None:
    benchmark = metadata.get("benchmark")
    if not isinstance(benchmark, dict):
        problems.append(ResearchProblem(metadata_path, "metadata field 'benchmark' must be an object"))
        return
    pactbench = benchmark.get("pactbench")
    if not isinstance(pactbench, dict) or not isinstance(pactbench.get("task_count"), int):
        problems.append(ResearchProblem(metadata_path, "benchmark.pactbench.task_count must be an integer"))
        return
    count = pactbench["task_count"]
    source = pactbench.get("source")
    if not isinstance(source, str) or not (root / source).is_file():
        problems.append(ResearchProblem(metadata_path, "benchmark PactBench count source must name an existing artifact"))
    for entry in _configured_list(pactbench, "documents", metadata_path, problems):
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str) or not isinstance(entry.get("pattern"), str):
            problems.append(ResearchProblem(metadata_path, "PactBench documents entries require path and pattern"))
            continue
        relative = entry["path"]
        text = _read(root, relative, problems)
        if text is None:
            continue
        match = re.search(entry["pattern"], text)
        if not match:
            problems.append(ResearchProblem(root / relative, "PactBench task-count fact is missing"))
            continue
        observed = int(match.group(1))
        if observed != count:
            problems.append(ResearchProblem(
                root / relative,
                f"PactBench task count contradicts metadata: expected {count}; observed {observed}",
            ))

    mappings = benchmark.get("study_hypotheses")
    if not isinstance(mappings, dict) or not mappings:
        problems.append(ResearchProblem(metadata_path, "benchmark.study_hypotheses must be a non-empty object"))
        return
    ordered = sorted(mappings.items(), key=lambda item: int(item[0][1:]))
    expected_start = int(ordered[0][1][1:])
    expected_end = int(ordered[-1][1][1:])
    for entry in _configured_list(benchmark, "range_documents", metadata_path, problems):
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str) or not isinstance(entry.get("pattern"), str):
            problems.append(ResearchProblem(metadata_path, "benchmark range_documents entries require path and pattern"))
            continue
        relative = entry["path"]
        text = _read(root, relative, problems)
        if text is None:
            continue
        match = re.search(entry["pattern"], text)
        if not match:
            problems.append(ResearchProblem(root / relative, "benchmark hypothesis range is missing"))
            continue
        observed = (int(match.group(1)), int(match.group(2)))
        if observed != (expected_start, expected_end):
            problems.append(ResearchProblem(
                root / relative,
                f"benchmark hypothesis range contradicts metadata: expected H{expected_start}–H{expected_end}; observed H{observed[0]}–H{observed[1]}",
            ))

    heading_pattern = re.compile(r"^###\s+(S[0-9]+)\b[^\n]*[→-]+\s*(H[0-9]+)\b", re.MULTILINE)
    table_pattern = re.compile(r"^\|\s*(S[0-9]+)\s*\|\s*(H[0-9]+)\s*\|", re.MULTILINE)
    for relative in _configured_list(benchmark, "mapping_documents", metadata_path, problems):
        if not isinstance(relative, str):
            problems.append(ResearchProblem(metadata_path, "benchmark mapping_documents entries must be paths"))
            continue
        text = _read(root, relative, problems)
        if text is None:
            continue
        observed = dict(heading_pattern.findall(text) or table_pattern.findall(text))
        if observed != mappings:
            problems.append(ResearchProblem(
                root / relative,
                f"study-to-hypothesis mapping contradicts metadata: expected {mappings}; observed {observed}",
            ))


def _validate_threats(
    root: Path, metadata: dict[str, Any], metadata_path: Path, problems: list[ResearchProblem]
) -> None:
    threats = metadata.get("threats")
    if not isinstance(threats, dict):
        problems.append(ResearchProblem(metadata_path, "metadata field 'threats' must be an object"))
        return
    identifiers = _configured_list(threats, "identifiers", metadata_path, problems)
    if any(not isinstance(value, str) or not re.fullmatch(r"T[1-9][0-9]*", value) for value in identifiers):
        problems.append(ResearchProblem(metadata_path, "threat identifiers must use T<number> form"))
        return
    if len(set(identifiers)) != len(identifiers):
        problems.append(ResearchProblem(metadata_path, "canonical threat identifiers must be unique"))
    heading_pattern = re.compile(r"^#{2,3}\s+(T[0-9]+)\s*(?::|—|-)", re.MULTILINE)
    for relative in _configured_list(threats, "documents", metadata_path, problems):
        if not isinstance(relative, str):
            problems.append(ResearchProblem(metadata_path, "threat documents entries must be paths"))
            continue
        text = _read(root, relative, problems)
        if text is None:
            continue
        observed = heading_pattern.findall(text)
        repeated = sorted(identifier for identifier, amount in Counter(observed).items() if amount > 1)
        missing = [identifier for identifier in identifiers if identifier not in observed]
        unexpected = [identifier for identifier in observed if identifier not in identifiers]
        if repeated or missing or unexpected:
            details = []
            if repeated:
                details.append(f"repeated {', '.join(repeated)}")
            if missing:
                details.append(f"missing {', '.join(missing)}")
            if unexpected:
                details.append(f"unexpected {', '.join(unexpected)}")
            problems.append(ResearchProblem(
                root / relative,
                f"threat identifiers contradict metadata: {'; '.join(details)}",
            ))


def _validate_trace(
    root: Path, metadata: dict[str, Any], metadata_path: Path, problems: list[ResearchProblem]
) -> None:
    trace = metadata.get("proposed_state_trace")
    if not isinstance(trace, dict):
        problems.append(ResearchProblem(metadata_path, "metadata field 'proposed_state_trace' must be an object"))
        return
    finding = trace.get("finding")
    capture = trace.get("capture")
    if finding != "F-014" or not isinstance(capture, str):
        problems.append(ResearchProblem(metadata_path, "proposed-state trace must identify F-014 and capture 013"))
        return
    local_fields = ["work_item", "implementation_evidence", "rollout_evidence"]
    decisions = trace.get("decisions")
    if not isinstance(decisions, list) or not decisions:
        problems.append(ResearchProblem(metadata_path, "proposed-state trace decisions must be a non-empty list"))
        decisions = []
    for relative in [capture, *decisions, *(str(trace.get(field, "")) for field in local_fields)]:
        if not relative or not (root / relative).is_file():
            problems.append(ResearchProblem(metadata_path, f"proposed-state trace target does not exist: {relative}"))
    findings = _read(root, "research/findings.md", problems)
    if findings is not None and (f"| {finding} |" not in findings or "captures/013-proposed-lifecycle-adoption-pressure.md" not in findings):
        problems.append(ResearchProblem(root / "research/findings.md", "F-014 must link capture 013 in the findings register"))
    capture_text = _read(root, capture, problems)
    required_tokens = [
        "0023", "025", "20260629-proposed-lifecycle-state", "0024",
        str(trace.get("release_tag", "")), str(trace.get("release_commit", "")),
        str(trace.get("adopter_commit", "")), "20260718-repopact-2-2-0-adopter-rollout",
    ]
    if capture_text is not None:
        absent = [token for token in required_tokens if token and token not in capture_text]
        if absent:
            problems.append(ResearchProblem(root / capture, f"proposed-state capture is missing trace token(s): {', '.join(absent)}"))


def validate(root: Path) -> list[ResearchProblem]:
    root = root.resolve()
    problems: list[ResearchProblem] = []
    metadata = _load_metadata(root, problems)
    if metadata is None:
        return problems
    metadata_path = root / "research" / "metadata.json"
    _validate_lifecycle(root, metadata, metadata_path, problems)
    _validate_benchmark(root, metadata, metadata_path, problems)
    _validate_threats(root, metadata, metadata_path, problems)
    _validate_trace(root, metadata, metadata_path, problems)
    return sorted(problems, key=lambda problem: (str(problem.path), problem.message))


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate canonical RepoPact research facts")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    root = parser.parse_args().root.resolve()
    problems = validate(root)
    if problems:
        for problem in problems:
            try:
                shown = problem.path.relative_to(root)
            except ValueError:
                shown = problem.path
            print(f"ERROR {shown}: {problem.message}")
        print(f"\nResearch validation failed with {len(problems)} error(s).")
        return 1
    print("Canonical research metadata validation passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
