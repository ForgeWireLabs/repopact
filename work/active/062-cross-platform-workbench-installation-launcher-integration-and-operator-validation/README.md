# 062 — Cross-Platform Workbench Installation, Launcher Integration, and Operator Validation

> **Status**: 🚧 Active
> **Owners**: tooling (lead).
> **Depends on**: 055, 058, 060.

## Intent

WI060 proved the Workbench *runs* correctly on Android once installed via
`adb`. It never proved RepoPact behaves like a normally installed
application a real user can find and launch on any platform. Jeremy
personally inspected the Precision after WI060 and confirmed the gap: no
Windows Start-menu/Desktop shortcut, no confirmed Android app-drawer
discoverability, and no Linux GUI install at all (WI059 only exercised Rust
CLI validation under WSL2 Debian 13).

This work item is packaging and launcher-integration only. It builds and
installs real platform packages (Windows NSIS, Linux `.deb`), proves
Start-menu/Desktop-shortcut/app-drawer discoverability, and — folded in after
a live Android install showed a Tauri placeholder icon — replaces the
application icon set with a minimal "RP" mark across all three platforms.

**In scope:** Windows NSIS installer + Desktop-shortcut NSIS hook, Android
launcher-resolution and app-drawer verification, Linux `.deb` build/install
under a Linux-native WSL2 checkout, the shared Tauri icon source and its
regenerated per-platform outputs, and the operator-confirmation evidence for
all of the above.

**Out of scope:** WI060's Android production repository-selection boundary
(not reopened), WI061's mobile repository-acquisition architecture (not
implemented here), code signing / notarization / store publication / public
auto-update (a separate concern from local install proof), and any
loosening of Tauri capabilities, CSP, filesystem/shell/process permissions,
WI050 admission, WI057 process guarantees, or WI059 validator authority.

## Decisions

- Windows: NSIS over MSI, because it supports a per-user install without
  requiring administrator privileges, matching this operator-proof context.
- The Desktop shortcut is added via Tauri's supported NSIS hook mechanism
  rather than replacing the generated installer template, to keep the
  change narrow and upgrade-safe.
- The application icon's master source is regenerated through Tauri's
  canonical icon-generation workflow rather than hand-patching individual
  platform outputs, so Android/Windows/Linux never drift from a single
  source of truth.

## Scope

- `rust/apps/repopact-desktop/src-tauri/tauri.conf.json` (bundle/NSIS config)
- `rust/apps/repopact-desktop/src-tauri/icons/` (new master "RP" source + regenerated outputs)
- `rust/apps/repopact-desktop/src-tauri/gen/android/app/src/main/res/mipmap-*` (regenerated launcher icons)
- a narrow NSIS hook script for the Desktop shortcut (installer-only, no runtime code)
- `work/active/062-.../` (this item)
- `evidence/runs/` (Windows/Android/Linux install-and-launch evidence)

## Closeout

Each acceptance criterion (AC-001 through AC-023) is satisfied by linked
evidence. Criteria requiring Jeremy's personal confirmation (Start-menu
launch, Desktop-shortcut launch, Android app-drawer launch, post-reboot
persistence, Linux WSLg window confirmation) are never marked satisfied on
the basis of scripts or agent automation alone — they stay `pending` until
Jeremy has personally done and confirmed them. See `work-item.json` for
exact criteria.
