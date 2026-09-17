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

use tauri::{AppHandle, Manager, Runtime};

use repopact_mobile_credential::AndroidCredentialStore;
use repopact_remote_provider::credential::{CredentialKey, CredentialKind, CredentialStore};
use repopact_remote_provider::error::RemoteProviderResult;
use repopact_remote_provider::redact::Secret;

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

// ---------------------------------------------------------------------
// WI067 Checkpoint E (GH-004): a bounded, debug-only native test/control
// surface for `AndroidCredentialStore`, exercised from a real emulator or
// device to prove the production Keystore-backed backend actually works
// at runtime -- put/get/overwrite/delete, distinct access/refresh keys,
// process-restart persistence.
//
// This exists purely to let the WI067 Checkpoint E verification procedure
// drive `AndroidCredentialStore` from outside the app (via `adb shell`
// invoking these Tauri commands through the WebView's own
// `window.__TAURI__.core.invoke`, exactly like any other frontend call).
// Every command here takes a caller-*supplied* synthetic canary as input
// and returns only a boolean/status outcome -- **never** the retrieved
// secret value itself. There is still no command capable of retrieving a
// real token: `debug_credential_get_matches` only ever tells the caller
// whether the stored value equals a value the caller already knows,
// which is meaningless for a real, unknown, previously-stored GitHub
// token an attacker does not already possess.
//
// Compiled only for a `debug_assertions` Android build with
// `--features android-debug-validation` (this whole module's own gate,
// see `lib.rs`); never present in a release build.

fn debug_key(connection_id: &str, kind: &str) -> CredentialKey {
    CredentialKey {
        provider: "debug-validation".to_string(),
        connection_id: connection_id.to_string(),
        kind: if kind == "refresh" {
            CredentialKind::RefreshToken
        } else {
            CredentialKind::AccessToken
        },
    }
}

pub(crate) fn debug_credential_put<R: Runtime>(
    app: &AppHandle<R>,
    connection_id: &str,
    kind: &str,
    secret: &str,
) -> RemoteProviderResult<()> {
    AndroidCredentialStore::new(app.clone())
        .put(&debug_key(connection_id, kind), Secret::new(secret))
}

/// Returns `true` only if a credential is currently stored for this key
/// *and* its decrypted value equals `expected` -- the decrypted value
/// itself is never returned to the caller.
pub(crate) fn debug_credential_get_matches<R: Runtime>(
    app: &AppHandle<R>,
    connection_id: &str,
    kind: &str,
    expected: &str,
) -> RemoteProviderResult<bool> {
    let found = AndroidCredentialStore::new(app.clone()).get(&debug_key(connection_id, kind))?;
    Ok(found.is_some_and(|secret| secret.expose() == expected))
}

pub(crate) fn debug_credential_present<R: Runtime>(
    app: &AppHandle<R>,
    connection_id: &str,
    kind: &str,
) -> RemoteProviderResult<bool> {
    let found = AndroidCredentialStore::new(app.clone()).get(&debug_key(connection_id, kind))?;
    Ok(found.is_some())
}

pub(crate) fn debug_credential_delete<R: Runtime>(
    app: &AppHandle<R>,
    connection_id: &str,
    kind: &str,
) -> RemoteProviderResult<()> {
    AndroidCredentialStore::new(app.clone()).delete(&debug_key(connection_id, kind))
}

pub(crate) fn debug_credential_key_info<R: Runtime>(
    app: &AppHandle<R>,
) -> RemoteProviderResult<(bool, Option<bool>)> {
    AndroidCredentialStore::new(app.clone()).key_info()
}
