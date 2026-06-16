---
id: 0011
title: repopact doctor Diagnoses and Repairs Adoption Drift
status: accepted
date: 2026-06-16
supersedes: []
---

# 0011: `repopact doctor` Diagnoses and Repairs Adoption Drift

## Context

Finding F-011: adoption is not one-shot. Two independent efforts left ForgeLink
RepoPact-*invalid* through ordinary drift (a `registry.json` referencing removed/renamed
paths, a missing root contract, an unregistered nested contract) and nothing detected
or guided the repair — it was hand-done twice. The reference validator says *what* is
invalid; it does not frame drift as fixable or distinguish the recurring, mechanical
cases an adopter hits over time.

## Decision

Add `repopact doctor [--root] [--fix]` (module `doctor.py`). Read-only by default, it
reports categorized findings and exits non-zero on errors. With `--fix` it applies
**safe, non-destructive** repairs and re-validates. Checks: missing root contract;
stale `registry.json` scopes (path/contract absent); unregistered nested contracts;
incomplete `_audit` triplets; schema skew vs the installed RepoPact; governance records
swallowed by `.gitignore` (F-008).

Two safety rules are load-bearing:

1. **Never overwrite a *differing* schema.** A schema that differs from the installed
   one may be an intentional local extension (ForgeLink added a `preflight` property);
   `--fix` only *adds missing* schemas. Differences are a `schema-differs` **warning**
   for human review, never an automatic overwrite. (Caught on the real ForgeLink run.)
2. **Dropping stale registry entries is the only removal**, and only of entries that
   already point at nonexistent paths (i.e. already invalid). Everything else `--fix`
   does is additive (create root contract, register contracts, stub `_audit` files,
   add `.gitignore` negations).

## Alternatives considered

- **Fold drift checks into the validator.** Rejected: the validator's job is a clean
  accept/reject for conformance; `doctor` is the upgrade/repair surface and may evolve
  faster, including `--fix` mutations the validator must never perform.
- **A blunt `upgrade` that overwrites all vendored files.** Rejected for the schema
  reason above; refreshing vendored *scripts* wholesale is safe and may be added later,
  but schemas and records must be treated as possibly-customized.

## Consequences

- Adopters get a one-command drift check (`doctor`) and guided repair (`doctor --fix`).
- F-011's manual reconciliation becomes a tool; the ForgeLink/forgewire upgrades would
  have been one command each.
- `doctor` cannot tell an *ahead* schema from a *stale* one (both "differ"); it
  conservatively flags for review. Distinguishing them is future work.
