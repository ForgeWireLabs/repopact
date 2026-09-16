//! Explicit authorization state machine (WI067 item 16). Never a bare
//! `Option<String>` token -- the frontend receives only this shape, and it
//! contains no credential field by construction.

use serde::{Deserialize, Serialize};

use crate::error::ErrorCode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AuthState {
    Disconnected,
    RequestingAuthorization,
    /// Device-flow (or equivalent) pending-user-action state. `user_code`
    /// and `verification_uri` are safe to show in UI -- neither is a
    /// credential -- but `verification_uri` must be validated against a
    /// compile-time/configured trusted origin before display (item 19),
    /// not trusted verbatim from provider response data.
    AwaitingUser {
        user_code: String,
        verification_uri: String,
        expires_at: String,
    },
    Authorized {
        account_label: String,
    },
    Refreshing,
    Expired,
    Revoked,
    Cancelled,
    Failed {
        code: ErrorCode,
    },
}
