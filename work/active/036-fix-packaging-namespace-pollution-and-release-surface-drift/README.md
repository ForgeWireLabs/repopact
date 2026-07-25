# 036 — Fix packaging namespace pollution and release-surface drift

> **Status**: 🚧 In progress — AC-3, AC-4, AC-7 landed; AC-1, AC-2, AC-5, AC-6 open.
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
   `import frontmatter` resolves to RepoPact's module. A wheel built from this
   commit lists all seventeen names in `dist-info/top_level.txt`, which is the
   authoritative statement of what the package claims in the global namespace.
   The fix is a real `repopact/` package with submodules, keeping only the
   `repopact` console script public.

   A secondary hazard sits behind the same fix: `py-modules` is a hand-maintained
   list that must track `scripts/*.py` by memory. It is in sync as of this commit
   (verified 17/17), and the published 2.3.0 wheel is internally consistent — a
   stale local `dist/` artifact missing `validate_research.py` was checked and
   ruled out, since that module postdates the 2.3.0 tag. But nothing enforces the
   parity, so omitting a new module would ship a wheel that raises
   `ModuleNotFoundError` on import. A package layout discovers submodules
   automatically and removes the failure mode rather than documenting it.
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
5. **Seeded repositories could not run their own validator** (found while fixing
   1–3; live on `main`, unreleased, fixed in this work item). `init_repo.MODULES`
   lists the tooling vendored into a new repository. `validate_repo.py` is on that
   list; `validate_research.py`, which it imports at module scope, was not. Every
   repository created by `repopact init` since commit `7597ebb` therefore carried
   a validator that died with `ModuleNotFoundError` on import instead of
   reporting findings — verified by bootstrapping a repo and running its
   `scripts/validate_repo.py` as a subprocess.

   The existing `test_bootstrap_produces_valid_repo` could not catch this: it
   calls `validate(target)` **in-process**, where the parent checkout is already
   on `sys.path`, so the vendored copy's imports resolve regardless of whether
   they were copied. It proved the seeded *records* were valid while the seeded
   *tooling* was broken. The new test runs the seeded validator as a subprocess
   with `PYTHONPATH` cleared, and fails without the fix.

   **This couples 5 to 1, and orders the work.** A repo seeded from an *installed*
   wheel did not exhibit the crash, because `validate_research` resolves out of
   site-packages — the namespace pollution of item 1 is precisely what masks the
   vendoring bug. Verified in a clean venv: the seeded `scripts/` had no
   `validate_research.py`, yet its validator passed, importing the module from
   `site-packages/validate_research.py`. Once the modules become submodules of a
   `repopact` package, that accidental rescue disappears and any gap in
   `MODULES` becomes a hard failure for every adopter. The vendoring closure must
   therefore be correct and tested **before** the package layout lands.

6. **Ungoverned paths (found while fixing 1–3, and the root cause of both).**
   82 of 323 tracked files — 25% of the repository — match no scope in
   `governance/owners.json`. `README.md` and `pyproject.toml`, the two files
   carrying the defects above, are both among them, as are `VERSION`,
   `CONFORMANCE.md`, and all of `conformance/`, `audits/`, and `templates/`.
   `AGENTS.md` states "every change belongs to one explicit owner scope" as a
   binding expectation, and nothing checks it. The specific drift was possible
   because the general guarantee was never enforced.

Out of scope: restoring remote CI (blocked work item `032`) and semantic ledger
freshness (`033`) — this item must not duplicate them.

## Decisions

- Converting flat modules to a package changes no public API (the only published
  entry point is the `repopact` console script), so this is a minor release,
  not a major one. If review disagrees, promote that discussion to `decisions/`.

## Open question — how self-contained is a seeded repository?

Surfaced by the import-closure test, and needing a maintainer answer before AC-1
lands. `init_repo.MODULES` vendors ten modules, and the vendored `repopact_cli`
imports `doctor`, `adopt_repo`, `plan_import`, `takeover`, and `fleet_verify`
*lazily*, inside their command branches. So the vendored CLI loads, but those five
subcommands fail in a seeded repo that does not also have RepoPact installed.

That is not a crash and may well be intended — the supported path is presumably
the installed `repopact` console script, with the vendored copy covering only
validation and generation. But it is currently undocumented and unasserted, which
is how the `validate_research` gap survived. Either vendor the full command set,
or state the boundary in `SPEC.md` and assert it. Do not leave it implicit.

## Scope

Landed in this work item:

- `scripts/validate_repo.py`: `validate_release_surface` (decision `0028`), plus
  its `SPEC.md` rule 11, conformance rule `SPEC-4-release-surface`, and the
  `invalid/readme-release-drift` fixture.
- `scripts/init_repo.py`: `validate_research.py` added to `MODULES`.
- `README.md`, `ROADMAP.md`: repaired drift.
- `tests/test_validate_repo.py`: release-surface cases, vendored-module import
  closure, and the standalone seeded-validator subprocess test.

Still to change:

- `pyproject.toml`, `scripts/` → `repopact/` package layout, seed-data loading
  in `init_repo`/`adopt_repo`/`doctor` (AC-1, AC-2).
- `governance/owners.json` + a coverage rule (AC-6).
- `tests/`: shared bootstrap fixtures to cut wall time (AC-5).

## Acceptance criteria

- [ ] **AC-1** Wheel installs exactly one top-level import name (`repopact`); no
  generic top-level modules; verified by evidence run inspecting wheel contents.
- [ ] **AC-2** Seeds ship as package data via `importlib.resources`; `data-files`
  removed; `init`/`adopt` proven from a wheel install in a clean venv.
- [x] **AC-3** Validator enforces README release-line parity with `VERSION` and a
  resolving changelog link; current README repaired; regression test added.
- [x] **AC-4** ROADMAP reconciled with the released 2.3.0 line.
- [ ] **AC-5** Test suite under 2 minutes locally, or an accepted decision records
  why the cost stands.
- [ ] **AC-6** Every tracked path resolves to one owner scope or is declared
  unowned, enforced by a validator rule.
- [x] **AC-7** A `repopact init` repository runs its own vendored validator,
  proven by a subprocess regression test that fails without the fix.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
