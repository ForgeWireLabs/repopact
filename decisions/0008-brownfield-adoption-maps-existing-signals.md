---
id: 0008
title: Brownfield Adoption Maps Existing Governance Signals
status: accepted
date: 2026-06-15
supersedes: []
---

# 0008: Brownfield Adoption Maps Existing Governance Signals

## Context

`init` serves greenfield repositories. The harder, more common case is a mature
project that already has ad-hoc governance — `CODEOWNERS`, CI workflows, nested
`AGENTS.md` contracts, and a long git history — and wants to come under RepoPact
without discarding any of it. Without a first-class path, adoption means hand-writing
dozens of records, which no maintainer will do.

## Decision

Add `repopact adopt --target <repo>` (module `adopt_repo.py`). It reads the
repository's existing signals and generates RepoPact records *around* them, and it
is **strictly non-destructive**: it creates a file only if that exact path is absent,
and reports what was created vs. skipped. `--dry-run` prints the plan without writing.

The mapping is:

- **`CODEOWNERS` → scopes and roles.** Each owner handle becomes a scope (its
  assigned path globs) and a role, alongside a always-present `governance` scope.
- **`.github/workflows/*` → binding gates.** Each workflow becomes a `status: active`
  policy describing the gate, plus invariant `INV-2` ("declared CI workflows are
  binding gates") with `enforced_by` pointing at the workflows directory, plus a
  frozen-surface entry protecting `.github/workflows/**`.
- **Nested `AGENTS.md` → registered contracts.** Every nested contract is registered
  in `audits/registry.json`; where an `_audit/` directory already exists, the
  RepoPact-required triplet (`README.md`, `inventory.md`, `alignment-report.md`) is
  stubbed only for the files that are missing.
- **git history → first evidence.** History stats (commit count, latest tag,
  contributors) are captured in an evidence run that satisfies a completed
  `000-adopt-repopact` work item, so the adoption itself is recorded the RepoPact way.

The adopted tree is validated before the command returns; a non-empty problem list
is reported and exits non-zero.

## Alternatives considered

- **Interactive wizard.** Rejected for v1: a deterministic, idempotent, re-runnable
  command composes with CI and scripting; interactivity can layer on later.
- **Overwrite/normalize existing files** (e.g. rewrite the repo's `AGENTS.md`).
  Rejected: destroying existing intent is the fastest way to lose a maintainer's
  trust. RepoPact adds records around what exists.
- **Fabricate work items from commits.** Rejected: inventing "completed" work from
  commit messages would manufacture evidence that does not correspond to acceptance
  criteria — a violation of the project's own evidence discipline.

## Consequences

- Existing CI and ownership become first-class, machine-checkable bindings.
- Re-running `adopt` is safe (idempotent: already-present records are skipped).
- Some repositories will not validate immediately (e.g. unusual `_audit` layouts);
  those are surfaced as validation errors for the maintainer, not silently patched.
