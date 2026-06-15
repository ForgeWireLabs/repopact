# Guide: Define your governance

*Diataxis mode: how-to (task-oriented).*

How to adapt a bootstrapped RepoPact to a real project. RepoPact ships structure,
not your team's specific rules — these are yours to declare.

## Declare scopes and roles

Edit `governance/owners.json`. A **scope** is a set of path globs with an owner; a
**role** binds to one or more scopes. Keep each change inside one role's scope (the
non-overlap principle).

```jsonc
{
  "scopes": [
    {"id": "backend", "paths": ["src/api/**"], "owner": "backend-owner"},
    {"id": "frontend", "paths": ["src/web/**"], "owner": "frontend-owner"}
  ],
  "roles": [
    {"id": "backend-owner", "description": "Owns the API.", "scopes": ["backend"]}
  ]
}
```

If you run multiple agents at once, set `concurrency.enforce_disjoint_active_scopes`
to `true` so two active work items cannot collide on the same scope.

## Write invariants that matter

In `governance/invariants.json`, add guarantees specific to your system. A good
invariant is binding, has a clear rationale, and names its escalation:

```jsonc
{
  "id": "INV-8",
  "statement": "No endpoint ships without authn/authz.",
  "rationale": "Auth gaps are the most expensive class of regression here.",
  "escalation": "Flag and get security sign-off before merging any public route.",
  "enforced_by": null
}
```

Use `enforced_by: null` when only human review can catch it; wire a real check and
name it otherwise.

## Protect a frozen surface

List paths and symbols that must not change without approval in
`governance/frozen-surface.json` — generated code, migrations, public API symbols,
CI. The `symbols` array catches identifiers even when the file path is not obvious.

## Register nested contracts

If a subtree needs its own `AGENTS.md`, add a row to `audits/registry.json` for it
(an `_audit/` companion is optional). The validator rejects unregistered contracts.

## Verify

```
python scripts/validate_repo.py
```
