# 020 — Integrate PactBench with RepoPact Proving Ground

> **Status**: 🟢 Active
> **Owners**: governance-owner (lead); tooling-owner (harness).
> **Depends on**: none. Establishes the repo boundary for the benchmark work.

## Intent

PactBench is the first named benchmark suite inside **RepoPact Proving Ground**, not a
benchmark implementation embedded directly in the RepoPact standard repository.

- **RepoPact** defines the contract language, validation semantics, evidence expectations,
  and the **benchmark protocol** (`research/benchmark-protocol.md`, `research/protocol.md`
  hypotheses H8–H13, `research/threats-to-validity.md`). It says *what to measure* and the
  falsification criteria.
- **RepoPact Proving Ground**
  ([ForgeWireLabs/repopact-proving-ground](https://github.com/ForgeWireLabs/repopact-proving-ground))
  hosts the runnable tasks, harnesses, captures, and reproducible result bundles that test
  whether RepoPact enforcement measurably reduces silent guarantee drift. It consumes
  RepoPact from PyPI and runs the suite against the *packaged* product.

> RepoPact defines the pact. RepoPact Proving Ground tests whether the pact holds under
> agent pressure.

## Scope

- **Move** `benchmarks/` (PactBench task format + tasks, fixtures, the multi-arm harness,
  the S5 drift harness) out of this repo and into the Proving Ground.
- **Keep** the protocol, hypotheses, threats, and the paper here in `research/`.
- **Reconcile references** in both repos (README, paper Appendix D, work item 022) so each
  repo points at the right home.
- Hosts the comparative studies S2–S6 (work item `022`) under the same Proving-Ground roof.

## Acceptance criteria

- **AC-1** — RepoPact defines only the protocol; no runnable benchmark embedded in the
  standard repo.
- **AC-2** — the runnable PactBench suite is hosted in the Proving Ground, consuming RepoPact
  from PyPI.
- **AC-3** — cross-repo references are consistent; real cross-model result bundles remain
  operator-gated.

## Reconciliation — 2026-07-26

- [x] **AC-1** — satisfied. `git ls-files benchmarks` is empty in the standard
  repository; the protocol, hypotheses, threats, and falsification criteria remain
  under `research/`.
- [x] **AC-2** — satisfied. The clean public Proving Ground default branch at
  `8f68bc4` contains the 24-task suite, fixtures, shared harness, and S5 mutation
  harness; its pinned PyPI dependency and deterministic PactBench selftest were
  verified.
- [ ] **AC-3** — pending. RepoPact points to the Proving Ground, but the Proving
  Ground's `benchmarks/README.md` and `benchmarks/harness/README.md` still point
  to a nonexistent local `research/` tree. Its 2.2.0 package pin is also stale
  against current 2.3.0. Proposed WI 037 owns those cross-repository repairs;
  real cross-model bundles remain operator-gated under WI 022.

Evidence:
[`20260726-semantic-ledger-freshness-reconciliation`](../../../evidence/runs/20260726-semantic-ledger-freshness-reconciliation.json).

## Integrity reconciliation — 2026-09-13

- [x] **AC-2** — satisfied again for the corrected, frozen task set. The first concrete WI-022 AC-5 smoke exposed that
  several preregistered prompts do not match their shared fixture states: some claimed
  red tests are green, some requested typos/artifacts/symbols are absent, and some
  graders name paths that do not exist. The original June registration and September
  smoke remain unchanged historical records.
- A model-free 24-case audit classified the original registration as 7 `ready`, 6
  `needs_seed_setup`, 5 `task_definition_mismatch`, and 6 `retire_or_replace`. A dated
  v2 task-set now supplies bounded deterministic setup overlays and superseding task
  definitions; all 24 effective cases are preflight-eligible in matched functional
  seeds. This satisfies the runnable-suite criterion for the corrected corpus; the
  separate WI022 live smoke and comparative criteria remain pending. See
  [`20260913-wi020-pactbench-integrity-reconciliation`](../../../evidence/runs/20260913-wi020-pactbench-integrity-reconciliation.json)
  and the audit artifact in the Proving Ground.
