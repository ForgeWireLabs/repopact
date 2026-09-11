# 061 — Mobile Repository Acquisition, App-Private Workspace, and Git Strategy

> **Status**: 📋 Planning
> **Owners**: tooling (lead).
> **Depends on**: 060.

## Intent

WI060 brought the RepoPact Workbench up on Android end-to-end: the shared
desktop/mobile Tauri crate builds and launches on a real device, the compact
IA renders correctly across all eight sections, Android system Back/orientation/
lifecycle/IME behavior is proven, and a real Rust-owned mutation plan/review/
apply flow was exercised on-device with zero Git-process dependency. All of
that ran against a debug-only, feature-gated, app-private *validation*
repository staged by the operator via `adb`/`run-as` — deliberately not a
production repository-selection mechanism.

WI060 explicitly declined to solve production Android repository
*acquisition*: `select_repository` on a normal Android build returns a typed
`repository.mobile-selection-unavailable` error, because Tauri's Android
dialog plugin cannot pick a folder, an Android `content://` URI is not a
filesystem path `DesktopService` can consume, and no system `git` binary can
be assumed to exist on the device.

This work item's outcome is a **decision**, not code: compare the realistic
architectures for letting a real Android user get a real RepoPact-governed
tree onto their device (and, where relevant, back off it), and record which
one RepoPact adopts — or an explicit staged sequence — as a durable decision.
Implementation is out of scope here and belongs to whatever follow-up work
item this one's closeout creates.

## Decisions

None yet — this work item exists to produce one. The resulting decision
should be promoted to `decisions/` per the usual convention once accepted.

## Scope

- A comparison document (or this work item's closeout evidence) weighing:
  app-private import/export via SAF, a live SAF working tree, archive import,
  an embedded Git implementation (libgit2/gix), and remote clone/sync.
- A recorded recommendation with rejected-alternative reasoning.
- No production code changes; no Android Gradle/Kotlin/Rust changes.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are
satisfied, move this directory to `work/completed/`, regenerate the
dashboard, and open the implementation follow-up work item AC-5 requires.
