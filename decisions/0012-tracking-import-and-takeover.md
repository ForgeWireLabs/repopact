---
id: 0012
title: Import Tracking-Style Governance, and Take Over Legacy Methods
status: accepted
date: 2026-06-16
supersedes: []
---

# 0012: Import Tracking-Style Governance, and Take Over Legacy Methods

## Context

Two gaps remained after `import-plan` (decision 0010): some teams plan with a
*governance folder* (a decision log, risk register, milestone list, status snapshot,
work log) rather than a todo list (SkillForge's `tracking/`), and after migration the
old method sits beside `work/`, which is confusing. RepoPact needs to (a) adapt to
tracking-style systems and (b) let RepoPact fully take over, retiring the old method.

## Decision

### Tracking import (folded into `import-plan`)

A `tracking/` directory maps to *different* record types, because its files are not
all plan items:

- `decisions.md` (DEC-NNN blocks) -> `decisions/NNNN-*.md` ADRs. RepoPact decision ids
  must be numeric (≥4 digits), so DEC-NNN is reallocated to a free 4-digit id with the
  original DEC-NNN preserved in the body; status/date parsed, defaulting to accepted/today.
- `risks.md` (RISK-NNN table) -> `audits/findings/NNN-*.json`. A risk is an open
  finding. The finding schema requires a numeric id, so RISK-NNN is preserved in the
  `source` field and prefixed into `observed`; severity P1/P2/P3 -> high/medium/low;
  status -> open/reconciled/accepted; scope defaults to `governance`.
- `milestones.md` (M-N) -> `work/` items (shipped -> completed/waived, else active).

`status.md` (derived snapshot), `work-log.md` (history), and per-initiative trackers
are **not** fabricated into records — RepoPact has no honest home for them — and are
left for review/archive. Imports are non-destructive and idempotent (keyed on `source`
provenance / a body marker).

### `repopact takeover`

`repopact takeover [--delete] [--dry-run]` retires a legacy plan directory, **archiving**
it under `archive/` (default) or **deleting** it (`--delete`), but only when:

1. the repository validates, and
2. every plan item in that directory is provably represented as a work item (matched
   via `source` provenance).

A directory with any un-migrated item is left intact and reported (run `import-plan`
first). Mixed-content or partially-representable sources (`tracking/`, `ROADMAP.md`,
`CHANGELOG.md`) are reported for review, never auto-retired, so nothing un-captured is
lost. Re-validates after changes.

## Alternatives considered

- **Fabricate records for `status.md`/`work-log.md`.** Rejected: a status snapshot and
  an activity log are not work items, decisions, or evidence; inventing records would
  violate the project's honesty rule (cf. 0008/0010).
- **Have `takeover` delete by default.** Rejected: archiving is reversible and keeps
  history; deletion is opt-in.
- **Auto-archive `tracking/` wholesale.** Rejected as automatic: it holds
  non-representable content; the operator decides (archive is offered, not forced).

## Consequences

- Governance-folder adopters get their decisions, risks, and milestones as first-class
  RepoPact records; `work/` becomes the single ledger after `takeover`.
- Proven on SkillForge: `tracking/` imported (3 decisions, 5 findings, 6 milestones),
  `todos/` archived, repository validates.
- Non-GitHub trackers and richer status/work-log mapping remain future work.
