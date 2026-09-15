//! Decision 0056/0057: the mobile acquisition adapter. Owns everything on
//! the RepoPact side of the boundary between "however a user picked or
//! handed over some content on a mobile device" and "an ordinary app-private
//! filesystem tree `DesktopService::open_repository` can consume unchanged."
//!
//! This crate never becomes a second repository model. It ends its
//! responsibility the moment bytes safely exist inside
//! `workspaces/<id>/repository/`; everything past that point is ordinary
//! `repopact-repository`/`repopact-desktop-api` behavior, exactly as on
//! desktop.

pub mod archive;
pub mod bounds;
pub mod divergence;
pub mod error;
pub mod export;
pub mod import;
pub mod operation;
pub mod paths;
pub mod registry;
pub mod sink;
pub mod source;
pub mod workspace;

pub use divergence::SourceStatus;
pub use error::{AcquisitionError, AcquisitionResult, ErrorCode};
pub use registry::{
    AcquisitionKind, ExportState, GitState, LifecycleState, SourceFingerprint, WorkspaceRecord,
};
pub use workspace::WorkspaceManager;
