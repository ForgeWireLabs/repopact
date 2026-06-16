# 005 — repopact console entry point

> **Status**: ✅ **CLOSED 2026-06-15.** Evidence `20260615-repopact-cli`. Closes issue #2: adopters can `pipx install` RepoPact and run `repopact init` instead of `python scripts/init_repo.py`.
> **Owners**: Tooling (lead), Docs.
> **Depends on**: 003 ✅ (init/templates), 004 ✅ (SPEC/generators).
> **Frozen surface**: none touched.

## Intent

Make RepoPact installable as a command so the adoption path is `pipx install …` →
`repopact init`, not a copied script invocation. The CLI must operate on the
*user's* repository, and `init` must work from an installed wheel with no source
checkout present.

## What shipped

- `pyproject.toml` — setuptools build, console script `repopact = repopact_cli:main`,
  dynamic version from `VERSION` (no drift), `jsonschema` dependency. Seed
  `schemas/*.json` and `templates/*` ship as data-files under
  `share/repopact/` — referencing the canonical top-level files, so there is **one
  source of truth** (policy 001), not a duplicated copy.
- `scripts/repopact_cli.py` — dispatcher for `init`, `validate`, `new`,
  `dashboard`, `spec`, `check-frozen`.
- Root-awareness: `generate_spec.render`, `new.new_*`, and `init_repo` take an
  explicit root (default preserved for loose-script mode), so the installed
  command targets the cwd/`--root` repository rather than the install location.
- `init_repo` resolves seed content from a checkout **or** the installed
  `share/repopact/` data, and copies the tooling modules by explicit list.

## Decisions in flight

- **Dual-mode, not a rewrite.** Kept the loose scripts working
  (`python scripts/validate_repo.py`) so repos that vendor the scripts via `init`
  are unaffected; the CLI is an additive convenience.
- **data-files over duplication.** Seed schemas/templates are shipped by reference
  to the top-level canonical files, avoiding a second copy that would rot.

## Verification

`pip install .` into a clean venv, then `repopact init --target <tmp>` →
`repopact validate` passes; `repopact new work-item` stamps and re-validates.
In-repo: validator green, tests 24 → 26, loose-script usage and CI gates unchanged.

## Closeout

All acceptance criteria satisfied with evidence `20260615-repopact-cli`. Issue #2
closed.
