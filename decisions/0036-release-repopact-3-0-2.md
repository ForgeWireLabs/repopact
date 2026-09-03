---
id: "0036"
title: Release RepoPact 3.0.2 corrective path and worktree fixes
status: accepted
date: 2026-09-02
supersedes: []
---

# 0036: Release RepoPact 3.0.2 corrective path and worktree fixes

## Context

The exact `v3.0.1..HEAD` inventory contains two completed, backwards-compatible
runtime corrections: WI043's record-relative `source_of_truth:` diagnosis and
WI042's structural same-repository linked-worktree exclusion. The remaining
changes are tests, decisions, research, evidence, audit/derived output, and
work-item lifecycle records needed to govern and prove those corrections. No
other package or runtime behavior is present in the release delta.

## Decision

Release RepoPact 3.0.2 as a backwards-compatible corrective patch. The exact
runtime/package delta is:

- `repopact/doctor.py` resolves every `source_of_truth:` token from the
  declaring record's parent, including bare names and `../` paths, and keeps
  stale-pointer repair non-destructive;
- `repopact/repo_model.py` prunes registered and embedded same-repository linked
  worktrees during contract discovery, handles Windows and space-containing
  paths, preserves independent nested repositories, and retains the literal
  `worktrees` fallback for stale/orphaned scratch trees; and
- focused regression tests and the release identity/evidence/derived records
  required to ship those fixes.

The stable release tree uses `VERSION=3.0.2`, no `RELEASE_LABEL`, and package
metadata exactly `3.0.2`, following decision 0032. The exact committed tree is
the only source for deterministic artifacts, tagging, and publication.

## Explicit boundaries

This release does not implement or absorb:

- WI044 typed capability-completion/cutover evidence contracts;
- WI046 runner-neutral verification/admission checkpoint architecture;
- WI047 documentation-impact/code-documentation closure;
- WI037 adopter-fleet migration or package-boundary reconciliation;
- WI032 remote cross-platform enforcement restoration.

WI032 remains blocked under the local-only execution directive. WI037 and the
proposed WI044/WI046/WI047 remain separate obligations.

## Consequences

Existing RepoPact records and adopter compatibility semantics remain unchanged;
the patch only removes false stale-pointer and nested-worktree diagnostics.
Public publication is proven separately from source and tag success, and no
GitHub-hosted execution is part of this release.
