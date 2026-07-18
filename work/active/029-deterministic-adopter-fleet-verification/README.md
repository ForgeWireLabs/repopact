# 029 — Deterministic adopter fleet verification

> **Status**: Active
> **Owners**: tooling-owner (lead); governance-owner and evidence-owner affected.
> **Depends on**: `028` (the manual 2.2.0 adopter rollout establishes the baseline).

## Intent

Make downstream release drift observable and release-closeout blocking. The 2.2.0
rollout proved the ecosystem at one instant, but the next release can silently make that
evidence stale because adopter discovery, version comparison, and vendored parity are
manual.

## Decisions

Package publication and ecosystem rollout are two phases. The verifier may make remote
reads, but it must be runnable locally while GitHub Actions is billing-blocked. Vendored
consumption is not equivalent to an exact PyPI pin and needs stronger proof than a version
marker.

## Scope

- A versioned adopter manifest and schema.
- A fleet-verification command with deterministic output and explicit network failures.
- Release closeout integration and evidence.
- Tests for PyPI and vendored consumers.

## Acceptance criteria

- [ ] **AC-1** — declare every known adopter and its consumption/validation contract.
- [ ] **AC-2** — fail closed when a declared public default branch is stale or unverifiable.
- [ ] **AC-3** — prove vendored upstream parity rather than trusting a version marker.
- [ ] **AC-4** — separate package publication from ecosystem rollout completion.
- [ ] **AC-5** — cover failure modes with tests and pass all repository gates.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
