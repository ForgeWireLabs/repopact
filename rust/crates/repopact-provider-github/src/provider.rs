//! WI067 item 8: the GitHub adapter behind the provider-neutral seam.
//! `DesktopService`, `RepositorySession`, `mobile_acquisition.rs`, and
//! React components never see this module -- they only ever see
//! `repopact_remote_provider::provider::RemoteRepositoryProvider`.
//!
//! Checkpoint B wires real network calls (current user, installations,
//! repositories, branches/tags, ref resolution) and real credential
//! persistence through an injected `CredentialStore`. `describe_snapshot`/
//! `open_snapshot` remain explicit "not yet implemented" errors -- archive
//! download/materialization is Checkpoint C's boundary, not this one's.

use std::sync::{Arc, Mutex};

use repopact_remote_provider::account::{ProviderScope, RemoteAccount, RemoteAccountId};
use repopact_remote_provider::auth::AuthState;
use repopact_remote_provider::credential::{CredentialKey, CredentialKind, CredentialStore};
use repopact_remote_provider::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
use repopact_remote_provider::provider::{ProviderCapabilities, RemoteRepositoryProvider};
use repopact_remote_provider::redact::Secret;
use repopact_remote_provider::refs::{RefKind, RemoteRef, ResolvedRevision};
use repopact_remote_provider::repository::{RemoteRepository, RepositoryVisibility};
use repopact_remote_provider::snapshot::{SnapshotArtifact, SnapshotDescriptor};

use crate::device_flow::{self, DeviceAuthorization, DevicePollOutcome};
use crate::rest;
use crate::transport::{HttpTransport, RestTransport};

/// No private key, no client secret: a GitHub App's `client_id` is not
/// confidential (Decision 0061, item 12).
pub struct GitHubProviderConfig {
    pub client_id: String,
}

/// Fixed single-connection identity for v1 (item 15's "active auth
/// operations" is a single active GitHub connection, not a multi-account
/// registry). Used as the `CredentialStore` key's `connection_id` before
/// the real GitHub numeric user id is known, and remains the key
/// afterward -- re-keying by user id would complicate the disconnect/
/// restart-restore path for no v1 benefit, since only one GitHub identity
/// can be connected at a time.
pub const PRIMARY_CONNECTION_ID: &str = "github-primary";

#[derive(Debug, Clone)]
struct TokenState {
    access_token: Secret,
    refresh_token: Option<Secret>,
    access_token_expires_at_epoch: Option<u64>,
}

enum InternalAuthState {
    Disconnected,
    AwaitingUser {
        pending: DeviceAuthorization,
    },
    Authorized {
        login: String,
        user_id: u64,
        tokens: TokenState,
    },
    Cancelled,
    Failed(ErrorCode),
}

/// `now_epoch_seconds` is injectable so expiry/refresh behavior (item 42)
/// is deterministically testable without an eight-hour wait; production
/// callers use `GitHubProvider::new`, which defaults to the real clock.
pub struct GitHubProvider {
    config: GitHubProviderConfig,
    form_transport: Arc<dyn HttpTransport>,
    rest_transport: Arc<dyn RestTransport>,
    credential_store: Arc<dyn CredentialStore>,
    state: Mutex<InternalAuthState>,
    now_epoch_seconds: fn() -> u64,
}

fn real_now_epoch_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn access_token_key() -> CredentialKey {
    CredentialKey {
        provider: "github".to_string(),
        connection_id: PRIMARY_CONNECTION_ID.to_string(),
        kind: CredentialKind::AccessToken,
    }
}

fn refresh_token_key() -> CredentialKey {
    CredentialKey {
        provider: "github".to_string(),
        connection_id: PRIMARY_CONNECTION_ID.to_string(),
        kind: CredentialKind::RefreshToken,
    }
}

impl GitHubProvider {
    pub fn new(
        config: GitHubProviderConfig,
        form_transport: Arc<dyn HttpTransport>,
        rest_transport: Arc<dyn RestTransport>,
        credential_store: Arc<dyn CredentialStore>,
    ) -> Self {
        Self {
            config,
            form_transport,
            rest_transport,
            credential_store,
            state: Mutex::new(InternalAuthState::Disconnected),
            now_epoch_seconds: real_now_epoch_seconds,
        }
    }

    #[cfg(test)]
    pub fn with_clock(mut self, now_epoch_seconds: fn() -> u64) -> Self {
        self.now_epoch_seconds = now_epoch_seconds;
        self
    }

    /// Restores an `Authorized` session from previously persisted
    /// credentials (item 41: restart persistence). Returns
    /// `AuthState::Disconnected` (not an error) if nothing was stored --
    /// "no saved connection" is a normal state, not a failure. Does not
    /// itself contact GitHub; the caller's next real operation (or an
    /// explicit refresh) discovers an expired/revoked token naturally.
    pub fn restore_from_credential_store(
        &self,
        login: String,
        user_id: u64,
        access_token_expires_at_epoch: Option<u64>,
    ) -> RemoteProviderResult<AuthState> {
        let access_token = match self.credential_store.get(&access_token_key())? {
            Some(token) => token,
            None => return Ok(AuthState::Disconnected),
        };
        let refresh_token = self.credential_store.get(&refresh_token_key())?;
        let mut state = self.state.lock().unwrap();
        *state = InternalAuthState::Authorized {
            login,
            user_id,
            tokens: TokenState {
                access_token,
                refresh_token,
                access_token_expires_at_epoch,
            },
        };
        Ok(Self::public_state(&state))
    }

    fn public_state(state: &InternalAuthState) -> AuthState {
        match state {
            InternalAuthState::Disconnected => AuthState::Disconnected,
            InternalAuthState::AwaitingUser { pending, .. } => AuthState::AwaitingUser {
                user_code: pending.user_code.clone(),
                verification_uri: pending.verification_uri.clone(),
                expires_at: format!("+{}s", pending.expires_in_secs),
            },
            InternalAuthState::Authorized { login, .. } => AuthState::Authorized {
                account_label: login.clone(),
            },
            InternalAuthState::Cancelled => AuthState::Cancelled,
            InternalAuthState::Failed(code) => AuthState::Failed { code: *code },
        }
    }

    fn not_yet_implemented(operation: &str) -> RemoteProviderError {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("{operation} is not implemented in WI067 Checkpoint B (Checkpoint C)"),
        )
    }

    /// Persists a fresh token pair to the credential store and (item 12)
    /// does so access-token-first, refresh-token-second: if the process is
    /// killed between the two writes, the worst case is a stored access
    /// token with a stale (or absent) refresh token, which only degrades
    /// silent refresh -- it never leaves a refresh token without a valid
    /// access token, and a subsequent explicit re-authorization always
    /// overwrites both cleanly. `keyring`'s underlying OS calls are
    /// themselves atomic per-entry; there is no multi-key transaction
    /// primitive to build on, so this ordering is the safest achievable
    /// two-write sequence.
    fn persist_tokens(&self, tokens: &TokenState) -> RemoteProviderResult<()> {
        self.credential_store
            .put(&access_token_key(), tokens.access_token.clone())?;
        match &tokens.refresh_token {
            Some(refresh_token) => self
                .credential_store
                .put(&refresh_token_key(), refresh_token.clone())?,
            None => self.credential_store.delete(&refresh_token_key())?,
        }
        Ok(())
    }

    fn clear_tokens(&self) -> RemoteProviderResult<()> {
        self.credential_store.delete(&access_token_key())?;
        self.credential_store.delete(&refresh_token_key())?;
        Ok(())
    }

    /// Returns a token guaranteed usable for the next request: refreshes
    /// natively first if the current access token is expired or about to
    /// expire (item 11), atomically replacing the stored pair on success,
    /// and clearing stored credentials and moving to a terminal state on
    /// refresh failure/denial/expiry (never leaving a half-valid pair
    /// behind).
    fn ensure_fresh_access_token(&self) -> RemoteProviderResult<Secret> {
        let now = (self.now_epoch_seconds)();
        let mut state = self.state.lock().unwrap();
        let (login, user_id, tokens) = match &*state {
            InternalAuthState::Authorized {
                login,
                user_id,
                tokens,
            } => (login.clone(), *user_id, tokens.clone()),
            _ => {
                return Err(RemoteProviderError::new(
                    ErrorCode::NotConnected,
                    "not authorized",
                ))
            }
        };

        const EXPIRY_SAFETY_MARGIN_SECONDS: u64 = 60;
        let needs_refresh = tokens
            .access_token_expires_at_epoch
            .is_some_and(|expiry| now + EXPIRY_SAFETY_MARGIN_SECONDS >= expiry);
        if !needs_refresh {
            return Ok(tokens.access_token.clone());
        }

        let Some(refresh_token) = tokens.refresh_token.clone() else {
            *state = InternalAuthState::Failed(ErrorCode::AuthorizationExpired);
            drop(state);
            let _ = self.clear_tokens();
            return Err(RemoteProviderError::new(
                ErrorCode::AuthorizationExpired,
                "access token expired and no refresh token is available",
            ));
        };

        match device_flow::refresh_access_token(
            self.form_transport.as_ref(),
            &self.config.client_id,
            &refresh_token,
        ) {
            Ok(outcome) => {
                let new_tokens = TokenState {
                    access_token: outcome.access_token.clone(),
                    refresh_token: outcome.refresh_token,
                    access_token_expires_at_epoch: outcome.expires_in_secs.map(|secs| now + secs),
                };
                self.persist_tokens(&new_tokens)?;
                *state = InternalAuthState::Authorized {
                    login,
                    user_id,
                    tokens: new_tokens,
                };
                Ok(outcome.access_token)
            }
            Err(error) => {
                *state = InternalAuthState::Failed(ErrorCode::RefreshFailed);
                drop(state);
                let _ = self.clear_tokens();
                Err(error)
            }
        }
    }
}

impl RemoteRepositoryProvider for GitHubProvider {
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
        let pending =
            device_flow::start_device_flow(self.form_transport.as_ref(), &self.config.client_id)?;
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
        let device_code = {
            let state = self.state.lock().unwrap();
            match &*state {
                InternalAuthState::AwaitingUser { pending, .. } => pending.device_code.clone(),
                other => return Ok(Self::public_state(other)),
            }
        };
        let outcome = device_flow::poll_device_flow(
            self.form_transport.as_ref(),
            &self.config.client_id,
            &device_code,
        )?;
        let (access_token, refresh_token, expires_in_secs) = match outcome {
            DevicePollOutcome::Pending | DevicePollOutcome::SlowDown { .. } => {
                return Ok(self.connection_status())
            }
            DevicePollOutcome::Authorized {
                access_token,
                refresh_token,
                expires_in_secs,
                ..
            } => (access_token, refresh_token, expires_in_secs),
            DevicePollOutcome::Expired => {
                *self.state.lock().unwrap() =
                    InternalAuthState::Failed(ErrorCode::AuthorizationExpired);
                return Ok(self.connection_status());
            }
            DevicePollOutcome::Denied => {
                *self.state.lock().unwrap() =
                    InternalAuthState::Failed(ErrorCode::AuthorizationDenied);
                return Ok(self.connection_status());
            }
            DevicePollOutcome::DeviceFlowDisabled => {
                *self.state.lock().unwrap() =
                    InternalAuthState::Failed(ErrorCode::ProviderProtocolError);
                return Ok(self.connection_status());
            }
        };

        // Establish stable GitHub identity (item 22) before declaring the
        // connection Authorized, per Decision 0061's "fetch authenticated
        // user identity" successful-connection sequence.
        let user = rest::get_current_user(self.rest_transport.as_ref(), &access_token)?;
        let now = (self.now_epoch_seconds)();
        let tokens = TokenState {
            access_token,
            refresh_token,
            access_token_expires_at_epoch: expires_in_secs.map(|secs| now + secs),
        };
        self.persist_tokens(&tokens)?;
        let mut state = self.state.lock().unwrap();
        *state = InternalAuthState::Authorized {
            login: user.login,
            user_id: user.id,
            tokens,
        };
        Ok(Self::public_state(&state))
    }

    fn cancel_authorization(&self) -> RemoteProviderResult<()> {
        let mut state = self.state.lock().unwrap();
        *state = InternalAuthState::Cancelled;
        Ok(())
    }

    fn disconnect(&self) -> RemoteProviderResult<()> {
        // Local-only (item 14): deletes the locally stored token pair and
        // clears in-process auth state. GitHub App user-token revocation
        // requires an authenticated endpoint call this checkpoint does not
        // implement (it would itself need the very token being revoked,
        // and v1 has no separate revocation authority) -- this is
        // "Disconnect" (local), never claimed as "Revoke GitHub
        // authorization" (remote). Already-materialized workspaces and
        // their provenance are untouched -- this method never touches the
        // workspace registry.
        self.clear_tokens()?;
        let mut state = self.state.lock().unwrap();
        *state = InternalAuthState::Disconnected;
        Ok(())
    }

    fn list_accounts(&self) -> RemoteProviderResult<Vec<RemoteAccount>> {
        let token = self.ensure_fresh_access_token()?;
        let mut accounts = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page =
                rest::list_installations(self.rest_transport.as_ref(), &token, cursor.as_deref())?;
            for installation in page.items {
                accounts.push(RemoteAccount {
                    id: RemoteAccountId {
                        provider: "github".to_string(),
                        provider_account_id: installation.installation_id.to_string(),
                    },
                    display_label: installation.account_login.clone(),
                    scope: ProviderScope {
                        label: format!(
                            "{} ({})",
                            installation.account_type, installation.repository_selection
                        ),
                        includes_private_repositories: true,
                    },
                });
            }
            if !page.has_more {
                break;
            }
            cursor = page.next_cursor;
        }
        Ok(accounts)
    }

    fn list_repositories(
        &self,
        account: &RemoteAccount,
    ) -> RemoteProviderResult<Vec<RemoteRepository>> {
        let token = self.ensure_fresh_access_token()?;
        let installation_id: u64 = account.id.provider_account_id.parse().map_err(|_| {
            RemoteProviderError::new(
                ErrorCode::ProviderProtocolError,
                "malformed installation id",
            )
        })?;
        let mut repositories = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = rest::list_installation_repositories(
                self.rest_transport.as_ref(),
                &token,
                installation_id,
                cursor.as_deref(),
            )?;
            for repo in page.items {
                repositories.push(RemoteRepository {
                    provider: "github".to_string(),
                    provider_repository_id: repo.id.to_string(),
                    owner_label: repo.owner_login,
                    name: repo.name,
                    full_display_name: repo.full_name,
                    visibility: if repo.private {
                        RepositoryVisibility::Private
                    } else {
                        RepositoryVisibility::Public
                    },
                    default_branch: repo.default_branch,
                });
            }
            if !page.has_more {
                break;
            }
            cursor = page.next_cursor;
        }
        Ok(repositories)
    }

    fn search_repositories(
        &self,
        account: &RemoteAccount,
        query: &str,
    ) -> RemoteProviderResult<Vec<RemoteRepository>> {
        // Item 25: never a global search endpoint that could expose
        // metadata outside the installation grant -- filter within the
        // already-authorized installation repository set.
        let repositories = self.list_repositories(account)?;
        if query.is_empty() {
            return Ok(repositories);
        }
        let query_lower = query.to_lowercase();
        Ok(repositories
            .into_iter()
            .filter(|repo| {
                repo.name.to_lowercase().contains(&query_lower)
                    || repo.full_display_name.to_lowercase().contains(&query_lower)
            })
            .collect())
    }

    fn list_refs(&self, repository: &RemoteRepository) -> RemoteProviderResult<Vec<RemoteRef>> {
        let token = self.optional_access_token();
        let mut refs = Vec::new();

        let mut cursor: Option<String> = None;
        loop {
            let page = rest::list_branches(
                self.rest_transport.as_ref(),
                token.as_ref(),
                &repository.owner_label,
                &repository.name,
                cursor.as_deref(),
            )?;
            for branch in page.items {
                refs.push(RemoteRef {
                    display_name: branch.name.clone(),
                    kind: RefKind::Branch,
                    provider_ref_id: format!("heads/{}", branch.name),
                });
            }
            if !page.has_more {
                break;
            }
            cursor = page.next_cursor;
        }

        let mut cursor: Option<String> = None;
        loop {
            let page = rest::list_tags(
                self.rest_transport.as_ref(),
                token.as_ref(),
                &repository.owner_label,
                &repository.name,
                cursor.as_deref(),
            )?;
            for tag in page.items {
                refs.push(RemoteRef {
                    display_name: tag.name.clone(),
                    kind: RefKind::Tag,
                    provider_ref_id: format!("tags/{}", tag.name),
                });
            }
            if !page.has_more {
                break;
            }
            cursor = page.next_cursor;
        }

        Ok(refs)
    }

    fn resolve_ref(
        &self,
        repository: &RemoteRepository,
        reference: &RemoteRef,
    ) -> RemoteProviderResult<ResolvedRevision> {
        let token = self.optional_access_token();
        let immutable_revision_id = match reference.kind {
            RefKind::Branch | RefKind::Tag => rest::resolve_branch_or_tag(
                self.rest_transport.as_ref(),
                token.as_ref(),
                &repository.owner_label,
                &repository.name,
                &reference.provider_ref_id,
            )?,
            RefKind::Commit => rest::resolve_commit(
                self.rest_transport.as_ref(),
                token.as_ref(),
                &repository.owner_label,
                &repository.name,
                &reference.provider_ref_id,
            )?,
        };
        Ok(ResolvedRevision {
            selected_ref: reference.clone(),
            immutable_revision_id,
        })
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

impl GitHubProvider {
    /// Best-effort token for read paths that also support unauthenticated
    /// public access (branch/tag listing and ref resolution): returns
    /// `Some` only if currently Authorized and the token is fresh, `None`
    /// otherwise (never surfaces a refresh failure here -- a caller
    /// browsing a public repository while disconnected is not an error).
    fn optional_access_token(&self) -> Option<Secret> {
        self.ensure_fresh_access_token().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{
        json_response, rest_json_response, ScriptedRestTransport, ScriptedTransport,
    };
    use repopact_remote_provider::credential::InMemoryCredentialStore;
    use serde_json::json;

    fn config() -> GitHubProviderConfig {
        GitHubProviderConfig {
            client_id: "test-client-id".to_string(),
        }
    }

    fn device_code_response() -> crate::transport::HttpResponse {
        json_response(
            200,
            &[
                ("device_code", "abc123"),
                ("user_code", "WDJB-MJHT"),
                ("verification_uri", "https://github.com/login/device"),
                ("expires_in", "900"),
                ("interval", "5"),
            ],
        )
    }

    #[test]
    fn begin_authorization_transitions_to_awaiting_user_without_a_client_secret() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        form.push_response(Ok(device_code_response()));
        let provider =
            GitHubProvider::new(config(), form.clone(), rest.clone(), credentials.clone());
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
        assert!(form.received_requests()[0]
            .fields
            .iter()
            .all(|(k, _)| k != "client_secret"));
    }

    #[test]
    fn poll_authorization_fetches_identity_and_persists_tokens_on_success() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        form.push_response(Ok(device_code_response()));
        form.push_response(Ok(json_response(
            200,
            &[
                ("access_token", "ghu_test0000000000000000000000000000"),
                ("refresh_token", "ghr_test0000000000000000000000000000"),
                ("expires_in", "28800"),
            ],
        )));
        rest.push_response(Ok(rest_json_response(
            200,
            json!({"id": 42, "login": "octocat"}),
        )));

        let provider =
            GitHubProvider::new(config(), form.clone(), rest.clone(), credentials.clone());
        provider.begin_authorization().unwrap();
        let state = provider.poll_authorization().unwrap();
        match state {
            AuthState::Authorized { account_label } => assert_eq!(account_label, "octocat"),
            other => panic!("expected Authorized, got {other:?}"),
        }

        let stored_access = credentials.get(&access_token_key()).unwrap().unwrap();
        assert_eq!(
            stored_access.expose(),
            "ghu_test0000000000000000000000000000"
        );
        let stored_refresh = credentials.get(&refresh_token_key()).unwrap().unwrap();
        assert_eq!(
            stored_refresh.expose(),
            "ghr_test0000000000000000000000000000"
        );
    }

    #[test]
    fn disconnect_clears_stored_credentials_and_state() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        form.push_response(Ok(device_code_response()));
        form.push_response(Ok(json_response(
            200,
            &[("access_token", "ghu_test0000000000000000000000000000")],
        )));
        rest.push_response(Ok(rest_json_response(200, json!({"id": 1, "login": "x"}))));

        let provider =
            GitHubProvider::new(config(), form.clone(), rest.clone(), credentials.clone());
        provider.begin_authorization().unwrap();
        provider.poll_authorization().unwrap();
        assert!(credentials.get(&access_token_key()).unwrap().is_some());

        provider.disconnect().unwrap();
        assert!(matches!(
            provider.connection_status(),
            AuthState::Disconnected
        ));
        assert!(credentials.get(&access_token_key()).unwrap().is_none());
    }

    #[test]
    fn restore_from_credential_store_recovers_a_prior_session_without_a_network_call() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        credentials
            .put(
                &access_token_key(),
                Secret::new("ghu_test0000000000000000000000000000"),
            )
            .unwrap();

        let provider =
            GitHubProvider::new(config(), form.clone(), rest.clone(), credentials.clone());
        let state = provider
            .restore_from_credential_store("octocat".to_string(), 42, Some(9_999_999_999))
            .unwrap();
        assert!(matches!(state, AuthState::Authorized { .. }));
        // No network call was made to restore -- the scripted transports
        // still have zero queued interactions consumed.
        assert!(form.received_requests().is_empty());
    }

    #[test]
    fn restore_from_credential_store_with_nothing_stored_is_disconnected_not_an_error() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider =
            GitHubProvider::new(config(), form.clone(), rest.clone(), credentials.clone());
        let state = provider
            .restore_from_credential_store("octocat".to_string(), 42, None)
            .unwrap();
        assert!(matches!(state, AuthState::Disconnected));
    }

    #[test]
    fn an_expired_access_token_is_refreshed_atomically_before_the_next_call() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        credentials
            .put(&access_token_key(), Secret::new("old-access"))
            .unwrap();
        credentials
            .put(&refresh_token_key(), Secret::new("old-refresh"))
            .unwrap();

        // Refresh response.
        form.push_response(Ok(json_response(
            200,
            &[
                ("access_token", "new-access"),
                ("refresh_token", "new-refresh"),
                ("expires_in", "28800"),
            ],
        )));
        // The actual call the refreshed token is used for.
        rest.push_response(Ok(rest_json_response(200, json!({"installations": []}))));

        let provider =
            GitHubProvider::new(config(), form.clone(), rest.clone(), credentials.clone())
                .with_clock(|| 1_000_000_000);
        provider
            .restore_from_credential_store("octocat".to_string(), 42, Some(1_000_000_000 - 10))
            .unwrap();

        let accounts = provider.list_accounts().unwrap();
        assert!(accounts.is_empty());

        // The refresh request never carried a client_secret.
        assert!(form.received_requests()[0]
            .fields
            .iter()
            .all(|(k, _)| k != "client_secret"));
        assert_eq!(
            credentials
                .get(&access_token_key())
                .unwrap()
                .unwrap()
                .expose(),
            "new-access"
        );
        assert_eq!(
            credentials
                .get(&refresh_token_key())
                .unwrap()
                .unwrap()
                .expose(),
            "new-refresh"
        );

        let sent_rest = rest.received_requests();
        assert!(sent_rest[0]
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v.contains("new-access")));
    }

    #[test]
    fn a_denied_refresh_clears_credentials_and_reports_expired() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        credentials
            .put(&access_token_key(), Secret::new("old-access"))
            .unwrap();
        credentials
            .put(&refresh_token_key(), Secret::new("revoked-refresh"))
            .unwrap();

        form.push_response(Ok(json_response(200, &[("error", "access_denied")])));

        let provider =
            GitHubProvider::new(config(), form.clone(), rest.clone(), credentials.clone())
                .with_clock(|| 1_000_000_000);
        provider
            .restore_from_credential_store("octocat".to_string(), 42, Some(1_000_000_000 - 10))
            .unwrap();

        let error = provider.list_accounts().unwrap_err();
        assert_eq!(error.code, ErrorCode::AuthorizationDenied);
        assert!(credentials.get(&access_token_key()).unwrap().is_none());
        assert!(credentials.get(&refresh_token_key()).unwrap().is_none());
    }

    #[test]
    fn an_expired_token_with_no_refresh_token_reports_expired_without_a_network_call() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        credentials
            .put(&access_token_key(), Secret::new("old-access"))
            .unwrap();

        let provider =
            GitHubProvider::new(config(), form.clone(), rest.clone(), credentials.clone())
                .with_clock(|| 1_000_000_000);
        provider
            .restore_from_credential_store("octocat".to_string(), 42, Some(1_000_000_000 - 10))
            .unwrap();

        let error = provider.list_accounts().unwrap_err();
        assert_eq!(error.code, ErrorCode::AuthorizationExpired);
        assert!(form.received_requests().is_empty());
    }

    #[test]
    fn list_accounts_maps_installations_to_remote_accounts() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        credentials
            .put(&access_token_key(), Secret::new("token"))
            .unwrap();
        rest.push_response(Ok(rest_json_response(
            200,
            json!({"installations": [
                {"id": 7, "account": {"login": "acme", "type": "Organization"}, "repository_selection": "all"}
            ]}),
        )));
        let provider =
            GitHubProvider::new(config(), form.clone(), rest.clone(), credentials.clone());
        provider
            .restore_from_credential_store("x".into(), 1, None)
            .unwrap();
        let accounts = provider.list_accounts().unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id.provider_account_id, "7");
        assert_eq!(accounts[0].display_label, "acme");
    }

    #[test]
    fn search_repositories_filters_within_the_authorized_installation_set() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        credentials
            .put(&access_token_key(), Secret::new("token"))
            .unwrap();
        rest.push_response(Ok(rest_json_response(
            200,
            json!({"repositories": [
                {"id": 1, "name": "alpha", "full_name": "acme/alpha", "owner": {"login": "acme"}, "private": false, "default_branch": "main", "archived": false},
                {"id": 2, "name": "beta", "full_name": "acme/beta", "owner": {"login": "acme"}, "private": true, "default_branch": "main", "archived": false}
            ]}),
        )));
        let provider =
            GitHubProvider::new(config(), form.clone(), rest.clone(), credentials.clone());
        provider
            .restore_from_credential_store("x".into(), 1, None)
            .unwrap();
        let account = RemoteAccount {
            id: RemoteAccountId {
                provider: "github".into(),
                provider_account_id: "7".into(),
            },
            display_label: "acme".into(),
            scope: ProviderScope {
                label: "x".into(),
                includes_private_repositories: true,
            },
        };
        let results = provider.search_repositories(&account, "alp").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "alpha");
    }

    #[test]
    fn repository_materialization_operations_are_explicitly_not_implemented_this_checkpoint() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider =
            GitHubProvider::new(config(), form.clone(), rest.clone(), credentials.clone());
        let repository = RemoteRepository {
            provider: "github".into(),
            provider_repository_id: "1".into(),
            owner_label: "o".into(),
            name: "r".into(),
            full_display_name: "o/r".into(),
            visibility: RepositoryVisibility::Public,
            default_branch: Some("main".into()),
        };
        let revision = ResolvedRevision {
            selected_ref: RemoteRef {
                display_name: "main".into(),
                kind: RefKind::Branch,
                provider_ref_id: "heads/main".into(),
            },
            immutable_revision_id: "a".repeat(40),
        };
        assert!(provider.describe_snapshot(&repository, &revision).is_err());
    }
}
