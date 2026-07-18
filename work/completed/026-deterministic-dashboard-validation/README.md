# 026 — Deterministic dashboard validation

> **Status**: Complete 2026-07-18. Evidence
> [`20260718-deterministic-dashboard-validation`](../../../evidence/runs/20260718-deterministic-dashboard-validation.json).
> **Owners**: tooling-owner (lead); work-coordinator and evidence-owner support.
> **Depends on**: none.

## Intent

Eliminate the state in which RepoPact source records validate while the committed
derived dashboard reports obsolete work, audit, or evidence state.

The dashboard is a checked-in projection of governed records. Validation must compare
it with the generator's canonical output, and RepoPact commands that mutate or repair
those records must refresh the projection before claiming success.

## Scope

- Make dashboard output stable when no source-derived value changed.
- Reject missing or byte-stale `audits/reports/dashboard.md` during validation.
- Regenerate the dashboard from RepoPact mutation and repair paths.
- Add lifecycle-blocking regression tests and adopter evidence.

## Acceptance criteria

- [x] **DDV-001** Dashboard generation is canonical and does not change solely because
  the calendar date advanced.
- [x] **DDV-002** Validation fails when the dashboard is missing or differs from freshly
  generated output.
- [x] **DDV-003** Mutation and repair commands regenerate before reporting validity.
- [x] **DDV-004** Regression tests cover rejection, regeneration, repair, and adopter
  command compatibility.
- [x] **DDV-005** An adopter pins the fix and proves stale output cannot validate.

## Safety and compatibility

- The validator remains read-only.
- The generator writes only `audits/reports/dashboard.md`.
- Invalid source records remain diagnosed by their authoritative validators; dashboard
  comparison must not hide or crash on those errors.
- Direct file edits remain allowed, but the next validation must fail until the derived
  dashboard is regenerated.

## Evidence

Evidence run `20260718-deterministic-dashboard-validation` records the 101-test
upstream pass and the ForgeLink adopter proof. ForgeLink pinned implementation commit
`126264a`, its stale dashboard failed validation before regeneration, canonical
regeneration restored a pass, and adopter commit `02d0083` passed the full pre-push
governance and product test gate.

## Closeout

Dashboard output is now a deterministic validated projection. Validation is read-only
and fails on missing or byte-stale output. RepoPact mutation and doctor paths refresh
the projection, and audit-cadence changes still alter the dashboard when the displayed
freshness result actually changes.
