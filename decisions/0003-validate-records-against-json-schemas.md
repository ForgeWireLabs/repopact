---
id: 0003
title: Validate Records Against JSON Schemas
status: accepted
date: 2026-06-15
supersedes: []
---

# 0003: Validate Records Against JSON Schemas

## Context

The `schemas/*.json` files existed but nothing loaded them: `validate_repo.py`
re-implemented every structural check in Python. Schema and validator could drift
silently, making the schemas documentation rather than enforcement (backlog item
`003` H3). The repository had been stdlib-only by constraint.

## Decision

Take a dependency on **`jsonschema`** and split validation into two layers with
clear authority:

- **Schemas are authoritative for structure.** Every JSON record is validated
  against its `schemas/*.json` (required fields, types, enums, patterns).
- **The validator is authoritative for cross-record semantics** that JSON Schema
  cannot express: id↔directory↔filename agreement, status↔directory match,
  evidence/dependency reference resolution, lifecycle rules, and cycles.

## Alternatives considered

- **Stay stdlib-only, keep hand-rolled structure checks.** Rejected: the
  operator approved the dependency, and a single declarative source for structure
  removes a whole class of drift.
- **Generate the Python checks from the schemas.** Rejected: more machinery than
  validating against the schemas directly.

## Consequences

- A `requirements.txt` pins `jsonschema`; CI installs it before validating.
- The stdlib-only constraint in `scripts/AGENTS.md` is relaxed for declared,
  operator-approved dependencies.
- Structural rules now live in the schema files; the validator focuses on
  semantics.
