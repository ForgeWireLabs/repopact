---
id: 0010
title: Plan Import Maps Existing Trackers Into work/, Honestly
status: accepted
date: 2026-06-16
supersedes: []
---

# 0010: Plan Import Maps Existing Trackers Into `work/`, Honestly

## Context

`adopt` scaffolds governance but leaves `work/` empty, so an adopted repository ends
up with a hollow `work/` beside the team's real planning (forgewire keeps ~75 items in
a `todos/` tree). The ledger only earns trust if it reflects the work the team
actually tracks. Developers record plan items in many shapes; the import must meet them
where they are without inventing history.

## Decision

Add `repopact import-plan --target <repo> [--dry-run]` (module `plan_import.py`),
runnable standalone on an already-adopted repo. It detects:

- **Plan directories** — `todos/`, `todo/`, `tasks/`, `plan/`, `planning/`, `backlog/`
  with one subdir or markdown file per item, optionally grouped under
  `completed/`, `deferred/`, `blocked/`, `active/` lifecycle folders.
- **Checklist files** — `TODO.md`, `TODOS.md`, `ROADMAP.md`, `BACKLOG.md`, `PLAN.md`,
  `TASKS.md`: each `- [ ]` / `- [x]` line becomes an item.

Each item becomes a work item under the matching `work/<status>/` directory. Three
properties are load-bearing:

1. **Honest, never fabricated.** A *completed* source item imports as `status:
   completed` with its acceptance criterion in state **`waived`** (allowed by the
   schema, evidence-free) and a note that it predates RepoPact — never as `satisfied`
   with manufactured evidence. Active/deferred/blocked items get a `pending` criterion.
2. **Non-destructive + traceable.** The source tree is never modified or deleted; each
   work item records its origin in a `source` field and in its README, so `todos/`
   remains the human-authored source and `work/` is the governed view.
3. **Idempotent.** IDs are taken from the source's leading number when free (zero-padded
   to ≥3 digits), else allocated from 200+; collisions (e.g. two `25-` items, or
   `00-` clashing with the reserved `000-adopt`) are remapped. Re-running skips items
   whose slug already exists, so import can be run repeatedly as planning evolves.

## Alternatives considered

- **Fold import into `adopt`.** Rejected as the only option: already-adopted repos
  (forgewire, ForgeLink) need import without re-scaffolding. `adopt` may call it, but a
  standalone command is the primitive.
- **Import completed items as `satisfied` with a synthetic evidence run.** Rejected:
  that manufactures evidence for work RepoPact never witnessed (cf. decision 0008).
  `waived` states the truth — done, but unproven under RepoPact.
- **Move/delete the source `todos/`.** Rejected for v1: too destructive; preserving the
  source and cross-linking lets the team retire it deliberately.

## Consequences

- An adopted repo's `work/` reflects its real backlog immediately after import.
- Imported completed items are visibly `waived`, so the dashboard never overstates
  proven completion.
- Section-only roadmaps (no checkboxes) and external trackers (GitHub issues) are not
  yet imported; they are the natural next sources.
