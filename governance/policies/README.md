# Policies

A policy is a durable operating rule that is neither an invariant (it does not
carry an escalation gate) nor a one-off decision (it applies continuously). It is
the home for hard-won operating lessons — the kind git history alone cannot
explain.

Each policy is `NNN-kebab-title.md` with a front-matter block:

```markdown
---
id: 001
title: Derived Artifacts Are Generated
status: active
applies_to: [tooling, governance]
---
```

- `status`: `active` | `retired`. Retire a policy by changing its status, not by
  deleting it.
- `applies_to`: scope IDs from `governance/owners.json`.

Validated by `scripts/validate_repo.py` (`validate_policies`).
