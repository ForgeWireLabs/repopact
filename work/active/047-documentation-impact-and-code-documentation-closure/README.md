# 047 — Documentation Impact and Code/Documentation Closure

> **Status**: 🚧 Active — core completion gate implemented; mapping/staleness/CLI follow-ups open (see Closeout).
> **Owners**: governance-owner (lead); tooling-owner, docs-owner, evidence-owner, and work-coordinator affected.
> **Depends on**: none.
> **Decision**: [`0065`](../../../decisions/0065-documentation-closure-is-mandatory-at-work-item-completion.md).

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

### What landed (2026-09-18)

Decision [`0065`](../../../decisions/0065-documentation-closure-is-mandatory-at-work-item-completion.md) accepts the model and the completion gate is implemented and enforced by both engines:

- `documentation_impact` work-item field (schema-validated shape: `affected` needs `surfaces`+`evidence`, `none` needs `rationale`) — `repopact/schemas/work-item.schema.json`.
- Opt-in, date-epoch-grandfathered enforcement at the `completed` transition, mirroring decision 0021's preflight mechanism — `repopact/validate_repo.py` (Python) and `rust/crates/repopact-validation/src/lib.rs` (canonical Rust engine), both wired to `governance/owners.json`'s `documentation_impact` block.
- `SPEC.md` rule 17 documents the contract and its relationship to F-016/WI044/rule 14.
- Two new conformance cases (`documentation-impact-missing-on-completed`, `documentation-impact-affected-unknown-evidence`) prove the negative direction on both the Python and canonical-Rust comparators (37/37 conformance cases pass).
- This work item is itself the first `documentation_impact: "affected"` record, evidenced by `evidence/runs/20260918-047-documentation-closure-implementation.json`.

### What's still open (AC-7, AC-8, AC-10, AC-12, part of AC-14)

- Adopter-declared source-to-documentation mappings / structural freshness detection (AC-7) and generated-documentation staleness wired into `doctor` (AC-8, AC-10) are deliberately deferred to a follow-up decision — see decision 0065 section E. Implementing them now, with zero lived experience of what real `documentation_impact` records look like, risked exactly the unfalsifiable generic-analysis claim the decision disclaims.
- A second, explicitly adopter-neutral validation fixture beyond the conformance corpus (AC-12) is not yet built.
- `repopact new` does not yet scaffold a `documentation_impact` stub, and there is no dedicated CLI/doctor surfacing of *which* completed items are missing it ahead of the hard rejection (part of AC-14).
