---
id: 0032
title: Reconcile stable source and artifact identity after publication
status: accepted
date: 2026-09-02
supersedes: []
---

# 0032: Reconcile stable source and artifact identity after publication

## Context

The `v3.0.0` tag and public `repopact==3.0.0` artifact predate the accepted
`worktrees` exclusion that is present on `main`. Both the released wheel and
later source therefore identified as `3.0.0` while carrying different package
behavior. `VERSION` cannot absorb a pre-release suffix because adopter equality
and vendored-overlay targeting compare its strict `MAJOR.MINOR.PATCH` value.

## Decision

Keep `VERSION` as the compatibility and equality core. A stable release commit
has no `RELEASE_LABEL`, and its package metadata is exactly `VERSION`. Once the
matching `vVERSION` tag exists, the exact package/runtime tree at that tag is
still valid without a label. Any materially later package/runtime-affecting
source at the same `VERSION` must carry a valid VERSION-pinned `RELEASE_LABEL`.

Package metadata derives from that label when present. Conventional labels map
to readable PEP 440 forms (`3.0.1-rc.1` → `3.0.1rc1`); other legal SemVer
pre-releases use a deterministic hexadecimal local segment. This avoids relying
on incidental setuptools normalization while making development artifacts
observably distinct from the stable wheel.

The rule is enforced by `validate_repo` against the matching Git tag and by the
packaging identity helper used by setuptools and `release-build`. Repositories
without Git metadata (fixtures and source exports) retain ordinary structural
validation; the real checkout and release builder provide the history-sensitive
proof.

## Alternatives considered

- **Use `RELEASE_LABEL` only as documentation/post-release identity.** Rejected:
  package metadata would continue to produce an indistinguishable stable wheel.
- **Immediately increment `VERSION` on every post-release development commit.**
  Rejected: it would make the strict adopter compatibility anchor describe an
  unreleased compatibility line and would require speculative fleet changes.
- **Derive package metadata from a distinct development-only `VERSION` source.**
  Rejected: it duplicates the governed identity and weakens the single-core
  invariant that `VERSION` and `RELEASE_LABEL` already provide.
- **Reject every post-tag checkout until a new stable version is cut.** Rejected:
  it would make ordinary development impossible and would incorrectly reject an
  exact unchanged release tree.

## Consequences

Adopter comparisons remain unchanged. Stable `3.0.1` artifacts are named exactly
`repopact-3.0.1-*`; labeled development builds have distinct PEP 440 metadata.
Maintainers must add a VERSION-pinned label before changing package/runtime
source after publication, and must remove it for the next exact stable release.
