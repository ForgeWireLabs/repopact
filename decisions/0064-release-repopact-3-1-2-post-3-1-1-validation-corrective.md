---
id: 0064
title: Release RepoPact 3.1.2 post-3.1.1 validation corrective
status: accepted
date: 2026-09-17
supersedes: []
---

# 0064: Release RepoPact 3.1.2 post-3.1.1 validation corrective

## Context

An independent validation pass against public `repopact==3.1.1` (work item 069)
confirmed the published package itself is sound — the wheel installs cleanly,
its bundled native engine reports `3.1.1` and agrees with the CLI, `repopact
init`/`repopact validate` pass on a fresh repository, and no silent Python
semantic fallback occurs. The same pass found two avoidable local/tooling
failure modes exposed, but not caused, by exercising the package end to end:

- there was no conventional top-level `repopact --version` observability
  surface, only the existing `validate`/`doctor` subcommands and the
  already-exposed `repopact.__version__` Python attribute;
- the Workbench type generator (`rust/apps/repopact-desktop/scripts/generate-types.mjs`)
  unconditionally defaulted `CARGO_TARGET_DIR` to a single machine-wide shared
  temp directory (`%TEMP%\repopact-rust-target`), which is implicated in a
  `tree-sitter` build corruption observed when two unrelated Rust builds ran
  concurrently against that shared cache.

Neither defect affects schema, lifecycle, protocol, or provenance behavior, and
neither is a regression introduced by 3.1.1 — both are gaps exposed by
validating 3.1.1 more thoroughly than prior release work had.

## Decision

Release RepoPact `3.1.2` as a backwards-compatible corrective patch, following
the precedent of decisions 0033 (`3.0.1`) and 0063 (`3.1.1`). The runtime/tooling
delta is:

- add a top-level `repopact --version` flag that exits 0 without requiring a
  subcommand and prints `repopact <version>` from the existing governed
  `repopact.__version__` identity (no duplicated version constant);
- stop defaulting `CARGO_TARGET_DIR` in `generate-types.mjs` to a shared temp
  directory; inherit the caller's environment unchanged so Cargo uses the
  normal, already-`.gitignore`d workspace target (`rust/target/`), while an
  explicitly supplied `CARGO_TARGET_DIR` continues to work exactly as before;
- reconcile release/publication evidence and current-state governance wording
  (WI021 AC-3, evidence run for the 3.1.1 clean-install proof) with the real
  published state.

No schema, protocol, CLI subcommand semantics, lifecycle, or provenance
behavior changes. `--version` is strictly additive. The `v3.1.0` and `v3.1.1`
tags and their already-published wheels/sdists are left untouched; `3.1.2`
supersedes `3.1.1` as the current stable identity and is the version new
installs should target. Both the wheel and sdist are rebuilt and published for
`3.1.2` from the corrected tree, preserving the 3.1.1 sdist `LICENSE`-file fix
from decision 0063.

## Alternatives considered

- **Fix the local environment only, without a release.** Rejected: the
  `CARGO_TARGET_DIR` default is packaged repository source
  (`generate-types.mjs`), not machine-local configuration — every adopter
  running the Workbench build from source inherits the same shared-cache
  corruption risk until the shipped tree changes.
- **Bundle these fixes into the next feature-carrying MINOR release instead of
  cutting a patch now.** Rejected: both defects are corrective (an additive CLI
  observability gap and a tooling isolation bug), not new capability, and the
  validation pass that found them is better closed out with a concrete,
  independently verifiable release than left open indefinitely.

## Consequences

`VERSION`, package metadata, `CONFORMANCE.md`, `SPEC.md`'s changelog line, and
`conformance/manifest.json`'s `suite_version` move to `3.1.2`. Per decision
0032, source between the `v3.1.1` tag and the `3.1.2` release commit carries a
VERSION-pinned `RELEASE_LABEL` (`3.1.1-dev.1`) so development package/runtime
identity remains observably distinct from both the `3.1.1` and `3.1.2` stable
wheels; the label is removed for the exact `3.1.2` release tree. The release
candidate is rebuilt with `repopact release-build`, verified reproducible, and
`twine check`-clean before publication of both the wheel and sdist.
