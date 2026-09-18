---
id: 0066
title: Release RepoPact 3.1.3 downloadable Workbench installers
status: accepted
date: 2026-09-18
supersedes: []
---

# 0066: Release RepoPact 3.1.3 downloadable Workbench installers

## Context

Work item 062 (completed 2026-09-12) proved that a real Windows NSIS
installer, Linux .deb package, and Android APK build, install, launch, and
uninstall correctly — the first real proof that RepoPact's Workbench packages
at all beyond a dev-tree `npm run tauri -- dev` invocation. That work
explicitly deferred code signing, notarization, store publication, and public
distribution as separate concerns from local install proof.

Since then, none of those installer artifacts have ever been attached to a
public GitHub Release; only the Python wheel and sdist have ever been
publicly downloadable (confirmed by inspecting `v2.1.0`'s actual release
assets). WI062's own build also predates all 3.1.x release work by several
commits, so it no longer reflects the current source tree.

## Decision

Release RepoPact `3.1.3` as a backwards-compatible packaging/distribution
release (work item 070). The delta from `3.1.2`:

- Windows NSIS (`.exe`) and MSI (`.msi`) installers, a Linux `.deb` package,
  and a signed Android release APK are rebuilt from the current tree and
  proved via the same install/launch/uninstall cycle WI062 established.
- Android moves from WI062's debug build to a signed release build. A fresh
  release keystore was generated 2026-09-17 (RSA 4096, 30-year validity,
  alias `repopact-release`); the private key material is held outside the
  repository by the operator and is never committed, logged, or placed in
  evidence — only its certificate fingerprint is recorded.
- Windows and macOS artifacts ship **unsigned**. Acquiring a code-signing
  certificate was considered and explicitly declined for this release
  (operator decision, cost/time tradeoff); README and release notes disclose
  that unsigned binaries may trigger SmartScreen/Gatekeeper warnings.
- All four installer artifacts are attached to the `v3.1.3` GitHub Release
  alongside the normal PyPI wheel/sdist publication — the first RepoPact
  release where these binaries are actually publicly downloadable rather than
  existing only as local build evidence.

This release also ships the already-implemented portion of work item 047
(decision 0065): an optional, schema-enforced `documentation_impact` work-item
field, validated at the `completed` transition, disabled by default and
epoch-grandfathered exactly like decision 0021's preflight mechanism (no
existing adopter or historical RepoPact work item is affected unless they
explicitly opt in and create new work after the epoch). WI047 itself remains
`active` — several of its acceptance criteria (adopter-declared
source-to-documentation mappings, generated-documentation staleness
detection, doctor/audit reconciliation) are explicitly deferred to a
follow-up decision per 0065 section E — but the schema field, validator rule,
and their conformance coverage are complete, tested, and backward-compatible,
so they ship now rather than waiting on WI047's full closure. This is an
additive capability, not a breaking schema/protocol/lifecycle change: no
existing work item, schema consumer, or CLI subcommand semantics changes
unless an adopter opts in.

The `v3.1.0`, `v3.1.1`, and `v3.1.2` tags and their already-published
artifacts are left untouched; `3.1.3` supersedes `3.1.2` as the current
stable identity and is the version new installs should target.

## Alternatives considered

- **Acquire a code-signing certificate before this release.** Rejected for
  now: real cost and lead time with no functional blocker on distribution —
  unsigned installers work, they just carry an OS warning. Revisit for a
  later release if warranted.
- **Publish installers as a separate, unversioned "assets" release rather
  than tying them to a PyPI version bump.** Rejected: RepoPact's release
  discipline (decision 0032/0033 lineage) treats the package/runtime source
  tree and its published identity as one unit; a distribution-only change
  still gets a real `VERSION` so the exact tree that produced each artifact
  is traceable and reproducible, consistent with every prior corrective.
- **Reuse WI062's original 2026-09-12 build artifacts instead of rebuilding.**
  Rejected: those predate the 3.1.1/3.1.2 corrective work by several commits;
  shipping them would distribute a stale tree under the 3.1.3 label.

## Consequences

`VERSION`, package metadata, `CONFORMANCE.md`, `SPEC.md`'s changelog line,
and `conformance/manifest.json`'s `suite_version` move to `3.1.3`. Per
Decision 0032, source between the `v3.1.2` tag and the `3.1.3` release commit
carries a VERSION-pinned `RELEASE_LABEL` (`3.1.2-dev.1`) so development
package/runtime identity remains observably distinct from the `3.1.2` stable
wheel; the label is removed for the exact `3.1.3` release tree. Future
signing-certificate acquisition, if pursued, is a separate decision — this
one only records that 3.1.3 ships unsigned by explicit choice.
