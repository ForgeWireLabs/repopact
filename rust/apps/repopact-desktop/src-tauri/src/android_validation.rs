//! WI060 AND-007/AND-008: a debug-only, app-private validation repository
//! path, unreachable from a normal production Android build.
//!
//! This module exists solely so the rest of the Workbench (IA, navigation,
//! Rust-backed reads/mutations) can be exercised on an Android runtime while
//! production repository *acquisition* remains an explicitly unresolved,
//! separately tracked problem (see the WI060 closeout report / WI061). It is
//! compiled in only when this crate is built with
//! `--features android-debug-validation` for a `debug_assertions` Android
//! target; a release build never includes this file's code at all.
//!
//! # How the validation tree is staged
//!
//! 1. On the Windows host, export a Git-free copy of the RepoPact source
//!    checkout (or a representative governed scratch tree) — the same
//!    `git ls-files --cached --others --exclude-standard` selection
//!    `repopact/dev_fixtures.py` already uses for Python test fixtures, so
//!    it excludes `.git`, `rust/target`, `node_modules`, and other ignored
//!    build output.
//! 2. Push that tree onto the device/emulator with `adb push <export-dir>
//!    /data/local/tmp/wi060-validation-repo`.
//! 3. Copy it from the world-readable staging location into the debug app's
//!    private data directory using the debuggable package's `run-as` shell,
//!    which is only available for a `debuggable="true"` (debug-signed)
//!    build — never for a release build:
//!    `adb shell run-as com.forgewirelabs.repopact.workbench sh -c
//!    'mkdir -p files/wi060-validation-repo && cp -r
//!    /data/local/tmp/wi060-validation-repo/. files/wi060-validation-repo/'`
//! 4. The path this function returns, `app.path().app_data_dir()` joined
//!    with `files/wi060-validation-repo`, resolves to that same app-private
//!    directory. Tauri's Android `app_data_dir()` is `activity.dataDir`
//!    (the package's data *root*, e.g.
//!    `/data/user/0/com.forgewirelabs.repopact.workbench`), not its `files/`
//!    subdirectory, so the join must include `files` explicitly — unlike
//!    desktop platforms, where `app_data_dir()` already names a leaf
//!    directory. Nothing outside the app's own private storage is read or
//!    granted.
//!
//! If the directory has not been staged, this returns `None` and
//! `select_repository` falls through to the same
//! `repository.mobile-selection-unavailable` error a production build
//! returns — staging failure never silently succeeds or fabricates a path.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

const VALIDATION_REPO_DIR_NAME: &str = "wi060-validation-repo";

/// Tauri's Android `app_data_dir()` resolves to `activity.dataDir`, i.e. the
/// package's data *root* (`/data/user/0/<package>`, containing `files/`,
/// `cache/`, `shared_prefs/`, ...), not its `files/` subdirectory — unlike
/// desktop, where `app_data_dir()` already points at a leaf directory safe to
/// write into directly. `run-as`'s shell cwd is that same data root, so
/// `files/<name>` in the staging command and `app_data_dir().join("files")`
/// here must agree.
pub(crate) fn debug_validation_repository_path(app: &AppHandle) -> Option<PathBuf> {
    let base = app.path().app_data_dir().ok()?;
    let candidate = base.join("files").join(VALIDATION_REPO_DIR_NAME);
    candidate.is_dir().then_some(candidate)
}
