# 008 — Brownfield adoption: convert an existing repo into a RepoPact

> **Status**: ✅ Complete
> **Owners**: tooling (lead); governance (decisions).
> **Depends on**: none.

## Intent

`init` only serves greenfield repos. Real adopters have an existing project with
ownership (CODEOWNERS), enforcement (CI workflows), contracts (nested `AGENTS.md`),
and history they will not throw away. Add `repopact adopt` to bring such a repo
under RepoPact by mapping those existing signals into records, non-destructively.

## Decisions

- The mapping and its non-destructive, dry-runnable contract — decision
  [`0008`](../../../decisions/0008-brownfield-adoption-maps-existing-signals.md).

## Scope

- `scripts/adopt_repo.py` — the adopter: CODEOWNERS → scopes/roles, workflows →
  binding-gate policies + `INV-2` + frozen surface, nested contracts → registry
  (with stubbed `_audit` triplets), git history → first evidence run + completed
  `000-adopt` work item.
- `scripts/repopact_cli.py` — `repopact adopt --target <repo> [--dry-run]`.
- `pyproject.toml` — ship `adopt_repo` in the wheel.
- `tests/test_validate_repo.py` — 4 adopt regression tests.

## Closeout

Satisfied by evidence run `20260615-211715-brownfield-adoption`: 34 tests pass, and
adopting the real **forgewire** repository (4569 commits, 19 nested contracts, 7
CODEOWNERS scopes, 4 CI gates) produces a tree that validates as a conformant
RepoPact. Raw capture: [`research/captures/004-brownfield-forgewire.md`](../../../research/captures/004-brownfield-forgewire.md).
