# 022 — Comparative benchmark suite (S2–S6 / H9–H13)

> **Status**: 🟢 Active
> **Owners**: tooling-owner (lead).
> **Depends on**: [`020`](../020-pactbench/) (shares the multi-arm harness; PactBench is S1).

## Needs you (operator gates)

- **Compute / model access.** Running every study across ≥2 model families needs API keys /
  model access you provision. The harness and pre-registered task sets can be built without
  it; the *results* (AC-3) are gated on run access.

## Intent

PactBench (`020`) covers S1 (guarantee-violation detection). This item covers the rest of
the comparative programme defined in
[`research/benchmark-protocol.md`](../../../research/benchmark-protocol.md), under one
harness so the arms stay matched:

- **S2** cross-session recovery + efficiency (H9) — SWE-bench Verified / SWE-EVO.
- **S3** multi-agent coordination (H10).
- **S4** context-provisioning token economy (H11) — full-prompt, convention-file,
  RAG, summarized/external memory, on-demand fetch, RepoPact, and the **C2+C3** hybrid;
  reported as a cost-vs-success Pareto frontier and a scaling curve.
- **S5** drift detection / staleness (H12) — convention files vs RepoPact, including
  RepoPact's own longitudinal blind spot (F-011).
- **S6** security (H13) — security-invariant enforcement (S6a) and context-file injection
  resistance (S6b), defensive and sandboxed (threat T8).

These are the paper's quantitative core (§6) and the source of the headline launch
numbers (token economy + guarantee/security catch rates).

## Decisions

Operationalizes hypotheses H9–H13 (protocol amendment 2026-06-24). Fairness and scoping
governed by threats T5, T6, T7, T8.

## Scope

- `benchmarks/` — the shared multi-arm harness, instrumentation, and pre-registered task
  sets for S2–S6 (reusing the PactBench task format from `020`).
- Results as evidence runs + raw `research/captures/`; figures feed paper §6.
- Out of scope: S1/PactBench (owned by `020`); building the runtime-guard arm beyond a
  thin composition shim (S6 `+ runtime guard` is optional/illustrative).

## Acceptance criteria

- **AC-1** — multi-arm harness covering S2–S6 with pre-registered task sets.
- **AC-2** — token/cost, drift, and security instrumentation as specified.
- **AC-3** — reproducible results across ≥2 model families with frontier + scaling-curve
  figures; disconfirming results recorded with equal weight.

## Progress (2026-06-24)

Foundation landed; criteria stay `pending` because the live-model runs they ultimately
require are operator-gated. Evidence:
[`20260624-pactbench-harness-selftest`](../../../evidence/runs/20260624-pactbench-harness-selftest.json)
(`partial`).

- **Harness** (`benchmarks/harness/`) runs end-to-end via a deterministic `MockRunner`:
  loader → arms → grader → confusion matrix + token/cost instrumentation; `--selftest`
  green over the task set. The `RealRunner` is the documented, operator-gated integration
  point (toward AC-1 plumbing + AC-2 instrumentation).
- **PactBench** scaled to **21 tasks** (14 `must_not_weaken` across six security classes +
  7 decoys) with two **real, runnable fixtures** (`calc-rounding`, `api-orders`; 3 + 5
  fixture tests pass).
- **Still gated:** S2/S3/S4 drivers and real-model runs across ≥2 model families
  (AC-2 full instrumentation, AC-3 results). No row in any harness report is a finding yet.

(State tracked in [`work-item.json`](work-item.json); `pending` until evidence-backed.)
