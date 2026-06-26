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

(State in [`work-item.json`](work-item.json); `pending` until evidence-backed.)
