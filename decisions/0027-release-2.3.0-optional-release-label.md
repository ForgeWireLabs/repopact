---
id: 0027
title: Release 2.3.0 with the optional RELEASE_LABEL record
status: accepted
date: 2026-07-20
supersedes: []
---

# 0027: Release 2.3.0 with the optional RELEASE_LABEL record

## Context

Decision `0026` added the optional `RELEASE_LABEL` record: a SemVer pre-release
whose `MAJOR.MINOR.PATCH` core is pinned to `VERSION`, giving a repository one
shared pre-release string without loosening the release line. The capability and
its conformance cases landed on `main` at 2.2.0 (commit `f371c54`), following the
`0023` add → `0024` release split. This decision cuts the release.

The change widens the conformant language: a repository carrying a malformed or
core-divergent `RELEASE_LABEL` is valid under 2.2.0 validators and invalid under
2.3.0. No 2.2.0 repository becomes invalid — the file did not exist before and the
only new rejection fires on the new record type — so the bump is minor, not major.

## Decision

Release RepoPact 2.3.0 as a minor release.

- `VERSION` and the conformance `suite_version` move to 2.3.0; `CONFORMANCE.md`
  states the 2.3.0 conformance claim.
- The published suite carries `SPEC-4-release-label` with its accept
  (`valid-release-label`) and core-mismatch reject
  (`invalid/release-label-version-mismatch`) fixtures; bidirectional coverage
  holds at 19 cases.
- The vendored adopter `moto-one-hyper` is re-vendored to 2.3.0 first
  (`moto-one-hyper-forgewire-rom-lab` commit `c073adb`): the fleet-coherence rule
  requires every vendored adopter to track the current `VERSION`, so the release
  cannot precede the downstream re-vendor. `governance/adopters.json` records the
  new `upstream_revision` (`f371c54`), the validator's new `upstream_sha256`, the
  re-derived `adopter_sha256`, and the regenerated overlay patch checksum.

## Evidence

On the release commit's tree (evidence `20260720-release-2-3-0`):

- `python scripts/validate_repo.py` — governance validation passed with the
  re-pinned adopter fleet and 2.3.0 records.
- `python scripts/run_conformance.py` — 19/19 conformance cases passed.
- `python scripts/fleet_verify.py` — the adopter fleet, including the re-vendored
  `moto-one-hyper`, verified.
- `python -m unittest discover -s tests` — the full suite passed.
- `python -m build` and `python -m twine check` — the sdist and wheel built and
  passed metadata checks; SHA-256 hashes recorded in the evidence run.

## Consequences

- 2.3.0 replaces 2.2.0 as the current release; adopters gain a governed
  pre-release identity while `VERSION` stays the equality/total-order anchor.
- Downstream consumers pinning the conformance suite should move to `suite_version`
  2.3.0 to pick up `SPEC-4-release-label`.
- Publication to PyPI follows the billing-locked manual-twine path in
  `research/release-runbook.md` (operator-held token), consistent with 2.1.0–2.2.0.
