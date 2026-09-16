//! WI067 Decision 0061 / item 17 / item 18: the GitHub App device
//! authorization flow, modeled exactly against GitHub's own protocol
//! (verified during Checkpoint A research against GitHub's REST API
//! device-flow documentation, 2026-09-15). No `client_secret` field is
//! ever sent by either request -- GitHub's own documentation states this
//! is the one flow that does not require one, which is why Decision 0061
//! selects it for a distributed native client that cannot keep a secret
//! confidential.

use repopact_remote_provider::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
use repopact_remote_provider::redact::Secret;
use serde::Deserialize;

use crate::transport::{FormRequest, HttpTransport};

pub const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
pub const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
pub const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
pub const REFRESH_GRANT_TYPE: &str = "refresh_token";

/// Internal-only: never derives `Serialize`, never crosses the Tauri
/// command boundary. The frontend receives only
/// `repopact_remote_provider::auth::AuthState::AwaitingUser`'s
/// `user_code`/`verification_uri`/`expires_at` (item 17) -- never
/// `device_code`.
#[derive(Debug, Clone)]
pub struct DeviceAuthorization {
    pub device_code: Secret,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in_secs: u64,
    pub interval_secs: u64,
}

pub enum DevicePollOutcome {
    Pending,
    SlowDown {
        new_interval_secs: u64,
    },
    Authorized {
        access_token: Secret,
        refresh_token: Option<Secret>,
        expires_in_secs: Option<u64>,
        refresh_token_expires_in_secs: Option<u64>,
    },
    Expired,
    Denied,
    DeviceFlowDisabled,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

pub fn start_device_flow(
    transport: &dyn HttpTransport,
    client_id: &str,
) -> RemoteProviderResult<DeviceAuthorization> {
    let request = FormRequest {
        url: DEVICE_CODE_URL.to_string(),
        fields: vec![("client_id".to_string(), client_id.to_string())],
        headers: vec![("Accept".to_string(), "application/json".to_string())],
    };
    let response = transport
        .post_form(&request)
        .map_err(|error| RemoteProviderError::new(ErrorCode::NetworkUnavailable, error.message))?;
    if response.status != 200 {
        return Err(RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("device code request failed with status {}", response.status),
        ));
    }
    let parsed: DeviceCodeResponse = serde_json::from_str(&response.body).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed device code response: {error}"),
        )
    })?;
    if !crate::redirect_policy::is_trusted_verification_uri(&parsed.verification_uri) {
        return Err(RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            "device code response returned an untrusted verification_uri",
        ));
    }
    Ok(DeviceAuthorization {
        device_code: Secret::new(parsed.device_code),
        user_code: parsed.user_code,
        verification_uri: parsed.verification_uri,
        expires_in_secs: parsed.expires_in,
        interval_secs: parsed.interval,
    })
}

pub fn poll_device_flow(
    transport: &dyn HttpTransport,
    client_id: &str,
    device_code: &Secret,
) -> RemoteProviderResult<DevicePollOutcome> {
    let request = FormRequest {
        url: ACCESS_TOKEN_URL.to_string(),
        fields: vec![
            ("client_id".to_string(), client_id.to_string()),
            ("device_code".to_string(), device_code.expose().to_string()),
            ("grant_type".to_string(), DEVICE_GRANT_TYPE.to_string()),
        ],
        headers: vec![("Accept".to_string(), "application/json".to_string())],
    };
    let response = transport
        .post_form(&request)
        .map_err(|error| RemoteProviderError::new(ErrorCode::NetworkUnavailable, error.message))?;
    let parsed: serde_json::Value = serde_json::from_str(&response.body).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed token response: {error}"),
        )
    })?;

    if let Some(error) = parsed.get("error").and_then(|v| v.as_str()) {
        return Ok(match error {
            "authorization_pending" => DevicePollOutcome::Pending,
            "slow_down" => {
                let new_interval = parsed
                    .get("interval")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10);
                DevicePollOutcome::SlowDown {
                    new_interval_secs: new_interval,
                }
            }
            "expired_token" | "token_expired" => DevicePollOutcome::Expired,
            "access_denied" => DevicePollOutcome::Denied,
            "device_flow_disabled" => DevicePollOutcome::DeviceFlowDisabled,
            other => {
                return Err(RemoteProviderError::new(
                    ErrorCode::ProviderProtocolError,
                    format!("unrecognized device-flow error code: {other}"),
                ))
            }
        });
    }

    let access_token = parsed
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            RemoteProviderError::new(
                ErrorCode::ProviderProtocolError,
                "token response missing access_token",
            )
        })?;
    Ok(DevicePollOutcome::Authorized {
        access_token: Secret::new(access_token),
        refresh_token: parsed
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(Secret::new),
        expires_in_secs: parsed.get("expires_in").and_then(|v| v.as_u64()),
        refresh_token_expires_in_secs: parsed
            .get("refresh_token_expires_in")
            .and_then(|v| v.as_u64()),
    })
}

#[derive(Debug, Clone)]
pub struct RefreshOutcome {
    pub access_token: Secret,
    pub refresh_token: Option<Secret>,
    pub expires_in_secs: Option<u64>,
}

/// Refreshes an expiring GitHub App user access token (item 11). Verified
/// against GitHub's own token-refresh documentation during Checkpoint B:
/// `client_secret` is documented as "Required unless the user access
/// token was generated using the device flow" -- exactly this crate's
/// case -- so this request, like the device-flow requests above, never
/// includes one.
pub fn refresh_access_token(
    transport: &dyn HttpTransport,
    client_id: &str,
    refresh_token: &Secret,
) -> RemoteProviderResult<RefreshOutcome> {
    let request = FormRequest {
        url: ACCESS_TOKEN_URL.to_string(),
        fields: vec![
            ("client_id".to_string(), client_id.to_string()),
            ("grant_type".to_string(), REFRESH_GRANT_TYPE.to_string()),
            (
                "refresh_token".to_string(),
                refresh_token.expose().to_string(),
            ),
        ],
        headers: vec![("Accept".to_string(), "application/json".to_string())],
    };
    let response = transport
        .post_form(&request)
        .map_err(|error| RemoteProviderError::new(ErrorCode::NetworkUnavailable, error.message))?;
    let parsed: serde_json::Value = serde_json::from_str(&response.body).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed refresh response: {error}"),
        )
    })?;

    if let Some(error) = parsed.get("error").and_then(|v| v.as_str()) {
        let code = match error {
            "access_denied" | "bad_refresh_token" => ErrorCode::AuthorizationDenied,
            "expired_token" | "token_expired" => ErrorCode::AuthorizationExpired,
            _ => ErrorCode::RefreshFailed,
        };
        return Err(RemoteProviderError::new(
            code,
            format!("GitHub refresh failed: {error}"),
        ));
    }

    let access_token = parsed
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            RemoteProviderError::new(
                ErrorCode::ProviderProtocolError,
                "refresh response missing access_token",
            )
        })?;
    Ok(RefreshOutcome {
        access_token: Secret::new(access_token),
        refresh_token: parsed
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(Secret::new),
        expires_in_secs: parsed.get("expires_in").and_then(|v| v.as_u64()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{json_response, ScriptedTransport};

    #[test]
    fn start_device_flow_never_sends_a_client_secret() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(
            200,
            &[
                ("device_code", "abc123"),
                ("user_code", "WDJB-MJHT"),
                ("verification_uri", "https://github.com/login/device"),
                ("expires_in", "900"),
                ("interval", "5"),
            ],
        )));
        let auth = start_device_flow(&transport, "test-client-id").unwrap();
        assert_eq!(auth.user_code, "WDJB-MJHT");
        assert_eq!(auth.interval_secs, 5);

        let requests = transport.received_requests();
        assert_eq!(requests.len(), 1);
        assert!(requests[0]
            .fields
            .iter()
            .all(|(key, _)| key != "client_secret"));
    }

    #[test]
    fn start_device_flow_rejects_an_untrusted_verification_uri() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(
            200,
            &[
                ("device_code", "abc123"),
                ("user_code", "WDJB-MJHT"),
                (
                    "verification_uri",
                    "https://github.com.evil.example/login/device",
                ),
                ("expires_in", "900"),
                ("interval", "5"),
            ],
        )));
        let result = start_device_flow(&transport, "test-client-id");
        assert!(result.is_err());
    }

    #[test]
    fn poll_never_sends_a_client_secret_and_reports_pending() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(
            200,
            &[("error", "authorization_pending")],
        )));
        let outcome =
            poll_device_flow(&transport, "test-client-id", &Secret::new("device-code")).unwrap();
        assert!(matches!(outcome, DevicePollOutcome::Pending));
        let requests = transport.received_requests();
        assert!(requests[0]
            .fields
            .iter()
            .all(|(key, _)| key != "client_secret"));
    }

    #[test]
    fn poll_honors_slow_down_with_the_servers_new_interval() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(
            200,
            &[("error", "slow_down"), ("interval", "10")],
        )));
        let outcome = poll_device_flow(&transport, "id", &Secret::new("dc")).unwrap();
        match outcome {
            DevicePollOutcome::SlowDown { new_interval_secs } => assert_eq!(new_interval_secs, 10),
            other => panic!("expected SlowDown, got {other:?}"),
        }
    }

    #[test]
    fn poll_reports_expired_denied_and_disabled_distinctly() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(200, &[("error", "expired_token")])));
        assert!(matches!(
            poll_device_flow(&transport, "id", &Secret::new("dc")).unwrap(),
            DevicePollOutcome::Expired
        ));

        transport.push_response(Ok(json_response(200, &[("error", "access_denied")])));
        assert!(matches!(
            poll_device_flow(&transport, "id", &Secret::new("dc")).unwrap(),
            DevicePollOutcome::Denied
        ));

        transport.push_response(Ok(json_response(200, &[("error", "device_flow_disabled")])));
        assert!(matches!(
            poll_device_flow(&transport, "id", &Secret::new("dc")).unwrap(),
            DevicePollOutcome::DeviceFlowDisabled
        ));
    }

    #[test]
    fn poll_success_extracts_tokens_and_expirations() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(
            200,
            &[
                ("access_token", "ghu_test0000000000000000000000000000"),
                ("refresh_token", "ghr_test0000000000000000000000000000"),
                ("expires_in", "28800"),
                ("refresh_token_expires_in", "15897600"),
            ],
        )));
        let outcome = poll_device_flow(&transport, "id", &Secret::new("dc")).unwrap();
        match outcome {
            DevicePollOutcome::Authorized {
                access_token,
                refresh_token,
                expires_in_secs,
                refresh_token_expires_in_secs,
            } => {
                assert_eq!(
                    access_token.expose(),
                    "ghu_test0000000000000000000000000000"
                );
                assert_eq!(
                    refresh_token.unwrap().expose(),
                    "ghr_test0000000000000000000000000000"
                );
                assert_eq!(expires_in_secs, Some(28800));
                assert_eq!(refresh_token_expires_in_secs, Some(15897600));
            }
            other => panic!("expected Authorized, got {other:?}"),
        }
    }

    #[test]
    fn refresh_never_sends_a_client_secret_and_returns_a_new_pair() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(
            200,
            &[
                ("access_token", "new-access"),
                ("refresh_token", "new-refresh"),
                ("expires_in", "28800"),
            ],
        )));
        let outcome =
            refresh_access_token(&transport, "client-id", &Secret::new("old-refresh")).unwrap();
        assert_eq!(outcome.access_token.expose(), "new-access");
        assert_eq!(outcome.refresh_token.unwrap().expose(), "new-refresh");
        assert_eq!(outcome.expires_in_secs, Some(28800));

        let requests = transport.received_requests();
        assert!(requests[0]
            .fields
            .iter()
            .all(|(key, _)| key != "client_secret"));
        assert!(requests[0]
            .fields
            .iter()
            .any(|(k, v)| k == "grant_type" && v == "refresh_token"));
    }

    #[test]
    fn refresh_denied_maps_to_authorization_denied() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(200, &[("error", "access_denied")])));
        let error =
            refresh_access_token(&transport, "client-id", &Secret::new("revoked")).unwrap_err();
        assert_eq!(error.code, ErrorCode::AuthorizationDenied);
    }

    #[test]
    fn refresh_expired_maps_to_authorization_expired() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(200, &[("error", "expired_token")])));
        let error =
            refresh_access_token(&transport, "client-id", &Secret::new("stale")).unwrap_err();
        assert_eq!(error.code, ErrorCode::AuthorizationExpired);
    }
}

impl std::fmt::Debug for DevicePollOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DevicePollOutcome::Pending => write!(f, "Pending"),
            DevicePollOutcome::SlowDown { new_interval_secs } => {
                write!(f, "SlowDown {{ new_interval_secs: {new_interval_secs} }}")
            }
            DevicePollOutcome::Authorized { .. } => write!(f, "Authorized {{ REDACTED }}"),
            DevicePollOutcome::Expired => write!(f, "Expired"),
            DevicePollOutcome::Denied => write!(f, "Denied"),
            DevicePollOutcome::DeviceFlowDisabled => write!(f, "DeviceFlowDisabled"),
        }
    }
}
