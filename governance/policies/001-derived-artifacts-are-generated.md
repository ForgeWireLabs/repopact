---
id: 001
title: Derived Artifacts Are Generated
status: active
applies_to: [tooling, governance]
---

# 001: Derived Artifacts Are Generated

## Rule

Any artifact that can be computed from source records is generated, not authored.
The dashboard, audit freshness, and coverage views are derived. Source records —
the charter, invariants, frozen surface, schemas, owners, work items, evidence,
decisions — are the only files edited by hand.

## Why this is a policy, not a slogan

This rule is a scar. Hand-maintained mirrors of derivable state (per-directory
audit inventories, status tables copied between files) rot faster than humans
reconcile them, and the rot then masquerades as drift. Mandating one declared
surface plus generation keeps the freshness signal honest. See `INV-7`.

## How to apply

- New status/coverage view? Add it to `scripts/generate_dashboard.py`, not to a
  Markdown file someone has to remember to update.
- A nested contract's audit coverage is declared once in `audits/registry.json`;
  an `_audit/` companion is optional and only validated for completeness if present.
- CI fails if a committed derived artifact differs from a fresh regeneration.
