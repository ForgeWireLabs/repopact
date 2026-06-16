---
id: 0009
title: Distribute via PyPI Trusted Publishing
status: accepted
date: 2026-06-15
supersedes: []
---

# 0009: Distribute via PyPI Trusted Publishing

## Context

The v1.0.0 GitHub release ships the wheel and sdist, but `pip install repopact` is
not yet possible. The operator asked for PyPI distribution to be set up. Two
mechanisms exist: a stored PyPI API token in CI, or **Trusted Publishing** (OIDC),
where PyPI verifies the GitHub workflow identity and no long-lived secret is stored.

## Decision

Publish to PyPI via **Trusted Publishing**. `.github/workflows/release.yml` builds
the distribution and runs `pypa/gh-action-pypi-publish` from a `pypi` environment
with `id-token: write`, triggered when a GitHub release is published (and manually via
`workflow_dispatch`). No API token is stored in the repository or CI.

Because `.github/workflows/**` is on the frozen surface (INV-6), adding this workflow
required operator approval; that approval was given when the operator requested the
setup, and `check-frozen` was acknowledged for the change.

The one step that cannot be automated from this repository is the **one-time
registration of the trusted publisher on PyPI** (project `repopact`, owner
`ForgeWireLabs`, repo `repopact`, workflow `release.yml`, environment `pypi`), which
requires the operator's PyPI account. After that, publishing is a release away.

## Alternatives considered

- **Stored API token.** Rejected as the default: a long-lived secret is a standing
  liability; Trusted Publishing removes it. The token path remains a documented manual
  fallback in the release runbook.

## Consequences

- A published GitHub release auto-publishes the matching version to PyPI once the
  trusted publisher is registered.
- Releasing requires `VERSION` to match the intended PyPI version (SemVer).
