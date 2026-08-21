# 042 — Structural Git-Worktree Awareness in Contract Discovery

> **Status**: 📋 Planning (proposed — not started)
> **Owners**: governance-owner (lead).
> **Depends on**: none.

## Intent

`repopact/repo_model.py`'s `iter_contracts` walks the filesystem for `AGENTS.md`
contracts and excludes a fixed set of directory names (`IGNORED_PARTS`). Upstream
`main` already added `"worktrees"` to that set (`0096d70`) after ForgeWire's original
WI 237 incident (finding F-015), but that is a convention-name allowlist, not genuine
awareness of `git worktree` checkouts — a worktree created under any other directory
name reproduces the identical false-positive class, and this was re-confirmed live
against the current public `3.0.0` release during ForgeWire's WI 238 migration
(capture 016). This work item evaluates a **structural** solution: make contract
discovery distinguish the canonical repository tree from an embedded/scratch git
worktree checkout by some property of the checkout itself (e.g. its `.git` entry being
a file, not a directory, pointing into the primary checkout's
`.git/worktrees/<name>/`), not merely by name. In scope: contract discovery's
worktree/nested-repository distinction. Out of scope: any other RepoPact command's
directory-exclusion behavior, and any change to how `takeover`, `doctor`, or `adopt`
treat worktrees unless the same root cause turns out to affect them too (to be
determined during implementation, not assumed here).

## Decisions

Not yet made — this item is proposed, not started. The central open decision is named
in AC-3/AC-6: whether contract discovery should key off `git`-level worktree identity
(structural), a directory-name allowlist (as today, just wider), or a combination, and
how nested repositories (a deliberately vendored or embedded sub-repository, as
distinct from a worktree of the same repository) should be treated. This work item
must not assume the answer in advance.

## Scope

Not yet started. Expected to touch `repopact/repo_model.py` (`iter_contracts`,
`IGNORED_PARTS`), `tests/test_validate_repo.py`, and likely a new `decisions/` record
for the structural choice made.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
