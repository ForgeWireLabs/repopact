# Tooling Alignment Report

## 2026-06-29 proposed lifecycle state (025)

- Added `proposed` to the shared lifecycle model as candidate work that does not
  grant implementation authority.
- Validator accepts structurally valid proposed work items but rejects active or
  completed work that depends on proposed work.
- Bootstrap and CLI record-stamping now create/use `work/proposed/`; conformance
  covers the new lifecycle rule.

## 2026-06-15 adoption surface and hardening (003)

- Records are now validated against `schemas/*.json` via `jsonschema` (decision
  0003); the validator retains cross-record semantic checks. Finding 001 closed.
- Added: audit-finding validation, spec-version check, dependency-cycle detection,
  symbol-level frozen-surface enforcement.
- Added bootstrap (`init_repo.py`) and record-stamping (`new.py`) tooling plus
  `templates/`, making RepoPact installable into a new repository.

## 2026-06-15 governance primitives (002)

- Validator enforces invariants, frozen-surface structure, role/scope references,
  decision and policy front matter, and registry-driven contract coverage.
- Mandatory per-contract `_audit` triples relaxed; an `_audit/` companion is
  validated for completeness only when present (INV-7, policy 001).
- Optional disjoint-active-scope check, off by default.

## 2026-06-14 bootstrap

- The validator is read-only and reports deterministic path-scoped errors.
- Dashboard generation writes only `audits/reports/dashboard.md`.
- Lifecycle-blocking rules have unit-test coverage.

## 2026-07-18 deterministic dashboard enforcement

- `validate_repo.py` compares the committed dashboard with a fresh canonical render
  and rejects missing or stale output.
- The generator no longer embeds its run date, so output stays byte-stable until a
  displayed source value or audit-cadence state changes.
- Bootstrap, adoption, record stamping, plan import, takeover, conformance
  materialization, and doctor repair refresh the derived dashboard as part of their
  governed mutation path.
- Regression tests cover missing/stale rejection, stable rendering, doctor repair,
  and command compatibility.
