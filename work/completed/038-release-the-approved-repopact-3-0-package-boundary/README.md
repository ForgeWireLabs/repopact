# 038 — Release the approved RepoPact 3.0 package boundary

> **Status**: Completed
> **Owners**: governance-owner (lead); tooling-owner, docs-owner,
> evidence-owner, and work-coordinator affected.
> **Depends on**: `029`, `036`.

## Intent

Deliver the breaking package boundary explicitly approved in decision `0029`.
WI-036 landed the source conversion and package-resource move, but it was closed
while `VERSION` and public PyPI still pointed at 2.3.0. The public wheel therefore
still installs flat modules and deprecated data-files even though `main` contains
the repair.

This corrective slice preserves WI-036's completed history and records the
missing outward delivery honestly. It also closes the release-build blind spot
found during verification: `python -m build --wheel` can inherit obsolete files
from a checkout's ignored `build/lib`, so release artifacts must be built from a
clean exported commit and structurally inspected.

## Decisions

Decision `0029` already records both operator choices: stop vendoring tooling and
ship the breaking interface as 3.0.0. A release decision records the artifact and
publication boundary; it does not rewrite WI-036.

## Scope

- Version, conformance, release narrative, and release decision.
- Deterministic clean-tree release builder and regression tests.
- Declared development dependencies and current roadmap.
- Exact artifact/publication evidence.

Out of scope: restoring billing-locked CI (WI-032) and performing the downstream
3.0 adopter migration (proposed WI-037).

## Acceptance criteria

- [x] **AC-1** — all release identities and narratives say 3.0.0.
- [x] **AC-2** — clean committed-source build rejects namespace/resource drift.
- [x] **AC-3** — exact wheel passes isolated init/adopt and import probes.
- [x] **AC-4** — dev environment and current documentation are repaired.
- [x] **AC-5** — repository, conformance, tests, metadata, and frozen checks pass.
- [x] **AC-6** — public commit/tag/PyPI artifact are remotely verified.
- [x] **AC-7** — rollout and CI remain explicitly separate/open.

## Closeout

All acceptance criteria are satisfied by
`20260726-repopact-3-0-0-release`. Package publication is complete; WI-037
continues to own the five-adopter migration, and billing-locked CI remains
separate under WI-032.
