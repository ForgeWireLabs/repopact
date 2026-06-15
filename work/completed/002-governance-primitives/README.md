# 002: Extract Governance Primitives From ForgeWire

## Intent

RepoPact's kernel (000, 001) captured ForgeWire's *structure* — layered
`AGENTS.md`, `_audit/` companions, numbered work items, a validator. It had not yet
extracted the *governance soul*: the parts that make the repository a pact rather
than a folder convention. This work item extracts them as a reusable standard,
deliberately leaving ForgeWire-specific machinery (the remote dispatcher, the
concrete role taxonomy, domain-specific test commands) behind.

## Decisions

- **Binding invariants are the wedge.** Promoted to a first-class, schema-validated
  record (`governance/invariants.json`) distinct from principles. Recorded as
  decision [`0001-repository-as-pact`](../../../decisions/0001-repository-as-pact.md).
- **Derive over declare.** Rather than mandating an `_audit` triple beside every
  contract (which rots, as observed in ForgeWire), coverage is declared once in
  `audits/registry.json` and an `_audit/` companion is validated for completeness
  only when present. Captured as policy `001`.
- **Roles are parameterized, not baked in.** `owners.json` gains a `roles` map and
  an optional, off-by-default disjoint-active-scope concurrency check, so adopters
  define their own roles.

## Scope

- Added: `governance/invariants.json`, `governance/frozen-surface.json`,
  `governance/policies/`, `decisions/`, and their schemas; `scripts/frontmatter.py`,
  `scripts/check_frozen_surface.py`.
- Changed: `scripts/validate_repo.py` (new validations), `scripts/generate_dashboard.py`
  (record counts + audit freshness), root `AGENTS.md`, `README.md`,
  `governance/{charter,record-types,workflow}.md`, `owners.json`, tooling `_audit`,
  tests.

## What was intentionally left out (ForgeWire instance, not standard)

Remote subagent dispatch, sealed-brief templates, runner hosts, and the concrete
domain validation matrix. The generalizable shard of the delegation logic — the
disjoint-scope rule — is included as an optional check.

## Closeout

All acceptance criteria satisfied with evidence
`20260615-governance-primitives`. Validation and tests pass; the dashboard was
regenerated from source records.
