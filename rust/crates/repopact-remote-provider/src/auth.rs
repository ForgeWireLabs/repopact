//! Explicit authorization state machine (Decision 0062). Never a bare
//! `Option<String>` token -- the frontend receives only this shape, and it
//! contains no credential, PKCE verifier, `state`, or authorization code
//! field by construction.
//!
//! This replaces the device-flow-shaped `AuthState` from Decision 0061
//! (`RequestingAuthorization`/`AwaitingUser{user_code,...}`/`Revoked`) with
//! the browser-redirect-PKCE model the Workbench now uses for every
//! platform. Device flow's protocol implementation
//! (`repopact-provider-github::device_flow`) still exists and is still
//! tested, but no production code path constructs its states anymore --
//! see Decision 0062's "Device flow disposition".

use serde::{Deserialize, Serialize};

use crate::error::ErrorCode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AuthState {
    Disconnected,
    /// A new authorization session (state + PKCE verifier/challenge +
    /// loopback/deep-link callback receiver) has just been created and the
    /// system browser is being opened at the trusted GitHub authorization
    /// URL. Transient -- the next poll normally observes `WaitingForCallback`.
    StartingBrowserAuthorization,
    /// The system browser is open and the native callback receiver is
    /// listening; the user has not yet completed (or has not yet been
    /// observed to complete) authorization on GitHub. `expires_at` bounds
    /// how long this session remains valid before it fails closed.
    WaitingForCallback {
        expires_at: String,
    },
    /// The callback was received, its `state` validated, and the native
    /// token exchange (authorization code + PKCE verifier) is in flight.
    ExchangingCode,
    Authorized {
        account_label: String,
    },
    Refreshing,
    Cancelled,
    Expired,
    Failed {
        code: ErrorCode,
    },
}
