//! The provider-neutral seam (Decision 0061, GH-001/GH-011). GitHub is the
//! first implementation (`repopact-provider-github`); a fake/non-GitHub
//! implementation ([`crate::fake::FakeProvider`]) exercises this same
//! trait in tests to prove the kernel never branches on GitHub to
//! implement repository semantics.

use crate::account::RemoteAccount;
use crate::auth::AuthState;
use crate::error::RemoteProviderResult;
use crate::refs::{RemoteRef, ResolvedRevision};
use crate::repository::RemoteRepository;
use crate::snapshot::{SnapshotArtifact, SnapshotDescriptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub supports_public_without_auth: bool,
    pub supports_private_repositories: bool,
    pub supports_organizations: bool,
}

/// Deliberately synchronous in signature shape for Checkpoint A --
/// selecting an async runtime is an implementation-time decision for the
/// adapter that actually performs network I/O, not part of this seam's
/// contract. A real GitHub adapter may block on its own runtime internally
/// (or this trait may grow an async variant later); nothing in the
/// provider-neutral core depends on that choice.
pub trait RemoteRepositoryProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn capabilities(&self) -> ProviderCapabilities;

    fn connection_status(&self) -> AuthState;
    fn begin_authorization(&self) -> RemoteProviderResult<AuthState>;
    fn poll_authorization(&self) -> RemoteProviderResult<AuthState>;
    fn cancel_authorization(&self) -> RemoteProviderResult<()>;
    /// Deletes local credential/session state only. Does not claim to
    /// perform remote/server-side authorization revocation unless the
    /// specific implementation documents that it genuinely does (item 21).
    fn disconnect(&self) -> RemoteProviderResult<()>;

    fn list_accounts(&self) -> RemoteProviderResult<Vec<RemoteAccount>>;
    fn list_repositories(
        &self,
        account: &RemoteAccount,
    ) -> RemoteProviderResult<Vec<RemoteRepository>>;
    fn search_repositories(
        &self,
        account: &RemoteAccount,
        query: &str,
    ) -> RemoteProviderResult<Vec<RemoteRepository>>;
    fn list_refs(&self, repository: &RemoteRepository) -> RemoteProviderResult<Vec<RemoteRef>>;
    fn resolve_ref(
        &self,
        repository: &RemoteRepository,
        reference: &RemoteRef,
    ) -> RemoteProviderResult<ResolvedRevision>;

    /// Describe (but do not yet download) the snapshot for a resolved
    /// revision, so a caller can inspect/log the non-secret descriptor
    /// before committing to a bounded download.
    fn describe_snapshot(
        &self,
        repository: &RemoteRepository,
        revision: &ResolvedRevision,
    ) -> RemoteProviderResult<SnapshotDescriptor>;

    /// Stream the snapshot to app-private/native staging and return the
    /// resulting bounded artifact. Implementations must enforce a
    /// compressed-byte bound while streaming and clean up partial staging
    /// on any failure/cancellation.
    fn open_snapshot(
        &self,
        descriptor: &SnapshotDescriptor,
    ) -> RemoteProviderResult<SnapshotArtifact>;
}
