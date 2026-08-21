# 039 — Promote ForgeWire Enforcement-Closure Case Study

> **Status**: Complete — see evidence
> `20260821-039-promote-forgewire-enforcement-closure-case-study` for the
> full validation record (repopact validate/dashboard/spec/conformance/
> check-frozen all pass; 2 pre-existing implementation-test literals now
> stale by design, disclosed in the evidence, not fixed — out of scope).
> **Owners**: governance-owner (lead).
> **Depends on**: none. Related: work item `032` (narrow cross-reference
> only, no status/criteria change).

## Intent

Promote the accepted, maintainer-reviewed ForgeWire WI230/WI237 case study
(`research/case-studies/2026-08-forgewire-wi230-wi237-enforcement-closure/`,
commits `279b714`, `45bc365`) into RepoPact's own research corpus: the
findings register, the formal model, the experiment protocol, and the
benchmark protocol — following RepoPact's own preregistration discipline
(dated amendments, appended not rewritten). This is a **research promotion
pass**, not an implementation pass and not a benchmark-results pass: it
records findings and preregisters new hypotheses/studies/scenarios. It does
not implement or run any new PactBench/drift scenario, does not change the
RepoPact CLI or validator, and does not modify ForgeWire.

**In scope**: `research/findings.md`, `research/formal-model.md`,
`research/protocol.md`, `research/benchmark-protocol.md`, `research/paper.md`,
a narrow cross-reference addition to work item `032` / decision `0031`.

**Out of scope**: implementing any new PactBench task or drift mutation;
running any benchmark; changing RepoPact's CLI, validator, schemas, or CI;
touching ForgeWire; changing WI032's status or marking any of its criteria
satisfied; rewriting WI032's chosen remote-enforcement design.

## Decisions

The enforcement-closure primitive (coverage/invocation/effectiveness over
governed admission transitions) is promoted as a formally-stated,
substrate-neutral cross-cutting property rather than a new numbered kernel
layer, since the existing L0–L5 layering does not require restructuring to
accommodate it — it composes with L1/L2 as a property of the admission
boundary those layers already reference. Recorded inline in
`formal-model.md` rather than as a separate decision record, consistent with
how other cross-cutting clarifications (e.g. the 2.2.0 dashboard-fixpoint
change) were integrated directly into the model document.

## Scope

Files this work adds or changes: `research/findings.md`,
`research/formal-model.md`, `research/protocol.md`,
`research/benchmark-protocol.md`, `research/paper.md`, and (narrowly) a
cross-reference note in `work/blocked/032-.../README.md`. No files under
`repopact/`, `schemas/`, `.github/`, `benchmarks/` (Proving Ground is a
separate repository and is not touched by this work item at all), or any
ForgeWire path.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are
satisfied, move this directory to `work/completed/` and regenerate the
dashboard.
