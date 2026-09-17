//! Decision 0061: the provider-neutral remote-repository-acquisition core.
//! GitHub (`repopact-provider-github`) is the first implementation of
//! [`provider::RemoteRepositoryProvider`], but nothing in this crate, and
//! nothing downstream that consumes it (workspace registry, materializer,
//! Tauri commands), may name GitHub. A snapshot acquired through any
//! provider becomes an ordinary app-private filesystem workspace through
//! WI065's existing safe archive materializer -- this crate never becomes
//! a second `Repository` implementation, and it never performs network
//! I/O itself (that is the adapter's job).
//!
//! Real Git synchronization (clone/fetch/pull/push against a live `.git`
//! working tree) is WI068's `GitBackend` seam, not this crate. A
//! `RemoteSnapshot` acquired here has no `.git` directory and no live
//! remote relationship after import; see
//! `repopact_mobile_acquisition::registry::AcquisitionKind::RemoteSnapshot`.

pub mod account;
pub mod auth;
pub mod credential;
pub mod credential_os;
pub mod error;
pub mod fake;
pub mod pkce;
pub mod provider;
pub mod redact;
pub mod refs;
pub mod repository;
pub mod snapshot;

pub use account::{ProviderScope, RemoteAccount, RemoteAccountId};
pub use auth::AuthState;
pub use credential::{CredentialKey, CredentialKind, CredentialStore, InMemoryCredentialStore};
pub use credential_os::OsCredentialStore;
pub use error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
pub use provider::{ProviderCapabilities, RemoteRepositoryProvider, SnapshotDownloadOptions};
pub use redact::{redact, Secret};
pub use refs::{RefKind, RemoteRef, ResolvedRevision};
pub use repository::{RemoteRepository, RepositoryVisibility};
pub use snapshot::{SnapshotArtifact, SnapshotDescriptor, SnapshotLayout};
