# 011 — Plan import: bring existing todos and checklists into work/

> **Status**: ✅ Complete
> **Owners**: tooling (lead).
> **Depends on**: 008 (brownfield adoption).

## Intent

Close the gap that `adopt` left a hollow `work/` next to a team's real planning (e.g.
forgewire's `todos/` tree). Detect the common ways developers track plan items and
import them into `work/` as work items — organized by lifecycle, honest about what is
actually proven, non-destructive, and idempotent.

## Decisions

- Sources, lifecycle mapping, the completed→`waived` honesty rule, non-destructive +
  idempotent contract — decision
  [`0010`](../../../decisions/0010-plan-import-maps-existing-trackers-to-work.md).

## Scope

- `scripts/plan_import.py` — detectors (plan directories + checklist files) and the
  importer; preserves source, records origin in a `source` field, remaps ID collisions.
- `scripts/repopact_cli.py` — `repopact import-plan --root <repo> [--dry-run]`.
- `pyproject.toml` — ship `plan_import` in the wheel.
- `tests/test_validate_repo.py` — 4 regression tests.

## Closeout

Satisfied by evidence run `20260616-091930-plan-import`: 39 tests pass, and forgewire's
`todos/` tree (~75 items) imports into a populated, conformant `work/` (21 active, 54
completed/waived, 1 deferred). Capture:
[`research/captures/007-plan-import-forgewire.md`](../../../research/captures/007-plan-import-forgewire.md).
