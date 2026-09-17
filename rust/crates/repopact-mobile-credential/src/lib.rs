//! WI067 Checkpoint E (GH-004): Android Keystore-backed protected
//! credential storage. Mirrors `repopact-mobile-saf`'s crate shape
//! exactly -- a plain Rust facade wrapping a Tauri Android
//! `PluginHandle`, with **zero frontend-invokable commands of its own**
//! (`.invoke_handler()` is never called on this plugin's `Builder`). It
//! exists purely so [`AndroidCredentialStore`] (a production
//! [`CredentialStore`] implementation) can call plain Rust methods that
//! internally invoke the Android Kotlin plugin via
//! `PluginHandle::run_mobile_plugin`. There is no frontend command capable
//! of retrieving a token, by construction: nothing in the frontend's
//! typed command surface ever reaches this crate at all.
//!
//! # Architecture (Decision 0061 extension, Checkpoint E brief)
//!
//! Android Keystore owns cryptographic key material, not arbitrary
//! application secret blobs -- so this crate never asks Keystore to store
//! an OAuth token string directly. Instead:
//!
//! ```text
//! AndroidCredentialStore (this crate, implements CredentialStore)
//!   -> RepopactCredentialPlugin.kt (Android Gradle module)
//!     -> a non-exportable AES-256-GCM key generated inside AndroidKeyStore
//!        (alias "com.forgewirelabs.repopact.remote-provider.v1")
//!     -> encrypts/decrypts a versioned envelope (version + IV + ciphertext+tag)
//!     -> the envelope (ciphertext only, never the key) is the only thing
//!        written to app-private storage (private SharedPreferences)
//! ```
//!
//! `GitHubProvider` and the rest of `repopact-remote-provider` remain
//! completely unaware this crate exists -- they depend only on the
//! existing `CredentialStore` trait object, exactly as they already do for
//! `OsCredentialStore`.

use sha2::{Digest, Sha256};
use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, Runtime,
};

use repopact_remote_provider::credential::{CredentialKey, CredentialKind, CredentialStore};
use repopact_remote_provider::error::RemoteProviderResult;
use repopact_remote_provider::redact::Secret;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;
#[cfg(mobile)]
mod models;

pub trait MobileCredentialExt<R: Runtime> {
    fn mobile_credential(&self) -> &MobileCredential<R>;
}

impl<R: Runtime, T: Manager<R>> MobileCredentialExt<R> for T {
    fn mobile_credential(&self) -> &MobileCredential<R> {
        self.state::<MobileCredential<R>>().inner()
    }
}

#[cfg(mobile)]
pub struct MobileCredential<R: Runtime>(tauri::plugin::PluginHandle<R>);
#[cfg(desktop)]
pub struct MobileCredential<R: Runtime>(std::marker::PhantomData<fn() -> R>);

impl<R: Runtime> MobileCredential<R> {
    fn put(&self, storage_key: &str, secret: &str) -> RemoteProviderResult<()> {
        #[cfg(mobile)]
        return mobile::put(&self.0, storage_key, secret);
        #[cfg(desktop)]
        {
            let _ = (storage_key, secret);
            desktop::put(storage_key, secret)
        }
    }

    fn get(&self, storage_key: &str) -> RemoteProviderResult<Option<String>> {
        #[cfg(mobile)]
        return mobile::get(&self.0, storage_key);
        #[cfg(desktop)]
        desktop::get(storage_key)
    }

    fn delete(&self, storage_key: &str) -> RemoteProviderResult<()> {
        #[cfg(mobile)]
        return mobile::delete(&self.0, storage_key);
        #[cfg(desktop)]
        desktop::delete(storage_key)
    }

    /// WI067 Checkpoint E runtime-proof surface only (see
    /// `AndroidCredentialStore::key_info`).
    fn key_info(&self) -> RemoteProviderResult<(bool, Option<bool>)> {
        #[cfg(mobile)]
        return mobile::key_info(&self.0);
        #[cfg(desktop)]
        desktop::key_info()
    }
}

pub fn init_plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("repopact-mobile-credential")
        .setup(|app, _api| {
            #[cfg(mobile)]
            let handle = MobileCredential(mobile::register(app, _api)?);
            #[cfg(desktop)]
            let handle = MobileCredential::<R>(std::marker::PhantomData);
            app.manage(handle);
            Ok(())
        })
        .build()
}

/// The identity component of a [`CredentialKey`] is not itself secret
/// (`provider`/`connection_id`/`kind` never carry token material), but the
/// Checkpoint E brief is explicit: "Do not put secret plaintext in
/// filenames or preference keys... Do not use access-token text as an
/// identifier." Hashing the composite identity keeps the Android-side
/// SharedPreferences key opaque and stable regardless of what a future
/// `connection_id` scheme looks like, without ever hashing anything
/// secret.
fn storage_key(key: &CredentialKey) -> String {
    let kind = match key.kind {
        CredentialKind::AccessToken => "access-token",
        CredentialKind::RefreshToken => "refresh-token",
    };
    let identity = format!("{}:{}:{}", key.provider, key.connection_id, kind);
    let mut hasher = Sha256::new();
    hasher.update(identity.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Production, Android Keystore-backed [`CredentialStore`]. Holds the
/// app's `AppHandle<R>` and looks up `MobileCredential<R>` from Tauri's
/// managed state on every call (the same state `init_plugin`'s `setup`
/// hook installs) rather than owning a `PluginHandle` directly -- this is
/// what lets `AndroidCredentialStore::new` be a cheap, `Clone`-free,
/// `'static` value that can be stored behind the same `Arc<dyn
/// CredentialStore>` seam `OsCredentialStore` already uses on desktop.
pub struct AndroidCredentialStore<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> AndroidCredentialStore<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> CredentialStore for AndroidCredentialStore<R> {
    fn put(&self, key: &CredentialKey, value: Secret) -> RemoteProviderResult<()> {
        self.app
            .mobile_credential()
            .put(&storage_key(key), value.expose())
    }

    fn get(&self, key: &CredentialKey) -> RemoteProviderResult<Option<Secret>> {
        Ok(self
            .app
            .mobile_credential()
            .get(&storage_key(key))?
            .map(Secret::new))
    }

    fn delete(&self, key: &CredentialKey) -> RemoteProviderResult<()> {
        self.app.mobile_credential().delete(&storage_key(key))
    }
}

impl<R: Runtime> AndroidCredentialStore<R> {
    /// WI067 Checkpoint E runtime-proof surface only: reports whether the
    /// dedicated Keystore alias currently exists and its measured
    /// secure-hardware backing, if any. Exposes nothing about the key's
    /// raw material -- there is no such operation to expose in the first
    /// place. Not part of the `CredentialStore` trait; used only by the
    /// app's debug-only `android_validation` test/control surface.
    pub fn key_info(&self) -> RemoteProviderResult<(bool, Option<bool>)> {
        self.app.mobile_credential().key_info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WI067 Checkpoint E: the storage-key hash must never simply be the
    /// plaintext identity string (defense in depth even though the
    /// identity itself is not secret -- see `storage_key`'s doc comment),
    /// and distinct keys must hash distinctly so access/refresh tokens and
    /// separate connections never collide on the Android-side storage
    /// key.
    #[test]
    fn storage_key_is_not_the_plaintext_identity() {
        let key = CredentialKey {
            provider: "github".into(),
            connection_id: "conn-1".into(),
            kind: CredentialKind::AccessToken,
        };
        let hashed = storage_key(&key);
        assert_ne!(hashed, "github:conn-1:access-token");
        assert_eq!(hashed.len(), 64);
    }

    #[test]
    fn distinct_kinds_for_the_same_connection_hash_distinctly() {
        let access = CredentialKey {
            provider: "github".into(),
            connection_id: "conn-1".into(),
            kind: CredentialKind::AccessToken,
        };
        let mut refresh = access.clone();
        refresh.kind = CredentialKind::RefreshToken;
        assert_ne!(storage_key(&access), storage_key(&refresh));
    }

    #[test]
    fn distinct_connection_ids_hash_distinctly() {
        let mut a = CredentialKey {
            provider: "github".into(),
            connection_id: "conn-1".into(),
            kind: CredentialKind::AccessToken,
        };
        let mut b = a.clone();
        b.connection_id = "conn-2".into();
        assert_ne!(storage_key(&a), storage_key(&b));
        a.connection_id = "conn-1".into();
        assert_eq!(storage_key(&a), storage_key(&a.clone()));
    }
}
