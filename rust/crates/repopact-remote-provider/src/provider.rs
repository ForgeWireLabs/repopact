//! The provider-neutral seam (Decision 0061, GH-001/GH-011). GitHub is the
//! first implementation (`repopact-provider-github`); a fake/non-GitHub
//! implementation ([`crate::fake::FakeProvider`]) exercises this same
//! trait in tests to prove the kernel never branches on GitHub to
//! implement repository semantics.

use std::path::Path;

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

/// WI067 Checkpoint C: `open_snapshot`'s real production shape needs more
/// than Checkpoint A's bare `&descriptor` -- a caller-chosen (never
/// provider-chosen) app-private staging destination, an explicit
/// compressed-byte download bound enforced while streaming, and a way to
/// observe cancellation mid-download. Checkpoint A's original signature
/// (`open_snapshot(&self, descriptor)` with no destination/bound/
/// cancellation) could not have supported any of these -- discovered as a
/// real defect only once Checkpoint C had to actually stream a multi-
/// megabyte archive rather than merely stat a test fixture.
pub struct SnapshotDownloadOptions<'a> {
    /// An already-allocated, empty, app-private staging path the provider
    /// must write the downloaded bytes to. Never chosen by the provider
    /// itself, and never a caller-supplied arbitrary path from outside the
    /// native process (the Tauri command layer allocates this the same way
    /// WI065's existing staging allocator does).
    pub destination_path: &'a Path,
    /// Enforced while streaming (never only after the fact, and never
    /// trusted from a declared `Content-Length` alone) -- exceeding it
    /// aborts the download and the caller removes the partial file.
    pub max_compressed_bytes: u64,
    /// Polled periodically during the download; returning `true` aborts
    /// the transfer with a typed cancellation result.
    pub should_cancel: &'a dyn Fn() -> bool,
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

    /// Stream the snapshot to the caller-chosen app-private staging path in
    /// `options` and return the resulting bounded artifact. Implementations
    /// must enforce `options.max_compressed_bytes` while streaming (not
    /// merely after the fact), poll `options.should_cancel` periodically,
    /// and leave no file at `options.destination_path` on any failure or
    /// cancellation -- the caller's own staging-cleanup path is the backstop,
    /// but a clean implementation does not rely on it alone.
    fn open_snapshot(
        &self,
        descriptor: &SnapshotDescriptor,
        options: &SnapshotDownloadOptions,
    ) -> RemoteProviderResult<SnapshotArtifact>;
}
