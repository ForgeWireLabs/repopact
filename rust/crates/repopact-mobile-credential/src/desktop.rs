//! Android Keystore has no desktop equivalent. This module exists only so
//! `repopact-mobile-credential` type-checks under `cargo check --workspace`
//! on a non-Android host; nothing here is ever actually called, because
//! the app only constructs `AndroidCredentialStore` under
//! `#[cfg(target_os = "android")]` (see `repopact-desktop`'s credential
//! store composition).

use repopact_remote_provider::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};

pub fn put(_storage_key: &str, _secret: &str) -> RemoteProviderResult<()> {
    Err(unavailable())
}

pub fn get(_storage_key: &str) -> RemoteProviderResult<Option<String>> {
    Err(unavailable())
}

pub fn delete(_storage_key: &str) -> RemoteProviderResult<()> {
    Err(unavailable())
}

pub fn key_info() -> RemoteProviderResult<(bool, Option<bool>)> {
    Err(unavailable())
}

fn unavailable() -> RemoteProviderError {
    RemoteProviderError::new(
        ErrorCode::CredentialUnavailable,
        "Android Keystore-backed credential storage is an Android-only capability",
    )
}
