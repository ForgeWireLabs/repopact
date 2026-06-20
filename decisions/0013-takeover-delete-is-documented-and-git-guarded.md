---
id: 0013
title: Takeover Delete Is Documented and Git-Guarded
status: accepted
date: 2026-06-20
supersedes: []
---

# 0013: Takeover Delete Is Documented and Git-Guarded

## Context

`repopact takeover` (decision [0012](0012-tracking-import-and-takeover.md)) retires a
legacy plan directory once every item is provably migrated, either **archiving** it
under `archive/` (default) or **deleting** it (`--delete`).

In practice the archive default just defers the confusion: an `archive/` folder sits
in the tree, no longer live but still read as if it were, until someone deletes it by
hand. The raw `--delete`, meanwhile, would `rmtree` whatever is on disk — including
files that were never committed — leaving no way back, and left no durable record of
*why* a directory full of planning history disappeared.

## Decision

`--delete` is now **documented and git-guarded**:

1. **Git-recoverability gate.** A directory is deleted only when it is fully tracked
   and clean in git (no uncommitted, untracked, or git-ignored files present), so its entire tree exists at
   `HEAD` and `git checkout <sha> -- <dir>` restores it. If it is not recoverable
   (not a git repo, untracked, or dirty), the delete is **downgraded to an archive**
   so nothing unrecoverable is lost.
2. **A decisions/ ADR is written** before the delete, recording what was retired, why
   (fully migrated, item-level `source` provenance), the commit it is recoverable at,
   and the exact `git` commands to restore it.

The retired directory's item-level narrative already lives in each work item's README;
finer-grained detail files live on in git history, pointed to by the ADR.

## Alternatives rejected

- **A new `--retire` flag.** Rejected — two delete-shaped flags to explain; the safety
  and documentation belong on the one destructive path, not beside it.
- **Auto-deleting the `archive/` folder after archiving.** Rejected — a two-step
  archive-then-delete is more moving parts than gating the delete directly.
- **Commit message as the only record.** Rejected — not discoverable in-tree; ADRs are
  RepoPact's durable home for hard-to-reverse choices.

## Consequences

- `archive/` is no longer the residue of a takeover on a committed repo; the working
  tree is left clean with the rationale captured as a decision.
- Deletion is safe by construction: if git cannot bring it back, takeover will not
  delete it.
