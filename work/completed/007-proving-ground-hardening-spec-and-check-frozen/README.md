# 007 — Proving-ground hardening: spec and check-frozen

> **Status**: ✅ Complete
> **Owners**: tooling (lead); governance (decisions).
> **Depends on**: none.

## Intent

Close the two defects the proving-ground evaluation surfaced in the *packaged*
product (see [`research/`](../../../research/)), so the CLI surface is closed under
its own `init` output and the frozen-surface gate is honest locally as well as in
CI. This is the hardening pass that gates the 1.0 declaration (decision `0007`).

## Decisions

- `spec` is a maintainer command, not an adopter command — decision
  [`0006`](../../../decisions/0006-spec-is-a-maintainer-command.md).
- Declare 1.0.0 on the strength of the adopter evidence — decision
  [`0007`](../../../decisions/0007-declare-1-0-0-on-adopter-evidence.md),
  superseding `0005`.

## Scope

- `scripts/repopact_cli.py` — `spec` degrades cleanly when no `SPEC.md` (F-001).
- `scripts/check_frozen_surface.py` — union committed, staged, and working-tree
  diffs so uncommitted protected-path edits are caught (F-002).
- `tests/test_validate_repo.py` — two regression tests.
- `decisions/0006`, `decisions/0007`.

## Closeout

All three acceptance criteria are satisfied by evidence run
`20260615-203707-proving-ground-hardening` (30 tests pass; both fixes verified).
On transition the directory moves to `work/completed/` and the dashboard regenerates.
