# RepoPact Proving Ground — Research Record

This directory is the lab notebook for an adversarial evaluation of the RepoPact
architecture. RepoPact claims to be a *repository-native contract, evidence, and
conformance layer for durable agent work*: authority, intent, evidence, and drift
recorded as versioned files so project state survives without a prior conversation.
This record exists
to test that claim — to **prove or disprove** it — rather than to assert it.

The exercise is deliberately reflexive: RepoPact is the instrument as well as the
subject. The proving ground is a throwaway project that adopts RepoPact *from the
published package* and is then driven hard across every primitive, including the
cases designed to make it fail. The evidence runs, audit findings, and validator
output produced along the way **are the data**.

## Contents

| File | Purpose |
| --- | --- |
| [`protocol.md`](protocol.md) | The experiment: research question, hypotheses, method, and falsification criteria. Write before running. |
| [`run-log.md`](run-log.md) | Chronological log of every experimental action and its result. Append-only. |
| [`findings.md`](findings.md) | Findings register: each place the architecture held or cracked, with severity and resolution. |
| [`paper.md`](paper.md) | The paper draft: "The Repository as the Operating System for Agentic Work" — the full text assembled from this record, targeting an arXiv cs.SE preprint. |
| [`paper-outline.md`](paper-outline.md) | Working outline for the paper (scaffold; superseded in detail by `paper.md`). |
| [`benchmark-protocol.md`](benchmark-protocol.md) | Pre-registered comparative benchmark protocol (S1–S6 / H8–H13); the runnable suites live in RepoPact Proving Ground. |
| [`figures.md`](figures.md) | Planned figures and tables for the paper, with data sources. |
| [`gap-audit-2026-07.md`](gap-audit-2026-07.md) | Pre-publication gap audit (2026-07-15): verified gaps across adopters, conformance, benchmarks, and logistics, with a sequenced worklist toward the end-of-August paper deadline. |
| [`formal-model.md`](formal-model.md) | Operational semantics of the kernel as layers (L0–L5): the state algebra, the lifecycle FSM as one layer (L1), the well-formedness predicate the validator decides, the typed invariant lattice, the adoption trilemma, and the theorems (T1–T6) the proving ground falsifies. |
| [`threats-to-validity.md`](threats-to-validity.md) | Why the evidence must not be over-read — reflexivity (forgewire is the progenitor), single evaluator, scale. |
| [`release-runbook.md`](release-runbook.md) | Credential-bound release steps, including the billing-locked Actions/direct-PyPI fallback used for 2.2.0 and the historical v1.0.0 handoff. |
| [`captures/`](captures/) | Raw, unedited terminal transcripts referenced by the run log and findings. |

## How to read this as evidence

A finding is only as good as the artifact behind it. Every entry in
[`findings.md`](findings.md) cites a capture under [`captures/`](captures/) and,
where the finding drove a change, the work item or audit finding that resolved it.
Confidence is not evidence; the captures are.
