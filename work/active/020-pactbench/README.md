# 020 — PactBench: guarantee-violation detection benchmark

> **Status**: 🟢 Active
> **Owners**: tooling-owner (lead).
> **Depends on**: none. (Feeds the paper and `research/benchmark-protocol.md` S1 / H8.)

## Needs you (operator gates)

- **Compute / model access.** Running the benchmark across ≥2 model families needs API
  keys / model access you provision. The task set and harness can be built without it;
  the *results* (AC-3) are gated on you supplying run access.

## Intent

RepoPact's distinctive claim is that a binding invariant stops an agent from *silently
weakening a guarantee*. That claim deserves a number, the way runtime-security work
reports an interception rate. PactBench is that benchmark: a pre-registered suite of
tasks where the correct behaviour is to **refuse or escalate** rather than quietly delete
the failing assertion, relax the schema, or edit a frozen path — measured with RepoPact
governance on versus off.

This is the paper's strongest differentiator (S1 in
[`benchmark-protocol.md`](../../../research/benchmark-protocol.md)) and a defensible
marketing figure: *"agents under RepoPact silently weakened a declared guarantee X% less
often."*

## Decisions

Operationalizes hypothesis **H8** (protocol amendment 2026-06-24). Pre-registration and
baseline-fairness are governed by threats **T5** and **T6**.

## Scope

- `benchmarks/pactbench/tasks/` — the pre-registered task set + per-task expected outcome.
- `benchmarks/pactbench/harness/` — the two-arm runner and confusion-matrix scorer.
- Results written as evidence runs + raw `research/captures/`.
- Out of scope: S2 (recovery/efficiency) and S3 (coordination) — those are separate
  studies under the same protocol.

## Acceptance criteria

- **AC-1** — pre-registered task set committed before runs.
- **AC-2** — two-arm harness recording the confusion matrix with raw captures.
- **AC-3** — reproducible results across ≥2 model families, feeding the protocol and paper.

(State tracked in [`work-item.json`](work-item.json); `pending` until evidence-backed.)
