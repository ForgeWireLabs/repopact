//! WI067 item 8, item 38: the GitHub adapter behind the provider-neutral
//! seam. `DesktopService`, `RepositorySession`, `mobile_acquisition.rs`,
//! and React components never see this module -- they only ever see
//! `repopact_remote_provider::provider::RemoteRepositoryProvider`.
//!
//! Checkpoint A wires the device-flow auth state machine fully (against an
//! injected [`HttpTransport`]); repository listing/ref resolution/snapshot
//! download are intentionally left as explicit "not yet implemented"
//! errors here -- they are later-checkpoint work, not part of this
//! checkpoint's boundary.

use std::sync::Mutex;

use repopact_remote_provider::account::RemoteAccount;
use repopact_remote_provider::auth::AuthState;
use repopact_remote_provider::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
use repopact_remote_provider::provider::{ProviderCapabilities, RemoteRepositoryProvider};
use repopact_remote_provider::redact::Secret;
use repopact_remote_provider::refs::{RemoteRef, ResolvedRevision};
use repopact_remote_provider::repository::RemoteRepository;
use repopact_remote_provider::snapshot::{SnapshotArtifact, SnapshotDescriptor};

use crate::device_flow::{self, DeviceAuthorization, DevicePollOutcome};
use crate::transport::HttpTransport;

/// No private key, no client secret: a GitHub App's `client_id` is not
/// confidential (Decision 0061, item 12).
pub struct GitHubProviderConfig {
    pub client_id: String,
}

enum InternalAuthState {
    Disconnected,
    AwaitingUser {
        pending: DeviceAuthorization,
    },
    Authorized {
        account_label: String,
        // Retained (not yet read) pending the `GET /user` call and token
        // refresh flow a later checkpoint wires in.
        #[allow(dead_code)]
        access_token: Secret,
        #[allow(dead_code)]
        refresh_token: Option<Secret>,
    },
    Cancelled,
    Failed(ErrorCode),
}

pub struct GitHubProvider<'a> {
    config: GitHubProviderConfig,
    transport: &'a dyn HttpTransport,
    state: Mutex<InternalAuthState>,
}

impl<'a> GitHubProvider<'a> {
    pub fn new(config: GitHubProviderConfig, transport: &'a dyn HttpTransport) -> Self {
        Self {
            config,
            transport,
            state: Mutex::new(InternalAuthState::Disconnected),
        }
    }

    fn public_state(state: &InternalAuthState) -> AuthState {
        match state {
            InternalAuthState::Disconnected => AuthState::Disconnected,
            InternalAuthState::AwaitingUser { pending, .. } => AuthState::AwaitingUser {
                user_code: pending.user_code.clone(),
                verification_uri: pending.verification_uri.clone(),
                expires_at: format!("+{}s", pending.expires_in_secs),
            },
            InternalAuthState::Authorized { account_label, .. } => AuthState::Authorized {
                account_label: account_label.clone(),
            },
            InternalAuthState::Cancelled => AuthState::Cancelled,
            InternalAuthState::Failed(code) => AuthState::Failed { code: *code },
        }
    }

    fn not_yet_implemented(operation: &str) -> RemoteProviderError {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("{operation} is not implemented in WI067 Checkpoint A"),
        )
    }
}

impl<'a> RemoteRepositoryProvider for GitHubProvider<'a> {
    fn provider_id(&self) -> &'static str {
        "github"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_public_without_auth: true,
            supports_private_repositories: true,
            supports_organizations: true,
        }
    }

    fn connection_status(&self) -> AuthState {
        Self::public_state(&self.state.lock().unwrap())
    }

    fn begin_authorization(&self) -> RemoteProviderResult<AuthState> {
        let pending = device_flow::start_device_flow(self.transport, &self.config.client_id)?;
        let mut state = self.state.lock().unwrap();
        let public = AuthState::AwaitingUser {
            user_code: pending.user_code.clone(),
            verification_uri: pending.verification_uri.clone(),
            expires_at: format!("+{}s", pending.expires_in_secs),
        };
        *state = InternalAuthState::AwaitingUser { pending };
        Ok(public)
    }

    fn poll_authorization(&self) -> RemoteProviderResult<AuthState> {
        let mut state = self.state.lock().unwrap();
        let device_code = match &*state {
            InternalAuthState::AwaitingUser { pending, .. } => pending.device_code.clone(),
            other => return Ok(Self::public_state(other)),
        };
        let outcome =
            device_flow::poll_device_flow(self.transport, &self.config.client_id, &device_code)?;
        *state = match outcome {
            DevicePollOutcome::Pending => return Ok(Self::public_state(&state)),
            DevicePollOutcome::SlowDown { .. } => return Ok(Self::public_state(&state)),
            DevicePollOutcome::Authorized {
                access_token,
                refresh_token,
                ..
            } => InternalAuthState::Authorized {
                // Checkpoint A does not yet call `GET /user`; the display
                // label is a placeholder until that endpoint is wired in a
                // later checkpoint.
                account_label: "GitHub account (label pending /user call)".to_string(),
                access_token,
                refresh_token,
            },
            DevicePollOutcome::Expired => {
                InternalAuthState::Failed(ErrorCode::AuthorizationExpired)
            }
            DevicePollOutcome::Denied => InternalAuthState::Failed(ErrorCode::AuthorizationDenied),
            DevicePollOutcome::DeviceFlowDisabled => {
                InternalAuthState::Failed(ErrorCode::ProviderProtocolError)
            }
        };
        Ok(Self::public_state(&state))
    }

    fn cancel_authorization(&self) -> RemoteProviderResult<()> {
        let mut state = self.state.lock().unwrap();
        *state = InternalAuthState::Cancelled;
        Ok(())
    }

    fn disconnect(&self) -> RemoteProviderResult<()> {
        // Local-only: deletes in-process auth state. Deleting the token
        // from the platform CredentialStore is the caller's job (this
        // adapter never owns storage -- see
        // `repopact_remote_provider::credential::CredentialStore`).
        // Remote/server-side revocation is not performed (item 21) --
        // GitHub App user-token revocation requires a separate
        // authenticated endpoint this checkpoint does not implement.
        let mut state = self.state.lock().unwrap();
        *state = InternalAuthState::Disconnected;
        Ok(())
    }

    fn list_accounts(&self) -> RemoteProviderResult<Vec<RemoteAccount>> {
        let state = self.state.lock().unwrap();
        match &*state {
            InternalAuthState::Authorized { .. } => Err(Self::not_yet_implemented("list_accounts")),
            _ => Err(RemoteProviderError::new(
                ErrorCode::NotConnected,
                "not authorized",
            )),
        }
    }

    fn list_repositories(
        &self,
        _account: &RemoteAccount,
    ) -> RemoteProviderResult<Vec<RemoteRepository>> {
        Err(Self::not_yet_implemented("list_repositories"))
    }

    fn search_repositories(
        &self,
        _account: &RemoteAccount,
        _query: &str,
    ) -> RemoteProviderResult<Vec<RemoteRepository>> {
        Err(Self::not_yet_implemented("search_repositories"))
    }

    fn list_refs(&self, _repository: &RemoteRepository) -> RemoteProviderResult<Vec<RemoteRef>> {
        Err(Self::not_yet_implemented("list_refs"))
    }

    fn resolve_ref(
        &self,
        _repository: &RemoteRepository,
        _reference: &RemoteRef,
    ) -> RemoteProviderResult<ResolvedRevision> {
        Err(Self::not_yet_implemented("resolve_ref"))
    }

    fn describe_snapshot(
        &self,
        _repository: &RemoteRepository,
        _revision: &ResolvedRevision,
    ) -> RemoteProviderResult<SnapshotDescriptor> {
        Err(Self::not_yet_implemented("describe_snapshot"))
    }

    fn open_snapshot(
        &self,
        _descriptor: &SnapshotDescriptor,
    ) -> RemoteProviderResult<SnapshotArtifact> {
        Err(Self::not_yet_implemented("open_snapshot"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{json_response, ScriptedTransport};

    fn config() -> GitHubProviderConfig {
        GitHubProviderConfig {
            client_id: "test-client-id".to_string(),
        }
    }

    #[test]
    fn begin_authorization_transitions_to_awaiting_user_without_a_client_secret() {
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
        let provider = GitHubProvider::new(config(), &transport);
        let state = provider.begin_authorization().unwrap();
        match state {
            AuthState::AwaitingUser {
                user_code,
                verification_uri,
                ..
            } => {
                assert_eq!(user_code, "WDJB-MJHT");
                assert_eq!(verification_uri, "https://github.com/login/device");
            }
            other => panic!("expected AwaitingUser, got {other:?}"),
        }
        assert!(transport.received_requests()[0]
            .fields
            .iter()
            .all(|(k, _)| k != "client_secret"));
    }

    #[test]
    fn poll_authorization_transitions_to_authorized_on_success() {
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
        transport.push_response(Ok(json_response(
            200,
            &[
                ("access_token", "ghu_test0000000000000000000000000000"),
                ("expires_in", "28800"),
            ],
        )));
        let provider = GitHubProvider::new(config(), &transport);
        provider.begin_authorization().unwrap();
        let state = provider.poll_authorization().unwrap();
        assert!(matches!(state, AuthState::Authorized { .. }));
    }

    #[test]
    fn poll_authorization_surfaces_pending_without_changing_state() {
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
        transport.push_response(Ok(json_response(
            200,
            &[("error", "authorization_pending")],
        )));
        let provider = GitHubProvider::new(config(), &transport);
        provider.begin_authorization().unwrap();
        let state = provider.poll_authorization().unwrap();
        assert!(matches!(state, AuthState::AwaitingUser { .. }));
    }

    #[test]
    fn cancel_authorization_moves_to_cancelled() {
        let transport = ScriptedTransport::new();
        let provider = GitHubProvider::new(config(), &transport);
        provider.cancel_authorization().unwrap();
        assert!(matches!(provider.connection_status(), AuthState::Cancelled));
    }

    #[test]
    fn disconnect_returns_to_disconnected_without_claiming_remote_revocation() {
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
        transport.push_response(Ok(json_response(
            200,
            &[("access_token", "ghu_test0000000000000000000000000000")],
        )));
        let provider = GitHubProvider::new(config(), &transport);
        provider.begin_authorization().unwrap();
        provider.poll_authorization().unwrap();
        provider.disconnect().unwrap();
        assert!(matches!(
            provider.connection_status(),
            AuthState::Disconnected
        ));
    }

    #[test]
    fn repository_operations_are_explicitly_not_implemented_this_checkpoint() {
        let transport = ScriptedTransport::new();
        let provider = GitHubProvider::new(config(), &transport);
        let account = RemoteAccount {
            id: repopact_remote_provider::account::RemoteAccountId {
                provider: "github".into(),
                provider_account_id: "1".into(),
            },
            display_label: "x".into(),
            scope: repopact_remote_provider::account::ProviderScope {
                label: "x".into(),
                includes_private_repositories: false,
            },
        };
        assert!(provider.list_repositories(&account).is_err());
    }
}
