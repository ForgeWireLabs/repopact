---
id: 0018
title: Generalized opt-in preflight marker
status: superseded
date: 2026-06-24
supersedes: []
---

# 0018: Generalized opt-in preflight marker

## Context

A "preflight marker" records that a work item was created **before** implementation
started — a guardrail against retrofitting a work item around code that already
exists. A downstream adopter (ForgeLink) had implemented this as a **vendored patch**
inside its copy of `validate_repo.py` (`PREFLIGHT_REQUIRED_FROM_ID = 10` plus a
`validate_work_preflight` function) and a `preflight` property in its
`work-item.schema.json`. Every RepoPact re-vendor had to re-apply that patch, and the
threshold (`id >= 010`) encoded ForgeLink's own history (items 000–009 predate the
guardrail). It was a generically useful check trapped in a local fork.

The requirement is conditional — *required for some items, not others* — which a JSON
schema cannot express on its own (a schema can validate the marker's shape, but not
"required only at/after id N"). So generalizing it needs validator logic plus
configuration, and it must not disturb existing adopters that never used it.

## Decision

Add preflight to RepoPact as a **first-class, opt-in, default-off** feature.

1. **Schema** (`schemas/work-item.schema.json`): add an optional `preflight` property
   — `created_before_work_started` (`const true`), `created_at` (date-time), `note`
   (non-empty). It validates the marker's *shape* whenever a marker is present. It is
   **not** in `required`; the conditional requirement is enforced by the validator.
2. **Config** (`governance/owners.json`, alongside the existing `concurrency` opt-in):
   ```json
   "preflight": { "enabled": true, "required_from_id": 10 }
   ```
   `required_from_date` (`"YYYY-MM-DD"`, matched against a work item's `created`) is
   supported as an alternative or in combination. Enabled with neither threshold means
   "required for every work item." **Absent / `enabled: false` is the default** and the
   behavior is identical to today.
3. **Validator** (`validate_repo.py`): `validate_work_preflight` reads the config and,
   for qualifying items, requires a `preflight` marker to be present (shape handled by
   the schema layer).

## Alternatives considered

- **Hardcode `id >= 10`.** Rejected: that is ForgeLink's history, meaningless to other
  repos.
- **Always-on.** Rejected: would immediately fail every existing adopter with
  marker-less work items.
- **Schema-only (no validator logic).** Impossible: a schema cannot express
  "required only at/after a threshold."
- **id-threshold only.** Kept id-threshold (matches ForgeLink exactly) but also added
  `required_from_date`, which is less coupled to id-numbering for repos that prefer
  "required for work started after we adopted it."

## Consequences

- Any repo can enable preflight with a few lines of config; no fork required.
- Default-off means existing adopters (and RepoPact itself, whose own items predate
  this) are unaffected.
- ForgeLink can drop its vendored preflight patch and enable the feature via config
  (`required_from_id: 10`), removing its last carried code patch.
- The schema change touches the frozen surface (`schemas/**`, INV-6) and ships with
  explicit operator approval.
