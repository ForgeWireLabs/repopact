# 022 — Comparative benchmark suite (S2–S6 / H9–H13)

> **Status**: 🟢 Active
> **Owners**: tooling-owner (lead).
> **Depends on**: [`020`](../020-pactbench/) (shares the multi-arm harness; PactBench is S1).

## Needs you (operator gates)

- **Compute / model access.** Running every study across ≥2 model families needs API keys /
  model access you provision. The harness and pre-registered task sets can be built without
  it; the *results* (AC-3) are gated on run access.
- **Comparative studies.** AC-3 remains operator-gated until a separate multi-family
  comparative programme is authorized.

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
- **Second real smoke (2026-09-14):** after the corrected task-set and v2 public-schema
  fixes were published to proving-ground `master@99b34d8`, one admission probe and exactly
  one v2 app-server invocation each for 0001, 0002, and 0021 completed in fresh isolated+  workspaces. All three envelopes carry public `thread/tokenUsage/updated` request-level
  ledgers, complete cache/context/task attribution, model/provider identity, prompts,
  outputs, deterministic postconditions, and raw captures. The reconciled outcomes were+  `proceeded_safely`, `errored`, and `errored`; these are empirical captured outcomes,+  not a claim of three successful catches. Evidence: [`20260914-wi022-ac5-realrunner-smoke-v2`](../../../evidence/runs/20260914-wi022-ac5-realrunner-smoke-v2.json).
- **Drivers and registrations:** Proving Ground now has deterministic S2 recovery and
  pinned-bed selectors/materialization from immutable SWE-bench Verified and SWE-EVO
  assets, S3 concurrent isolated-worker scoring, S4 C0-C9
  plus C2+C3 condition validation and cost/success analyses, an S5 shared-envelope
  adapter, and S6b scoring over frozen tasks 0023/0024. Mock/fake outputs are explicitly
  illustrative/non-empirical.
- **Still gated:** S2/S3/S4/S6b live execution and results across ≥2 model families.
  The three-case AC-5 smoke is reportable as a runtime/telemetry acceptance smoke, but its
  outcomes are not promoted to a comparative finding or full-study result.
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
- [x] **AC-2** — satisfied. The v2 public app-server path was exercised on three completed
  real-model turns and produced reconciled per-request input/output/cached/reasoning usage,
  context-vs-task attribution, cache-adjusted tokens, requests-per-task, and the explicit+  authenticated-subscription USD policy. The existing deterministic drift and security+  instrumentation remains covered by the prior evidence set.
- [ ] **AC-3** — pending and operator-gated. No results across two model families,
  Pareto frontier, or scaling curve exist.
- [x] **AC-4** — satisfied. The dated 2026-09-13 WI022 amendment freezes the enumerated
  analysis choices before any reportable live result.
- [x] **AC-5** — satisfied. Exactly one fresh v2 real-model invocation completed for each
  pre-registered task 0001, 0002, and 0021 after the admission ledger passed. Each envelope+  has the raw command, actual public provider/model, prompt/output, per-request and+  aggregate telemetry, deterministic grader reconciliation, and raw event capture. The+  historical first smoke remains unchanged and non-reportable; AC-3 was not started.

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
and
[`20260914-wi022-ac5-realrunner-smoke-v2`](../../../evidence/runs/20260914-wi022-ac5-realrunner-smoke-v2.json).

## AC-3 execution readiness checkpoint (2026-09-14)

The AC-3 execution substrate is published to Proving Ground
`master@0d43f9da959c57e854caac01e0e18f3195468620`. The shared empirical executor
supports strict study-specific output schemas, public app-server request ledgers,
capture/secret checks, runtime identity, and non-illustrative provenance. Narrow
empirical adapters now exist for S2, S3, S4, S6a, and S6b; S2 remains explicitly
blocked until a functional pinned SWE-bench/SWE-EVO checkout/evaluator is provisioned.
The historical AC-5 v2 adapter and captures remain unchanged in behavior.

S4 runnable conditions C0-C8 plus C2+C3 have frozen local operational semantics in
`2026-09-14.s4-methods.1`: no auxiliary model, embedding API, or memory-service calls;
C5 is an isolated in-memory SQLite store and C7 requires exact RepoPact records with
no full-corpus fallback. S5 is model-independent under
`2026-09-14.s5-model-independent.1`; its 135 deterministic cells are shared rather
than duplicated under two model labels.

The superseding manifest preserves the old 678-cell checkpoint and enumerates 543
logical cells (567 execution slots: 432 live turns plus 135 deterministic S5
observations). The revised budget estimates 3,888 internal inference requests,
92,466,288 point input tokens, 2,119,968 point output tokens, and approximately
13.394 hours, with zero auxiliary model/service calls and zero marginal USD under
the current authenticated subscription policy. This is still pre-inference planning;
no registered AC-3 benchmark cell has been executed.

- [ ] **AC-3** remains pending. The two permitted family admission probes are not
  comparative benchmark evidence and do not satisfy the criterion by themselves.
