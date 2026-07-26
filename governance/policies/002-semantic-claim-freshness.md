---
id: 002
title: Semantic Claims Have Review Deadlines
status: active
applies_to: [governance, tooling]
---

# 002: Semantic Claims Have Review Deadlines

## Rule

Current-state claims that cannot be regenerated from repository records must name
when they were verified, when they must be reviewed again, and the documents or
scope covered by that review.

- Audit-scope alignment claims use `last_reviewed` and `next_review` in
  `audits/registry.json`.
- Upstream research claims use the `claim_freshness` contract in
  `research/metadata.json`. One review window may cover the registered top-level
  research documents, but it may not exceed 30 days.
- Validation fails after a review deadline. Regenerating the dashboard does not
  cure an expired source claim; the underlying scope or research documents must
  be re-verified and the source contract advanced.

Immutable evidence runs, captures, decisions, and historical observations do not
expire merely because time passes. If one is reused to assert current external
state, the current claim that cites it is subject to this policy.

## Rationale

Canonical generation proves that a view exactly reflects its inputs. It cannot
prove that a human-authored input still describes a remote repository, service,
release, or research programme. Explicit deadlines make that review debt visible
and enforceable without pretending semantic truth can be regenerated.

## How to apply

1. Re-run the commands appropriate to the registered scope or research claim set.
2. Preserve contradictory or reopened findings instead of rewriting the original
   observation.
3. Record the verification in dated evidence.
4. Advance the source record's verification date and review deadline.
