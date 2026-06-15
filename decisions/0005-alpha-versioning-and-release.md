---
id: 0005
title: Alpha Versioning and First Release
status: accepted
date: 2026-06-15
supersedes: []
---

# 0005: Alpha Versioning and First Release

## Context

Work item `003` (N1) introduced `VERSION` and set it to `1.0.0`. RepoPact is days
old and has not been validated by any external adopter. Declaring `1.0.0` asserts
stability the project has not earned — a direct tension with the charter
("confidence is not evidence; completion requires proof").

## Decision

- Set the project version (`VERSION`) to **`0.1.0`**.
- Cut the first public release tagged **`v0.1.0-alpha`** (a GitHub pre-release).
- `VERSION` holds the semantic project version (`0.1.0`); the release tag carries
  the maturity label (`-alpha`). The validator's semver check accepts `0.1.0`.

## Alternatives considered

- **Keep `1.0.0`.** Rejected: overstates maturity; the standard is unproven.
- **Separate "spec version" (1.0) from "project version" (0.1.0-alpha).** Rejected
  for now: two version numbers confuse adopters more than they help at this stage.
  Revisit if the record formats stabilize independently of the project.

## Consequences

- `VERSION` becomes `0.1.0`; the dashboard and `SPEC.md` surface it.
- The first release is an explicit alpha, inviting use and evidence rather than
  claiming finality.
