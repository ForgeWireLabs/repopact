# 047 — Documentation Impact and Code/Documentation Closure

> **Status**: 📋 Planning (proposed — not started)
> **Owners**: governance-owner (lead); tooling-owner, docs-owner, evidence-owner, and work-coordinator affected.
> **Depends on**: none.

## Intent

RepoPact currently requires governed work, acceptance criteria, evidence, reconciliation, and generated artifacts, but it does not yet require a code change to explicitly resolve documentation impact before closeout.

This creates a durable-state gap: implementation can change while user, API/CLI, configuration, architecture, operations, contributor, experimental-status, example, or generated documentation remains stale or its impact is simply never considered.

The target rule is:

> A work item that changes governed implementation code cannot complete until documentation impact is explicitly resolved. If documented behavior, interfaces, configuration, architecture, operations, maturity, or other durable contracts changed, the corresponding documentation must be created, updated, or regenerated and linked to concrete evidence. A no-documentation-impact outcome requires an explicit rationale; silence is not acceptable.

The purpose is not to force meaningless README churn for every internal refactor. The purpose is to make documentation impact a required closeout decision rather than an optional afterthought.

## Relationship to existing RepoPact work

This item is distinct from:

- **F-016** — parity between a work-item README and its canonical manifest;
- **WI044** — whether a completion/cutover claim has semantically sufficient evidence;
- **WI046** — whether verification is invoked/effective at an admission boundary.

WI047 adds a different closure dimension: whether the repository's durable explanation remains reconciled with changed implementation.

## Candidate model — not yet a decision

A governed code change should close in one of two states:

1. **documentation affected** — identify the affected documentation surfaces and prove they were created, updated, or regenerated; or
2. **documentation not affected** — record an explicit reviewable rationale.

An unresolved or omitted documentation-impact state must not permit a work item containing governed code changes to transition to completed.

Potential documentation surfaces include user behavior, public API/CLI, configuration, architecture/decisions, operations/runbooks, contributor/developer workflow, experimental or maturity status, examples, and generated documentation. Adopters remain free to define their own paths and layouts.

RepoPact should evaluate optional source-to-documentation mappings or equivalent relationships so it can detect structural freshness risks without pretending generic static analysis can prove arbitrary prose semantically correct.

## Non-goals

- Do not require arbitrary Markdown edits merely because source code changed.
- Do not assume every internal refactor affects documentation.
- Do not make README.md the universal documentation target.
- Do not duplicate F-016 representation-parity work.
- Do not make a particular CI provider the source of truth for documentation closure.
- Do not claim generic source analysis can prove that prose is semantically correct.
- Do not retroactively invent documentation-impact decisions for historical completed work.

## Implementation ordering

Contract and representation design come first. The work must compare possible placement at work-item, acceptance-criterion, evidence, and dedicated mapping/impact-record layers before changing schemas or validators.

After a design is accepted, implementation should cover validation, templates, workflow guidance, doctor/audit behavior, conformance, generated documentation freshness, evidence linkage, and negative tests proving unresolved documentation impact actually blocks closeout.

## Closeout

Every acceptance criterion in `work-item.json` must be linked to concrete evidence. Closeout must include both a positive code-plus-documentation case and a justified no-documentation-impact case, plus negative proof that code changes with unresolved documentation impact are rejected.
