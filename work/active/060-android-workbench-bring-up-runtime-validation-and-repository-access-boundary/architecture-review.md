# WI060 Architecture Baseline

Recorded before any code was modified, from completed WI055, WI057, WI058,
WI059, and Decision 0041.

## Current desktop crate shape (pre-WI060)

- `rust/apps/repopact-desktop/src-tauri/src/main.rs` is the **only** source
  file in the crate. There is no `src/lib.rs` and `Cargo.toml` declares no
  `[lib]` target — only the default binary target. This is the standard
  desktop-only Tauri shape; Tauri's Android/iOS mobile runtime requires a
  `[lib]` target with `crate-type = ["staticlib", "cdylib", "rlib"]` and a
  `#[cfg_attr(mobile, tauri::mobile_entry_point)]` entry function, which does
  not exist yet.
- `select_repository` (main.rs:24-40) calls
  `app.dialog().file().blocking_pick_folder()` and then `.into_path()` on the
  result. `blocking_pick_folder()` is `tauri-plugin-dialog`'s desktop folder
  picker. Tauri's Android dialog implementation does not implement folder
  selection at all — Android's OS-level folder concept for third-party apps
  is the Storage Access Framework's `ACTION_OPEN_DOCUMENT_TREE`, which does
  not return a filesystem path but a `content://` URI scoped by the
  document-tree grant. `into_path()` has no way to produce a real recursive
  filesystem directory from that URI even if the picker existed on Android.
- `DesktopService::open_repository` (repopact-desktop-api/src/lib.rs:420-450ish)
  takes a `PathBuf` and constructs a `Repository`/`RepositorySession` directly
  from it — it fundamentally consumes a filesystem directory. There is no
  document-tree or URI-based repository abstraction anywhere in the reusable
  Rust core.
- `repopact_repository::git::NativeGitRunner` invokes an external `git`
  executable via `std::process::Command::new("git")` (through the approved
  process-containment helper). Ordinary Android application packages do not
  ship or have access to a system `git` binary; nothing in the current
  bring-up plan assumes one is available inside the Android app process.
- RepoPact's repository/topology discovery is nonetheless resilient to the
  absence of `.git`: `RepositoryTopology::build` gates every Git query on
  `repository.root.join(".git").exists()` first (see
  `rust/crates/repopact-repository/src/lib.rs`), and falls back to plain
  filesystem discovery for work items, decisions, evidence, and other
  records when there is no `.git`. This is why RepoPact can validate an
  **exported, non-Git tree** at all — it is not a special Android
  accommodation, it is an existing property of the repository model that
  WI060 relies on for its app-private validation fixture.
- WI058 ("User-Centric Tabbed Workbench Information Architecture") designed
  and shipped the compact/adaptive tab primitive, pagination, and IA
  restructuring intended to work down to phone-width viewports, and proved
  it via the desktop WebView2 host resized to mobile/tablet breakpoints. Its
  own closeout evidence (`evidence/runs/20260910-058-adaptive-ia.json`)
  records the platform matrix explicitly: `"android": {"runtime":
  "unexecuted", "build": "unexecuted; adb/Android SDK unavailable"}`. WI058
  did **not** produce Android runtime evidence, and it did not attempt or
  claim to solve Android repository selection — its scope was desktop IA
  only, resized. WI060 does not reopen WI058; it is the first item to
  actually run that IA on an Android runtime.
- Decision 0041 (Tauri desktop uses Rust-owned repository sessions and
  opaque mutation-plan handles) establishes the authority boundary WI060
  must preserve on Android too: the native process owns the
  `RepositorySession`, mutation plans are opaque native-owned handles never
  round-tripped through the frontend, Tauri commands are narrow/typed rather
  than generic filesystem or shell operations, and no plugin/capability is
  granted beyond what a specific operation requires. Decision 0041 does not
  discuss mobile; WI060 must apply the same reasoning to Android rather than
  relax it for mobile convenience.

## What this means for WI060's scope

Android production repository selection is **not solved** by anything that
currently exists in the repository. The correct WI060 posture is:

1. Do not implement a fake/placeholder Android repository-selection path
   that appears to work but silently does something unsafe (broad storage
   permission, hard-coded `/sdcard` path, treating a `content://` URI as a
   filesystem path, or an unrestricted `open_repository(path)` command).
2. Gate `select_repository` by platform: keep the desktop native picker
   exactly as-is, and return an explicit, typed
   `repository.mobile-selection-unavailable` error on a normal Android
   build when no mobile acquisition architecture exists yet.
3. Prove the rest of the Workbench (IA, Back, orientation, safe-area, real
   Rust-backed reads and mutations) against a debug-only, app-private,
   Git-free validation tree — a deliberately narrow bring-up mechanism, not
   a production feature — gated behind a Cargo feature or
   `cfg(all(target_os = "android", debug_assertions))` so it cannot be
   reached from a normal Android production build.
4. If, after real bring-up, the production repository-acquisition boundary
   is still unresolved (the expected outcome, given the facts above), record
   that explicitly and open a follow-up work item (WI061) to design it,
   rather than deciding or implementing that architecture inside WI060.
