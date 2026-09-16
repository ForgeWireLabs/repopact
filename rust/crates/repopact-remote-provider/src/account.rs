//! Provider-neutral account/connection identity (WI067 item 20, item 29).
//! Never keyed only by a mutable human-readable login, and never carrying a
//! credential.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RemoteAccountId {
    pub provider: String,
    /// Provider-stable opaque account id (e.g. GitHub's numeric user id),
    /// never a mutable display login alone.
    pub provider_account_id: String,
}

/// Provider-neutral description of what a connected account can see.
/// GitHub's "installation" concept becomes adapter-internal metadata
/// folded into `label`/`includes_private_repositories`; no future provider
/// is required to invent an "installation" of its own (item 29).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderScope {
    pub label: String,
    pub includes_private_repositories: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteAccount {
    pub id: RemoteAccountId,
    pub display_label: String,
    pub scope: ProviderScope,
}
