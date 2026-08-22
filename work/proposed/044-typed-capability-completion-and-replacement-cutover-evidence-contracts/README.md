# 044 — Typed Capability Completion and Replacement Cutover Evidence Contracts

> **Status**: 📋 Planning (proposed — not started)
> **Owners**: governance-owner (lead); tooling-owner and docs-owner affected.
> **Depends on**: none.

## Intent

ForgeWire ADR 0010 / INV-4 exposed a governance gap that is broader than ForgeWire:
RepoPact can require acceptance criteria to have linked evidence, but it cannot currently
express what *kind* of completion claim is being made or whether the evidence is sufficient
for that claim class.

The field pattern is repeated across unrelated ForgeWire domains: production-unreachable
services with green unit tests, replacement implementations whose predecessors remain
load-bearing, synthetic-success fallbacks, experimental capabilities represented too strongly,
duplicate authorities, registered-but-no-op actions, and architectural migrations that stop at
a compatibility bridge. This item investigates and, only after reconciliation, implements a
generic RepoPact abstraction for typed capability-completion and replacement-cutover evidence.

The goal is **not** to hard-code ForgeWire's `INV-4` into every adopter. RepoPact already
correctly treats project invariants as adopter-owned policy. The upstream target is the generic
claim/evidence primitive that INV-4 discovered.

## Source field evidence

Primary downstream evidence comes from ForgeWire's accepted
`decisions/0010-product-capability-completion-and-cutover.md`, `governance/invariants.json`
(`INV-4`), WI233's cutover enforcement, and the audit-spawned WI239-WI246 family.
Representative repeated shapes include:

- a capability implementation exists and unit tests pass, but no canonical production
  composition path reaches it;
- a replacement exists, but callers or accidental fallback paths still reach the predecessor;
- a fallback reports success without the claimed persistence/delivery/action actually occurring;
- experimental/library-only code is represented as a product-complete capability;
- two implementations remain peer authorities after an alleged consolidation/cutover;
- a structural migration is contained by a shrink-only exception list but has not reached its
  declared zero-breach target.

These cases are evidence inputs, not universal vocabulary requirements.

## Research question

RepoPact's existing H14 / enforcement-closure line asks whether an applicable governance
checkpoint has **coverage**, is **invoked**, and is **effective** at the admission boundary.
This item asks a different question: whether the evidence attached to a completion assertion is
semantically sufficient to establish the asserted state in the first place.

Working terms such as **claim closure** or **completion closure** may be evaluated, but no new
formal property or hypothesis is accepted merely by naming it. The work must determine whether
this is genuinely distinct from existing formal-model concepts and record the relationship
without rewriting H14 after the fact.

## Candidate generic shape — not yet a decision

A possible design is an optional typed claim attached to a work item, acceptance criterion,
evidence record, or separate claim record. Example claim classes might include:

- **capability completion** — implementation plus production reachability plus
  composition/end-to-end proof, with explicit degraded/unavailable semantics where applicable;
- **replacement/cutover completion** — replacement implementation plus caller migration plus
  predecessor retirement plus a static/contract absence guard.

This is a hypothesis to evaluate, not a schema prescription. The work must compare placement,
extensibility, adopter opt-in, backwards compatibility, and evidence-reference semantics before
changing the kernel.

## Non-goals

- Do not make ForgeWire-specific names such as `INV-4`, `modules_shims`, `ForgeLink`, or a
  particular composition-root pattern part of the universal RepoPact contract.
- Do not add four mandatory string fields to every work item merely because ForgeWire uses a
  four-part cutover equation.
- Do not declare every code change a product capability or replacement claim.
- Do not weaken or conflate H14 enforcement closure to absorb this problem.
- Do not retroactively rewrite completed work items solely because a later claim model is richer.
- Do not mix already-recorded WI042/F-015 worktree discovery or WI043/F-017 path semantics into
  this implementation.

## Adjacent upstream candidates to triage

The same ForgeWire audit also surfaced potentially generic mechanisms that should be explicitly
dispositioned during this investigation rather than silently forgotten or bundled into one
feature:

1. **Known-breach shrink-only ratchets** — freeze an explicit baseline of existing violations,
   reject growth and stale entries, then transition to a zero-tolerance guard when the baseline
   reaches zero.
2. **Canonical-record / human-representation parity** — related to RepoPact finding F-016
   (README/manifest representation drift).
3. **Concurrent work-item identifier allocation** — the cross-branch collision class observed in
   the ForgeWire WI230/WI237 field case.
4. **Fail-closed success semantics** — whether operations that did not persist/deliver/execute may
   ever return a success-shaped result, and whether this belongs inside completion-claim semantics
   or deserves a separate generic contract.

Each candidate must end this item with a durable disposition: fold into the generic claim model,
spawn a separate proposed RepoPact work item, link to an existing item/finding, or reject as
adopter-specific with rationale.

## Implementation ordering

Research and contract reconciliation come first. Schema, validator, template, conformance, or
CLI changes may begin only after the field cases have been classified and the generic record
shape has been chosen explicitly. The implementation must remain additive/opt-in for existing
repositories unless a separate breaking-version decision is accepted.

## Closeout

Each acceptance criterion in `work-item.json` must have linked concrete evidence. Closeout must
prove the chosen model with both ForgeWire-shaped field cases and at least one non-ForgeWire
fixture so the kernel is demonstrably not overfit to its progenitor adopter. If a new research
hypothesis/property is warranted, preregistration precedes any benchmark result claimed in its
support.
