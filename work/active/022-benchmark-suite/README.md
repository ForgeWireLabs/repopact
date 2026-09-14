# 022 — Comparative benchmark suite (S2–S6 / H9–H13)

> **Status**: 🟢 Active
> **Owners**: tooling-owner (lead).
> **Depends on**: [`020`](../020-pactbench/) (shares the multi-arm harness; PactBench is S1).

## Needs you (operator gates)

- **Compute / model access.** Running every study across ≥2 model families needs API keys /
  model access you provision. The harness and pre-registered task sets can be built without
  it; the *results* (AC-3) are gated on run access.
- **First live smoke.** AC-5 requires one provisioned real-model command before the full
  comparative programme can be treated as de-risked.

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

- The shared multi-arm harness, instrumentation, and pre-registered task sets for S2–S6 live
  in the **RepoPact Proving Ground** repo (`benchmarks/`), reusing the PactBench task format.
  This repo holds only the protocol + hypotheses (`research/`) — see work item `020`.
- Results as evidence runs + raw `research/captures/`; figures feed paper §6.
- Out of scope: S1/PactBench (owned by `020`); building the runtime-guard arm beyond a
  thin composition shim (S6 `+ runtime guard` is optional/illustrative).

## Acceptance criteria

- **AC-1** — multi-arm harness covering S2–S6 with pre-registered task sets.
- **AC-2** — token/cost, drift, and security instrumentation as specified.
- **AC-3** — reproducible results across ≥2 model families with frontier + scaling-curve
  figures; disconfirming results recorded with equal weight.
- **AC-4** — freeze the statistical analysis plan before interpreting any live result.
- **AC-5** — run and capture a three-task RealRunner smoke against one real model.

## Progress (reconciled 2026-09-13)

The deterministic/non-operator-gated tranche is implemented in the isolated Proving
Ground WI022 branch; live-model criteria remain pending. Evidence:
[`20260624-pactbench-harness-selftest`](../../../evidence/runs/20260624-pactbench-harness-selftest.json)
(`partial`).

- **Common substrate:** `repopact.experiment-run.v1` carries study/case/condition,
  fixture/task-set versions, repetition/seed, model identity, policy/scorer versions,
  completion/failure state, per-request and aggregate telemetry, observations, capture
  references, exact command, provenance, and illustrative classification.
- **PactBench** now has **24 tasks**; the registered S1/S6a suite remains unchanged and
  its deterministic selftest is plumbing evidence only.
- **RealRunner:** `repopact.real-runner.v1` is provider-neutral and rejects malformed or
  incomplete telemetry instead of turning it into zeroes. A failed/incomplete response is
  retained as an explicit non-reportable failure.
- **Telemetry v2 checkpoint (2026-09-13):** the additive `repopact.real-runner.v2` /
  `repopact.experiment-run.v2` path is implemented against the installed Codex public
  app-server `thread/tokenUsage/updated` notification. It preserves request-level
  `last` usage, cumulative-ledger reconciliation, cache reads, `cache_write_input_tokens`,
  reasoning output, and pinned-tokenizer context/task attribution. No live model call was
  made in this checkpoint; AC-2 remains pending until the corrected smoke validates the
  complete path. Evidence: [`20260913-wi022-telemetry-v2-checkpoint`](../../../evidence/runs/20260913-wi022-telemetry-v2-checkpoint.json).
- **Next smoke selection (2026-09-13):** before inference, the corrected v2 smoke was
  frozen to 0001 (correctness must-not-weaken), 0002 (security/frozen-surface
  must-not-weaken), and 0021 (legitimate decoy), with 0003 explicitly excluded. The
  task-set digest and both-arm materialization fingerprints are recorded in
  [`20260913-wi022-ac5-v2-selection`](../../../evidence/runs/20260913-wi022-ac5-v2-selection.json).
- **First real smoke (2026-09-13):** the repopact arm ran tasks 0001, 0002, and 0003
  once each on the authenticated Codex CLI runtime. Raw captures and postconditions are
  preserved, but the strict RealRunner envelopes are non-reportable because the runtime
  did not expose the required context-vs-task or cache-adjusted token attribution.
- **Drivers and registrations:** Proving Ground now has deterministic S2 recovery and
  pinned-bed selectors/materialization from immutable SWE-bench Verified and SWE-EVO
  assets, S3 concurrent isolated-worker scoring, S4 C0-C9
  plus C2+C3 condition validation and cost/success analyses, an S5 shared-envelope
  adapter, and S6b scoring over frozen tasks 0023/0024. Mock/fake outputs are explicitly
  illustrative/non-empirical.
- **Still gated:** S2/S3/S4/S6b live execution, complete applicable-study telemetry,
  and results across ≥2 model families. The AC-5 smoke is captured but non-reportable;
  no row in any harness report is a finding.
- **S5 dependency:** WI037 repaired the drift harness's removed flat imports. The
  packaged RepoPact 3.0.2 S5 selftest now passes in the Proving Ground; this does
  not turn deterministic plumbing into a live empirical result.
- **Analysis freeze:** the dated 2026-09-13 amendment in
  [`research/benchmark-protocol.md`](../../../research/benchmark-protocol.md) freezes
  repetitions, seeds, temperature/model/version policy, scorers, paired effects,
  uncertainty, multiplicity, missingness, exclusions, stopping, pricing timestamps, and
  illustrative-vs-reportable treatment.

## Criterion state

- [x] **AC-1** — satisfied for the deterministic benchmark tranche. S2-S6 driver
  boundaries and pre-registrations exist, and the pinned S2 task beds now have a
  reproducible acquisition, source-schema validation, model/evaluation projection,
  base-commit preflight, and offline verification path. Live study execution remains
  separately gated.
- [ ] **AC-2** — pending. The smoke exposed real input/output/cached usage and an explicit
  subscription pricing policy, but not accurate `context_tokens`, `task_tokens`, or
  `cache_adjusted_input_tokens`; the missing attribution is recorded rather than inferred.
- [ ] **AC-3** — pending and operator-gated. No results across two model families,
  Pareto frontier, or scaling curve exist.
- [x] **AC-4** — satisfied. The dated 2026-09-13 WI022 amendment freezes the enumerated
  analysis choices before any reportable live result.
- [ ] **AC-5** — pending. Three real task attempts completed with prompts, outputs, raw event
  captures, postconditions, and failures, but all three envelopes are non-reportable due
  to the missing required telemetry attribution.

The 2026-07-26 live check also found that Proving Ground's S5 selftest fails
against the current package boundary because it imports removed flat module
names. Proposed WI 037 owns that repair. Deterministic MockRunner output remains
plumbing evidence only.

Evidence:
[`20260624-pactbench-harness-selftest`](../../../evidence/runs/20260624-pactbench-harness-selftest.json)
and
[`20260726-semantic-ledger-freshness-reconciliation`](../../../evidence/runs/20260726-semantic-ledger-freshness-reconciliation.json),
and
[`20260913-wi022-deterministic-benchmark-tranche`](../../../evidence/runs/20260913-wi022-deterministic-benchmark-tranche.json),
and
[`20260913-wi022-s5-package-boundary-reconciliation`](../../../evidence/runs/20260913-wi022-s5-package-boundary-reconciliation.json).
and
[`20260913-wi022-s2-deterministic-materialization`](../../../evidence/runs/20260913-wi022-s2-deterministic-materialization.json).
and
[`20260913-wi022-ac5-realrunner-smoke`](../../../evidence/runs/20260913-wi022-ac5-realrunner-smoke.json).
and
[`20260913-wi022-telemetry-v2-checkpoint`](../../../evidence/runs/20260913-wi022-telemetry-v2-checkpoint.json).
