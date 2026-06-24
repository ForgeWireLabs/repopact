---
id: 0017
title: Add deferred and rejected decision statuses
status: accepted
date: 2026-06-23
supersedes: []
---

# 0017: Add deferred and rejected decision statuses

## Context

A decision record's `status` was constrained to
`proposed | accepted | superseded | deprecated` (in `scripts/validate_repo.py`,
`scripts/track_import.py`, and `schemas/record-frontmatter.schema.json`). Two
common dispositions had no home in that vocabulary:

- **deferred** — the choice was considered and is coherent, but is deliberately
  postponed, with stated conditions for revisiting it.
- **rejected** — the choice was considered and decided against.

Neither maps onto the existing four. `proposed` means "awaiting a decision," but a
deferral or a rejection **is** a decision. `deprecated` implies prior adoption being
phased out, so it cannot describe a proposal that was never adopted. `superseded`
requires a replacement record. The result was that a deferred or rejected proposal
had to be filed as `proposed` plus a prose note explaining the real disposition —
the README contradicting its own front matter.

The asymmetry made the gap concrete. RepoPact already treats `deferred` as a
first-class **work-item lifecycle** state (`active | blocked | deferred | completed`;
`plan_import` even maps "Later / future" sections to `deferred`). Decisions and work
items are the two durable record types; canonizing `deferred` for one and not the
other was an arbitrary inconsistency. A downstream adopter (ForgeLink) hit exactly
this: a deferred channel-design decision had no valid status to carry it.

## Decision

Extend the decision `status` vocabulary to
`proposed | accepted | rejected | deferred | superseded | deprecated` in all three
authoritative locations: `DECISION_STATUSES` in `scripts/validate_repo.py` and
`scripts/track_import.py`, and the decision `status` enum in
`schemas/record-frontmatter.schema.json`.

Definitions:

- **deferred** — a decision to not pursue the proposal now, kept for its reasoning,
  with stated reactivation criteria. Distinct from `proposed` (no decision has been
  made yet).
- **rejected** — a decision to not pursue the proposal at all. Distinct from
  `deprecated` (which retires a previously accepted decision).

The schema is on the frozen surface (INV-6); this change ships with explicit
operator approval.

## Alternatives considered

- **Keep `proposed` plus a prose note.** Rejected: it overloads `proposed` to mean
  both "awaiting decision" and "decided against / postponed," and forces readers to
  parse prose to learn the disposition the status field should carry.
- **Reuse `deprecated` for both.** Rejected: `deprecated` implies the decision was
  once accepted; a never-adopted proposal cannot be deprecated, and it conflates
  "postponed" with "rejected."
- **Add only `deferred`.** Rejected: `rejected` is the same one-line change and the
  same class of gap (a recordable decision the vocabulary could not express); adding
  both completes the lifecycle in one coherent change.
- **Model deferral/rejection as `superseded` by a tombstone record.** Rejected:
  heavyweight, and `superseded` implies a concrete replacement that a deferral or
  rejection does not have.

## Consequences

- Decision records can express deferral and rejection as first-class statuses,
  matching the work-item lifecycle's treatment of `deferred`.
- The decision vocabulary stays small and each value is distinct: `proposed`
  (pending), `accepted` (adopted), `rejected` (decided against), `deferred`
  (postponed with revisit criteria), `superseded` (replaced), `deprecated` (retired
  after adoption).
- Adopters carrying a local patch for `deferred` (e.g. ForgeLink) can drop it on
  their next re-vendor once they pick up this release.
- No existing record changes status; the four prior values remain valid.
