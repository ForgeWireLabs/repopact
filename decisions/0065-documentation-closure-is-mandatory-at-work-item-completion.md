---
id: 0065
title: Documentation closure is mandatory at work-item completion
status: accepted
date: 2026-09-17
supersedes: []
---

# 0065: Documentation closure is mandatory at work-item completion

## Context

RepoPact already requires governed work, acceptance criteria, evidence,
provenance, and reconciliation (decisions 0018/0021 and others), but nothing
required a code change to explicitly resolve its documentation impact before
closeout (work item 047). Implementation could change while user-visible
behavior, API/CLI surface, configuration, architecture, operations,
contributor workflow, experimental/maturity status, examples, or generated
documentation went stale, and closeout never forced a decision about it
either way.

WI047 is a distinct closure dimension from three existing mechanisms it must
not duplicate:

- **F-016 (README/manifest parity)** checks that a work item's README
  checkboxes agree with its machine-readable acceptance criteria — a
  representation-consistency check between two records of the *same* work
  item, not a check that external documentation reflects the change.
- **WI044 (completion-claim evidence)** governs whether a *satisfied*
  acceptance criterion has semantically sufficient evidence at all.
- **Rule 14 / WI046 (verification contracts)** governs whether a declared
  local verification profile was invoked and effective at an admission
  boundary.

None of the three asks "does the repository's durable explanation still
match this governed change?" WI047 adds that question as a fourth,
independent closure gate.

## Decision

### A. `documentation_impact` is a new optional work-item field

A work item may carry:

```json
"documentation_impact": {
  "state": "affected",
  "surfaces": ["user_behavior", "configuration"],
  "evidence": ["20260917-wi047-doc-closure"]
}
```

or

```json
"documentation_impact": {
  "state": "none",
  "rationale": "Internal refactor; no documented behavior, interface, configuration, or contract changed."
}
```

`state: "affected"` requires at least one named surface (drawn from a fixed,
adopter-neutral enum: `user_behavior`, `api_cli`, `configuration`,
`architecture`, `operations`, `contributor_workflow`, `maturity_status`,
`examples`, `generated`) and at least one linked evidence run proving those
surfaces were created, updated, or regenerated — a bare "docs updated"
statement is not evidence. `state: "none"` requires a non-empty reviewable
rationale. This shape is schema-enforced (`work-item.schema.json`) whenever
the field is present, independent of whether presence itself is required.

The surface enum is deliberately generic: it classifies *kinds* of durable
contract, not repository layout. RepoPact does not mandate that
`README.md`, or any other specific path, is the universal documentation
target; adopters keep their own layout.

### B. Presence is mandatory at completion, grandfathered like preflight

Mirroring decision 0021's preflight mechanism exactly:

- The validator's `_documentation_impact_required` defaults to **disabled**
  (an adopter opts in via `governance/owners.json`).
- RepoPact's own `owners.json` sets `documentation_impact.enabled: true,
  required_from_date: "2026-09-17"` (this decision's acceptance date) —
  every item created on or before that date, including WI047 itself and
  every work item already completed through WI069, is grandfathered. Only
  a work item *created after* the epoch must resolve documentation impact
  before completing. A date epoch, not an id epoch, was chosen because
  several work items above id 046 (048, 049, 051-062, 064-069) were
  already `completed` by the time this decision was written — an id
  threshold at 047 would have retroactively broken them, which AC-13
  explicitly forbids. This mirrors how `doctor`/`adopt` grandfather
  preflight (decision 0021) for upgrading adopters via
  `required_from_date` rather than `required_from_id`.
- The rule fires only at the `completed` transition (rule 9's intent:
  closure, not perpetual mid-flight nagging). A `proposed`, `active`,
  `blocked`, or `deferred` item may omit the field entirely.
- When `state: "affected"`, linked evidence IDs must resolve to a known
  evidence run, exactly like acceptance-criterion evidence links.

### C. No CI provider becomes the source of truth

WI046's verification checkpoints may *invoke* this same rule as one of their
steps, but the rule itself is enforced by the validator against the
work-item record, not by any hosted CI configuration. A repository with no
CI at all still gets the guarantee locally via `repopact validate`/`doctor`.

### D. Non-goals stay non-goals

- No arbitrary Markdown edit is required merely because source changed.
- No internal refactor is assumed to affect documentation by default; a
  justified `"none"` closes it.
- No specific file is elevated to "the" documentation target.
- Generic source analysis is never claimed to prove prose is semantically
  correct — the mechanism proves a documentation-impact *decision* was made
  and evidenced, not that the resulting prose is accurate.
- Historical `completed` work is not retroactively rejected; grandfathering
  is the compatibility boundary (AC-13).

### E. Source-to-documentation mapping and generated-documentation staleness are future work, not this decision

WI047's AC-7 (adopter-declared source-to-documentation mappings, for
structural freshness detection analogous to decision 0055's
`documentation_refs[].claim_basis`) and AC-8/AC-10 (generated-documentation
staleness detection wired into `doctor`) are deferred to a follow-up
decision once real usage data exists on what "affected" declarations look
like in practice. Forcing that design now, ahead of any lived experience
with the simpler completion gate, risks exactly the kind of unfalsifiable
generic-analysis claim this decision explicitly disclaims (D above).
Shipping the completion gate first, and observing how `documentation_impact`
records accumulate across 047+, is the evidence base the mapping design
needs.

## Alternatives considered

- **Make README.md the universal target.** Rejected: adopters have their
  own documentation layouts (Diataxis, generated API docs, monorepo-local
  READMEs); a single-file mandate would not generalize.
- **Enforce via a dedicated mapping/impact-record layer instead of a
  work-item field.** Considered per AC-3. Rejected for the first cut:
  the work-item field reuses the existing evidence-linkage and schema
  machinery (like `preflight` and `provenance` before it) with no new
  record type, keeping the surface small until mapping-layer needs are
  proven by real `documentation_impact` usage (see E above).
- **Require documentation-impact resolution at every status transition,
  not just completion.** Rejected: it would nag on `proposed`/`active`
  work before there is anything to document, contradicting the "closure,
  not perpetual mid-flight nagging" framing in B.
- **Make presence mandatory with no epoch/grandfathering.** Rejected on the
  same "immediately breaks every existing adopter" grounds decision 0021
  rejected for preflight; the epoch mechanism is proven and reused as-is.

## Consequences

- `repopact validate`/`doctor` reject a `completed` work item created after
  the configured epoch date that lacks a resolved `documentation_impact`.
- Every work item created on or before the epoch, and repositories that
  never enable the field, are unaffected.
- WI047 itself predates its own epoch and is grandfathered like every
  other pre-epoch item, but records a `documentation_impact` anyway as
  demonstration evidence that the mechanism works end to end.
