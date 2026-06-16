# 009 — PyPI trusted-publishing release workflow

> **Status**: ✅ Complete
> **Owners**: tooling (lead); governance (frozen-surface approval, decision).
> **Depends on**: none.

## Intent

Make `pip install repopact` possible by publishing to PyPI from CI, without storing a
long-lived API token. Set up the release workflow; the act of publishing is a
separate operator action.

## Decisions

- Distribute via PyPI Trusted Publishing — decision
  [`0009`](../../../decisions/0009-pypi-distribution-via-trusted-publishing.md).
- `.github/workflows/release.yml` is on the frozen surface (INV-6); the operator
  approved adding it ("set it up") and `check-frozen` was acknowledged.

## Scope

- `.github/workflows/release.yml` — build + Trusted-Publishing publish job.

## Closeout

Satisfied by evidence run `20260615-215140-pypi-release-workflow`. **Operator action
still required (cannot be automated from this repo):** register the trusted publisher
on PyPI (project `repopact`, owner `ForgeWireLabs`, repo `repopact`, workflow
`release.yml`, environment `pypi`). After that, publishing the matching GitHub release
auto-publishes to PyPI. See `research/release-runbook.md`.
