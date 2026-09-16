//! WI067 Checkpoint B, item 6: every header this adapter sends to GitHub
//! is centralized here so no endpoint call site duplicates a header name
//! or value string. GitHub's REST API rejects requests with no
//! `User-Agent` header (returns 403), so this is a real functional
//! requirement, not cosmetic.

use repopact_remote_provider::redact::Secret;

use crate::api_version::GITHUB_API_VERSION;

/// GitHub requires a `User-Agent` identifying the calling application; a
/// generic HTTP-library default is rejected. Includes a contact URL per
/// GitHub's own guidance for API clients.
pub const USER_AGENT: &str = "RepoPact (+https://github.com/ForgeWireLabs/repopact)";

pub const ACCEPT: &str = "application/vnd.github+json";

/// One request's worth of headers, built centrally. `bearer_token` is
/// `None` for the unauthenticated public-repository path (Decision 0061
/// item 42 / GH-007's "public repository import may operate without
/// login").
pub struct RequestHeaders<'a> {
    pub bearer_token: Option<&'a Secret>,
}

impl<'a> RequestHeaders<'a> {
    pub fn new() -> Self {
        Self { bearer_token: None }
    }

    pub fn with_bearer_token(token: &'a Secret) -> Self {
        Self {
            bearer_token: Some(token),
        }
    }

    /// `(name, value)` pairs. Never includes a value with `Debug`/`Display`
    /// visibility beyond this exact call site -- `value` for the
    /// `Authorization` entry is read from `Secret::expose` once, here, and
    /// handed directly to the HTTP client, never stored or logged.
    pub fn as_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = vec![
            ("Accept", ACCEPT.to_string()),
            ("X-GitHub-Api-Version", GITHUB_API_VERSION.to_string()),
            ("User-Agent", USER_AGENT.to_string()),
        ];
        if let Some(token) = self.bearer_token {
            pairs.push(("Authorization", format!("Bearer {}", token.expose())));
        }
        pairs
    }
}

impl<'a> Default for RequestHeaders<'a> {
    fn default() -> Self {
        Self::new()
    }
}
