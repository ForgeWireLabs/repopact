# RepoPact benchmarks

Pre-registered, runnable benchmarks for the comparative evaluation in
[`research/benchmark-protocol.md`](../research/benchmark-protocol.md). Each study's task
set and per-task expected outcome are committed **before** any run (threats T5, T6, T8),
so results cannot be manufactured by post-hoc curation.

| Dir | Study | Measures | Hypothesis | Work item |
|---|---|---|---|---|
| `pactbench/` | S1, S6a | guarantee-violation catch/escalate rate (correctness + security) | H8, H13a | `020` |
| `drift/` | S5 | drift detection / staleness vs convention files | H12 | `022` |
| _(S2/S3/S4/S6b harnesses)_ | S2–S6 | recovery, coordination, token economy, injection | H9–H13 | `022` |

PactBench (S1) and the security-invariant slice (S6a) **share one task format** — they
differ only in the `category` field — so they live together under `pactbench/`. See
[`pactbench/TASK-FORMAT.md`](pactbench/TASK-FORMAT.md).

## Seed coverage (pre-registered, 2026-06-24)

- **PactBench:** 15 seed tasks — 10 `must_not_weaken` (5 correctness, 5 security) + 5
  `legitimate` decoys (the false-stop control). Security spans six classes: `authz`,
  `authn`, `secret`, `input-validation`, `egress`, `crypto`.
- **Drift (S5):** 15 mutations (M1–M15). M4/M5/M7/M9 are deliberately recorded as RepoPact
  blind spots/partials; the rest are clear catches across the invariant types.

These are seeds, sized to be balanced and honest, not final N. The harness scales them up;
new tasks/mutations get new ids (never silent edits to a registered one).

## Discipline

- **Pre-registration.** A task file, once committed for a run, is frozen: corrections are
  new task ids, never silent edits (mirrors the protocol's amendment rule).
- **Matched arms.** Every task is run in at least the `baseline` (convention-file) and
  `repopact` arms over identical source; only the governance layer differs (threat T6).
- **Honest scoring.** Disconfirming results are recorded in `research/findings.md` with
  the same weight as confirming ones.
- **Defensive only.** Security tasks are sandboxed and benign-by-construction — no real
  exploit development, no live targets (threat T8).
