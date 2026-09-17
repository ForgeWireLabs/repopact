//! The real Android bridge. Registers `RepopactCredentialPlugin` (this
//! crate's `android/` Gradle module) via `PluginHandle::run_mobile_plugin`,
//! exactly the pattern `repopact-mobile-saf`'s own `mobile.rs` uses
//! (verified against the same vendored Tauri 2.11.5 `PluginApi`/
//! `PluginHandle` source, WI067 Checkpoint E Phase 1).
//!
//! Every call here is synchronous/blocking (`run_mobile_plugin`, not the
//! `_async` variant), matching `repopact-mobile-saf`'s own convention --
//! `AndroidCredentialStore::put`/`get`/`delete` are plain blocking
//! `CredentialStore` trait methods, never expected to run on a UI thread.
//!
//! The secret value crosses this boundary as a plain JSON string field.
//! This is the one place in the whole application where a plaintext
//! credential value legitimately exists in process memory outside the
//! `Secret` wrapper -- the native Rust<->Kotlin bridge is inside the
//! trusted application boundary (WI067 Checkpoint E brief, "Secret
//! lifecycle"). It never crosses into the web frontend: this crate
//! registers zero Tauri `#[command]`s (see `lib.rs`'s module doc comment).

use serde::{de::DeserializeOwned, Serialize};
use tauri::{
    plugin::{mobile::PluginInvokeError, PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use repopact_remote_provider::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};

use crate::models::{DeleteResponse, GetResponse, KeyInfoResponse, PutResponse};

const PLUGIN_IDENTIFIER: &str = "com.forgewirelabs.repopact.mobilecredential";

pub fn register<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<PluginHandle<R>, PluginInvokeError> {
    let _ = app;
    api.register_android_plugin(PLUGIN_IDENTIFIER, "RepopactCredentialPlugin")
}

fn invoke<T: DeserializeOwned, R: Runtime>(
    handle: &PluginHandle<R>,
    command: &str,
    payload: impl Serialize,
) -> RemoteProviderResult<T> {
    handle.run_mobile_plugin(command, payload).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::CredentialUnavailable,
            format!("credential plugin invocation '{command}' failed: {error}"),
        )
    })
}

/// Maps a Kotlin-reported failure `reason` code to the provider-neutral
/// error taxonomy. Every reason -- a missing/invalidated Keystore key, a
/// corrupt or wrong-version envelope, an authenticated-encryption tag
/// failure, an I/O failure, or an unclassified plugin failure -- maps to
/// the same `CredentialUnavailable` code `OsCredentialStore` already uses
/// for its own failure modes (WI067 Checkpoint E brief: "no fallback to
/// plaintext or memory-only credentials; user must reconnect"). Never
/// silently treated as "no credential" (`Ok(None)`), which would let a
/// caller mistake corruption for a first-time connection.
fn map_reason(reason: &str) -> RemoteProviderError {
    RemoteProviderError::new(
        ErrorCode::CredentialUnavailable,
        format!("Android credential plugin reported '{reason}'"),
    )
}

#[derive(Serialize)]
struct PutRequest<'a> {
    #[serde(rename = "storageKey")]
    storage_key: &'a str,
    secret: &'a str,
}

#[derive(Serialize)]
struct GetRequest<'a> {
    #[serde(rename = "storageKey")]
    storage_key: &'a str,
}

#[derive(Serialize)]
struct DeleteRequest<'a> {
    #[serde(rename = "storageKey")]
    storage_key: &'a str,
}

pub fn put<R: Runtime>(
    handle: &PluginHandle<R>,
    storage_key: &str,
    secret: &str,
) -> RemoteProviderResult<()> {
    let response: PutResponse = invoke(
        handle,
        "putCredential",
        PutRequest {
            storage_key,
            secret,
        },
    )?;
    match response {
        PutResponse::Ok => Ok(()),
        PutResponse::Error { reason } => Err(map_reason(&reason)),
    }
}

pub fn get<R: Runtime>(
    handle: &PluginHandle<R>,
    storage_key: &str,
) -> RemoteProviderResult<Option<String>> {
    let response: GetResponse = invoke(handle, "getCredential", GetRequest { storage_key })?;
    match response {
        GetResponse::Found { secret } => Ok(Some(secret)),
        GetResponse::NotFound => Ok(None),
        GetResponse::Error { reason } => Err(map_reason(&reason)),
    }
}

pub fn delete<R: Runtime>(handle: &PluginHandle<R>, storage_key: &str) -> RemoteProviderResult<()> {
    let response: DeleteResponse =
        invoke(handle, "deleteCredential", DeleteRequest { storage_key })?;
    match response {
        DeleteResponse::Ok => Ok(()),
        DeleteResponse::Error { reason } => Err(map_reason(&reason)),
    }
}

/// WI067 Checkpoint E runtime-proof surface only: reports whether the
/// dedicated Keystore alias currently exists and, only when it does, its
/// actually-measured secure-hardware backing. Never used by production
/// put/get/delete.
pub fn key_info<R: Runtime>(
    handle: &PluginHandle<R>,
) -> RemoteProviderResult<(bool, Option<bool>)> {
    let response: KeyInfoResponse = invoke(handle, "keyInfo", ())?;
    Ok((response.exists, response.inside_secure_hardware))
}
