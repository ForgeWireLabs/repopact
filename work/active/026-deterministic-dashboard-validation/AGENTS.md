# Deterministic Dashboard Validation Agent

## Scope

- Own canonical dashboard generation, stale-derived-output validation, mutation-path
  regeneration, repair behavior, tests, and adopter proof for work item 026.
- Tooling is the lead scope; work and evidence changes are closeout support.

## Required checks

- Run `python scripts/validate_repo.py`.
- Run `python -m unittest discover -s tests -v`.
- Demonstrate that a deliberately stale dashboard fails validation and regeneration
  restores a passing state.
- Validate at least one pinned adopter with the upstream implementation.

## Constraints

- Validators remain read-only and deterministic.
- Generators may overwrite only `audits/reports/dashboard.md`.
- Do not weaken source-record validation to avoid secondary dashboard errors.
