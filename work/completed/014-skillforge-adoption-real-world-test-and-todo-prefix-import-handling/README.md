# 014 — SkillForge adoption (real-world test) and TODO-prefix import handling

> **Status**: ✅ Complete
> **Owners**: tooling (lead).
> **Depends on**: 011 (plan import), 013 (doctor).

## Intent

A third real-world adoption test on an independent, different-domain project
(SkillForge Academy, an offline-first Tauri certification-learning app) to exercise the
full lifecycle and surface format gaps the earlier subjects didn't.

## Scope

- `scripts/plan_import.py` — `_split_num` strips a leading tracker prefix
  (`TODO-`/`TASK-`/`ISSUE-`) so flat `TODO-NNN-*.md` files keep their numbers.
- `tests/test_validate_repo.py` — `test_split_num_strips_tracker_prefix`.
- (SkillForge repo, separate) branch `adopt-repopact`: the adopted tree.

## Closeout

Satisfied by evidence run `20260616-102316-skillforge-adoption`: 45 tests pass; the
SkillForge repo ran adopt → import-plan → doctor to a conformant RepoPact (14 active,
9 completed), with `todos/TODO-001..007` mapped to clean `work/active/001..007` and the
`ROADMAP.md` Shipped/Next sections imported; existing planning artifacts preserved.
Finding F-012 (holds); threats T1 further narrowed. Capture:
[`research/captures/011-skillforge-adoption.md`](../../../research/captures/011-skillforge-adoption.md).
