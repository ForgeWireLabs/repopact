# 017 — Add deferred and rejected decision statuses

> **Status**: ✅ Complete
> **Owners**: governance-owner (lead); tooling-owner (validator + tests).
> **Depends on**: none.

## Intent

Extend the decision `status` vocabulary from
`proposed | accepted | superseded | deprecated` to also include **deferred** and
**rejected**. Both are common, distinct dispositions the four prior values could not
express: a deferral or rejection *is* a decision, so `proposed` is wrong; neither was
ever adopted, so `deprecated` is wrong; and neither has a replacement, so
`superseded` is wrong.

In scope: the three authoritative status locations and a validator test. Out of
scope: any change to the work-item lifecycle (which already has `deferred`) or to
existing records' statuses.

## Decisions

Promoted to [`decisions/0017-add-deferred-and-rejected-decision-statuses.md`](../../../decisions/0017-add-deferred-and-rejected-decision-statuses.md):
the vocabulary change and its rationale (the work-item/decision asymmetry), with
rejected alternatives. The decision `status` enum lives in
`schemas/record-frontmatter.schema.json`, which is on the frozen surface (INV-6);
this change ships with explicit operator approval.

## Scope

- `scripts/validate_repo.py` — `DECISION_STATUSES`.
- `scripts/track_import.py` — `DECISION_STATUSES`.
- `schemas/record-frontmatter.schema.json` — decision `status` enum (frozen surface).
- `tests/test_validate_repo.py` — `test_decision_status_accepts_deferred_and_rejected`.
- `decisions/0017-*.md` — rationale.
- `decisions/README.md` — documents the extended status vocabulary.
- `VERSION` — 1.7.0 → 1.8.0 (additive vocabulary; minor bump).

## Closeout

| Criterion | Evidence |
| --- | --- |
| AC-1 vocabulary extended in all three locations | [20260623-deferred-rejected-statuses](../../../evidence/runs/20260623-deferred-rejected-statuses.json) |
| AC-2 validator test covers the new statuses | [20260623-deferred-rejected-statuses](../../../evidence/runs/20260623-deferred-rejected-statuses.json) |
| AC-3 decision recorded; all gates pass | [20260623-deferred-rejected-statuses](../../../evidence/runs/20260623-deferred-rejected-statuses.json) |

All acceptance criteria are satisfied with evidence; the directory now lives in
`work/completed/`.
