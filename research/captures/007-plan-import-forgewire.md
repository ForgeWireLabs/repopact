# Capture 007 — plan import (forgewire todos/ -> work/)

Raw transcript. 2026-06-16. The gap: `adopt` left forgewire with a hollow `work/`
beside its real `todos/` tree (~75 items). `repopact import-plan` closes it.

## Read-only dry-run against real forgewire

```
$ python scripts/plan_import.py --target C:/Projects/forgewire --dry-run
Would import 75 plan item(s) into work/; skipped 1.
  + work/active/009-knowledge
  + work/active/101-experimental
  ...
  + work/completed/200-foundation        # 00-foundation remapped (000 is the adopt item)
  + work/completed/001-calendar
  ...
  + work/deferred/025-eval-frameworks
```

`skipped 1` = a `todos/` symlink duplicating a `completed/` item (same slug → deduped).
ID handling: original leading number kept when free and ≥3 digits (`101`, `053`,
`013`); collisions and the reserved `000` remapped from 200+ (`200-foundation`).

## Real import on an adopted export, then validate

```
$ python scripts/plan_import.py --target $TEMP/fw-adopt
  ...
  + work/deferred/025-eval-frameworks

work/ ledger imported; repository validates as a conformant RepoPact.

# populated counts:
active: 21   completed: 54   deferred: 1   blocked: 0
```

## Honesty check (no fabricated evidence)

A completed source item imports as `status: completed` with the criterion **waived**,
not satisfied:

```json
{ "id": "AC-1",
  "text": "Delivered prior to RepoPact adoption; imported from `todos/completed/03-login`. Waived: no RepoPact evidence was captured at the time.",
  "state": "waived", "evidence": [] }
```

The source path is recorded in a `source` field and the README, and `todos/` is left
untouched — so the human-authored tree stays the source and `work/` is the governed
view. Re-running the import is a no-op (idempotent by slug).

**Outcome.** The hollow-`work/` problem (operator-reported) is fixed: an adopted repo's
backlog is reflected in `work/`, organized by lifecycle, without overstating what is
proven.
