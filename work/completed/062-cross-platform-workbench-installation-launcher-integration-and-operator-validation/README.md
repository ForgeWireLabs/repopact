# 062 — Cross-Platform Workbench Installation, Launcher Integration, and Operator Validation

> **Status**: ✅ Completed
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

All 23 acceptance criteria are satisfied. Jeremy personally confirmed all
three platforms:

- **Windows**: "ran it and windows rp works as well as displays the icon
  and shortcut" — after a real defect was found and fixed (see below).
- **Android**: "android rp icon is correct and works."
- **Linux**: "yep that worked" — after a stale WSLg session was cleared.

Two genuine defects were discovered and fixed during operator validation,
both worth remembering for any future agent-driven install work on this
machine:

1. **Windows Desktop shortcut pointed at a virtualized path.** Running the
   NSIS installer through this agent's own sandboxed process caused Windows
   to silently redirect the Desktop-folder resolution to a Claude-internal
   package cache path (`AppData\Local\Packages\Claude_...\...`), invisible
   to Jeremy's real session. Every earlier "verified" check of that
   shortcut was unknowingly checking the same redirected view and reported
   a false positive. Fixed by having Jeremy run the installer directly
   himself — the only reliable way to get a correctly resolved per-user
   shortcut when the installer touches Desktop/AppData paths.
2. **WSLg session staleness.** Repeated agent-driven launch/kill cycles of
   the Linux GUI app across one long session left the WSLg taskbar entry
   showing a live thumbnail but not responding to clicks, for both the
   agent's session and Jeremy's own terminal launch. A full `wsl --shutdown`
   followed by a fresh terminal and a single launch resolved it.

See `work-item.json` and `evidence/runs/2026091{1,2}-062-*.json` for the
full acceptance-criteria-to-evidence mapping.
