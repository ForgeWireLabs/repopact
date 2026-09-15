//! SAF has no desktop equivalent. This module exists only so
//! `repopact-mobile-saf` type-checks under `cargo check --workspace` on a
//! non-Android host; nothing here is ever actually called, because the app
//! only registers `init_plugin()` under `#[cfg(target_os = "android")]`.

use repopact_mobile_acquisition::{AcquisitionError, AcquisitionResult, ErrorCode};

use crate::{ExportRootOutcome, PickedDocument, PickedTree};

pub fn pick_directory_tree() -> AcquisitionResult<Option<PickedTree>> {
    Err(unavailable())
}

pub fn pick_archive_document() -> AcquisitionResult<Option<PickedDocument>> {
    Err(unavailable())
}

pub fn pick_export_directory() -> AcquisitionResult<Option<PickedTree>> {
    Err(unavailable())
}

pub fn create_export_root(_tree_uri: &str, _name: &str) -> AcquisitionResult<ExportRootOutcome> {
    Err(unavailable())
}

pub fn delete_document(_uri: &str) -> AcquisitionResult<()> {
    Err(unavailable())
}

pub fn pick_export_archive_destination(
    _suggested_name: &str,
) -> AcquisitionResult<Option<PickedDocument>> {
    Err(unavailable())
}

pub fn upload_completed_archive(
    _document_uri: &str,
    _local_zip_path: &std::path::Path,
) -> AcquisitionResult<u64> {
    Err(unavailable())
}

fn unavailable() -> AcquisitionError {
    AcquisitionError::new(
        ErrorCode::SourceUnavailable,
        "SAF acquisition is an Android-only capability",
    )
}
