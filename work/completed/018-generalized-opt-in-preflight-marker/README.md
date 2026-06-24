# 018 — Generalized opt-in preflight marker

> **Status**: ✅ Complete
> **Owners**: governance-owner (lead); tooling-owner (validator + tests).
> **Depends on**: none.

## Intent

Make the "preflight marker" guardrail (a work item was recorded before
implementation started) a first-class, **opt-in, default-off** RepoPact feature, so
adopters no longer need to fork the validator to get it. Lift it out of ForgeLink's
vendored patch and generalize the threshold (id and/or date).

In scope: schema property, config read, validator check, tests. Out of scope:
enabling preflight for RepoPact's own repo (its items predate the guardrail).

## Decisions

Promoted to [`decisions/0018-generalized-opt-in-preflight-marker.md`](../../../decisions/0018-generalized-opt-in-preflight-marker.md):
opt-in, default-off, configured in `governance/owners.json`, with `required_from_id`
and/or `required_from_date`. The schema change touches the frozen surface (INV-6) and
ships with operator approval.

## Scope

- `schemas/work-item.schema.json` — optional `preflight` property (frozen surface).
- `scripts/validate_repo.py` — `_preflight_config`, `_preflight_required`,
  `validate_work_preflight`, wired into `validate_work`.
- `tests/test_validate_repo.py` — off-by-default, id threshold, marker-satisfies,
  date threshold.
- `VERSION` — 1.8.0 → 1.9.0 (additive opt-in feature; minor bump).

## Closeout

| Criterion | Evidence |
| --- | --- |
| AC-1 schema preflight property | [20260624-generalized-preflight](../../../evidence/runs/20260624-generalized-preflight.json) |
| AC-2 config + validator check | [20260624-generalized-preflight](../../../evidence/runs/20260624-generalized-preflight.json) |
| AC-3 tests + gates | [20260624-generalized-preflight](../../../evidence/runs/20260624-generalized-preflight.json) |

All acceptance criteria satisfied with evidence; directory moved to `work/completed/`.
