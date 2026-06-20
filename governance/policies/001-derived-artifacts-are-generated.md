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

## Rationale

Hand-maintained mirrors of derivable state — per-directory audit inventories, status
tables copied between files — fall out of date faster than they are reconciled, and
the resulting staleness is indistinguishable from genuine drift. Mandating a single
declared surface plus generation keeps the freshness signal reliable. See `INV-7`.

## How to apply

- A new status or coverage view is added to `scripts/generate_dashboard.py`, not to a
  Markdown file that must be updated manually.
- A nested contract's audit coverage is declared once in `audits/registry.json`;
  an `_audit/` companion is optional and only validated for completeness if present.
- CI fails if a committed derived artifact differs from a fresh regeneration.
