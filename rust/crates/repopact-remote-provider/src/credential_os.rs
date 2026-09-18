//! WI067 Checkpoint B (GH-004): the production, OS-protected
//! [`CredentialStore`] backend. `keyring` maps to a real per-platform
//! protected facility:
//!
//! - Windows -> Windows Credential Manager (`windows-native` backend).
//! - macOS/iOS -> Keychain (`apple-native` backend).
//! - Linux -> Secret Service/libsecret (`sync-secret-service` backend).
//!
//! Android has no backend here -- `keyring` does not support it. A
//! Keystore-backed Android implementation of the same [`CredentialStore`]
//! trait is a distinct, tracked gate (see the WI067 Checkpoint B evidence
//! record); this module never silently falls back to plaintext for it.
//!
//! Only the credential value itself goes through this store. Non-secret
//! connection metadata (expiry timestamps, account labels) belongs in
//! ordinary native app state, not here -- keeping the protected-storage
//! surface as narrow as possible per Decision 0061.

use crate::credential::{CredentialKey, CredentialKind, CredentialStore};
use crate::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
use crate::redact::Secret;

/// The keyring "service" namespace RepoPact registers all its entries
/// under, so they are identifiable (and, if ever necessary, bulk-
/// removable) in the OS credential manager without colliding with
/// unrelated applications' entries.
const KEYRING_SERVICE: &str = "com.forgewirelabs.repopact.remote-provider";

fn entry_name(key: &CredentialKey) -> String {
    let kind = match key.kind {
        CredentialKind::AccessToken => "access-token",
        CredentialKind::RefreshToken => "refresh-token",
    };
    format!("{}:{}:{}", key.provider, key.connection_id, kind)
}

fn map_keyring_error(error: keyring::Error) -> RemoteProviderError {
    RemoteProviderError::new(
        ErrorCode::CredentialUnavailable,
        format!("OS credential store error: {error}"),
    )
}

/// Production `CredentialStore` backed by the real OS-protected facility.
/// Contains no in-memory fallback and no plaintext file path -- if the OS
/// facility is unavailable (`keyring::Error::NoStorageAccess` and similar),
/// every call fails typed rather than degrading silently.
#[derive(Default)]
pub struct OsCredentialStore;

impl OsCredentialStore {
    pub fn new() -> Self {
        Self
    }

    fn entry(&self, key: &CredentialKey) -> RemoteProviderResult<keyring::Entry> {
        keyring::Entry::new(KEYRING_SERVICE, &entry_name(key)).map_err(map_keyring_error)
    }
}

impl CredentialStore for OsCredentialStore {
    fn put(&self, key: &CredentialKey, value: Secret) -> RemoteProviderResult<()> {
        self.entry(key)?
            .set_password(value.expose())
            .map_err(map_keyring_error)
    }

    fn get(&self, key: &CredentialKey) -> RemoteProviderResult<Option<Secret>> {
        match self.entry(key)?.get_password() {
            Ok(password) => Ok(Some(Secret::new(password))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(other) => Err(map_keyring_error(other)),
        }
    }

    fn delete(&self, key: &CredentialKey) -> RemoteProviderResult<()> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(other) => Err(map_keyring_error(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests touch the real OS credential facility: Windows Credential
    // Manager on Windows, Keychain on macOS, Secret Service/libsecret on
    // Linux (see this module's own doc comment). They are the strongest
    // available proof of GH-004 short of the manual Workbench restart
    // procedure recorded in evidence -- each test cleans up the real OS
    // entry it creates, including on assertion failure via a drop guard, so
    // a failed run never leaves a stray credential behind.
    //
    // On Linux, `cargo test` for this crate requires a running Secret
    // Service provider (e.g. `gnome-keyring-daemon --components=secrets`
    // under an active D-Bus session). A bare/headless environment with no
    // such session (common in a fresh WSL or container install) will fail
    // these four tests with a `DBus error: The name org.freedesktop.secrets
    // was not provided by any .service files` panic. That is a missing-host-
    // service condition, not a defect in this crate: the same code path
    // passes normally under a real desktop session or CI image that
    // provisions a Secret Service. It does not affect the Windows build,
    // where Credential Manager is always available.

    struct CleanupGuard(CredentialKey);
    impl Drop for CleanupGuard {
        fn drop(&mut self) {
            // Windows Credential Manager occasionally returns a transient
            // error on a delete issued immediately after a put/overwrite
            // in the same process (observed empirically running this
            // suite -- not a defect in this crate's own logic, since a
            // dedicated `delete_removes_the_real_os_entry` test with no
            // other credential activity around it deletes reliably on the
            // first attempt). Retry briefly rather than silently leaving a
            // real OS credential-store entry behind after a test run.
            let store = OsCredentialStore::new();
            for attempt in 0..5 {
                if store.delete(&self.0).is_ok() {
                    return;
                }
                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            panic!(
                "CleanupGuard could not delete real OS credential entry for {:?} after 5 attempts -- a real Windows Credential Manager entry may have been left behind",
                self.0
            );
        }
    }

    fn unique_key(label: &str) -> CredentialKey {
        CredentialKey {
            provider: "github".into(),
            connection_id: format!("checkpoint-b-test-{label}-{}", std::process::id()),
            kind: CredentialKind::AccessToken,
        }
    }

    #[test]
    fn put_then_get_round_trips_through_the_real_os_store() {
        let store = OsCredentialStore::new();
        let key = unique_key("roundtrip");
        let _guard = CleanupGuard(key.clone());

        store
            .put(&key, Secret::new("ghu_test0000000000000000000000000000"))
            .unwrap();
        let got = store.get(&key).unwrap().expect("credential should exist");
        assert_eq!(got.expose(), "ghu_test0000000000000000000000000000");
    }

    #[test]
    fn delete_removes_the_real_os_entry() {
        let store = OsCredentialStore::new();
        let key = unique_key("delete");
        let _guard = CleanupGuard(key.clone());

        store.put(&key, Secret::new("value")).unwrap();
        store.delete(&key).unwrap();
        assert!(store.get(&key).unwrap().is_none());
    }

    #[test]
    fn missing_entry_returns_none_not_an_error() {
        let store = OsCredentialStore::new();
        let key = unique_key("missing");
        assert!(store.get(&key).unwrap().is_none());
    }

    #[test]
    fn put_overwrites_the_previous_value_atomically_from_the_readers_perspective() {
        let store = OsCredentialStore::new();
        let key = unique_key("overwrite");
        let _guard = CleanupGuard(key.clone());

        store.put(&key, Secret::new("first")).unwrap();
        store.put(&key, Secret::new("second")).unwrap();
        assert_eq!(store.get(&key).unwrap().unwrap().expose(), "second");
    }

    /// Not run by default (`cargo test` skips `#[ignore]`). Run explicitly
    /// with `cargo test -p repopact-remote-provider --
    /// leaves_a_real_entry_for_cross_process_inspection --ignored` to
    /// write a real, durable OS Credential Manager entry that a *separate*
    /// process (`cmdkey /list`, or another `cargo test` invocation) can
    /// observe -- the strongest available automated proof that storage
    /// survives past this process's own lifetime, standing in for a
    /// manual close-Workbench/reopen-Workbench restart in this
    /// non-interactive session. Clean up afterward with
    /// `deletes_the_cross_process_inspection_entry --ignored`.
    #[test]
    #[ignore]
    fn leaves_a_real_entry_for_cross_process_inspection() {
        let store = OsCredentialStore::new();
        let key = CredentialKey {
            provider: "github".into(),
            connection_id: "checkpoint-b-cross-process-proof".into(),
            kind: CredentialKind::AccessToken,
        };
        store
            .put(&key, Secret::new("ghu_test0000000000000000000000000000"))
            .unwrap();
    }

    #[test]
    #[ignore]
    fn reads_the_cross_process_inspection_entry_written_by_a_separate_process() {
        let store = OsCredentialStore::new();
        let key = CredentialKey {
            provider: "github".into(),
            connection_id: "checkpoint-b-cross-process-proof".into(),
            kind: CredentialKind::AccessToken,
        };
        let value = store
            .get(&key)
            .unwrap()
            .expect("a prior process must have written this entry first");
        assert_eq!(value.expose(), "ghu_test0000000000000000000000000000");
    }

    #[test]
    #[ignore]
    fn deletes_the_cross_process_inspection_entry() {
        let store = OsCredentialStore::new();
        let key = CredentialKey {
            provider: "github".into(),
            connection_id: "checkpoint-b-cross-process-proof".into(),
            kind: CredentialKind::AccessToken,
        };
        store.delete(&key).unwrap();
        assert!(store.get(&key).unwrap().is_none());
    }

    #[test]
    fn distinct_credential_kinds_for_the_same_connection_do_not_collide() {
        let store = OsCredentialStore::new();
        let mut access_key = unique_key("kinds");
        access_key.kind = CredentialKind::AccessToken;
        let mut refresh_key = access_key.clone();
        refresh_key.kind = CredentialKind::RefreshToken;
        let _guard_a = CleanupGuard(access_key.clone());
        let _guard_r = CleanupGuard(refresh_key.clone());

        store.put(&access_key, Secret::new("access-value")).unwrap();
        store
            .put(&refresh_key, Secret::new("refresh-value"))
            .unwrap();

        assert_eq!(
            store.get(&access_key).unwrap().unwrap().expose(),
            "access-value"
        );
        assert_eq!(
            store.get(&refresh_key).unwrap().unwrap().expose(),
            "refresh-value"
        );
    }
}
