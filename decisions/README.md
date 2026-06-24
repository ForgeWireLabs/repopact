# Decision Records

A decision record (ADR) captures a material, hard-to-reverse choice and the
alternatives rejected. Decisions are a durable record type because their rationale
outlives the work item that produced them: archiving a work item must not bury
the reasoning that still governs the codebase.

Each record is `NNNN-kebab-title.md` with a front-matter block:

```markdown
---
id: 0001
title: Repository as Pact
status: accepted
date: 2026-06-15
supersedes: []
---
```

- `status`: `proposed` | `accepted` | `rejected` | `deferred` | `superseded` |
  `deprecated`.
  - `proposed`: awaiting a decision.
  - `accepted`: adopted.
  - `rejected`: decided against.
  - `deferred`: postponed, kept for its reasoning, with reactivation criteria.
  - `superseded`: replaced by a later decision (see `supersedes`).
  - `deprecated`: previously accepted, now retired.
- `supersedes`: IDs of decisions this one replaces. Never edit a superseded record;
  set its status to `superseded` and link forward.

Validated by `scripts/validate_repo.py` (`validate_decisions`).
