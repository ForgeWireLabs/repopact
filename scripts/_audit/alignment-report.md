# Tooling Alignment Report

## 2026-07-22 canonical research metadata and trace repair

- `research/metadata.json` is the machine source for lifecycle states, PactBench task
  count, study/hypothesis mappings, threat identifiers, and the proposed-state trace.
- `validate_research.py` cross-checks repeated human-authored facts without generating
  or token-substituting semantic research claims.
- The normal repository gate activates this check only for the upstream research record,
  so adopters are not required to carry RepoPact's paper metadata.
- Mutation tests reject duplicate/missing threats, a four-state figure, stale task count,
  stale hypothesis range, and pre-2.0 provenance wording.

## 2026-07-18 complete conformance rule coverage

- The conformance manifest now inventories every repository-tree rule named by the
  SPEC and machine-enforced invariants covered by the reference validator.
- Coverage is bidirectional: omitted-rule and unknown-rule mappings fail before
  conformance execution, and repository tests reject undeclared fixture directories.
- Reject fixtures must produce exactly one intended reference violation. The runner
  reports unexpected secondary diagnostics deterministically and fails the case.
- Added provenance acceptance/rejection, lifecycle identity, semantic version,
  schema validity, orphan work, disjoint scope, and missing/stale dashboard cases.

## 2026-07-18 deterministic adopter fleet verification

- Added a versioned, schema-validated public adopter manifest covering exact PyPI
  pins and vendored consumers as distinct contracts.
- `repopact fleet-verify` resolves each declared public default branch, reads its
  version marker at the resolved commit, and fails closed on stale or unreachable
  state while reporting unregistered local candidates separately.
- Vendored parity is checksum-backed: exact files must remain byte-identical and
  declared overlays must reconstruct the adopter bytes from an immutable upstream
  revision. A version marker alone cannot pass.
- `repopact release-closeout` reports package publication and ecosystem rollout as
  separate phases and succeeds only when both have evidence.

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
