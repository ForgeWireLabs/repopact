---
id: 0029
title: Ship a repopact package and stop vendoring tooling into seeded repositories
status: accepted
date: 2026-07-25
supersedes: []
---

# 0029: Ship a repopact package and stop vendoring tooling into seeded repositories

## Context

Two defects in work item `036` turn out to be the same defect.

**The distribution pollutes its host.** `pyproject.toml` publishes seventeen flat
`py-modules`. A wheel built from `2.3.0` declares all seventeen in
`dist-info/top_level.txt`: `adopt_repo`, `check_frozen_surface`, `doctor`,
`fleet_verify`, `frontmatter`, `generate_dashboard`, `generate_spec`,
`init_repo`, `new`, `plan_import`, `repo_model`, `repopact_cli`,
`run_conformance`, `takeover`, `track_import`, `validate_repo`,
`validate_research`. Installing RepoPact therefore claims seventeen generic names
in the global import namespace of the environment. `frontmatter` collides
outright with the widely used `python-frontmatter` distribution, whose import
name is also `frontmatter`; on the maintainer's machine `import frontmatter`
already resolves to RepoPact's module. A governance tool that corrupts the
environment it is installed into has an evidence problem, not a style problem.

**Seeded repositories carry a copy of the tooling.** `init_repo.MODULES` vendors
ten modules into every repository created by `repopact init`, and the adopter
docs teach `python scripts/validate_repo.py` as the way to run them. That copy is
a second distribution channel with no packaging, no versioning, and no tests
against its real execution mode — which is exactly how the `validate_research`
gap (work item `036`, AC-7) survived: the vendored validator imported a module
`MODULES` did not copy, and every seeded repository since commit `7597ebb`
carried a validator that died on import.

The two are coupled. A repository seeded from an *installed* wheel did not
exhibit that crash, because `validate_research` resolved out of site-packages.
The namespace pollution is what masked the vendoring bug. Fixing the packaging
without addressing the vendoring would convert a hidden failure into an
immediate one for every adopter.

## Decision

Ship RepoPact as a single `repopact` package, and stop vendoring tooling.

1. The seventeen modules become submodules of a `repopact` package with relative
   imports. The wheel declares exactly one top-level name. `repopact_cli` becomes
   `repopact.cli`; the console script is unchanged.
2. `repopact init` no longer copies tooling into the target repository. A seeded
   repository contains records, schemas, and templates — its state — and runs the
   tooling from the installed `repopact` command.
3. This breaks the documented `python scripts/validate_repo.py` interface, so it
   ships as **3.0.0**, and the documentation moves to `repopact validate`,
   `repopact new`, `repopact dashboard`.

The operator selected both the vendoring decision and the major version
explicitly, having been shown the alternatives below.

## Alternatives considered

- **Vendor the package plus flat shims.** `init_repo` copies `repopact/` into
  `scripts/repopact/` and writes one-line shims so `python scripts/validate_repo.py`
  keeps working. Preserves the adopter contract with no migration. Rejected by the
  operator: it keeps a second distribution channel alive, which is the thing that
  produced the bug, and pays that cost forever to avoid a one-time migration.
- **Keep vendored modules flat, with dual-mode imports.** Each module would try a
  relative import and fall back to a flat one. Rejected: it makes every module
  carry import machinery whose only purpose is to support the channel we are
  trying to retire, and the fallback path would be exercised only in the seeded
  repository — the mode already shown to be under-tested.
- **Ship as 2.4.0.** Defensible on the grounds that the only *published* entry
  point is the console script, which does not change. Rejected by the operator:
  `python scripts/validate_repo.py` is taught throughout the adopter docs, so it
  is a real interface to the people using it, whatever its packaging status. A
  major version says so honestly.

## Consequences

- Installing RepoPact claims one name. The `python-frontmatter` collision, and
  the shadowing risk on fifteen other generic names, are gone.
- Existing adopters must migrate from `python scripts/<tool>.py` to
  `repopact <command>`. This is a breaking change and the reason for 3.0.0.
- A seeded repository is no longer runnable without installing RepoPact. That is
  a real loss of portability, accepted deliberately: the previous portability was
  partly an illusion, since the vendored copy was silently broken.
- `init_repo.MODULES`, and the tests added in work item `036` that guard its
  import closure, become obsolete and are removed with it. Those tests did their
  job — they found and proved a live bug — and the layout change is what retires
  them, not a regression in coverage.
- **Not included: the seed-data mechanism.** Schemas and templates still ship via
  setuptools `data-files`. Moving them to package data resolved by
  `importlib.resources` (work item `036`, AC-2) requires relocating `schemas/`,
  which 98 committed records reference by relative path and which
  `governance/frozen-surface.json` protects — so it needs its own operator
  acknowledgement under `INV-6`. It is deliberately left out of this decision
  rather than smuggled in alongside it.
