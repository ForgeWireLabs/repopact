# 010 — Adopt hardening (F-008 guardrail) and independent OSS adoption evidence

> **Status**: ✅ Complete
> **Owners**: tooling (lead).
> **Depends on**: 008 (brownfield adoption).

## Intent

Close finding F-008 (an existing `.gitignore` silently un-tracking RepoPact records)
at the source, and obtain *independent* brownfield evidence — adoption of a repo with
no lineage to RepoPact — to offset the reflexivity caveat that forgewire is the
progenitor (see `research/threats-to-validity.md`).

## Scope

- `scripts/adopt_repo.py` — `gitignored_records()` runs `git check-ignore` on every
  written record; `_print_report()` warns and suggests `.gitignore` negations.
- `scripts/repopact_cli.py` — `adopt` uses the shared report printer.
- `tests/test_validate_repo.py` — `test_adopt_warns_on_gitignored_records`.

## Closeout

Satisfied by evidence run `20260615-215134-adopt-hardening`: 35 tests pass (guardrail
flags a `runs/` collision), and adopting the clean-room **pallets/flask** repo yields
a conformant RepoPact (F-009). Independent generality now has one datum; the
CODEOWNERS/nested-contract path still rests on forgewire (T1, open). Capture:
[`research/captures/006-independent-oss-adoption-flask.md`](../../../research/captures/006-independent-oss-adoption-flask.md).
