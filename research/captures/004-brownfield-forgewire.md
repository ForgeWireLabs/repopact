# Capture 004 — brownfield adoption of forgewire

Raw transcript. 2026-06-15. Subject: `C:/Projects/forgewire`, a real, unrelated
project (4569 commits, GTK/HTTP app) with no prior RepoPact. RepoPact branch
`008-brownfield-adoption`.

## Read-only dry-run against the real repo

`adopt --dry-run` writes nothing, so it is safe to run against the live repo.

```
$ git -C C:/Projects/forgewire rev-list --count HEAD
4569

$ python scripts/adopt_repo.py --target C:/Projects/forgewire --dry-run
Would create 27 record(s); skipped 52 existing file(s).
  + schemas/... templates/... VERSION
  + governance/owners.json governance/charter.md governance/workflow.md
  + governance/invariants.json governance/frozen-surface.json
  + governance/policies/001-ci-audit-validation.md
  + governance/policies/002-ci-forgecorenative-ci.md
  + governance/policies/003-ci-python-ci.md
  + governance/policies/004-ci-windows-ci.md
  + audits/registry.json
  + evidence/runs/<ts>-adopt.json
  + work/completed/000-adopt-repopact/{work-item.json,README.md}
  + decisions/0001-adopt-repopact.md

Dry run: nothing written.
```

52 existing files skipped = the repo's own AGENTS.md, nested `_audit` files, etc.,
all preserved.

## Real adoption of an exported working tree, then validate

```
$ git -C C:/Projects/forgewire archive HEAD | tar -x -C $TEMP/fw-adopt   # no .git, 3260 files
$ python scripts/adopt_repo.py --target $TEMP/fw-adopt
Created 27 record(s); skipped 52 existing file(s).
  ...
Adopted repository validates as a conformant RepoPact.
```

## What it mapped (verification that validation was non-trivial)

```
# 19 nested AGENTS.md contracts registered in audits/registry.json:
.  core  core/config  docs  forgewire-fabric  forgewire_core  modules
modules/budget  modules/calendar_store  modules/conversation_store  modules/storage
scripts/remote  shell  shell/gtk  shell/gtk/budget_manager  shell/gtk/calendar_manager
shell/http  tests  todos/114-forgewire-fabric

# 7 scopes from CODEOWNERS:
governance  infra-team  backend-team  docs-team  ui-team  data-team  testing-team

# 4 CI workflows -> binding-gate policies (e.g. 003-ci-python-ci.md):
---
id: 003
title: 'CI gate: Python CI'
status: active
applies_to: '.github/workflows/python-ci.yml'
---
... binding gate ... disabling or weakening it requires operator approval (INV-2).
```

A real, mature, RepoPact-naive repository was brought under RepoPact governance —
ownership, enforcement, and 19 contracts mapped — and the result validated, without
overwriting a single existing file. **H7 (brownfield adoptability) holds.**
