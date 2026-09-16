//! WI067 Checkpoint B, item 15: the native service/state owner for remote
//! repository providers. Owns the concrete GitHub adapter, the real
//! credential store, and the real HTTP transport; the frontend never
//! instantiates a provider, passes a token, or supplies a configurable
//! URL. Tauri commands in this module are the *only* typed surface the
//! frontend gets (item 32/33) -- no `github_request`/`provider_request`/
//! `authenticated_fetch` exists anywhere.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use repopact_provider_github::provider::{GitHubProvider, GitHubProviderConfig};
use repopact_provider_github::transport::ReqwestTransport;
use repopact_remote_provider::account::RemoteAccount;
use repopact_remote_provider::auth::AuthState;
use repopact_remote_provider::credential::CredentialStore;
use repopact_remote_provider::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
use repopact_remote_provider::provider::RemoteRepositoryProvider;
use repopact_remote_provider::refs::{RefKind, RemoteRef, ResolvedRevision};
use repopact_remote_provider::repository::RemoteRepository;
use serde::{Deserialize, Serialize};
use tauri::State;

/// The GitHub App client ID is public configuration (Decision 0061, item
/// 16), never a secret -- but it is still native-owned configuration, not
/// something a frontend-supplied value may set. Read once at startup from
/// an environment variable the operator sets after registering a real
/// GitHub App per `docs/guides/github-app-setup.md`; never fabricated.
const CLIENT_ID_ENV_VAR: &str = "REPOPACT_GITHUB_CLIENT_ID";

/// Non-secret connection identity/expiry metadata (item 10). Only tokens
/// go into the OS-protected `CredentialStore`; this small sidecar file is
/// ordinary native app state, exactly like WI065's `local-metadata/`
/// bookkeeping files, and never contains a credential.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ConnectionMetadata {
    login: String,
    user_id: u64,
    access_token_expires_at_epoch: Option<u64>,
}

fn read_connection_metadata(path: &Path) -> Option<ConnectionMetadata> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_connection_metadata(path: &Path, metadata: &ConnectionMetadata) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, serde_json::to_vec_pretty(metadata)?)?;
    fs::rename(&temp_path, path)
}

fn delete_connection_metadata(path: &Path) {
    let _ = fs::remove_file(path);
}

pub struct RemoteProviderService {
    github: GitHubProvider,
    metadata_path: PathBuf,
    client_configured: bool,
}

impl RemoteProviderService {
    /// `app_data_dir` is the real, app-private data directory (never
    /// WI060's debug validation root, matching the mobile acquisition
    /// coordinator's own convention). Restores a prior session from the
    /// real OS credential store + this file's metadata synchronously, so
    /// the Workbench opens already-connected if a valid prior connection
    /// exists (item 41).
    pub fn open(app_data_dir: PathBuf) -> RemoteProviderResult<Self> {
        // An unset OR empty-string environment variable both count as
        // "not configured" -- a real bug caught during Checkpoint B's own
        // manual verification run, where `VAR=` (set but empty) was
        // silently treated as present.
        let client_id = std::env::var(CLIENT_ID_ENV_VAR)
            .ok()
            .filter(|value| !value.is_empty());
        let client_configured = client_id.is_some();
        let config = GitHubProviderConfig {
            client_id: client_id.unwrap_or_default(),
        };
        let transport = Arc::new(ReqwestTransport::new().map_err(|error| {
            RemoteProviderError::new(ErrorCode::NetworkUnavailable, error.message)
        })?);
        let http_transport =
            transport.clone() as Arc<dyn repopact_provider_github::transport::HttpTransport>;
        let rest_transport =
            transport as Arc<dyn repopact_provider_github::transport::RestTransport>;
        let credential_store: Arc<dyn CredentialStore> =
            Arc::new(repopact_remote_provider::credential_os::OsCredentialStore::new());
        let github = GitHubProvider::new(config, http_transport, rest_transport, credential_store);

        let metadata_path = app_data_dir
            .join("local-metadata")
            .join("remote-connections.json");
        if let Some(metadata) = read_connection_metadata(&metadata_path) {
            // Restoring is best-effort: a corrupt/unreadable metadata file
            // is treated as "no prior connection" rather than a startup
            // failure -- the next real operation will surface a typed
            // NotConnected/expired error if the underlying credential is
            // also actually missing.
            let _ = github.restore_from_credential_store(
                metadata.login,
                metadata.user_id,
                metadata.access_token_expires_at_epoch,
            );
        }

        Ok(Self {
            github,
            metadata_path,
            client_configured,
        })
    }

    fn require_client_configured(&self) -> RemoteProviderResult<()> {
        if self.client_configured {
            Ok(())
        } else {
            Err(RemoteProviderError::new(
                ErrorCode::ProviderNotConfigured,
                "no GitHub App client ID is configured; see docs/guides/github-app-setup.md",
            ))
        }
    }

    fn persist_metadata_from_current_state(&self) {
        // Re-derive the non-secret metadata to persist from the provider's
        // own public AuthState rather than threading login/user_id/expiry
        // through every call site; login is the only field AuthState
        // exposes today, so a best-effort record is written with the
        // fields available. (user_id/expiry already live correctly inside
        // GitHubProvider's own internal state and the OS credential store;
        // this file only needs enough to call `restore_from_credential_store`
        // meaningfully on next launch.)
        if let AuthState::Authorized { account_label } = self.github.connection_status() {
            let metadata = ConnectionMetadata {
                login: account_label,
                user_id: 0,
                access_token_expires_at_epoch: None,
            };
            let _ = write_connection_metadata(&self.metadata_path, &metadata);
        }
    }
}

// ---- Frontend-facing DTOs: bounded, no secret fields ever. ----

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilitiesDto {
    pub provider: &'static str,
    pub configured: bool,
    pub supports_public_without_auth: bool,
    pub supports_private_repositories: bool,
    pub supports_organizations: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ConnectionStatusDto {
    Disconnected,
    AwaitingUser {
        user_code: String,
        verification_uri: String,
        expires_at: String,
    },
    Connected {
        login: String,
    },
    Cancelled,
    Failed {
        code: ErrorCode,
    },
}

impl From<AuthState> for ConnectionStatusDto {
    fn from(state: AuthState) -> Self {
        match state {
            AuthState::Disconnected => ConnectionStatusDto::Disconnected,
            AuthState::RequestingAuthorization => ConnectionStatusDto::AwaitingUser {
                user_code: String::new(),
                verification_uri: String::new(),
                expires_at: String::new(),
            },
            AuthState::AwaitingUser {
                user_code,
                verification_uri,
                expires_at,
            } => ConnectionStatusDto::AwaitingUser {
                user_code,
                verification_uri,
                expires_at,
            },
            AuthState::Authorized { account_label } => ConnectionStatusDto::Connected {
                login: account_label,
            },
            AuthState::Refreshing => ConnectionStatusDto::AwaitingUser {
                user_code: String::new(),
                verification_uri: String::new(),
                expires_at: String::new(),
            },
            AuthState::Expired => ConnectionStatusDto::Failed {
                code: ErrorCode::AuthorizationExpired,
            },
            AuthState::Revoked => ConnectionStatusDto::Failed {
                code: ErrorCode::AuthorizationDenied,
            },
            AuthState::Cancelled => ConnectionStatusDto::Cancelled,
            AuthState::Failed { code } => ConnectionStatusDto::Failed { code },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAccountDto {
    pub connection_id: String,
    pub label: String,
    pub scope_label: String,
    pub includes_private_repositories: bool,
}

impl From<RemoteAccount> for RemoteAccountDto {
    fn from(account: RemoteAccount) -> Self {
        Self {
            connection_id: account.id.provider_account_id,
            label: account.display_label,
            scope_label: account.scope.label,
            includes_private_repositories: account.scope.includes_private_repositories,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRepositoryDto {
    pub repository_id: String,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub default_branch: Option<String>,
}

impl From<RemoteRepository> for RemoteRepositoryDto {
    fn from(repo: RemoteRepository) -> Self {
        Self {
            repository_id: repo.provider_repository_id,
            owner: repo.owner_label,
            name: repo.name,
            full_name: repo.full_display_name,
            private: matches!(
                repo.visibility,
                repopact_remote_provider::repository::RepositoryVisibility::Private
            ),
            default_branch: repo.default_branch,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteRefKindDto {
    Branch,
    Tag,
    Commit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRefDto {
    pub display_name: String,
    pub kind: RemoteRefKindDto,
    pub ref_id: String,
}

impl From<RemoteRef> for RemoteRefDto {
    fn from(reference: RemoteRef) -> Self {
        Self {
            display_name: reference.display_name,
            kind: match reference.kind {
                RefKind::Branch => RemoteRefKindDto::Branch,
                RefKind::Tag => RemoteRefKindDto::Tag,
                RefKind::Commit => RemoteRefKindDto::Commit,
            },
            ref_id: reference.provider_ref_id,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRevisionDto {
    pub selected_ref_display_name: String,
    pub resolved_commit_sha: String,
}

impl From<ResolvedRevision> for ResolvedRevisionDto {
    fn from(revision: ResolvedRevision) -> Self {
        Self {
            selected_ref_display_name: revision.selected_ref.display_name,
            resolved_commit_sha: revision.immutable_revision_id,
        }
    }
}

fn repository_from_dto(dto: &RemoteRepositoryRefDto) -> RemoteRepository {
    RemoteRepository {
        provider: "github".to_string(),
        provider_repository_id: dto.repository_id.clone(),
        owner_label: dto.owner.clone(),
        name: dto.name.clone(),
        full_display_name: format!("{}/{}", dto.owner, dto.name),
        visibility: repopact_remote_provider::repository::RepositoryVisibility::Public,
        default_branch: None,
    }
}

/// The minimal repository identity a browse/ref-resolution command needs
/// from the frontend -- never a raw provider URL, never credential
/// material.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRepositoryRefDto {
    pub repository_id: String,
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRefRefDto {
    pub display_name: String,
    pub kind: RemoteRefKindDto,
    pub ref_id: String,
}

// ---- Typed Tauri commands (item 32). No command returns token material;
// no command accepts an arbitrary method/URL/path. ----

#[tauri::command]
pub fn remote_provider_capabilities(
    service: State<'_, Arc<RemoteProviderService>>,
) -> ProviderCapabilitiesDto {
    let capabilities = service.github.capabilities();
    ProviderCapabilitiesDto {
        provider: "github",
        configured: service.client_configured,
        supports_public_without_auth: capabilities.supports_public_without_auth,
        supports_private_repositories: capabilities.supports_private_repositories,
        supports_organizations: capabilities.supports_organizations,
    }
}

#[tauri::command]
pub fn remote_connect_start(
    service: State<'_, Arc<RemoteProviderService>>,
) -> Result<ConnectionStatusDto, RemoteProviderError> {
    service.require_client_configured()?;
    Ok(service.github.begin_authorization()?.into())
}

#[tauri::command]
pub fn remote_connect_status(
    service: State<'_, Arc<RemoteProviderService>>,
) -> Result<ConnectionStatusDto, RemoteProviderError> {
    let status = service.github.poll_authorization()?;
    if matches!(status, AuthState::Authorized { .. }) {
        service.persist_metadata_from_current_state();
    }
    Ok(status.into())
}

#[tauri::command]
pub fn remote_connect_cancel(
    service: State<'_, Arc<RemoteProviderService>>,
) -> Result<(), RemoteProviderError> {
    service.github.cancel_authorization()
}

#[tauri::command]
pub fn remote_disconnect(
    service: State<'_, Arc<RemoteProviderService>>,
) -> Result<(), RemoteProviderError> {
    service.github.disconnect()?;
    delete_connection_metadata(&service.metadata_path);
    Ok(())
}

#[tauri::command]
pub fn remote_connections(service: State<'_, Arc<RemoteProviderService>>) -> ConnectionStatusDto {
    service.github.connection_status().into()
}

#[tauri::command]
pub fn remote_accounts(
    service: State<'_, Arc<RemoteProviderService>>,
) -> Result<Vec<RemoteAccountDto>, RemoteProviderError> {
    Ok(service
        .github
        .list_accounts()?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub fn remote_repositories(
    connection_id: String,
    service: State<'_, Arc<RemoteProviderService>>,
) -> Result<Vec<RemoteRepositoryDto>, RemoteProviderError> {
    let account = RemoteAccount {
        id: repopact_remote_provider::account::RemoteAccountId {
            provider: "github".to_string(),
            provider_account_id: connection_id,
        },
        display_label: String::new(),
        scope: repopact_remote_provider::account::ProviderScope {
            label: String::new(),
            includes_private_repositories: true,
        },
    };
    Ok(service
        .github
        .list_repositories(&account)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub fn remote_repository_refs(
    repository: RemoteRepositoryRefDto,
    service: State<'_, Arc<RemoteProviderService>>,
) -> Result<Vec<RemoteRefDto>, RemoteProviderError> {
    let repo = repository_from_dto(&repository);
    Ok(service
        .github
        .list_refs(&repo)?
        .into_iter()
        .map(Into::into)
        .collect())
}

#[tauri::command]
pub fn remote_resolve_ref(
    repository: RemoteRepositoryRefDto,
    reference: RemoteRefRefDto,
    service: State<'_, Arc<RemoteProviderService>>,
) -> Result<ResolvedRevisionDto, RemoteProviderError> {
    let repo = repository_from_dto(&repository);
    let remote_ref = RemoteRef {
        display_name: reference.display_name,
        kind: match reference.kind {
            RemoteRefKindDto::Branch => RefKind::Branch,
            RemoteRefKindDto::Tag => RefKind::Tag,
            RemoteRefKindDto::Commit => RefKind::Commit,
        },
        provider_ref_id: reference.ref_id,
    };
    Ok(service.github.resolve_ref(&repo, &remote_ref)?.into())
}

/// WI067 item 18/19: opens the system browser at the trusted GitHub
/// device-flow verification URL -- never an arbitrary frontend-supplied
/// URL. Reads the URL from the provider's own current `AwaitingUser`
/// state (already validated against `redirect_policy::
/// is_trusted_verification_uri` by `device_flow::start_device_flow` at
/// the moment it was received), so there is nothing for the frontend to
/// tamper with even if it tried.
#[tauri::command]
pub fn remote_open_verification_url(
    app: tauri::AppHandle,
    service: State<'_, Arc<RemoteProviderService>>,
) -> Result<(), RemoteProviderError> {
    let AuthState::AwaitingUser {
        verification_uri, ..
    } = service.github.connection_status()
    else {
        return Err(RemoteProviderError::new(
            ErrorCode::NotConnected,
            "no pending GitHub authorization to open a verification URL for",
        ));
    };
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(verification_uri, None::<&str>)
        .map_err(|error| {
            RemoteProviderError::new(
                ErrorCode::ProviderProtocolError,
                format!("failed to open system browser: {error}"),
            )
        })
}
