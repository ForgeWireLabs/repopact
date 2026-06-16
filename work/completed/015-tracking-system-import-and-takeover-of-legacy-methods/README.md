# 015 — Tracking-system import and takeover of legacy methods

> **Status**: ✅ Complete
> **Owners**: tooling (lead).
> **Depends on**: 011 (plan import), 014 (SkillForge adoption).

## Intent

Close two gaps SkillForge surfaced: (1) teams that plan with a governance folder
(`tracking/`: decisions, risks, milestones, status, work-log) weren't adapted by
RepoPact; (2) after migration the old method sat beside `work/`, which is confusing.

## Decisions

- Tracking→record-type mapping, the honesty rules, and `takeover`'s provenance/
  validation gating — decision
  [`0012`](../../../decisions/0012-tracking-import-and-takeover.md).

## Scope

- `scripts/track_import.py` — decisions.md→decisions/, risks.md→audits/findings/,
  milestones.md→work/; status.md/work-log.md left for review.
- `scripts/plan_import.py` — calls track_import; skips scaffolding (TEMPLATE/index).
- `scripts/takeover.py` + `scripts/repopact_cli.py` — `repopact takeover [--delete]
  [--dry-run]`.
- `pyproject.toml` — ship `track_import`, `takeover`.
- `tests/test_validate_repo.py` — tracking-import + takeover regressions.

## Closeout

Satisfied by evidence run `20260616-105305-tracking-and-takeover`: 50 tests pass; on
real SkillForge, `tracking/` imported (3 decisions, 5 findings, 6 milestones) and
`todos/` archived to `archive/todos/`, leaving `work/` as the single ledger; validates,
doctor healthy. Capture:
[`research/captures/012-tracking-and-takeover.md`](../../../research/captures/012-tracking-and-takeover.md).
