//! Decision 0062: the GitHub App browser-redirect authorization-code flow
//! with PKCE (`S256`), which replaces the device flow
//! (`crate::device_flow`) as RepoPact's interactive Workbench
//! authentication path on every platform (desktop and Android).
//!
//! Verified against GitHub's current (2026-09-16) documentation for GitHub
//! App user authorization: the authorize step accepts `code_challenge`/
//! `code_challenge_method=S256` exactly as RFC 7636 describes, but the
//! token-exchange step still documents `client_secret` as a required
//! parameter regardless of PKCE -- GitHub's own device-flow documentation
//! names device flow as the *only* flow that omits it. RepoPact's GitHub
//! App `client_secret` is therefore packaged **public application
//! configuration**, not confidential material a native/public client could
//! ever actually protect (see Decision 0062's threat model); this module
//! never claims otherwise, never routes it through `CredentialStore`, and
//! never logs it.

use repopact_remote_provider::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
use repopact_remote_provider::pkce::CodeVerifier;
use repopact_remote_provider::redact::Secret;
use serde::Deserialize;

use crate::transport::{FormRequest, HttpTransport};

pub const AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
pub const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
pub const AUTHORIZATION_CODE_GRANT_TYPE: &str = "authorization_code";
pub const REFRESH_GRANT_TYPE: &str = "refresh_token";

/// Builds the trusted GitHub authorization URL entirely from typed,
/// natively-generated inputs -- never from anything the frontend supplies
/// (Decision 0062: "Frontend code must not supply authorization host,
/// redirect host, client ID, client secret, state, PKCE verifier"). Only
/// `code_challenge` (never the verifier) appears in the URL.
pub fn build_authorization_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
) -> String {
    let mut url = reqwest::Url::parse(AUTHORIZE_URL).expect("AUTHORIZE_URL is a fixed valid URL");
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("state", state)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256");
    url.into()
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    refresh_token_expires_in: Option<u64>,
    error: Option<String>,
}

#[derive(Debug)]
pub struct TokenExchangeOutcome {
    pub access_token: Secret,
    pub refresh_token: Option<Secret>,
    pub expires_in_secs: Option<u64>,
    pub refresh_token_expires_in_secs: Option<u64>,
}

/// Exchanges an authorization code for a token pair. `client_secret` is
/// GitHub's required (non-confidential, per the module doc above)
/// public-client configuration value -- sent because GitHub's web flow
/// requires it, not because RepoPact treats it as a trust boundary.
/// `code_verifier` proves this exchange came from the same native process
/// that generated the `code_challenge` in the authorization URL. Neither
/// `code` nor `code_verifier` is logged; both are consumed by this one call
/// and dropped by the caller immediately afterward.
pub fn exchange_code_for_token(
    transport: &dyn HttpTransport,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &CodeVerifier,
) -> RemoteProviderResult<TokenExchangeOutcome> {
    let request = FormRequest {
        url: ACCESS_TOKEN_URL.to_string(),
        fields: vec![
            ("client_id".to_string(), client_id.to_string()),
            ("client_secret".to_string(), client_secret.to_string()),
            ("code".to_string(), code.to_string()),
            ("redirect_uri".to_string(), redirect_uri.to_string()),
            (
                "code_verifier".to_string(),
                code_verifier.expose().to_string(),
            ),
            (
                "grant_type".to_string(),
                AUTHORIZATION_CODE_GRANT_TYPE.to_string(),
            ),
        ],
        headers: vec![("Accept".to_string(), "application/json".to_string())],
    };
    let response = transport
        .post_form(&request)
        .map_err(|error| RemoteProviderError::new(ErrorCode::NetworkUnavailable, error.message))?;
    let parsed: TokenResponse = serde_json::from_str(&response.body).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed token response: {error}"),
        )
    })?;

    if let Some(error) = parsed.error {
        let code = match error.as_str() {
            "access_denied" => ErrorCode::AuthorizationDenied,
            "bad_verification_code" | "incorrect_client_credentials" | "redirect_uri_mismatch" => {
                ErrorCode::CallbackRejected
            }
            _ => ErrorCode::ProviderProtocolError,
        };
        return Err(RemoteProviderError::new(
            code,
            format!("GitHub token exchange failed: {error}"),
        ));
    }

    let access_token = parsed.access_token.ok_or_else(|| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            "token response missing access_token",
        )
    })?;
    Ok(TokenExchangeOutcome {
        access_token: Secret::new(access_token),
        refresh_token: parsed.refresh_token.map(Secret::new),
        expires_in_secs: parsed.expires_in,
        refresh_token_expires_in_secs: parsed.refresh_token_expires_in,
    })
}

pub struct RefreshOutcome {
    pub access_token: Secret,
    pub refresh_token: Option<Secret>,
    pub expires_in_secs: Option<u64>,
}

/// Refreshes an expiring GitHub App user access token. Verified against
/// GitHub's own token-refresh documentation: `client_secret` is
/// "Required unless the user access token was generated using the device
/// flow" -- since Decision 0062 retires device flow from the Workbench,
/// every refresh now includes it (as public application configuration, per
/// this module's doc comment, never as `CredentialStore` material).
pub fn refresh_access_token(
    transport: &dyn HttpTransport,
    client_id: &str,
    client_secret: &str,
    refresh_token: &Secret,
) -> RemoteProviderResult<RefreshOutcome> {
    let request = FormRequest {
        url: ACCESS_TOKEN_URL.to_string(),
        fields: vec![
            ("client_id".to_string(), client_id.to_string()),
            ("client_secret".to_string(), client_secret.to_string()),
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
    let parsed: TokenResponse = serde_json::from_str(&response.body).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed refresh response: {error}"),
        )
    })?;

    if let Some(error) = parsed.error {
        let code = match error.as_str() {
            "access_denied" | "bad_refresh_token" => ErrorCode::AuthorizationDenied,
            "expired_token" | "token_expired" => ErrorCode::AuthorizationExpired,
            _ => ErrorCode::RefreshFailed,
        };
        return Err(RemoteProviderError::new(
            code,
            format!("GitHub refresh failed: {error}"),
        ));
    }

    let access_token = parsed.access_token.ok_or_else(|| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            "refresh response missing access_token",
        )
    })?;
    Ok(RefreshOutcome {
        access_token: Secret::new(access_token),
        refresh_token: parsed.refresh_token.map(Secret::new),
        expires_in_secs: parsed.expires_in,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{json_response, ScriptedTransport};

    #[test]
    fn authorization_url_contains_the_required_public_parameters() {
        let url = build_authorization_url(
            "client-123",
            "http://127.0.0.1:54321/repopact/github/callback",
            "state-abc",
            "challenge-xyz",
        );
        assert!(url.starts_with(AUTHORIZE_URL));
        assert!(url.contains("client_id=client-123"));
        assert!(url.contains("state=state-abc"));
        assert!(url.contains("code_challenge=challenge-xyz"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("redirect_uri="));
    }

    #[test]
    fn authorization_url_never_contains_a_secret_or_verifier() {
        let url = build_authorization_url(
            "client-123",
            "http://127.0.0.1:1/repopact/github/callback",
            "state-abc",
            "challenge-xyz",
        );
        assert!(!url.contains("client_secret"));
        assert!(!url.contains("code_verifier"));
        assert!(!url.to_lowercase().contains("access_token"));
        assert!(!url.to_lowercase().contains("refresh_token"));
    }

    #[test]
    fn exchange_sends_the_expected_form_fields_and_returns_tokens() {
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
        let verifier = CodeVerifier::generate();
        let outcome = exchange_code_for_token(
            &transport,
            "client-id",
            "public-client-secret",
            "auth-code",
            "http://127.0.0.1:1/repopact/github/callback",
            &verifier,
        )
        .unwrap();
        assert_eq!(
            outcome.access_token.expose(),
            "ghu_test0000000000000000000000000000"
        );
        assert_eq!(outcome.expires_in_secs, Some(28800));

        let sent = transport.received_requests();
        assert_eq!(sent.len(), 1);
        let field = |name: &str| {
            sent[0]
                .fields
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(field("client_id"), Some("client-id".to_string()));
        assert_eq!(
            field("client_secret"),
            Some("public-client-secret".to_string())
        );
        assert_eq!(field("code"), Some("auth-code".to_string()));
        assert_eq!(
            field("grant_type"),
            Some(AUTHORIZATION_CODE_GRANT_TYPE.to_string())
        );
        assert_eq!(field("code_verifier"), Some(verifier.expose().to_string()));
    }

    #[test]
    fn exchange_maps_an_invalid_verifier_error_to_callback_rejected() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(
            200,
            &[("error", "bad_verification_code")],
        )));
        let error = exchange_code_for_token(
            &transport,
            "id",
            "secret",
            "bad-code",
            "http://127.0.0.1:1/cb",
            &CodeVerifier::generate(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::CallbackRejected);
    }

    #[test]
    fn exchange_maps_denial() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(200, &[("error", "access_denied")])));
        let error = exchange_code_for_token(
            &transport,
            "id",
            "secret",
            "code",
            "http://127.0.0.1:1/cb",
            &CodeVerifier::generate(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::AuthorizationDenied);
    }

    #[test]
    fn refresh_includes_client_secret_since_device_flow_is_no_longer_used() {
        let transport = ScriptedTransport::new();
        transport.push_response(Ok(json_response(
            200,
            &[
                ("access_token", "new-access"),
                ("refresh_token", "new-refresh"),
                ("expires_in", "28800"),
            ],
        )));
        let outcome = refresh_access_token(
            &transport,
            "client-id",
            "public-secret",
            &Secret::new("old"),
        )
        .unwrap();
        assert_eq!(outcome.access_token.expose(), "new-access");
        let sent = transport.received_requests();
        assert!(sent[0]
            .fields
            .iter()
            .any(|(k, v)| k == "client_secret" && v == "public-secret"));
    }
}
