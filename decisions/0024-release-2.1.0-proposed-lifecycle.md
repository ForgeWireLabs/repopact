---
id: 0024
title: Release 2.1.0 with the proposed lifecycle state
status: accepted
date: 2026-07-15
supersedes: []
---

# 0024: Release 2.1.0 with the proposed lifecycle state

## Context

Decision `0023` added `proposed` as a first-class work lifecycle state: captured
candidate work that is not accepted or authorized for implementation. The change
landed on `main` after the 2.0.2 tag and widens the specification surface:

- a new lifecycle state and `work/proposed/` directory,
- a widened `status` enum in `schemas/work-item.schema.json`,
- a new semantic rule (`active`/`completed` work may not depend on `proposed`
  work, SPEC §4.3),
- two new conformance cases (a valid proposed item; a rejection overlay for
  active-depends-on-proposed),
- CLI support (`repopact new work-item "Title" --status proposed`) and bootstrap
  creation of `work/proposed/`.

## Decision

Release RepoPact 2.1.0 as a minor release. The version bump is minor, not patch,
because the conformant language changes: repositories containing proposed items
are valid under 2.1.0 and invalid under 2.0.x validators. No existing 2.0.x
repository becomes invalid; the change is additive to the record language while
introducing one new rejection rule that only fires on records the old language
could not express.

The `suite_version` of the conformance manifest moves to 2.1.0 with it, so
third-party implementations can target the lifecycle-authority semantics
explicitly.

## Evidence

On the release commit's tree:

- `python scripts/validate_repo.py` — governance validation passed.
- `python scripts/run_conformance.py` — 8/8 cases passed, including
  `work/proposed` acceptance and the active-depends-on-proposed rejection.
- The regression test suite passed (see the release run's CI).

## Consequences

- 2.1.0 replaces 2.0.2 as the current release; adopters gain durable candidate
  capture without conflating it with implementation authority.
- Downstream consumers pinning the conformance suite should move to
  `suite_version` 2.1.0 to pick up the authority semantics.
- The paper and ROADMAP already describe the five-state lifecycle; this release
  makes the published package match them.
