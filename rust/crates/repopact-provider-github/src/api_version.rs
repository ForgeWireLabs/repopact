//! WI067 item 23: a single, centrally-sourced GitHub REST API version
//! string. No call site should hardcode its own version literal.

/// Value for the `X-GitHub-Api-Version` header GitHub's REST API requires.
/// Verified against GitHub's REST API versioning documentation during
/// WI067 Checkpoint A research (2026-09-15); update this constant (and
/// re-verify the permission/endpoint matrix in `permissions.rs`) if a
/// future checkpoint upgrades it.
pub const GITHUB_API_VERSION: &str = "2022-11-28";
