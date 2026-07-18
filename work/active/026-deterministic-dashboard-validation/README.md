# 026 — Deterministic dashboard validation

> **Status**: Active.
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

- [ ] **DDV-001** Dashboard generation is canonical and does not change solely because
  the calendar date advanced.
- [ ] **DDV-002** Validation fails when the dashboard is missing or differs from freshly
  generated output.
- [ ] **DDV-003** Mutation and repair commands regenerate before reporting validity.
- [ ] **DDV-004** Regression tests cover rejection, regeneration, repair, and adopter
  command compatibility.
- [ ] **DDV-005** An adopter pins the fix and proves stale output cannot validate.

## Safety and compatibility

- The validator remains read-only.
- The generator writes only `audits/reports/dashboard.md`.
- Invalid source records remain diagnosed by their authoritative validators; dashboard
  comparison must not hide or crash on those errors.
- Direct file edits remain allowed, but the next validation must fail until the derived
  dashboard is regenerated.

## Evidence

Pending implementation and adopter validation.
