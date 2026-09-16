//! Provider-neutral ref/revision model (WI067 item 30, item 31, Decision
//! 0061). RepoPact never materializes from a moving branch/tag directly --
//! every acquisition resolves to an immutable revision first, and that
//! revision is authoritative even if the branch advances mid-download.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    Branch,
    Tag,
    Commit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteRef {
    pub display_name: String,
    pub kind: RefKind,
    /// Opaque provider identifier for the ref, not necessarily equal to
    /// `display_name`.
    pub provider_ref_id: String,
}

/// The authoritative outcome of resolving a `RemoteRef` to an exact,
/// immutable revision. For GitHub this is a 40-character commit SHA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRevision {
    pub selected_ref: RemoteRef,
    pub immutable_revision_id: String,
}
