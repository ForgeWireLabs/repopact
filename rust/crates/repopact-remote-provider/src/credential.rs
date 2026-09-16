//! Native-owned credential storage abstraction (WI067 item 14, item 15).
//! Implementations must map to a real OS-protected facility (Windows
//! Credential Manager, macOS/iOS Keychain, Android Keystore-backed storage,
//! Linux Secret Service/libsecret); none may use `registry.json`, plain
//! JSON, frontend storage (localStorage/sessionStorage/IndexedDB),
//! repository files, or environment variables as durable token storage.
//!
//! Checkpoint A ships this trait plus an in-memory test double
//! ([`InMemoryCredentialStore`]); real platform backends are a later
//! checkpoint's job (Decision 0061 records the per-platform target, not
//! the implementation).

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
use crate::redact::Secret;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialKind {
    AccessToken,
    RefreshToken,
}

/// Typed key into the credential store -- never a bare string a caller
/// assembles ad hoc.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CredentialKey {
    pub provider: String,
    pub connection_id: String,
    pub kind: CredentialKind,
}

pub trait CredentialStore: Send + Sync {
    fn put(&self, key: &CredentialKey, value: Secret) -> RemoteProviderResult<()>;
    fn get(&self, key: &CredentialKey) -> RemoteProviderResult<Option<Secret>>;
    fn delete(&self, key: &CredentialKey) -> RemoteProviderResult<()>;
}

/// Process-memory-only test double. Never durable, never written to disk;
/// exists solely so the auth/provider state machine can be exercised in
/// tests without a real OS credential facility.
#[derive(Default)]
pub struct InMemoryCredentialStore {
    values: Mutex<HashMap<CredentialKey, String>>,
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn put(&self, key: &CredentialKey, value: Secret) -> RemoteProviderResult<()> {
        let mut values = self.values.lock().map_err(|_| lock_poisoned())?;
        values.insert(key.clone(), value.expose().to_string());
        Ok(())
    }

    fn get(&self, key: &CredentialKey) -> RemoteProviderResult<Option<Secret>> {
        let values = self.values.lock().map_err(|_| lock_poisoned())?;
        Ok(values.get(key).cloned().map(Secret::new))
    }

    fn delete(&self, key: &CredentialKey) -> RemoteProviderResult<()> {
        let mut values = self.values.lock().map_err(|_| lock_poisoned())?;
        values.remove(key);
        Ok(())
    }
}

fn lock_poisoned() -> RemoteProviderError {
    RemoteProviderError::new(
        ErrorCode::ProviderProtocolError,
        "credential store lock poisoned",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> CredentialKey {
        CredentialKey {
            provider: "fake".into(),
            connection_id: "conn-1".into(),
            kind: CredentialKind::AccessToken,
        }
    }

    #[test]
    fn put_then_get_round_trips_the_value() {
        let store = InMemoryCredentialStore::new();
        store.put(&key(), Secret::new("value")).unwrap();
        let got = store.get(&key()).unwrap().unwrap();
        assert_eq!(got.expose(), "value");
    }

    #[test]
    fn delete_removes_the_value() {
        let store = InMemoryCredentialStore::new();
        store.put(&key(), Secret::new("value")).unwrap();
        store.delete(&key()).unwrap();
        assert!(store.get(&key()).unwrap().is_none());
    }

    #[test]
    fn missing_key_returns_none_not_error() {
        let store = InMemoryCredentialStore::new();
        assert!(store.get(&key()).unwrap().is_none());
    }
}
