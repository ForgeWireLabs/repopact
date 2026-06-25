"""PactBench harness CLI.

Usage:
    python benchmarks/harness/run.py --tasks-dir benchmarks/pactbench/tasks
    python benchmarks/harness/run.py --selftest

Loads pre-registered tasks, runs each in the requested arms with the chosen runner, grades
the outcomes, and prints a markdown report (confusion matrix + metrics + token summary).
The default runner is the deterministic MockRunner, so the pipeline runs without a model.
"""
from __future__ import annotations

import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from graders import classify          # noqa: E402
from model import Task                 # noqa: E402
from report import render_markdown, summarize  # noqa: E402
from runners import get_runner         # noqa: E402


def load_tasks(tasks_dir: str) -> list[Task]:
    tasks: list[Task] = []
    for name in sorted(os.listdir(tasks_dir)):
        if not name.endswith(".json") or name.endswith(".schema.json"):
            continue
        with open(os.path.join(tasks_dir, name), encoding="utf-8") as fh:
            d = json.load(fh)
        inv = d.get("invariant", {})
        seed = d.get("seed", {})
        tasks.append(Task(
            id=d["id"],
            title=d["title"],
            category=d["category"],
            polarity=d["polarity"],
            frozen_surface=bool(inv.get("frozen_surface", False)),
            arms=d.get("arms", ["baseline", "repopact"]),
            security_class=d.get("security_class"),
            fixture=seed.get("fixture"),
        ))
    return tasks


def run_tasks(tasks: list[Task], arms: list[str], runner) -> list:
    results = []
    for task in tasks:
        for arm in arms:
            if arm not in task.arms:
                continue
            action = runner.run(task, arm)
            results.append(classify(task, arm, action))
    return results


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description="PactBench benchmark harness")
    here = os.path.dirname(os.path.abspath(__file__))
    default_tasks = os.path.normpath(os.path.join(here, "..", "pactbench", "tasks"))
    p.add_argument("--tasks-dir", default=default_tasks)
    p.add_argument("--arms", default="baseline,repopact")
    p.add_argument("--runner", default="mock", choices=["mock", "real"])
    p.add_argument("--out", default=None, help="write the markdown report here")
    p.add_argument("--selftest", action="store_true",
                   help="assert the pipeline produces a sane confusion matrix and exit")
    args = p.parse_args(argv)

    arms = [a.strip() for a in args.arms.split(",") if a.strip()]
    runner = get_runner(args.runner)
    tasks = load_tasks(args.tasks_dir)
    results = run_tasks(tasks, arms, runner)
    summary = summarize(results)
    illustrative = args.runner == "mock"
    md = render_markdown(summary, illustrative=illustrative)

    if args.selftest:
        return _selftest(tasks, summary)

    print(md)
    if args.out:
        with open(args.out, "w", encoding="utf-8") as fh:
            fh.write(md)
        print(f"\n[wrote {args.out}]")
    return 0


def _selftest(tasks: list[Task], summary: dict) -> int:
    """Assert the plumbing is sane against the MockRunner's known policy."""
    assert tasks, "no tasks loaded"
    assert "baseline" in summary and "repopact" in summary, "both arms must report"
    base = summary["baseline"]["metrics"]
    repo = summary["repopact"]["metrics"]
    # MockRunner policy: repopact catches must_not_weaken; baseline does not.
    assert repo["catch_rate"] > base["catch_rate"], (
        f"expected repopact catch_rate > baseline, got {repo} vs {base}")
    assert repo["silent_violation_rate"] < base["silent_violation_rate"], (
        "expected repopact to lower silent-violation rate")
    assert repo["false_stop_rate"] == 0.0, "mock should not false-stop legitimate tasks"
    # every result classified into a known cell, none errored
    for arm in ("baseline", "repopact"):
        assert summary[arm]["counts"]["errored"] == 0, f"{arm} had errored classifications"
    print(f"SELFTEST OK: {len(tasks)} tasks; "
          f"baseline catch={base['catch_rate']} silent={base['silent_violation_rate']}; "
          f"repopact catch={repo['catch_rate']} silent={repo['silent_violation_rate']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
