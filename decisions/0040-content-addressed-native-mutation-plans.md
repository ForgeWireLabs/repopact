---
id: 0040
title: Content-addressed plan/apply boundary for native governed mutations
status: accepted
date: 2026-09-09
supersedes: []
---

# 0040: Content-addressed plan/apply boundary for native governed mutations

## Context

WI053 established a conformant, read-only Rust repository/schema/validation foundation. WI054 is the first work allowed to add native governed mutation authority for explicitly proven surfaces. A desktop or native client needs a stable mutation contract, but RepoPact repositories may contain uncommitted state, linked worktrees, external editor changes, generated projections, and files whose semantics are independent of the current Git commit.

A mutation plan therefore cannot safely mean "apply this if HEAD is unchanged" or "apply this if mtimes look the same." It must identify the repository facts it actually read and prove that those facts have not changed before apply.

## Decision

1. The native write boundary is `MutationRequest -> MutationPlan -> MutationResult`. Clients request a plan, inspect its diagnostics and exact durable/generated effects, and explicitly apply that plan. Clients do not gain a generic governed-file write API.
2. A reusable `RepositorySession` produces immutable snapshots/read models. Validation, graph construction, deterministic analysis, and mutation planning for one operation use the same snapshot semantics so a plan is not assembled from mutually inconsistent repository reads.
3. Every `MutationPlan` carries repository identity plus a deterministic content-addressed read set. Each relevant path is represented by its expected state, including an explicit absent state for paths that are expected not to exist. A stable plan token is derived from the sorted read-set facts and repository identity.
4. Git metadata may contribute repository identity or tracked-path facts where existing semantics require it, but Git HEAD, branch name, wall-clock timestamps, and filesystem mtimes are not mutation correctness preconditions.
5. Apply re-checks the plan's read set and target preimages. Any material drift rejects the plan as stale. RepoPact does not silently rebase, merge, or re-plan a stale mutation.
6. Supported writes are staged as a complete write set with recoverable preimages, canonical serialization, owned derived-projection regeneration, and post-write validation. Success is reported only after the supported repository post-state validates. Failure during apply must restore the pre-state or leave an explicit recoverable failure rather than report partial success.
7. This decision does not mandate a persistent repository-local transaction database or hidden journal directory. If implementation discovers that durable crash recovery requires new persistent runtime state inside an adopted repository, that state boundary must be recorded separately before it becomes architectural fact.
8. Lifecycle transitions preserve the complete work-item directory and update the machine `status`. The existing guarded lifecycle semantics remain authoritative: any state may move to any other state, including reopening completed work, while the resulting repository must satisfy validation and history/evidence must not be silently dropped.
9. Frozen-surface intersections are visible during planning. WI054 does not synthesize human operator approval or port WI050 enforcement. A protected mutation is therefore not applyable by the WI054 native mutation path unless an already-governed approval mechanism explicitly covers it; absent that mechanism, apply fails closed.
10. Existing Python mutation/generation surfaces that move to Rust require executable Python/Rust before-after parity. New typed Rust mutations with no Python command equivalent require canonical before/after fixtures plus successful Python-reference and Rust validation of the resulting state. A duplicate Python implementation is not created merely to manufacture parity.
11. WI054 does not make Rust globally canonical. Authority moves only for the mutation surfaces whose parity, recovery, and post-validation gates are proven. Python remains independently usable and remains the reference for unported surfaces until a later governed cutover.

## Alternatives considered

- Use Git commit SHA as the plan version: rejected because uncommitted and exported repositories are valid operating states and because relevant content may change without a commit.
- Use mtimes or file sizes: rejected because they are not reliable semantic identity and differ across filesystems and tooling.
- Let callers send arbitrary file patches and validate afterward: rejected because it would make clients a second mutation authority and would bypass RepoPact-owned planning semantics.
- Build a second Python mutation implementation for every new Rust operation: rejected because it would create code solely to satisfy a testing label rather than preserve an existing reference behavior.
- Introduce a persistent hidden transaction database immediately: deferred because WI054 can first prove the typed plan/apply and recoverable write-set contract without committing the product to a new repository-local runtime-state format.

## Consequences

Tauri, CLI, IDE, and agent-facing consumers can share one deterministic mutation protocol. Stale plans fail predictably even when no commit occurred. Graph and analysis can explain the exact records behind a plan. The cost is that planners must track their real read sets and write tests for failure/recovery paths, and conservative plans may become stale when an input they genuinely consumed changes. That cost is intentional: RepoPact prefers explicit re-planning to silently applying governance changes against a different repository state.
