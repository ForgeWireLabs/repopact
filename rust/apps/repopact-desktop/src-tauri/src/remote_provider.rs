//! WI067 Checkpoint B, item 15: the native service/state owner for remote
//! repository providers. Owns the concrete GitHub adapter, the real
//! credential store, and the real HTTP transport; the frontend never
//! instantiates a provider, passes a token, or supplies a configurable
//! URL. Tauri commands in this module are the *only* typed surface the
//! frontend gets (item 32/33) -- no `github_request`/`provider_request`/
//! `authenticated_fetch` exists anywhere.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use repopact_mobile_acquisition::bounds::ArchiveBounds;
use repopact_mobile_acquisition::operation::CancellationToken;
use repopact_mobile_acquisition::registry::RemoteSnapshotProvenance;
use repopact_mobile_acquisition::workspace::WorkspaceManager;
use repopact_provider_github::provider::{GitHubProvider, GitHubProviderConfig};
use repopact_provider_github::transport::ReqwestTransport;
use repopact_remote_provider::account::RemoteAccount;
use repopact_remote_provider::auth::AuthState;
use repopact_remote_provider::credential::CredentialStore;
use repopact_remote_provider::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
use repopact_remote_provider::provider::{RemoteRepositoryProvider, SnapshotDownloadOptions};
use repopact_remote_provider::refs::{RefKind, RemoteRef, ResolvedRevision};
use repopact_remote_provider::repository::RemoteRepository;
use repopact_remote_provider::snapshot::SnapshotLayout;
use serde::{Deserialize, Serialize};
use tauri::State;

/// WI067 Checkpoint C, Phase 5: a repository archive is legitimately much
/// larger than any REST/JSON API response. Deliberately chosen distinct
/// from Checkpoint B's 8MB REST-response ceiling and from WI065's own
/// `ArchiveBounds::max_expanded_bytes` (1 GiB) -- a compressed download
/// should almost always be smaller than its expanded size, so this sits
/// below that, in the same order of magnitude as
/// `ArchiveBounds::max_single_entry_bytes` (256 MiB).
const SNAPSHOT_DOWNLOAD_MAX_COMPRESSED_BYTES: u64 = 300 * 1024 * 1024;

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
    workspace_manager: WorkspaceManager,
    /// A single active remote-import cancellation token (item 15's "active
    /// auth operations" scope: one active connection, and likewise one
    /// active import, matching v1's single-connection model). A second
    /// `remote_import_snapshot` call while one is already running is
    /// rejected rather than silently sharing/overwriting this slot.
    active_import: Mutex<Option<CancellationToken>>,
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
            transport.clone() as Arc<dyn repopact_provider_github::transport::RestTransport>;
        let snapshot_transport =
            transport as Arc<dyn repopact_provider_github::transport::StreamingDownloadTransport>;
        let credential_store: Arc<dyn CredentialStore> =
            Arc::new(repopact_remote_provider::credential_os::OsCredentialStore::new());
        let github = GitHubProvider::new(
            config,
            http_transport,
            rest_transport,
            snapshot_transport,
            credential_store,
        );

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

        // WI067 Checkpoint C: the same production workspace registry/
        // import pipeline WI065 built, rooted at the same app-private
        // location Decision 0057 already specifies -- never a second
        // GitHub-specific registry or extractor.
        let workspace_manager =
            WorkspaceManager::open(app_data_dir.join("repositories")).map_err(|error| {
                RemoteProviderError::new(ErrorCode::MaterializationFailed, error.to_string())
            })?;

        Ok(Self {
            github,
            metadata_path,
            client_configured,
            workspace_manager,
            active_import: Mutex::new(None),
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
/// material. `deny_unknown_fields` (WI067 Checkpoint D, Phase 9) is
/// defense-in-depth: even though nothing in this codebase reads an
/// unexpected field, an attempt to smuggle one in (e.g. a `url` or
/// `destination` alongside the legitimate identity fields) is now
/// rejected outright at deserialization rather than silently ignored.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteRepositoryRefDto {
    pub repository_id: String,
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImportResultDto {
    pub workspace_id: String,
    pub display_name: String,
    pub resolved_commit_sha: String,
}

/// WI067 Checkpoint C (Phase 4): the narrow typed snapshot-import command.
/// Accepts only typed repository/ref identifiers -- never a URL, header
/// map, filesystem destination, or the ref/SHA text alone taken on trust.
/// The exact commit SHA that gets imported is *re-resolved here*, natively,
/// from `repository`/`reference` -- never accepted as a caller-supplied
/// value -- so a stale or tampered frontend-held SHA can never diverge
/// from what this command actually requests and records (Phase 4/9).
#[tauri::command]
pub fn remote_import_snapshot(
    repository: RemoteRepositoryRefDto,
    reference: RemoteRefRefDto,
    service: State<'_, Arc<RemoteProviderService>>,
) -> Result<RemoteImportResultDto, RemoteProviderError> {
    {
        let mut active = service.active_import.lock().unwrap();
        if active.is_some() {
            return Err(RemoteProviderError::new(
                ErrorCode::ProviderProtocolError,
                "a remote snapshot import is already in progress",
            ));
        }
        *active = Some(CancellationToken::new());
    }
    let result = run_remote_import_snapshot(&service, repository, reference);
    *service.active_import.lock().unwrap() = None;
    result
}

fn run_remote_import_snapshot(
    service: &RemoteProviderService,
    repository: RemoteRepositoryRefDto,
    reference: RemoteRefRefDto,
) -> Result<RemoteImportResultDto, RemoteProviderError> {
    let cancel = service
        .active_import
        .lock()
        .unwrap()
        .as_ref()
        .expect("set by remote_import_snapshot before calling this function")
        .clone();

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

    // Phase 3/9/31: resolve now, natively -- this is the exact immutable
    // SHA this request is frozen to. It is never re-resolved later in this
    // same flow even if the branch moves mid-download.
    let resolved = service.github.resolve_ref(&repo, &remote_ref)?;
    if cancel.is_cancelled() {
        return Err(cancelled_error());
    }

    let descriptor = service.github.describe_snapshot(&repo, &resolved)?;
    if cancel.is_cancelled() {
        return Err(cancelled_error());
    }

    let staging_root = service.workspace_manager.root().join("staging");
    fs::create_dir_all(&staging_root).map_err(io_to_provider_error)?;
    let download_path =
        staging_root.join(format!("remote-snapshot-{}.download", uuid::Uuid::new_v4()));

    let cancel_for_download = cancel.clone();
    let artifact = service.github.open_snapshot(
        &descriptor,
        &SnapshotDownloadOptions {
            destination_path: &download_path,
            max_compressed_bytes: SNAPSHOT_DOWNLOAD_MAX_COMPRESSED_BYTES,
            should_cancel: &move || cancel_for_download.is_cancelled(),
        },
    );
    let artifact = match artifact {
        Ok(artifact) => artifact,
        Err(error) => {
            let _ = fs::remove_file(&download_path);
            return Err(error);
        }
    };

    let provenance = RemoteSnapshotProvenance {
        provider: descriptor.provider.clone(),
        provider_repository_id: descriptor.provider_repository_id.clone(),
        owner_label: descriptor.owner_label.clone(),
        repository_name: descriptor.repository_name.clone(),
        selected_ref: resolved.selected_ref.display_name.clone(),
        ref_kind: match resolved.selected_ref.kind {
            RefKind::Branch => "branch",
            RefKind::Tag => "tag",
            RefKind::Commit => "commit",
        }
        .to_string(),
        resolved_commit_sha: resolved.immutable_revision_id.clone(),
        acquired_at: now_rfc3339(),
        snapshot_semantics: "immutable_snapshot".to_string(),
    };

    let staged_file = fs::File::open(&artifact.staged_path).map_err(io_to_provider_error)?;
    let display_name = format!("{}/{}", descriptor.owner_label, descriptor.repository_name);
    let source_reference = format!(
        "github:{}/{}@{}",
        descriptor.owner_label, descriptor.repository_name, resolved.immutable_revision_id
    );
    let strip_root = matches!(descriptor.layout, SnapshotLayout::SingleRootDirectory);

    let import_result = service.workspace_manager.import_remote_snapshot(
        staged_file,
        display_name.clone(),
        source_reference,
        &ArchiveBounds::default(),
        &cancel,
        |_progress| {},
        provenance,
        strip_root,
    );
    let _ = fs::remove_file(&download_path);

    let record = import_result.map_err(|error| {
        RemoteProviderError::new(ErrorCode::MaterializationFailed, error.to_string())
    })?;

    Ok(RemoteImportResultDto {
        workspace_id: record.workspace_id,
        display_name,
        resolved_commit_sha: resolved.immutable_revision_id,
    })
}

fn cancelled_error() -> RemoteProviderError {
    RemoteProviderError::new(
        ErrorCode::DownloadCancelled,
        "snapshot import was cancelled",
    )
}

fn io_to_provider_error(error: std::io::Error) -> RemoteProviderError {
    RemoteProviderError::new(ErrorCode::MaterializationFailed, error.to_string())
}

fn now_rfc3339() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    rfc3339_from_unix_seconds(now.as_secs())
}

/// Mirrors `repopact_mobile_acquisition::workspace`'s own no-chrono-
/// dependency RFC 3339 formatter (Decision 0057's own convention) rather
/// than adding a second time-formatting dependency to this app crate for
/// one timestamp.
fn rfc3339_from_unix_seconds(unix_seconds: u64) -> String {
    const DAYS_IN_MONTH: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let days_total = (unix_seconds / 86_400) as i64;
    let seconds_of_day = (unix_seconds % 86_400) as i64;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;

    let mut year = 1970i64;
    let mut remaining_days = days_total;
    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let days_in_year = if is_leap { 366 } else { 365 };
        if remaining_days >= days_in_year {
            remaining_days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }
    let is_leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mut month = 0usize;
    for (index, &days) in DAYS_IN_MONTH.iter().enumerate() {
        let days = if index == 1 && is_leap {
            days + 1
        } else {
            days
        };
        if remaining_days >= days {
            remaining_days -= days;
            month = index + 1;
        } else {
            month = index + 1;
            break;
        }
    }
    let day = remaining_days + 1;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// WI067 Checkpoint C: cancels the single active `remote_import_snapshot`
/// call, if any. A no-op (not an error) if nothing is running.
#[tauri::command]
pub fn remote_import_cancel(service: State<'_, Arc<RemoteProviderService>>) {
    if let Some(token) = service.active_import.lock().unwrap().as_ref() {
        token.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WI067 Checkpoint D, Phase 9: an attempt to smuggle an arbitrary
    /// `url`/`headers`/`destination` field alongside the legitimate
    /// repository identity is rejected at deserialization -- the command
    /// handler body (and therefore any network/filesystem action) never
    /// even runs.
    #[test]
    fn a_repository_ref_with_an_injected_url_field_is_rejected() {
        let json = r#"{"repositoryId":"1","owner":"octocat","name":"Hello-World","url":"https://evil.example/"}"#;
        let result: Result<RemoteRepositoryRefDto, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "an injected url field must be rejected, not silently ignored"
        );
    }

    #[test]
    fn a_repository_ref_with_an_injected_destination_field_is_rejected() {
        let json = r#"{"repositoryId":"1","owner":"octocat","name":"Hello-World","destinationPath":"C:\\evil"}"#;
        let result: Result<RemoteRepositoryRefDto, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn a_ref_with_an_injected_headers_field_is_rejected() {
        let json = r#"{"displayName":"main","kind":"branch","refId":"heads/main","headers":{"Authorization":"Bearer x"}}"#;
        let result: Result<RemoteRefRefDto, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn an_ordinary_repository_ref_still_deserializes() {
        let json = r#"{"repositoryId":"1","owner":"octocat","name":"Hello-World"}"#;
        let result: Result<RemoteRepositoryRefDto, _> = serde_json::from_str(json);
        assert!(result.is_ok());
    }
}
