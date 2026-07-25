# 036 — Fix packaging namespace pollution and release-surface drift

> **Status**: 📋 Planning
> **Owners**: tooling-owner (lead), governance-owner (release-surface rule), docs-owner (README/ROADMAP).
> **Depends on**: none.

## Intent

An external review (2026-07-25) of the repository found that the two surfaces
RepoPact presents to the outside world — the PyPI distribution and the release
narrative in README/ROADMAP — are the least governed parts of the project.

1. **Packaging namespace pollution.** `pyproject.toml` publishes seventeen flat
   `py-modules` at the top level of site-packages. Installing `repopact` drops
   modules named `frontmatter`, `doctor`, `new`, `takeover`, `init_repo`,
   `validate_repo`, ... into the global import namespace. `frontmatter` collides
   outright with the widely used `python-frontmatter` package (import name
   `frontmatter`); the rest are generic enough to shadow or be shadowed by other
   code. This was confirmed on the maintainer machine, where
   `import frontmatter` resolves to RepoPact's module. The fix is a real
   `repopact/` package with submodules, keeping only the `repopact` console
   script public.
2. **Seed-data mechanism.** Schemas/templates ship via setuptools `data-files`,
   a deprecated mechanism whose install location varies by installer — the root
   cause of the 2.0.2 user-site lookup bug. Moving seeds inside the package and
   resolving them with `importlib.resources` removes the whole failure class.
3. **Release-surface drift.** README states "current release **2.2.0**" and links
   the 2.2.0 changelog while `VERSION` is `2.3.0` and v2.3.0 is on PyPI.
   ROADMAP still describes the `proposed` lifecycle state as unreleased, though
   it shipped in 2.1.0. The validator checks dashboard and SPEC parity but not
   these hand-written release claims — drift the tool exists to catch, in its own
   front door. The fix is a validator rule, not a one-off edit.
4. **Test-suite cost.** The suite passes (116 tests) but takes ~5.5 minutes
   locally because most tests bootstrap a full repository per test. That tax is
   paid on every CI run and every local loop.

Out of scope: restoring remote CI (blocked work item `032`) and semantic ledger
freshness (`033`) — this item must not duplicate them.

## Decisions

- Converting flat modules to a package changes no public API (the only published
  entry point is the `repopact` console script), so this is a minor release,
  not a major one. If review disagrees, promote that discussion to `decisions/`.

## Scope

- `pyproject.toml`, `scripts/` → `repopact/` package layout, seed-data loading
  in `init_repo`/`adopt_repo`/`doctor`.
- `scripts/validate_repo.py` + tests: release-surface parity rule.
- `README.md`, `ROADMAP.md`: repair current drift.
- `tests/`: shared bootstrap fixtures to cut wall time.

## Acceptance criteria

- [ ] **AC-1** Wheel installs exactly one top-level import name (`repopact`); no
  generic top-level modules; verified by evidence run inspecting wheel contents.
- [ ] **AC-2** Seeds ship as package data via `importlib.resources`; `data-files`
  removed; `init`/`adopt` proven from a wheel install in a clean venv.
- [ ] **AC-3** Validator enforces README release-line parity with `VERSION` and a
  resolving changelog link; current README repaired; regression test added.
- [ ] **AC-4** ROADMAP reconciled with the released 2.3.0 line.
- [ ] **AC-5** Test suite under 2 minutes locally, or an accepted decision records
  why the cost stands.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
