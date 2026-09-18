# 070 — RepoPact 3.1.3 downloadable Workbench installers

> **Status**: 🚧 Active
> **Owners**: tooling-owner (lead), docs-owner, governance-owner.
> **Depends on**: [`062`](../../completed/062-cross-platform-workbench-installation-launcher-integration-and-operator-validation/) (installer build/install/launch proof already exists; this item rebuilds and publishes it).

## Intent

WI062 proved that a real Windows NSIS installer, Linux .deb, and Android APK
build, install, launch, and uninstall correctly — but that build predates all
3.1.x release work, and none of those artifacts were ever attached to a public
GitHub Release; only the Python wheel/sdist has ever been publicly
downloadable. This work item:

- rebuilds all three installer artifacts from the current 3.1.2-based tree;
- moves Android from WI062's debug build to a signed release build;
- publishes RepoPact 3.1.3 through the normal governed Python release path
  (VERSION, decision record, build/verify/tag/publish) with the installers
  attached to the same GitHub Release;
- updates README with real download instructions.

Out of scope (operator decision, recorded 2026-09-17): code signing /
notarization for Windows and macOS. Artifacts ship unsigned; the README and
release notes disclose this plainly (SmartScreen/Gatekeeper warnings
expected). macOS/iOS native builds remain out of scope — this repository has
no native macOS/iOS validation evidence yet (see README's platform-support
wording) and this item does not change that.

## Decisions

- Ship Windows/macOS artifacts unsigned rather than acquire a code-signing
  certificate for this release — cost/time tradeoff, operator's explicit
  choice. Revisit for a later release if warranted.
- Reuse WI062's NSIS-over-MSI choice and icon set; no fresh packaging
  architecture decisions expected here, only a rebuild against current source
  plus real publication.
- Android signing key: operator-supplied (new or reused); the private key
  material itself is never committed to the repository, evidence, or CI
  configuration.

## Scope

- `rust/apps/repopact-desktop/src-tauri/` (bundler invocation only; no
  capability/CSP/permission changes).
- `README.md` (download links/instructions).
- `VERSION`, `RELEASE_LABEL` (transient per Decision 0032), `pyproject.toml`,
  `CITATION.cff`, `CONFORMANCE.md`, `conformance/manifest.json`, `SPEC.md`
  (regenerated) — the usual release-identity set.
- A new release decision record under `decisions/`.
- `evidence/runs/` — installer build/install/launch proof per platform, and
  the Python package release proof.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
