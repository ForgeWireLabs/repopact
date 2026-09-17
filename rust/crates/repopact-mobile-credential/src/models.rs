//! Typed JSON shapes crossing the Rust<->Kotlin `run_mobile_plugin`
//! boundary. Kotlin never sends free-text prose the Rust side would need
//! to parse -- only these fixed `status`/`reason` shapes, mirroring
//! `repopact-mobile-saf`'s own `models.rs` convention exactly.
//!
//! `reason` is always one of a small fixed vocabulary
//! (`key_unavailable`, `corrupt_envelope`, `auth_failed`, `io_error`,
//! `provider_failure`) -- never a raw exception message, and never
//! anything that could embed credential material.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PutResponse {
    Ok,
    Error { reason: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GetResponse {
    Found { secret: String },
    NotFound,
    Error { reason: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DeleteResponse {
    Ok,
    Error { reason: String },
}

/// WI067 Checkpoint E: honest, measured Keystore introspection for the
/// runtime-proof debug surface only -- never used by production
/// put/get/delete. `inside_secure_hardware` is `None` when the platform
/// could not report it (never defaulted to `true`).
#[derive(Debug, Deserialize)]
pub struct KeyInfoResponse {
    pub exists: bool,
    #[serde(default, rename = "insideSecureHardware")]
    pub inside_secure_hardware: Option<bool>,
}
