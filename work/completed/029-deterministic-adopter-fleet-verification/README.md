# 029 — Deterministic adopter fleet verification

> **Status**: Complete — 2026-07-18
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

- [x] **AC-1** — declare every known adopter and its consumption/validation contract.
- [x] **AC-2** — fail closed when a declared public default branch is stale or unverifiable.
- [x] **AC-3** — prove vendored upstream parity rather than trusting a version marker.
- [x] **AC-4** — separate package publication from ecosystem rollout completion.
- [x] **AC-5** — cover failure modes with tests and pass all repository gates.

## Closeout

Evidence `20260718-deterministic-adopter-fleet-verification` links every criterion
to the versioned five-adopter manifest, deterministic CLI output, Moto's immutable
2.2.0 base plus reviewable ROM-lab overlay, the separate publication/ecosystem
closeout phases, and the complete passing gate set. Duplicate SkillForge checkouts
resolve to one remote identity; no unregistered local RepoPact candidates were
reported. The item is complete and lives under `work/completed/`.
