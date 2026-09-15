//! JSON shapes exchanged with the Android Kotlin plugin only (see this
//! crate's `lib.rs` doc comment: none of this ever reaches the frontend --
//! the app's own mobile acquisition coordinator translates these into the
//! `repopact_mobile_acquisition::AcquisitionError`/DTO shapes it exposes to
//! JS). Field names are `camelCase` to match this repository's existing
//! Tauri command argument convention (see `apply_mutation_plan`'s
//! `sessionId`/`planHandle`).

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PickDirectoryResponse {
    #[serde(rename_all = "camelCase")]
    Selected {
        tree_uri: String,
        display_name: String,
    },
    Cancelled,
    #[serde(rename_all = "camelCase")]
    Error {
        reason: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PickArchiveResponse {
    #[serde(rename_all = "camelCase")]
    Selected {
        document_uri: String,
        display_name: String,
    },
    Cancelled,
    #[serde(rename_all = "camelCase")]
    Error {
        reason: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChildEntry {
    pub uri: String,
    /// Untrusted -- flows through Checkpoint A's `reject_unsafe_relative_path`
    /// and `CollisionGuard` exactly like any other source-reported name.
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "isDirectory")]
    pub is_directory: bool,
    /// Hint only; never trusted as an upper bound (Decision 0057).
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ListChildrenResponse {
    #[serde(rename_all = "camelCase")]
    Ok { entries: Vec<ChildEntry> },
    #[serde(rename_all = "camelCase")]
    Error { reason: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OpenDocumentResponse {
    #[serde(rename_all = "camelCase")]
    Ok {
        staging_path: String,
        byte_count: u64,
    },
    #[serde(rename_all = "camelCase")]
    Error { reason: String },
}
