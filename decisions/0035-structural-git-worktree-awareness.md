---
id: 0035
title: Detect same-repository linked worktrees structurally during contract discovery
status: accepted
date: 2026-09-02
supersedes: []
---

# 0035: Detect same-repository linked worktrees structurally during contract discovery

## Context

Finding F-015 and capture 016 showed that a linked checkout nested below a
RepoPact root can expose its own `AGENTS.md` files to the parent filesystem scan.
RepoPact 3.0.1 retained the literal `worktrees` directory exclusion as a narrow
compatibility correction, but a linked checkout under `scratch-agent/feature-x`
has the same identity without that name. A blanket exclusion of nested Git
repositories would hide legitimate governed vendor trees.

## Decision

Contract discovery uses layered structural awareness:

1. Existing `IGNORED_PARTS` remains the first, cheap filesystem fast path. In
   particular, `worktrees` stays in the set to protect stale/orphaned conventional
   scratch trees whose Git metadata has been removed or cannot be read.
2. When the scan root has usable Git metadata, `git worktree list --porcelain`
   identifies registered worktree paths. Registered worktrees that are strict
   descendants of the scan root are pruned from contract discovery; the scan root
   itself is never pruned.
3. A nested checkout whose `.git` is a file is independently recognized when its
   `gitdir:` target resolves under the primary repository's `.git/worktrees`
   directory. This catches stale-but-identifiable linked checkouts even when Git's
   registry no longer reports them.
4. A nested `.git/` directory is not a linked-worktree signal. Independent nested
   repositories and intentionally governed subtrees remain discoverable and must
   satisfy normal registration and audit rules.

All path comparisons use normalized `Path` values, preserving Windows drive-letter,
backslash, and space-containing paths. Git invocation is best-effort: an unavailable
executable, absent metadata, exported tree, or nonzero Git probe leaves ordinary
discovery deterministic and retains the name-based fallback. The walker prunes
identified linked roots before descending into them and never mutates the tree.

## Alternatives evaluated

- **Embedded `.git` file alone:** useful for stale linked roots, but insufficient
  when metadata is unusual and cannot independently enumerate registered paths.
- **`git worktree list --porcelain` alone:** authoritative for live registrations,
  but misses stale/orphaned scratch trees left on disk after a session.
- **Combination (chosen):** covers both live and stale-identifiable same-repository
  worktrees while keeping independent nested repositories visible.
- **Directory-name allowlist only:** rejected as the primary mechanism because a
  renamed linked worktree reproduces the defect unchanged; the existing
  `worktrees` name is retained only as a compatibility/performance fallback.

## Consequences

The ForgeWire-shaped conventional path remains protected, and an otherwise
identical linked worktree moved to an arbitrary directory is also excluded. A
genuine nested repository continues to influence ownership and validation. No
other command's traversal policy, package version, or release tag is changed.
