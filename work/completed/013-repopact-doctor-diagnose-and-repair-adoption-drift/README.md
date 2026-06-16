# 013 — repopact doctor: diagnose and repair adoption drift

> **Status**: ✅ Complete
> **Owners**: tooling (lead).
> **Depends on**: 008 (brownfield adoption).

## Intent

Close finding F-011: adoption drifts over time, and nothing detected or guided the
repair (ForgeLink was reconciled by hand, twice). Provide a one-command drift check and
guided, safe repair.

## Decisions

- Checks, the read-only/`--fix` split, and the two safety rules (never overwrite a
  differing schema; only drop already-invalid registry entries) — decision
  [`0011`](../../../decisions/0011-repopact-doctor-diagnoses-drift.md).

## Scope

- `scripts/doctor.py` — diagnostics + `--fix`.
- `scripts/repopact_cli.py` — `repopact doctor --root <repo> [--fix]`.
- `pyproject.toml` — ship `doctor` in the wheel.
- `tests/test_validate_repo.py` — 3 regression tests.

## Closeout

Satisfied by evidence run `20260616-100320-repopact-doctor`: 44 tests pass; a drifted
synthetic repo is diagnosed and `--fix`-ed to valid; real ForgeLink reports healthy
with its intentional `preflight` schema extension flagged for review (not clobbered).
Capture: [`research/captures/010-repopact-doctor.md`](../../../research/captures/010-repopact-doctor.md).
