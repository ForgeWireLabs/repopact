//! Decision 0061 (provider seam) / Decision 0062 (browser-redirect PKCE
//! authorization): the GitHub adapter behind the provider-neutral seam.
//! `DesktopService`, `RepositorySession`, `mobile_acquisition.rs`, and
//! React components never see this module -- they only ever see
//! `repopact_remote_provider::provider::RemoteRepositoryProvider`.
//!
//! Checkpoint B wired real network calls (current user, installations,
//! repositories, branches/tags, ref resolution) and real credential
//! persistence through an injected `CredentialStore`. Checkpoint C wires
//! `describe_snapshot`/`open_snapshot` to a real bounded, streaming zipball
//! download -- extraction/materialization/publication remain entirely
//! WI065's job (`repopact_mobile_acquisition::workspace::WorkspaceManager
//! ::import_remote_snapshot`), never duplicated here. Decision 0062 (the
//! WI067 browser-PKCE authorization revision) replaces the interactive
//! authorization mechanism the earlier checkpoints built
//! (`crate::device_flow`) with the browser-redirect-with-PKCE flow in
//! `crate::browser_flow` and `crate::callback`; nothing about repository
//! listing/ref resolution/snapshot download changes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use repopact_remote_provider::account::{ProviderScope, RemoteAccount, RemoteAccountId};
use repopact_remote_provider::auth::AuthState;
use repopact_remote_provider::credential::{CredentialKey, CredentialKind, CredentialStore};
use repopact_remote_provider::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
use repopact_remote_provider::pkce::{AuthorizationState, CodeVerifier};
use repopact_remote_provider::provider::{
    ProviderCapabilities, RemoteRepositoryProvider, SnapshotDownloadOptions,
};
use repopact_remote_provider::redact::Secret;
use repopact_remote_provider::refs::{RefKind, RemoteRef, ResolvedRevision};
use repopact_remote_provider::repository::{RemoteRepository, RepositoryVisibility};
use repopact_remote_provider::snapshot::{SnapshotArtifact, SnapshotDescriptor, SnapshotLayout};

use crate::browser_flow;
use crate::callback::{CallbackListener, CallbackWaitOutcome};
use crate::rest;
use crate::transport::{
    HttpTransport, RestTransport, StreamingDownloadTransport, StreamingRequest,
};

/// How long a browser-authorization session stays valid before the native
/// callback listener gives up and the session fails closed with `Expired`.
/// Generous for a real human completing a GitHub sign-in/authorize flow,
/// bounded so an abandoned session does not linger indefinitely holding a
/// loopback port open.
const AUTHORIZATION_SESSION_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// GitHub-named "client secret" -- required by GitHub's web/OAuth
/// token-exchange contract, but never confidential in this native/public
/// client architecture (Decision 0062). `app_slug` names the installed
/// GitHub App for the "Install RepoPact on GitHub" trusted-URL action.
pub struct GitHubProviderConfig {
    pub client_id: String,
    pub public_client_secret: String,
    pub app_slug: Option<String>,
}

impl std::fmt::Debug for GitHubProviderConfig {
    /// Decision 0062: the public client secret is not confidential, but it
    /// still must not be sprayed through diagnostics/logs/evidence
    /// incidentally via a derived `Debug` impl on a struct that contains
    /// it -- print only its presence, never its value.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubProviderConfig")
            .field("client_id", &self.client_id)
            .field(
                "public_client_secret",
                &if self.public_client_secret.is_empty() {
                    "<empty>"
                } else {
                    "<configured>"
                },
            )
            .field("app_slug", &self.app_slug)
            .finish()
    }
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

/// Decision 0062: an ephemeral, in-memory-only authorization session. Never
/// serialized, never persisted -- `code_verifier`/`state` are dropped the
/// moment the session resolves (success, failure, expiry, or cancellation).
struct BrowserAuthSession {
    state: AuthorizationState,
    code_verifier: CodeVerifier,
    redirect_uri: String,
    cancel_flag: Arc<AtomicBool>,
    /// Monotonically increasing generation counter. A background thread
    /// only ever commits its result if it is still the *current*
    /// generation -- a superseded (retried/cancelled) session's thread
    /// running to completion late can never clobber a newer session's
    /// state (Decision 0062: "Never reuse an old auth session").
    generation: u64,
}

enum InternalAuthState {
    Disconnected,
    StartingBrowserAuthorization,
    WaitingForCallback {
        expires_at_epoch: u64,
    },
    ExchangingCode,
    Authorized {
        login: String,
        user_id: u64,
        tokens: TokenState,
    },
    Cancelled,
    Expired,
    Failed(ErrorCode),
}

/// `now_epoch_seconds` is injectable so expiry/refresh behavior is
/// deterministically testable without a real wait; production callers use
/// `GitHubProvider::new`, which defaults to the real clock.
pub struct GitHubProvider {
    config: GitHubProviderConfig,
    form_transport: Arc<dyn HttpTransport>,
    rest_transport: Arc<dyn RestTransport>,
    snapshot_transport: Arc<dyn StreamingDownloadTransport>,
    credential_store: Arc<dyn CredentialStore>,
    state: Arc<Mutex<InternalAuthState>>,
    /// The current session's cancel flag + generation, so `cancel_authorization`
    /// and a fresh `begin_browser_authorization` call can invalidate an
    /// in-flight background thread without needing to inspect `state`
    /// (which the background thread itself is concurrently mutating).
    session_control: Arc<Mutex<Option<(Arc<AtomicBool>, u64)>>>,
    next_generation: Arc<Mutex<u64>>,
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

/// Result of starting a browser authorization: the public `AuthState` plus
/// the trusted authorization URL for the native layer (Tauri command) to
/// open in the system browser. The URL is never derived from anything the
/// frontend supplies.
pub struct BrowserAuthorizationStart {
    pub state: AuthState,
    pub authorization_url: String,
}

impl GitHubProvider {
    pub fn new(
        config: GitHubProviderConfig,
        form_transport: Arc<dyn HttpTransport>,
        rest_transport: Arc<dyn RestTransport>,
        snapshot_transport: Arc<dyn StreamingDownloadTransport>,
        credential_store: Arc<dyn CredentialStore>,
    ) -> Self {
        Self {
            config,
            form_transport,
            rest_transport,
            snapshot_transport,
            credential_store,
            state: Arc::new(Mutex::new(InternalAuthState::Disconnected)),
            session_control: Arc::new(Mutex::new(None)),
            next_generation: Arc::new(Mutex::new(0)),
            now_epoch_seconds: real_now_epoch_seconds,
        }
    }

    #[cfg(test)]
    pub fn with_clock(mut self, now_epoch_seconds: fn() -> u64) -> Self {
        self.now_epoch_seconds = now_epoch_seconds;
        self
    }

    /// The trusted "Install RepoPact on GitHub" / "Configure repository
    /// access on GitHub" URL, built entirely from native configuration
    /// (the app slug) -- never a frontend-supplied URL. `None` if no app
    /// slug is configured (development build).
    pub fn installation_url(&self) -> Option<String> {
        self.config
            .app_slug
            .as_ref()
            .map(|slug| format!("https://github.com/apps/{slug}/installations/new"))
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
            InternalAuthState::StartingBrowserAuthorization => {
                AuthState::StartingBrowserAuthorization
            }
            InternalAuthState::WaitingForCallback { expires_at_epoch } => {
                AuthState::WaitingForCallback {
                    expires_at: format!("epoch:{expires_at_epoch}"),
                }
            }
            InternalAuthState::ExchangingCode => AuthState::ExchangingCode,
            InternalAuthState::Authorized { login, .. } => AuthState::Authorized {
                account_label: login.clone(),
            },
            InternalAuthState::Cancelled => AuthState::Cancelled,
            InternalAuthState::Expired => AuthState::Expired,
            InternalAuthState::Failed(code) => AuthState::Failed { code: *code },
        }
    }

    /// Persists a fresh token pair to the credential store and does so
    /// access-token-first, refresh-token-second: if the process is killed
    /// between the two writes, the worst case is a stored access token
    /// with a stale (or absent) refresh token, which only degrades silent
    /// refresh -- it never leaves a refresh token without a valid access
    /// token, and a subsequent explicit re-authorization always overwrites
    /// both cleanly.
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
    /// expire, atomically replacing the stored pair on success, and
    /// clearing stored credentials and moving to a terminal state on
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

        match browser_flow::refresh_access_token(
            self.form_transport.as_ref(),
            &self.config.client_id,
            &self.config.public_client_secret,
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

    /// Decision 0062: starts a new browser-redirect + PKCE authorization
    /// session. Invalidates any prior in-flight session first (retry never
    /// reuses an old session's state/verifier/challenge/listener). Returns
    /// the trusted authorization URL for the native (Tauri) layer to open
    /// in the system browser -- this method never opens a browser itself,
    /// since this crate has no Tauri/AppHandle dependency; the browser-open
    /// action and this call are both owned by the same native command, so
    /// no separate frontend-facing "open URL" command is needed.
    pub fn start_browser_authorization(&self) -> RemoteProviderResult<BrowserAuthorizationStart> {
        self.invalidate_current_session();

        let listener = CallbackListener::bind().map_err(|error| {
            RemoteProviderError::new(
                ErrorCode::ProviderProtocolError,
                format!("failed to bind local callback listener: {error}"),
            )
        })?;
        let redirect_uri = listener.redirect_uri();
        let state = AuthorizationState::generate();
        let code_verifier = CodeVerifier::generate();
        let authorization_url = browser_flow::build_authorization_url(
            &self.config.client_id,
            &redirect_uri,
            state.as_str(),
            &code_verifier.s256_challenge(),
        );

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let generation = {
            let mut next = self.next_generation.lock().unwrap();
            *next += 1;
            *next
        };
        *self.session_control.lock().unwrap() = Some((cancel_flag.clone(), generation));
        // Momentarily observable via `connection_status()`/polling: the
        // listener/URL are already built above, but the browser has not
        // yet been opened by the native (Tauri) caller, matching Decision
        // 0062's state diagram (`StartingBrowserAuthorization` ->
        // `WaitingForCallback`).
        *self.state.lock().unwrap() = InternalAuthState::StartingBrowserAuthorization;

        let now = (self.now_epoch_seconds)();
        let expires_at_epoch = now + AUTHORIZATION_SESSION_TIMEOUT.as_secs();
        {
            let mut public = self.state.lock().unwrap();
            *public = InternalAuthState::WaitingForCallback { expires_at_epoch };
        }

        let session = BrowserAuthSession {
            state: state.clone(),
            code_verifier,
            redirect_uri: redirect_uri.clone(),
            cancel_flag,
            generation,
        };
        self.spawn_callback_worker(listener, session);

        Ok(BrowserAuthorizationStart {
            state: AuthState::WaitingForCallback {
                expires_at: format!("epoch:{expires_at_epoch}"),
            },
            authorization_url,
        })
    }

    /// Invalidates any currently in-flight session: signals its cancel
    /// flag so a running background thread observes cancellation the next
    /// time it checks (before the listener resolves, or immediately after,
    /// via the generation check), without blocking on that thread here.
    fn invalidate_current_session(&self) {
        if let Some((flag, _generation)) = self.session_control.lock().unwrap().take() {
            flag.store(true, Ordering::SeqCst);
        }
    }

    fn spawn_callback_worker(&self, listener: CallbackListener, session: BrowserAuthSession) {
        let form_transport = self.form_transport.clone();
        let rest_transport = self.rest_transport.clone();
        let credential_store = self.credential_store.clone();
        let state_handle = self.state.clone();
        let session_control = self.session_control.clone();
        let client_id = self.config.client_id.clone();
        let client_secret = self.config.public_client_secret.clone();
        let now_epoch_seconds = self.now_epoch_seconds;

        std::thread::spawn(move || {
            let cancel_flag = session.cancel_flag.clone();
            let generation = session.generation;
            let outcome = listener.wait_for_callback(AUTHORIZATION_SESSION_TIMEOUT, &|| {
                cancel_flag.load(Ordering::SeqCst)
            });

            // A superseded session (a newer `start_browser_authorization`
            // call already replaced this one) must never write its result
            // over the newer session's state, even if this thread finishes
            // its network calls after the newer one started.
            let still_current = matches!(
                &*session_control.lock().unwrap(),
                Some((_, current_generation)) if *current_generation == generation
            );

            let raw_callback = match outcome {
                CallbackWaitOutcome::Cancelled => {
                    if still_current {
                        *state_handle.lock().unwrap() = InternalAuthState::Cancelled;
                        *session_control.lock().unwrap() = None;
                    }
                    return;
                }
                CallbackWaitOutcome::TimedOut => {
                    if still_current {
                        *state_handle.lock().unwrap() = InternalAuthState::Expired;
                        *session_control.lock().unwrap() = None;
                    }
                    return;
                }
                CallbackWaitOutcome::Received(callback) => callback,
            };

            if !still_current {
                // Superseded mid-flight: drop the callback silently rather
                // than committing it to a session that is no longer the
                // active one.
                return;
            }

            // State validation: fail closed on missing, wrong, or
            // GitHub-error callbacks before ever attempting a token
            // exchange. `state` is consumed exactly once here regardless of
            // outcome -- the listener itself is already one-shot (dropped
            // after this point), so a second physical callback on the same
            // port cannot occur, and this session's `generation` is cleared
            // below so even a conceptually "replayed" call into this
            // function again could not re-validate against it.
            let validated = match raw_callback.error {
                Some(_) => Err(ErrorCode::AuthorizationDenied),
                None => match (&raw_callback.code, &raw_callback.state) {
                    (Some(code), Some(returned_state)) if session.state.matches(returned_state) => {
                        Ok(code.clone())
                    }
                    _ => Err(ErrorCode::CallbackRejected),
                },
            };

            let code = match validated {
                Ok(code) => code,
                Err(error_code) => {
                    *state_handle.lock().unwrap() = InternalAuthState::Failed(error_code);
                    *session_control.lock().unwrap() = None;
                    return;
                }
            };

            *state_handle.lock().unwrap() = InternalAuthState::ExchangingCode;

            let exchange = browser_flow::exchange_code_for_token(
                form_transport.as_ref(),
                &client_id,
                &client_secret,
                &code,
                &session.redirect_uri,
                &session.code_verifier,
            );
            // The authorization code and PKCE verifier are not referenced
            // again after this call; `session` (and everything it owns) is
            // dropped at the end of this closure.
            let exchange = match exchange {
                Ok(exchange) => exchange,
                Err(error) => {
                    *state_handle.lock().unwrap() = InternalAuthState::Failed(error.code);
                    *session_control.lock().unwrap() = None;
                    return;
                }
            };

            let user = match rest::get_current_user(rest_transport.as_ref(), &exchange.access_token)
            {
                Ok(user) => user,
                Err(error) => {
                    *state_handle.lock().unwrap() = InternalAuthState::Failed(error.code);
                    *session_control.lock().unwrap() = None;
                    return;
                }
            };

            let now = now_epoch_seconds();
            let tokens = TokenState {
                access_token: exchange.access_token,
                refresh_token: exchange.refresh_token,
                access_token_expires_at_epoch: exchange.expires_in_secs.map(|secs| now + secs),
            };
            if let Err(error) = (|| -> RemoteProviderResult<()> {
                credential_store.put(&access_token_key(), tokens.access_token.clone())?;
                match &tokens.refresh_token {
                    Some(refresh_token) => {
                        credential_store.put(&refresh_token_key(), refresh_token.clone())?
                    }
                    None => credential_store.delete(&refresh_token_key())?,
                }
                Ok(())
            })() {
                *state_handle.lock().unwrap() = InternalAuthState::Failed(error.code);
                *session_control.lock().unwrap() = None;
                return;
            }

            *state_handle.lock().unwrap() = InternalAuthState::Authorized {
                login: user.login,
                user_id: user.id,
                tokens,
            };
            *session_control.lock().unwrap() = None;
        });
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

    /// Trait-level entry point (provider-neutrality contract). Delegates to
    /// [`GitHubProvider::start_browser_authorization`] but discards the
    /// authorization URL, since the trait is provider-neutral and a fake/
    /// non-browser provider may have nothing to open. Production Tauri
    /// commands call the concrete `start_browser_authorization` method
    /// directly (see `remote_provider.rs`) so they can open the browser.
    fn begin_authorization(&self) -> RemoteProviderResult<AuthState> {
        Ok(self.start_browser_authorization()?.state)
    }

    /// Decision 0062: authorization now progresses on a native background
    /// thread (started by `begin_authorization`/`start_browser_authorization`),
    /// not by an active poll performing network calls. This simply reports
    /// whatever state that thread has reached.
    fn poll_authorization(&self) -> RemoteProviderResult<AuthState> {
        Ok(self.connection_status())
    }

    fn cancel_authorization(&self) -> RemoteProviderResult<()> {
        self.invalidate_current_session();
        let mut state = self.state.lock().unwrap();
        if matches!(
            &*state,
            InternalAuthState::StartingBrowserAuthorization
                | InternalAuthState::WaitingForCallback { .. }
                | InternalAuthState::ExchangingCode
        ) {
            *state = InternalAuthState::Cancelled;
        }
        Ok(())
    }

    fn disconnect(&self) -> RemoteProviderResult<()> {
        // Local-only: deletes the locally stored token pair and clears
        // in-process auth state. GitHub App user-token revocation requires
        // an authenticated endpoint call this checkpoint does not
        // implement -- this is "Disconnect" (local), never claimed as
        // "Revoke GitHub authorization" (remote). Already-materialized
        // workspaces and their provenance are untouched -- this method
        // never touches the workspace registry.
        self.invalidate_current_session();
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
        repository: &RemoteRepository,
        revision: &ResolvedRevision,
    ) -> RemoteProviderResult<SnapshotDescriptor> {
        // GitHub's zipball archives always wrap their contents in one
        // synthetic `owner-repo-shortsha/` directory -- verified against
        // real downloaded archives, not assumed (see the live Checkpoint C
        // evidence). WI065's materializer is the only thing that strips it,
        // via `SnapshotLayout::SingleRootDirectory`.
        Ok(SnapshotDescriptor {
            provider: self.provider_id().to_string(),
            provider_repository_id: repository.provider_repository_id.clone(),
            owner_label: repository.owner_label.clone(),
            repository_name: repository.name.clone(),
            revision: revision.clone(),
            layout: SnapshotLayout::SingleRootDirectory,
            reported_size_hint_bytes: None,
        })
    }

    fn open_snapshot(
        &self,
        descriptor: &SnapshotDescriptor,
        options: &SnapshotDownloadOptions,
    ) -> RemoteProviderResult<SnapshotArtifact> {
        // Item 9/Phase 9: the download URL is derived entirely from the
        // already-resolved typed descriptor (owner/name + the immutable
        // commit SHA that produced this exact request) -- never from an
        // archive filename, a Content-Disposition header, or a redirect
        // target. No caller (frontend or otherwise) supplies this URL.
        let url = format!(
            "https://api.github.com/repos/{}/{}/zipball/{}",
            descriptor.owner_label,
            descriptor.repository_name,
            descriptor.revision.immutable_revision_id
        );
        let token = self.optional_access_token();
        let headers = match &token {
            Some(token) => crate::headers::RequestHeaders::with_bearer_token(token),
            None => crate::headers::RequestHeaders::new(),
        }
        .as_pairs()
        .into_iter()
        .map(|(name, value)| (name.to_string(), value))
        .collect();

        let outcome = self
            .snapshot_transport
            .download(
                &StreamingRequest { url, headers },
                options.destination_path,
                options.max_compressed_bytes,
                options.should_cancel,
            )
            .map_err(|error| map_download_error(&error))?;

        Ok(SnapshotArtifact {
            descriptor: descriptor.clone(),
            staged_path: options.destination_path.to_string_lossy().into_owned(),
            received_bytes: outcome.bytes_written,
        })
    }
}

/// WI067 Checkpoint D (GH-010): the download path's typed error mapping,
/// extended to cover 5xx and secondary rate-limiting (a 403 accompanied by
/// a server-supplied `Retry-After`, mirroring `rest.rs`'s own convention
/// for the REST GET path) in addition to Checkpoint C's original
/// cancellation/oversized/401/403/404/429 mapping. `error.message` is
/// already bounded and redacted by `transport::download_inner` before it
/// ever reaches this function.
fn map_download_error(error: &crate::transport::TransportError) -> RemoteProviderError {
    let message = error.message.as_str();
    if message.contains("cancelled") {
        RemoteProviderError::new(
            ErrorCode::DownloadCancelled,
            "snapshot download was cancelled",
        )
    } else if message.contains("compressed bound") {
        RemoteProviderError::new(ErrorCode::SnapshotTooLarge, message.to_string())
    } else if message.contains("status 401") {
        RemoteProviderError::new(
            ErrorCode::CredentialExpired,
            "GitHub rejected the download request (401)",
        )
    } else if message.contains("status 403") && message.contains("retry after") {
        RemoteProviderError::new(
            ErrorCode::ProviderRateLimited,
            "GitHub secondary rate limit on the download request",
        )
    } else if message.contains("status 403") {
        RemoteProviderError::new(
            ErrorCode::ProviderForbidden,
            "GitHub forbade the download request (403)",
        )
    } else if message.contains("status 404") {
        RemoteProviderError::new(
            ErrorCode::ProviderNotFound,
            "GitHub reported the requested snapshot does not exist or is not visible",
        )
    } else if message.contains("status 429") {
        RemoteProviderError::new(
            ErrorCode::ProviderRateLimited,
            "GitHub rate-limited the download request",
        )
    } else if is_server_error_status(message) {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            "GitHub reported a server error for the download request",
        )
    } else if message.contains("empty body")
        || (message.contains("exceeded") && message.contains("redirects"))
    {
        RemoteProviderError::new(ErrorCode::ProviderProtocolError, message.to_string())
    } else {
        // Includes a genuinely truncated transfer (the underlying HTTP
        // stack surfaces a short-body read error with its own message
        // text, not one of the categories matched above) -- still a fail-
        // closed, typed outcome, just a network-layer one rather than a
        // GitHub API response.
        RemoteProviderError::new(ErrorCode::NetworkUnavailable, message.to_string())
    }
}

/// Matches `"...status 5XX..."` for any 5xx code without hardcoding each
/// one individually.
fn is_server_error_status(message: &str) -> bool {
    message.match_indices("status 5").any(|(index, _)| {
        let after = &message[index + "status ".len()..];
        after.len() >= 3
            && after.as_bytes()[0] == b'5'
            && after.as_bytes()[1].is_ascii_digit()
            && after.as_bytes()[2].is_ascii_digit()
    })
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
        json_response, rest_json_response, ScriptedRestTransport, ScriptedStreamingTransport,
        ScriptedTransport,
    };
    use repopact_remote_provider::credential::InMemoryCredentialStore;
    use serde_json::json;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;

    fn config() -> GitHubProviderConfig {
        GitHubProviderConfig {
            client_id: "test-client-id".to_string(),
            public_client_secret: "test-public-secret".to_string(),
            app_slug: Some("repopact".to_string()),
        }
    }

    fn new_provider(
        form: Arc<ScriptedTransport>,
        rest: Arc<ScriptedRestTransport>,
        snapshot: Arc<ScriptedStreamingTransport>,
        credentials: Arc<InMemoryCredentialStore>,
    ) -> GitHubProvider {
        GitHubProvider::new(config(), form, rest, snapshot, credentials)
    }

    /// Sends a real HTTP GET to the loopback callback listener the provider
    /// just bound, mirroring exactly what a real browser redirect does.
    fn deliver_callback(port: u16, query: &str) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let request =
            format!("GET /repopact/github/callback?{query} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        stream.write_all(request.as_bytes()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
    }

    fn authorization_url_port(url: &str) -> u16 {
        let parsed = reqwest::Url::parse(url).unwrap();
        let redirect_uri = parsed
            .query_pairs()
            .find(|(k, _)| k == "redirect_uri")
            .unwrap()
            .1
            .into_owned();
        reqwest::Url::parse(&redirect_uri).unwrap().port().unwrap()
    }

    fn authorization_url_state(url: &str) -> String {
        reqwest::Url::parse(url)
            .unwrap()
            .query_pairs()
            .find(|(k, _)| k == "state")
            .unwrap()
            .1
            .into_owned()
    }

    fn wait_until<F: Fn() -> bool>(condition: F) {
        for _ in 0..200 {
            if condition() {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        panic!("condition was never satisfied within the test timeout");
    }

    #[test]
    fn start_browser_authorization_returns_a_trusted_url_without_a_secret_or_verifier() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider = new_provider(form, rest, snapshot, credentials);

        let start = provider.start_browser_authorization().unwrap();
        assert!(matches!(start.state, AuthState::WaitingForCallback { .. }));
        assert!(start
            .authorization_url
            .starts_with(browser_flow::AUTHORIZE_URL));
        assert!(!start.authorization_url.contains("client_secret"));
        assert!(!start.authorization_url.contains("code_verifier"));
        provider.cancel_authorization().unwrap();
    }

    #[test]
    fn a_valid_callback_completes_authorization_and_persists_tokens() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
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
        let provider = new_provider(form.clone(), rest.clone(), snapshot, credentials.clone());

        let start = provider.start_browser_authorization().unwrap();
        let port = authorization_url_port(&start.authorization_url);
        let state = authorization_url_state(&start.authorization_url);
        deliver_callback(port, &format!("code=real-code&state={state}"));

        wait_until(|| matches!(provider.connection_status(), AuthState::Authorized { .. }));
        match provider.connection_status() {
            AuthState::Authorized { account_label } => assert_eq!(account_label, "octocat"),
            other => panic!("expected Authorized, got {other:?}"),
        }
        assert!(credentials.get(&access_token_key()).unwrap().is_some());
        // No client_secret in the authorization request; the token
        // exchange (a POST, not the browser URL) legitimately does carry
        // one -- confirmed separately in browser_flow's own tests.
        let sent = form.received_requests();
        assert_eq!(sent.len(), 1);
    }

    #[test]
    fn a_callback_with_the_wrong_state_is_rejected_and_stores_no_credential() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider = new_provider(form.clone(), rest, snapshot, credentials.clone());

        let start = provider.start_browser_authorization().unwrap();
        let port = authorization_url_port(&start.authorization_url);
        deliver_callback(port, "code=real-code&state=totally-wrong-state");

        wait_until(|| {
            !matches!(
                provider.connection_status(),
                AuthState::WaitingForCallback { .. }
            )
        });
        assert!(matches!(
            provider.connection_status(),
            AuthState::Failed {
                code: ErrorCode::CallbackRejected
            }
        ));
        assert!(credentials.get(&access_token_key()).unwrap().is_none());
        assert!(
            form.received_requests().is_empty(),
            "no token exchange must be attempted"
        );
    }

    #[test]
    fn a_callback_missing_state_is_rejected() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider = new_provider(form.clone(), rest, snapshot, credentials.clone());

        let start = provider.start_browser_authorization().unwrap();
        let port = authorization_url_port(&start.authorization_url);
        deliver_callback(port, "code=real-code");

        wait_until(|| {
            !matches!(
                provider.connection_status(),
                AuthState::WaitingForCallback { .. }
            )
        });
        assert!(matches!(
            provider.connection_status(),
            AuthState::Failed {
                code: ErrorCode::CallbackRejected
            }
        ));
        assert!(form.received_requests().is_empty());
    }

    #[test]
    fn a_callback_missing_code_is_rejected() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider = new_provider(form.clone(), rest, snapshot, credentials.clone());

        let start = provider.start_browser_authorization().unwrap();
        let port = authorization_url_port(&start.authorization_url);
        let state = authorization_url_state(&start.authorization_url);
        deliver_callback(port, &format!("state={state}"));

        wait_until(|| {
            !matches!(
                provider.connection_status(),
                AuthState::WaitingForCallback { .. }
            )
        });
        assert!(matches!(
            provider.connection_status(),
            AuthState::Failed {
                code: ErrorCode::CallbackRejected
            }
        ));
    }

    #[test]
    fn a_github_error_callback_is_reported_as_authorization_denied() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider = new_provider(form.clone(), rest, snapshot, credentials.clone());

        let start = provider.start_browser_authorization().unwrap();
        let port = authorization_url_port(&start.authorization_url);
        let state = authorization_url_state(&start.authorization_url);
        deliver_callback(
            port,
            &format!("error=access_denied&error_description=denied&state={state}"),
        );

        wait_until(|| {
            !matches!(
                provider.connection_status(),
                AuthState::WaitingForCallback { .. }
            )
        });
        assert!(matches!(
            provider.connection_status(),
            AuthState::Failed {
                code: ErrorCode::AuthorizationDenied
            }
        ));
        assert!(form.received_requests().is_empty());
    }

    #[test]
    fn cancel_stops_the_session_and_a_later_callback_is_ignored() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider = new_provider(form.clone(), rest, snapshot, credentials.clone());

        let start = provider.start_browser_authorization().unwrap();
        let port = authorization_url_port(&start.authorization_url);
        let state = authorization_url_state(&start.authorization_url);
        provider.cancel_authorization().unwrap();
        wait_until(|| matches!(provider.connection_status(), AuthState::Cancelled));

        // A callback arriving after cancellation must not resurrect the
        // session or store a credential -- the listener may still be
        // reachable briefly during teardown, but the state machine must
        // fail closed regardless of whether the socket accepts it.
        let _ = std::panic::catch_unwind(|| {
            deliver_callback(port, &format!("code=late-code&state={state}"))
        });
        std::thread::sleep(Duration::from_millis(200));
        assert!(matches!(provider.connection_status(), AuthState::Cancelled));
        assert!(credentials.get(&access_token_key()).unwrap().is_none());
        assert!(form.received_requests().is_empty());
    }

    #[test]
    fn retrying_never_reuses_the_previous_sessions_state_or_verifier() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider = new_provider(form.clone(), rest, snapshot, credentials.clone());

        let first = provider.start_browser_authorization().unwrap();
        let first_state = authorization_url_state(&first.authorization_url);
        let second = provider.start_browser_authorization().unwrap();
        let second_state = authorization_url_state(&second.authorization_url);
        assert_ne!(first_state, second_state);

        // The first session's port is no longer the active one; a callback
        // using the *old* state against the *new* session's port must be
        // rejected as a wrong/replayed state.
        let second_port = authorization_url_port(&second.authorization_url);
        deliver_callback(second_port, &format!("code=x&state={first_state}"));
        wait_until(|| {
            !matches!(
                provider.connection_status(),
                AuthState::WaitingForCallback { .. }
            )
        });
        assert!(matches!(
            provider.connection_status(),
            AuthState::Failed {
                code: ErrorCode::CallbackRejected
            }
        ));
        provider.cancel_authorization().unwrap();
    }

    #[test]
    fn a_second_callback_on_the_same_session_is_not_accepted() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        form.push_response(Ok(json_response(
            200,
            &[("access_token", "ghu_test0000000000000000000000000000")],
        )));
        rest.push_response(Ok(rest_json_response(200, json!({"id": 1, "login": "x"}))));
        let provider = new_provider(form.clone(), rest.clone(), snapshot, credentials.clone());

        let start = provider.start_browser_authorization().unwrap();
        let port = authorization_url_port(&start.authorization_url);
        let state = authorization_url_state(&start.authorization_url);
        deliver_callback(port, &format!("code=first&state={state}"));
        wait_until(|| matches!(provider.connection_status(), AuthState::Authorized { .. }));

        // The listener is one-shot -- a second connection attempt to the
        // same (now-closed) port must not be accepted as a new terminal
        // callback and must not trigger a second token exchange.
        let _ = std::panic::catch_unwind(|| {
            deliver_callback(port, &format!("code=second&state={state}"))
        });
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(form.received_requests().len(), 1);
    }

    #[test]
    fn a_listener_timeout_with_no_callback_expires_the_session() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        // A GitHubProvider whose internal auth-session timeout is exercised
        // indirectly here would take AUTHORIZATION_SESSION_TIMEOUT (10
        // minutes) to fire in a real test; that constant is proven directly
        // by `callback::tests::a_timeout_with_no_request_reports_timed_out`
        // against a short duration instead. This test proves only that
        // cancellation -- the mechanism `Expired` shares -- correctly
        // reaches a session with no callback ever delivered.
        let provider = new_provider(form, rest, snapshot, credentials);
        let _start = provider.start_browser_authorization().unwrap();
        provider.cancel_authorization().unwrap();
        wait_until(|| matches!(provider.connection_status(), AuthState::Cancelled));
    }

    #[test]
    fn disconnect_clears_stored_credentials_and_state() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        form.push_response(Ok(json_response(
            200,
            &[("access_token", "ghu_test0000000000000000000000000000")],
        )));
        rest.push_response(Ok(rest_json_response(200, json!({"id": 1, "login": "x"}))));
        let provider = new_provider(form.clone(), rest.clone(), snapshot, credentials.clone());

        let start = provider.start_browser_authorization().unwrap();
        let port = authorization_url_port(&start.authorization_url);
        let state = authorization_url_state(&start.authorization_url);
        deliver_callback(port, &format!("code=x&state={state}"));
        wait_until(|| matches!(provider.connection_status(), AuthState::Authorized { .. }));
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
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        credentials
            .put(
                &access_token_key(),
                Secret::new("ghu_test0000000000000000000000000000"),
            )
            .unwrap();

        let provider = new_provider(form.clone(), rest, snapshot, credentials);
        let state = provider
            .restore_from_credential_store("octocat".to_string(), 42, Some(9_999_999_999))
            .unwrap();
        assert!(matches!(state, AuthState::Authorized { .. }));
        assert!(form.received_requests().is_empty());
    }

    #[test]
    fn restore_from_credential_store_with_nothing_stored_is_disconnected_not_an_error() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider = new_provider(form, rest, snapshot, credentials);
        let state = provider
            .restore_from_credential_store("octocat".to_string(), 42, None)
            .unwrap();
        assert!(matches!(state, AuthState::Disconnected));
    }

    #[test]
    fn an_expired_access_token_is_refreshed_atomically_before_the_next_call() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        credentials
            .put(&access_token_key(), Secret::new("old-access"))
            .unwrap();
        credentials
            .put(&refresh_token_key(), Secret::new("old-refresh"))
            .unwrap();

        form.push_response(Ok(json_response(
            200,
            &[
                ("access_token", "new-access"),
                ("refresh_token", "new-refresh"),
                ("expires_in", "28800"),
            ],
        )));
        rest.push_response(Ok(rest_json_response(200, json!({"installations": []}))));

        let provider = GitHubProvider::new(
            config(),
            form.clone(),
            rest.clone(),
            snapshot.clone(),
            credentials.clone(),
        )
        .with_clock(|| 1_000_000_000);
        provider
            .restore_from_credential_store("octocat".to_string(), 42, Some(1_000_000_000 - 10))
            .unwrap();

        let accounts = provider.list_accounts().unwrap();
        assert!(accounts.is_empty());

        // The refresh request now legitimately carries client_secret
        // (Decision 0062: refresh is no longer device-flow-shaped).
        assert!(form.received_requests()[0]
            .fields
            .iter()
            .any(|(k, v)| k == "client_secret" && v == "test-public-secret"));
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
    }

    #[test]
    fn a_denied_refresh_clears_credentials_and_reports_expired() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        credentials
            .put(&access_token_key(), Secret::new("old-access"))
            .unwrap();
        credentials
            .put(&refresh_token_key(), Secret::new("revoked-refresh"))
            .unwrap();

        form.push_response(Ok(json_response(200, &[("error", "access_denied")])));

        let provider = GitHubProvider::new(
            config(),
            form.clone(),
            rest.clone(),
            snapshot.clone(),
            credentials.clone(),
        )
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
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        credentials
            .put(&access_token_key(), Secret::new("old-access"))
            .unwrap();

        let provider = GitHubProvider::new(
            config(),
            form.clone(),
            rest.clone(),
            snapshot.clone(),
            credentials.clone(),
        )
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
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
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
        let provider = new_provider(form.clone(), rest.clone(), snapshot, credentials.clone());
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
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
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
        let provider = new_provider(form.clone(), rest.clone(), snapshot, credentials.clone());
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
    fn installation_url_is_built_from_the_configured_app_slug_only() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider = new_provider(form, rest, snapshot, credentials);
        assert_eq!(
            provider.installation_url(),
            Some("https://github.com/apps/repopact/installations/new".to_string())
        );
    }

    #[test]
    fn installation_url_is_none_without_a_configured_app_slug() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let provider = GitHubProvider::new(
            GitHubProviderConfig {
                client_id: "id".into(),
                public_client_secret: "secret".into(),
                app_slug: None,
            },
            form,
            rest,
            snapshot,
            credentials,
        );
        assert!(provider.installation_url().is_none());
    }

    fn snapshot_descriptor() -> SnapshotDescriptor {
        SnapshotDescriptor {
            provider: "github".into(),
            provider_repository_id: "1".into(),
            owner_label: "octocat".into(),
            repository_name: "Hello-World".into(),
            revision: ResolvedRevision {
                selected_ref: RemoteRef {
                    display_name: "master".into(),
                    kind: RefKind::Branch,
                    provider_ref_id: "heads/master".into(),
                },
                immutable_revision_id: "a".repeat(40),
            },
            layout: repopact_remote_provider::snapshot::SnapshotLayout::SingleRootDirectory,
            reported_size_hint_bytes: None,
        }
    }

    #[test]
    fn open_snapshot_builds_the_zipball_url_from_typed_descriptor_fields_only() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        snapshot.push_response(Ok(crate::transport::StreamingDownloadOutcome {
            bytes_written: 42,
        }));
        let provider = new_provider(form, rest, snapshot.clone(), credentials);

        let destination =
            std::env::temp_dir().join(format!("repopact-test-{}", std::process::id()));
        let cancelled = false;
        let artifact = provider
            .open_snapshot(
                &snapshot_descriptor(),
                &SnapshotDownloadOptions {
                    destination_path: &destination,
                    max_compressed_bytes: 1024,
                    should_cancel: &|| cancelled,
                },
            )
            .unwrap();
        assert_eq!(artifact.received_bytes, 42);

        let sent = snapshot.received_requests();
        assert_eq!(sent.len(), 1);
        assert_eq!(
            sent[0].url,
            format!(
                "https://api.github.com/repos/octocat/Hello-World/zipball/{}",
                "a".repeat(40)
            )
        );
        assert!(sent[0].headers.iter().all(|(k, _)| k != "Authorization"));
    }

    #[test]
    fn open_snapshot_attaches_authorization_only_when_connected() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        credentials
            .put(&access_token_key(), Secret::new("ghu_test_token"))
            .unwrap();
        snapshot.push_response(Ok(crate::transport::StreamingDownloadOutcome {
            bytes_written: 1,
        }));
        let provider = new_provider(form, rest, snapshot.clone(), credentials);
        provider
            .restore_from_credential_store("octocat".into(), 1, None)
            .unwrap();

        let destination =
            std::env::temp_dir().join(format!("repopact-test-auth-{}", std::process::id()));
        let cancelled = false;
        provider
            .open_snapshot(
                &snapshot_descriptor(),
                &SnapshotDownloadOptions {
                    destination_path: &destination,
                    max_compressed_bytes: 1024,
                    should_cancel: &|| cancelled,
                },
            )
            .unwrap();

        let sent = snapshot.received_requests();
        assert!(sent[0]
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v.contains("ghu_test_token")));
    }

    #[test]
    fn open_snapshot_maps_download_errors_to_typed_codes() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        snapshot.push_response(Err(crate::transport::TransportError {
            message: "download exceeded the 10-byte compressed bound".into(),
        }));
        let provider = new_provider(form, rest, snapshot.clone(), credentials);
        let destination =
            std::env::temp_dir().join(format!("repopact-test-oversized-{}", std::process::id()));
        let cancelled = false;
        let error = provider
            .open_snapshot(
                &snapshot_descriptor(),
                &SnapshotDownloadOptions {
                    destination_path: &destination,
                    max_compressed_bytes: 10,
                    should_cancel: &|| cancelled,
                },
            )
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::SnapshotTooLarge);
    }

    #[test]
    fn open_snapshot_maps_cancellation() {
        let form = Arc::new(ScriptedTransport::new());
        let rest = Arc::new(ScriptedRestTransport::new());
        let snapshot: Arc<ScriptedStreamingTransport> = Arc::new(ScriptedStreamingTransport::new());
        let credentials = Arc::new(InMemoryCredentialStore::new());
        snapshot.push_response(Err(crate::transport::TransportError {
            message: "cancelled mid-download".into(),
        }));
        let provider = new_provider(form, rest, snapshot.clone(), credentials);
        let destination =
            std::env::temp_dir().join(format!("repopact-test-cancel-{}", std::process::id()));
        let cancelled = true;
        let error = provider
            .open_snapshot(
                &snapshot_descriptor(),
                &SnapshotDownloadOptions {
                    destination_path: &destination,
                    max_compressed_bytes: 1024,
                    should_cancel: &|| cancelled,
                },
            )
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::DownloadCancelled);
    }

    // WI067 Checkpoint C, Phase 10/21: the live, end-to-end public-
    // repository proof. Not run by default (`cargo test` skips `#[ignore]`)
    // to avoid spending unauthenticated GitHub rate-limit budget on every
    // routine test run; run explicitly with `--ignored` for checkpoint
    // evidence. Requires no client ID, no browser flow, no operator gate --
    // proves the whole snapshot pipeline (resolve -> describe -> download
    // -> WI065's real WorkspaceManager::import_remote_snapshot -> publish
    // -> open) against real GitHub data, entirely unauthenticated.
    #[test]
    #[ignore]
    fn live_downloads_and_publishes_a_real_public_snapshot_end_to_end() {
        use repopact_mobile_acquisition::bounds::ArchiveBounds;
        use repopact_mobile_acquisition::operation::CancellationToken;
        use repopact_mobile_acquisition::registry::RemoteSnapshotProvenance;
        use repopact_mobile_acquisition::workspace::WorkspaceManager;

        let transport = Arc::new(crate::transport::ReqwestTransport::new().unwrap());
        let http = transport.clone() as Arc<dyn HttpTransport>;
        let rest = transport.clone() as Arc<dyn RestTransport>;
        let snapshot_transport = transport as Arc<dyn StreamingDownloadTransport>;
        let credentials =
            Arc::new(repopact_remote_provider::credential::InMemoryCredentialStore::new());
        let provider = GitHubProvider::new(
            GitHubProviderConfig {
                client_id: String::new(),
                public_client_secret: String::new(),
                app_slug: None,
            },
            http,
            rest,
            snapshot_transport,
            credentials,
        );

        let repository = RemoteRepository {
            provider: "github".into(),
            provider_repository_id: "1".into(),
            owner_label: "octocat".into(),
            name: "Hello-World".into(),
            full_display_name: "octocat/Hello-World".into(),
            visibility: RepositoryVisibility::Public,
            default_branch: Some("master".into()),
        };
        let reference = RemoteRef {
            display_name: "master".into(),
            kind: RefKind::Branch,
            provider_ref_id: "heads/master".into(),
        };
        let resolved = provider.resolve_ref(&repository, &reference).unwrap();
        assert_eq!(resolved.immutable_revision_id.len(), 40);

        let descriptor = provider.describe_snapshot(&repository, &resolved).unwrap();

        let temp = tempfile::tempdir().unwrap();
        let download_path = temp.path().join("snapshot.zip");
        let cancelled = false;
        let artifact = provider
            .open_snapshot(
                &descriptor,
                &SnapshotDownloadOptions {
                    destination_path: &download_path,
                    max_compressed_bytes: 50 * 1024 * 1024,
                    should_cancel: &|| cancelled,
                },
            )
            .unwrap();
        assert!(artifact.received_bytes > 0);

        let manager = WorkspaceManager::open(temp.path().join("workspace-root")).unwrap();
        let staged = std::fs::File::open(&artifact.staged_path).unwrap();
        let provenance = RemoteSnapshotProvenance {
            provider: "github".into(),
            provider_repository_id: repository.provider_repository_id.clone(),
            owner_label: repository.owner_label.clone(),
            repository_name: repository.name.clone(),
            selected_ref: resolved.selected_ref.display_name.clone(),
            ref_kind: "branch".into(),
            resolved_commit_sha: resolved.immutable_revision_id.clone(),
            acquired_at: "2026-09-16T00:00:00Z".into(),
            snapshot_semantics: "immutable_snapshot".into(),
        };
        let cancel = CancellationToken::new();
        let record = manager
            .import_remote_snapshot(
                staged,
                "octocat/Hello-World".into(),
                format!(
                    "github:octocat/Hello-World@{}",
                    resolved.immutable_revision_id
                ),
                &ArchiveBounds::default(),
                &cancel,
                |_| {},
                provenance,
                true, // GitHub zipballs always wrap in a synthetic owner-repo-sha/ root.
            )
            .unwrap();

        assert_eq!(
            record.acquisition_kind,
            repopact_mobile_acquisition::registry::AcquisitionKind::RemoteSnapshot
        );
        let stored_provenance = record.remote_snapshot_provenance.clone().unwrap();
        assert_eq!(
            stored_provenance.resolved_commit_sha,
            resolved.immutable_revision_id
        );

        let published = manager.repository_path(&record.workspace_id).unwrap();
        let entries: Vec<_> = std::fs::read_dir(&published).unwrap().collect();
        assert!(
            !entries.is_empty(),
            "the published workspace must contain real repository files"
        );
        assert!(!entries.iter().any(|entry| entry
            .as_ref()
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("octocat-Hello-World-")));

        let reopened_manager = WorkspaceManager::open(temp.path().join("workspace-root")).unwrap();
        let reopened_record = reopened_manager
            .get_workspace(&record.workspace_id)
            .unwrap();
        assert_eq!(reopened_record.workspace_id, record.workspace_id);
        let reopened_path = reopened_manager
            .repository_path(&record.workspace_id)
            .unwrap();
        assert!(std::fs::read_dir(&reopened_path).unwrap().next().is_some());
    }
}
