# 032 — Restore remote cross-platform governance enforcement

> **Status**: Blocked
> **Owners**: tooling-owner (lead); governance-owner and evidence-owner affected.
> **Depends on**: `009`, `027`.

## Intent

Restore a public, cross-platform commit checkpoint. Direct PyPI upload recovered package
distribution, but it did not restore the paper's CI enforcement claim or exercise Windows
and Linux on every change.

## Blocker

GitHub Actions is payment-locked. Progress requires an operator to clear that lock or
authorize and provision an alternative CI service and its repository credentials.

## Decisions

Do not treat direct PyPI publication as CI restoration. The chosen remote service and
required-gate policy are durable operational choices and require an operator-approved
decision.

## Scope

- Remote Linux and Windows governance/test/build jobs.
- Required merge enforcement and a negative gate proof.
- Public run evidence and accurate release/paper language.

## Acceptance criteria

- [ ] **AC-1** — operator selects and provisions a usable remote checkpoint.
- [ ] **AC-2** — Linux and Windows run the complete release-relevant gate set.
- [ ] **AC-3** — required enforcement demonstrably rejects an invalid change.
- [ ] **AC-4** — publication and enforcement claims remain explicitly separated.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
