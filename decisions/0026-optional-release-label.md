---
id: 0026
title: Optional RELEASE_LABEL for pre-release identity
status: accepted
date: 2026-07-20
supersedes: []
---

# 0026: Optional RELEASE_LABEL for pre-release identity

## Context

Adopters shipping pre-release builds want a single pre-release string
(`1.4.1-beta.1`) shared across product surfaces — package metadata, installers,
diagnostics — and asked whether `VERSION` could carry it. It cannot: `VERSION` is
the totally-ordered `MAJOR.MINOR.PATCH` triple that adopter equality
(`validate_adopter_manifest`) and vendored-overlay targeting compare against
exactly. A pre-release suffix in `VERSION` would silently fail those equality
checks, so SPEC §8 already keeps maturity on the git tag rather than in `VERSION`.

That leaves a gap: a repository has no governed home for one shared pre-release
string decoupled from the release line.

## Decision

Add an optional root `RELEASE_LABEL` record. When present it is a full SemVer
pre-release, `MAJOR.MINOR.PATCH-prerelease[+build]`, whose `MAJOR.MINOR.PATCH` core
must equal `VERSION`. Pinning the core to `VERSION` preserves the equality and
total-order guarantees the fleet depends on while letting the label add maturity
information. When the file is absent it constrains nothing.

The change is additive to the record language: no repository that was valid before
becomes invalid, because the file did not exist before and the only new rejection
fires on the new record type. Following the `0023` (add) → `0024` (release) split,
this decision lands the capability and its conformance cases on `main` at the
current version; a later release decision bumps `VERSION` and the conformance
`suite_version` and re-pins the adopter fleet.

- New §4.7 clause and §8 paragraph in `SPEC.md`; `RELEASE_LABEL` in the layout.
- New conformance rule `SPEC-4-release-label` with an accept fixture
  (`valid-release-label`) and a core-mismatch reject fixture
  (`invalid/release-label-version-mismatch`).
- `validate_release_label` in `scripts/validate_repo.py`.
- No schema is added: like `VERSION`, `RELEASE_LABEL` is a plain file validated by
  rule, so the frozen `schemas/**` surface is untouched.

## Evidence

On this commit's tree (work item `035`, evidence
`20260720-optional-release-label`):

- `python scripts/validate_repo.py` — governance validation passed.
- `python scripts/run_conformance.py` — every case passed, including the new
  accept and core-mismatch cases.
- `python -m unittest discover -s tests` — the full suite passed.

## Consequences

- Repositories gain a governed pre-release identity without weakening `VERSION`;
  adopters can mirror `RELEASE_LABEL` into package metadata, installers, and tags.
- `VERSION` remains the equality/total-order anchor for the adopter fleet.
- The next release decision promotes the capability by bumping `VERSION` and the
  conformance `suite_version` and re-pinning vendored adopters.
