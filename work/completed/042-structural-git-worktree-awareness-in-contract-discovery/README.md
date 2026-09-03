# 042 — Structural Git-Worktree Awareness in Contract Discovery

> **Status**: ✅ Completed
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

Decision [0035](../../../decisions/0035-structural-git-worktree-awareness.md) adopts
layered structural detection: registered worktree paths from `git worktree list
--porcelain`, embedded `.git` files pointing into the primary `.git/worktrees`
directory, and the retained `worktrees` name fallback for stale/orphaned scratch
trees. Independent nested repositories with `.git/` directories remain discoverable.

## Scope

Implemented in `repopact/repo_model.py` (`discover_embedded_worktree_roots` and
`iter_contracts`), with real Git worktree and nested-repository regression coverage
in `tests/test_validate_repo.py`. The walker normalizes Windows paths, handles Git
unavailability/exported trees deterministically, and prunes identified linked roots
before descending.

## Evidence and closeout

Evidence run [20260902-042-structural-worktree-awareness](../../../evidence/runs/20260902-042-structural-worktree-awareness.json)
records the structural alternatives, chosen layered detector, real conventional and
non-conventional linked-worktree cases, stale metadata fallback, independent nested
repository preservation, Windows path handling, Git-free exported-tree behavior,
negative controls, cleanup, and complete local validation. All six acceptance
criteria are satisfied. The complete directory is now under `work/completed/`, and
dashboard/SPEC outputs were regenerated and validated after the lifecycle transition.
