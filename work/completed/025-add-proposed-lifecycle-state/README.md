# 025 — Add proposed lifecycle state

> **Status**: Complete 2026-06-29. Evidence
> [`20260629-proposed-lifecycle-state`](../../../evidence/runs/20260629-proposed-lifecycle-state.json).
> **Owners**: governance-owner (lead); tooling-owner support.
> **Depends on**: none.

## Intent

Add `work/proposed/` as a first-class lifecycle state for captured candidate work
that has not yet been accepted or authorized for implementation. Preserve the
authority distinction: proposed work is not active, blocked, or deferred work.

## Decisions

Decision [`0023`](../../../decisions/0023-add-proposed-lifecycle-state.md)
defines the lifecycle state and the authority boundary.

## Scope

- Lifecycle constants, schema enum, bootstrap directory creation, and CLI
  `--status proposed`.
- Validator rule preventing active/completed work from depending on proposed work.
- Tests, docs, dashboard/spec regeneration, and conformance cases.

## Acceptance criteria

- **AC-1** — `work/proposed/` is a valid lifecycle directory and bootstrap creates it.
- **AC-2** — proposed work items validate structurally, but active/completed work
  cannot depend on proposed work.
- **AC-3** — CLI can create proposed work items via `repopact new work-item ... --status proposed`.
- **AC-4** — documentation and conformance reflect the new lifecycle semantics.

## Closeout

All acceptance criteria are satisfied. `proposed` is a valid lifecycle status and
directory, bootstrap creates `work/proposed/`, the CLI can stamp proposed work,
the validator rejects active/completed dependencies on proposed work, and
conformance covers both the accepted and rejected lifecycle cases.
