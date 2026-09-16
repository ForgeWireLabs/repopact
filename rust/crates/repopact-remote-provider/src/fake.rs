//! A non-GitHub provider implementation (WI067 item 7, GH-011). Exists so
//! tests can prove the fake provider's own path -- resolved immutable
//! revision -> snapshot descriptor/artifact -> the existing safe archive
//! materializer -- contains zero GitHub-specific branching anywhere in
//! repository semantics. See `repopact-remote-provider`'s
//! `tests/provider_neutrality.rs` for the end-to-end proof against the
//! real materializer.

use std::sync::Mutex;

use crate::account::{ProviderScope, RemoteAccount, RemoteAccountId};
use crate::auth::AuthState;
use crate::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
use crate::provider::{ProviderCapabilities, RemoteRepositoryProvider, SnapshotDownloadOptions};
use crate::refs::{RefKind, RemoteRef, ResolvedRevision};
use crate::repository::{RemoteRepository, RepositoryVisibility};
use crate::snapshot::{SnapshotArtifact, SnapshotDescriptor, SnapshotLayout};

/// A deliberately trivial in-memory provider: one account, one repository,
/// one branch that resolves to a fixed fake commit id, and a snapshot
/// whose staged bytes are wherever the test fixture set them up.
pub struct FakeProvider {
    state: Mutex<AuthState>,
    staged_snapshot_path: String,
    layout: SnapshotLayout,
}

impl FakeProvider {
    /// `staged_snapshot_path` must already point at a real archive file on
    /// disk (a test fixture) -- this provider does not perform any network
    /// I/O of its own.
    pub fn new(staged_snapshot_path: impl Into<String>, layout: SnapshotLayout) -> Self {
        Self {
            state: Mutex::new(AuthState::Disconnected),
            staged_snapshot_path: staged_snapshot_path.into(),
            layout,
        }
    }

    fn account(&self) -> RemoteAccount {
        RemoteAccount {
            id: RemoteAccountId {
                provider: self.provider_id().to_string(),
                provider_account_id: "fake-account-1".into(),
            },
            display_label: "Fake Account".into(),
            scope: ProviderScope {
                label: "fake-scope".into(),
                includes_private_repositories: true,
            },
        }
    }

    fn repository(&self) -> RemoteRepository {
        RemoteRepository {
            provider: self.provider_id().to_string(),
            provider_repository_id: "fake-repo-1".into(),
            owner_label: "fake-owner".into(),
            name: "fake-repo".into(),
            full_display_name: "fake-owner/fake-repo".into(),
            visibility: RepositoryVisibility::Public,
            default_branch: Some("main".into()),
        }
    }
}

impl RemoteRepositoryProvider for FakeProvider {
    fn provider_id(&self) -> &'static str {
        "fake"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_public_without_auth: true,
            supports_private_repositories: true,
            supports_organizations: false,
        }
    }

    fn connection_status(&self) -> AuthState {
        self.state.lock().unwrap().clone()
    }

    fn begin_authorization(&self) -> RemoteProviderResult<AuthState> {
        let mut state = self.state.lock().unwrap();
        *state = AuthState::Authorized {
            account_label: "Fake Account".into(),
        };
        Ok(state.clone())
    }

    fn poll_authorization(&self) -> RemoteProviderResult<AuthState> {
        Ok(self.connection_status())
    }

    fn cancel_authorization(&self) -> RemoteProviderResult<()> {
        let mut state = self.state.lock().unwrap();
        *state = AuthState::Cancelled;
        Ok(())
    }

    fn disconnect(&self) -> RemoteProviderResult<()> {
        let mut state = self.state.lock().unwrap();
        *state = AuthState::Disconnected;
        Ok(())
    }

    fn list_accounts(&self) -> RemoteProviderResult<Vec<RemoteAccount>> {
        Ok(vec![self.account()])
    }

    fn list_repositories(
        &self,
        _account: &RemoteAccount,
    ) -> RemoteProviderResult<Vec<RemoteRepository>> {
        Ok(vec![self.repository()])
    }

    fn search_repositories(
        &self,
        account: &RemoteAccount,
        query: &str,
    ) -> RemoteProviderResult<Vec<RemoteRepository>> {
        let repos = self.list_repositories(account)?;
        Ok(repos
            .into_iter()
            .filter(|r| r.name.contains(query) || query.is_empty())
            .collect())
    }

    fn list_refs(&self, _repository: &RemoteRepository) -> RemoteProviderResult<Vec<RemoteRef>> {
        Ok(vec![RemoteRef {
            display_name: "main".into(),
            kind: RefKind::Branch,
            provider_ref_id: "refs/heads/main".into(),
        }])
    }

    fn resolve_ref(
        &self,
        _repository: &RemoteRepository,
        reference: &RemoteRef,
    ) -> RemoteProviderResult<ResolvedRevision> {
        Ok(ResolvedRevision {
            selected_ref: reference.clone(),
            // A fixed, obviously-fake 40-hex-character revision id --
            // shaped like a GitHub commit SHA only because that is the
            // provider-neutral contract's shape, not because this provider
            // is GitHub.
            immutable_revision_id: "f".repeat(40),
        })
    }

    fn describe_snapshot(
        &self,
        repository: &RemoteRepository,
        revision: &ResolvedRevision,
    ) -> RemoteProviderResult<SnapshotDescriptor> {
        Ok(SnapshotDescriptor {
            provider: self.provider_id().to_string(),
            provider_repository_id: repository.provider_repository_id.clone(),
            owner_label: repository.owner_label.clone(),
            repository_name: repository.name.clone(),
            revision: revision.clone(),
            layout: self.layout,
            reported_size_hint_bytes: None,
        })
    }

    fn open_snapshot(
        &self,
        descriptor: &SnapshotDescriptor,
        options: &SnapshotDownloadOptions,
    ) -> RemoteProviderResult<SnapshotArtifact> {
        if (options.should_cancel)() {
            return Err(RemoteProviderError::new(
                ErrorCode::DownloadCancelled,
                "cancelled before the fake snapshot copy began",
            ));
        }
        let bytes = std::fs::read(&self.staged_snapshot_path).map_err(|error| {
            RemoteProviderError::new(
                ErrorCode::MaterializationFailed,
                format!("fake snapshot fixture missing: {error}"),
            )
        })?;
        if bytes.len() as u64 > options.max_compressed_bytes {
            return Err(RemoteProviderError::new(
                ErrorCode::SnapshotTooLarge,
                format!(
                    "fake snapshot fixture ({} bytes) exceeds the {}-byte bound",
                    bytes.len(),
                    options.max_compressed_bytes
                ),
            ));
        }
        // Proves the real contract, not just this test double's own
        // convenience: the bytes land exactly at the caller-chosen
        // destination, never wherever the provider felt like staging them.
        std::fs::write(options.destination_path, &bytes).map_err(|error| {
            RemoteProviderError::new(
                ErrorCode::MaterializationFailed,
                format!("failed to write fake snapshot to destination: {error}"),
            )
        })?;
        Ok(SnapshotArtifact {
            descriptor: descriptor.clone(),
            staged_path: options.destination_path.to_string_lossy().into_owned(),
            received_bytes: bytes.len() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_provider_never_reports_the_github_provider_id() {
        let provider = FakeProvider::new("unused", SnapshotLayout::Flat);
        assert_ne!(provider.provider_id(), "github");
    }

    #[test]
    fn resolve_ref_produces_an_immutable_revision_distinct_from_the_moving_ref() {
        let provider = FakeProvider::new("unused", SnapshotLayout::Flat);
        let repo = provider.repository();
        let refs = provider.list_refs(&repo).unwrap();
        let resolved = provider.resolve_ref(&repo, &refs[0]).unwrap();
        assert_eq!(resolved.selected_ref.display_name, "main");
        assert_eq!(resolved.immutable_revision_id.len(), 40);
        assert_ne!(resolved.immutable_revision_id, "main");
    }
}
