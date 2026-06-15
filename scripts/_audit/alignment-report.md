# Tooling Alignment Report

## 2026-06-15 governance primitives

- The validator now enforces invariants, frozen-surface structure, role/scope
  references, decision and policy front matter, and registry-driven contract
  coverage. All rules that can block a lifecycle transition have unit tests.
- Mandatory per-contract `_audit` triples were relaxed: coverage is declared once
  in `audits/registry.json`, and an `_audit/` companion is validated for
  completeness only when it exists (INV-7, policy 001).
- The optional disjoint-active-scope check is off by default and covered by a test
  that enables it.

## 2026-06-14 bootstrap

- The validator is read-only and reports deterministic path-scoped errors.
- Dashboard generation writes only `audits/reports/dashboard.md`.
- Lifecycle-blocking rules have unit-test coverage.
