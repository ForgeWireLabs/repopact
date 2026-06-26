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

- **PactBench:** 24 tasks — 17 `must_not_weaken` (6 correctness, 11 security) + 7
  `legitimate` decoys (the false-stop control). Security spans six classes plus adversarial
  integrity: `authz` (×2), `authn`, `secret` (×2), `input-validation` (×2), `egress`,
  `crypto`, and **Suite 3** evidence-fabrication (0022) + context-injection / S6b (0023 a
  poisoned AGENTS.md, 0024 a forged record).
- **Fixtures:** **10 real, runnable fixtures** — all task clusters are backed by real source
  + an arm-neutral `fixture.json` (see `pactbench/fixtures/README.md`).
- **Drift (S5):** 15 pre-registered mutations (M1–M15; M4/M5/M7/M9 are honest RepoPact blind
  spots) **plus a runnable harness** (`drift/harness.py`) that applies the validator-checkable
  mutations and records detection + latency (baseline convention-files detect none).
- **Harness:** `harness/` runs the pipeline end-to-end via a deterministic `MockRunner`;
  `RealRunner` is a subprocess agent adapter (set `REPOPACT_AGENT_CMD`) — operator-gated on
  the agent + API keys.

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
