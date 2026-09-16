//! Provider-neutral typed failure taxonomy (WI067 Decision 0061, GH-010,
//! GH-026). Frontend and evidence consumers must be able to branch on a
//! stable `ErrorCode`, never on provider prose; `Display` never includes
//! credential material (see `crate::redact`).

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NotConnected,
    /// WI067 Checkpoint B, item 16: the provider requires a client ID
    /// (public, non-secret configuration) that has not been supplied --
    /// distinct from `NotConnected` (a user simply hasn't authorized yet).
    /// Never fabricated; this is the exact operator-registration gate.
    ProviderNotConfigured,
    AuthorizationPending,
    AuthorizationCancelled,
    AuthorizationExpired,
    AuthorizationDenied,
    CredentialUnavailable,
    CredentialExpired,
    RefreshFailed,
    ProviderRateLimited,
    ProviderForbidden,
    ProviderNotFound,
    InstallationRevoked,
    OrganizationAuthorizationRequired,
    NetworkUnavailable,
    TlsFailure,
    ProviderProtocolError,
    SnapshotTooLarge,
    DownloadCancelled,
    MaterializationFailed,
}

/// A provider-neutral error. `detail` is a short, non-secret, redacted
/// description suitable for logs/evidence/UI; it must never contain a
/// token, refresh token, device code, authorization code, or
/// credential-bearing URL (enforced by `crate::redact::assert_redacted`
/// in tests, not by this type alone).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteProviderError {
    pub code: ErrorCode,
    pub detail: String,
}

impl RemoteProviderError {
    pub fn new(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: crate::redact::redact(&detail.into()),
        }
    }
}

impl fmt::Display for RemoteProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.detail)
    }
}

impl std::error::Error for RemoteProviderError {}

pub type RemoteProviderResult<T> = Result<T, RemoteProviderError>;

#[cfg(test)]
mod tests {
    use super::*;

    const CANARY_ACCESS_TOKEN: &str = "ghu_REPOPACT_CANARY_dO_NOT_USE_0000000000";
    const CANARY_REFRESH_TOKEN: &str = "ghr_REPOPACT_CANARY_dO_NOT_USE_1111111111";

    /// WI067 Checkpoint D, Phase 6: `RemoteProviderError::new` redacts its
    /// `detail` argument on construction, so a canary embedded by any
    /// caller (a lower-layer error message that happened to quote a
    /// token-shaped string) never survives into `.detail`, `Display`, or
    /// `Debug`.
    #[test]
    fn a_canary_access_token_embedded_in_error_detail_is_redacted() {
        let error = RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("upstream said token {CANARY_ACCESS_TOKEN} was rejected"),
        );
        assert!(!error.detail.contains(CANARY_ACCESS_TOKEN));
        assert!(!format!("{error}").contains(CANARY_ACCESS_TOKEN));
        assert!(!format!("{error:?}").contains(CANARY_ACCESS_TOKEN));
    }

    #[test]
    fn a_canary_refresh_token_embedded_in_error_detail_is_redacted() {
        let error = RemoteProviderError::new(
            ErrorCode::RefreshFailed,
            format!("refresh_token={CANARY_REFRESH_TOKEN} was denied"),
        );
        assert!(!error.detail.contains(CANARY_REFRESH_TOKEN));
    }

    #[test]
    fn a_credential_bearing_url_embedded_in_error_detail_is_redacted() {
        let error = RemoteProviderError::new(
            ErrorCode::NetworkUnavailable,
            format!(
                "failed to fetch https://x-access-token:{CANARY_ACCESS_TOKEN}@github.com/o/r.git"
            ),
        );
        assert!(!error.detail.contains(CANARY_ACCESS_TOKEN));
    }

    #[test]
    fn a_canary_survives_neither_serialized_json_nor_debug_output() {
        let error = RemoteProviderError::new(
            ErrorCode::CredentialExpired,
            format!("token {CANARY_ACCESS_TOKEN} expired"),
        );
        let json = serde_json::to_string(&error).unwrap();
        assert!(!json.contains(CANARY_ACCESS_TOKEN));
    }
}
