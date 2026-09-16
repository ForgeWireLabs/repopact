//! Provider-neutral snapshot descriptor/artifact and archive-layout model
//! (WI067 item 34, item 35, item 36, item 37). This is the handoff contract
//! into WI065's existing safe archive materializer -- this crate does not
//! implement a second extractor.

use serde::{Deserialize, Serialize};

use crate::refs::ResolvedRevision;

/// How a downloaded snapshot archive is laid out on disk, so the
/// materializer knows whether/how to strip a provider-generated wrapper
/// directory. Deliberately not a generic "strip N path components" knob --
/// only this one verified shape, reusing the materializer's own path/
/// collision/security checks rather than a bespoke stripping routine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotLayout {
    /// Archive root already matches the intended workspace root.
    Flat,
    /// Exactly one top-level directory contains every entry; the
    /// materializer must verify this before stripping it, refusing to
    /// proceed if a second top-level entry exists.
    SingleRootDirectory,
}

/// Non-secret, serializable description of a resolved snapshot -- distinct
/// from the byte artifact itself, so a caller can inspect/log/evidence it
/// before committing to a bounded download.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDescriptor {
    pub provider: String,
    pub provider_repository_id: String,
    pub revision: ResolvedRevision,
    pub layout: SnapshotLayout,
    /// Best-effort size hint from provider metadata. Never trusted as the
    /// enforcement bound -- the materializer counts real received bytes.
    pub reported_size_hint_bytes: Option<u64>,
}

/// A bounded byte artifact for a downloaded snapshot, already staged to
/// app-private/native storage by the provider adapter -- never frontend
/// memory, never base64 IPC (item 37).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotArtifact {
    pub descriptor: SnapshotDescriptor,
    /// Path to the staged archive bytes in app-private/native temp
    /// storage.
    pub staged_path: String,
    /// Actual bytes received while streaming, enforced against a bound
    /// during download -- never derived from `Content-Length` alone.
    pub received_bytes: u64,
}
