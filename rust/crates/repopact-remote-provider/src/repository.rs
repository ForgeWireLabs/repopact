//! Provider-neutral repository DTO (WI067 item 28). Bounded metadata only
//! -- never a persisted raw provider API payload.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryVisibility {
    Public,
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteRepository {
    pub provider: String,
    /// Provider-stable opaque repository id.
    pub provider_repository_id: String,
    pub owner_label: String,
    pub name: String,
    pub full_display_name: String,
    pub visibility: RepositoryVisibility,
    pub default_branch: Option<String>,
}
