//! SAF has no desktop equivalent. This module exists only so
//! `repopact-mobile-saf` type-checks under `cargo check --workspace` on a
//! non-Android host; nothing here is ever actually called, because the app
//! only registers `init_plugin()` under `#[cfg(target_os = "android")]`.

use repopact_mobile_acquisition::{AcquisitionError, AcquisitionResult, ErrorCode};

use crate::{PickedDocument, PickedTree};

pub fn pick_directory_tree() -> AcquisitionResult<Option<PickedTree>> {
    Err(unavailable())
}

pub fn pick_archive_document() -> AcquisitionResult<Option<PickedDocument>> {
    Err(unavailable())
}

fn unavailable() -> AcquisitionError {
    AcquisitionError::new(
        ErrorCode::SourceUnavailable,
        "SAF acquisition is an Android-only capability",
    )
}
