# 012 — Plan import sources: section roadmaps and GitHub issues

> **Status**: ✅ Complete
> **Owners**: tooling (lead).
> **Depends on**: 011 (plan import).

## Intent

Broaden `import-plan` to the remaining common ways teams track plan items, beyond
todo-directories and checkboxes: roadmap/backlog files organized by `##` sections
(no checkboxes), and external GitHub issues.

## Scope

- `scripts/plan_import.py`:
  - section parsing — a `##` heading whose text matches Now/Next/In-progress (active),
    Later/Future/Someday (deferred), Blocked/On-hold (blocked), Done/Shipped/Released
    (completed) classifies the plain bullets beneath it. Top-level bullets only.
  - `collect_from_github()` — opt-in `--issues`, best-effort via `gh issue list`;
    open → active, closed → completed/waived; safe no-op without gh/remote.
- `scripts/repopact_cli.py` — `import-plan --issues`.
- `tests/test_validate_repo.py` — section-roadmap + keyword-mapping tests.

## Closeout

Satisfied by evidence run `20260616-093218-plan-import-sources`: 41 tests pass, and a
single `import-plan --issues` pass on repopact drew items from both `ROADMAP.md`
sections and GitHub issues. Fulfills the "next sources" noted in decision 0010.
Capture: [`research/captures/008-plan-import-sources.md`](../../../research/captures/008-plan-import-sources.md).
