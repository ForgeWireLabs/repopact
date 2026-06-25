# Benchmark harness

Model-agnostic harness for the comparative studies in
[`../../research/benchmark-protocol.md`](../../research/benchmark-protocol.md). It loads
pre-registered tasks, runs each in the matched arms (`baseline` = convention-file,
`repopact` = RepoPact records), grades the outcome against the confusion-matrix taxonomy,
and reports metrics + token/cost instrumentation.

## Run it

```bash
# Illustrative pipeline check against the 15 pre-registered PactBench tasks (no model):
python benchmarks/harness/run.py

# Self-test (asserts the pipeline produces a sane matrix; exits non-zero on failure):
python benchmarks/harness/run.py --selftest

# Write a report file, choose arms:
python benchmarks/harness/run.py --arms baseline,repopact --out report.md
```

## What is real vs. operator-gated

| Piece | Status |
|---|---|
| Task loader, arm runner, grader, confusion-matrix + metrics, token instrumentation | **implemented**, self-tested |
| `MockRunner` | implemented — deterministic test double; **numbers are illustrative, not findings** |
| `RealRunner` (drive a live agent over a fixture, read post-conditions) | **operator-gated** — needs a model/agent + API keys |

The MockRunner exists so the plumbing is testable without a model. Real results require the
`RealRunner` across ≥2 model families (work item 022, AC-3); until then no row here is a
finding.

## The runner interface

A runner implements `run(task, arm) -> AgentAction`. To add a live model, implement
`RealRunner.run` in [`runners.py`](runners.py): materialize the arm over the task fixture
(RepoPact records vs a convention-file `AGENTS.md`), drive the agent with the task prompt,
then read back post-conditions (diff inspection, `check-frozen` exit code, emitted approval
requests) per [`../pactbench/TASK-FORMAT.md`](../pactbench/TASK-FORMAT.md) and return an
`AgentAction`. The grader ([`graders.py`](graders.py)) turns the action into an `Outcome`;
runners never assign outcomes themselves.

## Files

| File | Role |
|---|---|
| `model.py` | `Task`, `AgentAction`, `RunResult`, `Outcome`, `TokenUsage` |
| `runners.py` | `MockRunner`, `RealRunner` (gated), `get_runner` |
| `graders.py` | action → outcome classification (polarity-aware) |
| `report.py` | confusion matrix, metrics, token summary, markdown render |
| `run.py` | CLI: load → run → grade → report; `--selftest` |

## Coverage

Today the harness implements the **PactBench (S1/S6a)** and **drift task-loading (S5)**
plumbing with the mock runner. The longer-horizon studies — S2 (recovery/efficiency on
SWE-bench Verified / SWE-EVO), S3 (multi-agent coordination), and the full S4 token-economy
sweep across all context regimes — reuse this scaffolding (loader, arms, instrumentation,
report) but need the `RealRunner` and additional drivers; they are tracked under work item
022 AC-1/AC-3.
