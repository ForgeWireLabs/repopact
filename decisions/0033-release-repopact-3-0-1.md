---
id: 0033
title: Release RepoPact 3.0.1 corrective worktree exclusion
status: accepted
date: 2026-09-02
supersedes: []
---

# 0033: Release RepoPact 3.0.1 corrective worktree exclusion

## Decision

Release RepoPact 3.0.1 as a backwards-compatible corrective patch. The complete
runtime/package delta shipped by this release is:

- contract discovery excludes conventional `worktrees/` scratch checkouts,
  with regression coverage in `tests/test_validate_repo.py`;
- `package_version.py` derives stable or VERSION-pinned development metadata,
  `validate_repo.py` enforces the post-tag identity rule, and `release_build.py`
  validates the derived artifact identity; and
- `MANIFEST.in` preserves the governed version records in isolated source
  exports, with focused identity/build regression coverage.

The research, evidence, governance, and planning records in the intervening
history are not package feature scope.

Proposed implementation work from WI042, WI043, WI044, WI046, and WI047 is not
pulled into 3.0.1 merely because those planning records are present on `main`.
WI032 remains blocked under its temporary local-only execution directive, and
WI037 remains responsible for adopter-fleet migration.

The exact committed release tree is validated, built twice from independent Git
exports, and the resulting wheel/sdist are the only artifacts eligible for
publication. Public publication and remote enforcement remain separate
operator-bound obligations.
