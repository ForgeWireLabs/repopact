---
id: 0022
title: Release 2.0.2 for installed seed lookup
status: accepted
date: 2026-06-29
supersedes: []
---

# 0022: Release 2.0.2 for installed seed lookup

## Context

RepoPact 2.0.1 shipped seed data under `share/repopact/schemas` and
`share/repopact/templates`, but `init_repo._seed_dir()` only checked the checkout
path and `sys.prefix/share/repopact/<seed>`.

On Windows user-site installs, pip installs data files under `site.USER_BASE`,
for example:

    C:\Users\<user>\AppData\Roaming\Python\Python312\share\repopact\schemas

That meant the wheel contained the seed data, but installed-wheel execution could
still fail to locate it. This affected any installed path that relied on
`init_repo.bootstrap()` or helpers that reuse `_seed_dir()`, including `init`,
`adopt`, `doctor`, and Proving Ground drift-harness setup.

## Decision

Release RepoPact 2.0.2 as a patch release.

`init_repo._seed_dir()` now searches:

1. the source checkout,
2. `sys.prefix/share/repopact/<seed>`, and
3. `site.USER_BASE/share/repopact/<seed>`.

The error message now reports every candidate path checked.

## Evidence

Before the fix, Proving Ground S5 drift selftest failed with:

    FileNotFoundError: cannot locate seed 'schemas'

After building and installing the fixed local wheel, the Proving Ground validation
suite passed:

    python -m validate_repo --root .
    python -m unittest discover -s tests
    python benchmarks/harness/run.py --selftest
    python benchmarks/drift/harness.py --selftest

The RepoPact validator tests and conformance suite also passed.

## Consequences

2.0.2 should replace 2.0.1 for installed-wheel use. No schema or record-format
change is introduced.
