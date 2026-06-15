# 000: Bootstrap Repository Kernel

## Outcome

Create the smallest repository that demonstrates durable scoped authority,
structured work history, evidence-backed closure, audits, and enforcement.

## Scope

In scope: governance documents, schemas, validation scripts, tests, CI, and one
representative work item. Product-specific conventions are intentionally absent.

## Decisions

- JSON is authoritative for lifecycle state; Markdown carries reasoning.
- Work-item status is represented both by directory and JSON, with validation
  preventing disagreement.
- Evidence is append-only by convention and linked by stable run ID.
- Audit dashboards are generated and never treated as source records.

## Acceptance criteria

- [x] Repository invariants are documented.
- [x] Invalid work-item state causes validation to fail.
- [x] Missing audit companions for governed scopes cause validation to fail.
- [x] A dashboard can be regenerated from repository records.
- [x] Unit tests and CI exercise the validator.

## Closeout

Completed 2026-06-14. Initial validation passed, but the first completed-state
transition exposed test fixtures coupled to `work/active/`. The failed transition
is preserved as `20260614-193850-closeout-transition`; corrected closeout evidence
is `20260614-193922-closeout-corrected`.

Final evidence after adding evidence-manifest validation is
`20260614-194016-final`.
