//! Decision 0061: the GitHub adapter behind
//! `repopact_remote_provider::provider::RemoteRepositoryProvider`.
//! GitHub-specific REST/auth behavior lives entirely in this crate; no
//! other crate (desktop, mobile-acquisition, frontend) may reference
//! GitHub directly.

pub mod api_version;
pub mod browser_flow;
pub mod callback;
/// Decision 0062: retained as a tested protocol library (state machine +
/// request/response shape), but no longer wired into `GitHubProvider` or
/// any Tauri command. GitHub's own guidance is not to enable device flow
/// without a constrained/headless reason, so the production GitHub App
/// registration keeps it OFF; this module survives only as executable
/// documentation and a base for a future explicitly-justified headless/CLI
/// capability, never as a second ordinary Workbench login choice.
pub mod device_flow;
pub mod headers;
pub mod permissions;
pub mod provider;
pub mod redirect_policy;
pub mod rest;
pub mod transport;

pub use provider::{GitHubProvider, GitHubProviderConfig};
