//! Decision 0061: the GitHub adapter behind
//! `repopact_remote_provider::provider::RemoteRepositoryProvider`.
//! GitHub-specific REST/auth behavior lives entirely in this crate; no
//! other crate (desktop, mobile-acquisition, frontend) may reference
//! GitHub directly.

pub mod api_version;
pub mod device_flow;
pub mod permissions;
pub mod provider;
pub mod redirect_policy;
pub mod transport;

pub use provider::{GitHubProvider, GitHubProviderConfig};
