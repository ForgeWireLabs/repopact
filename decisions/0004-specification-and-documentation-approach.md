---
id: 0004
title: Specification and Documentation Approach
status: accepted
date: 2026-06-15
supersedes: []
---

# 0004: Specification and Documentation Approach

## Context

RepoPact is an adoptable standard (decision `0001`) but its spec-like content was
spread across the charter, record-types, workflow, schemas, and ADRs. A second
implementation or a new adopter had no single normative reference, and the project
had no outward-facing documentation. Restating the schemas in prose would violate
policy `001` (derive over declare).

## Decision

- **SPEC.md is a normative reference.** It specifies the rules JSON Schema cannot
  express (cross-record semantics, lifecycle, invariant/escalation and
  frozen-surface semantics, conformance) and **generates** its record-type catalog
  and invariant list from `schemas/*.json` and `governance/invariants.json` via
  `scripts/generate_spec.py`. Generated blocks are checked for freshness in CI, so
  the reference cannot drift from what the validator enforces.
- **Docs follow Diátaxis** under a `docs/` scope: tutorial (`adopt-repopact.md`),
  how-to (`guides/`), reference (`SPEC.md` at root), explanation (`concepts.md`).
- **`docs/` is governed lightly:** one `docs/AGENTS.md` registered in the audit
  registry, but no per-page audit inventories — honoring the staleness lesson
  (policy `001`).

## Alternatives considered

- **Self-contained SPEC restating schemas.** Rejected: highest rot risk; conflicts
  with policy `001`.
- **Flat top-level docs, no docs/ scope.** Rejected: a growing doc set benefits
  from the Diátaxis split and a single ownership scope.

## Consequences

- `SPEC.md` + `scripts/generate_spec.py`; CI gains a SPEC-freshness gate.
- `owners.json` gains a `docs` scope and `docs-owner` role; the registry gains a
  `docs` entry.
