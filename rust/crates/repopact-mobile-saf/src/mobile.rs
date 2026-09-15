//! The real Android bridge. Registers `SafAcquisitionPlugin` (this crate's
//! `android/` Gradle module) via `PluginHandle::run_mobile_plugin`, exactly
//! the pattern `tauri-plugin-dialog` 2.7.3's own `mobile.rs` uses against
//! Tauri 2.11.5's `PluginApi`/`PluginHandle` (verified against that
//! vendored source, not a generic example, per WI065 Checkpoint B §2).
//!
//! Every call here is synchronous/blocking (`run_mobile_plugin`, not the
//! `_async` variant) because the app's own mobile-acquisition coordinator
//! always drives these from a dedicated operation thread already (see
//! `repopact-desktop`'s `mobile_acquisition` module), never from the Tauri
//! command-handler thread directly.

use std::fs;
use std::io::Read;

use serde::{de::DeserializeOwned, Serialize};
use tauri::{
    plugin::{mobile::PluginInvokeError, PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use repopact_mobile_acquisition::error::{AcquisitionError, AcquisitionResult, ErrorCode};
use repopact_mobile_acquisition::source::{AcquisitionSource, SourceEntry, SourceEntryKind};

use crate::models::{
    ListChildrenResponse, OpenDocumentResponse, PickArchiveResponse, PickDirectoryResponse,
};
use crate::{PickedDocument, PickedTree};

const PLUGIN_IDENTIFIER: &str = "com.forgewirelabs.repopact.mobileacquisition";

pub fn register<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<PluginHandle<R>, PluginInvokeError> {
    let _ = app;
    api.register_android_plugin(PLUGIN_IDENTIFIER, "SafAcquisitionPlugin")
}

fn invoke<T: DeserializeOwned, R: Runtime>(
    handle: &PluginHandle<R>,
    command: &str,
    payload: impl Serialize,
) -> AcquisitionResult<T> {
    handle.run_mobile_plugin(command, payload).map_err(|error| {
        AcquisitionError::new(
            ErrorCode::SourceUnavailable,
            format!("SAF plugin invocation '{command}' failed: {error}"),
        )
    })
}

/// Maps a Kotlin-reported failure `reason` code to Checkpoint A's typed
/// error taxonomy (WI065 Checkpoint B §20). Kotlin never sends free-text
/// prose the frontend would need to parse -- only these fixed codes.
fn map_reason(reason: &str) -> AcquisitionError {
    let code = match reason {
        "permission_denied" => ErrorCode::PermissionDenied,
        "unsupported_entry" => ErrorCode::UnsupportedEntry,
        "not_found" | "provider_failure" | "io_error" | "activity_unavailable" | "missing_uri" => {
            ErrorCode::SourceUnavailable
        }
        _ => ErrorCode::SourceUnavailable,
    };
    AcquisitionError::new(code, format!("SAF provider reported '{reason}'"))
}

pub fn pick_directory_tree<R: Runtime>(
    handle: &PluginHandle<R>,
) -> AcquisitionResult<Option<PickedTree>> {
    let response: PickDirectoryResponse = invoke(handle, "pickDirectoryTree", ())?;
    match response {
        PickDirectoryResponse::Selected {
            tree_uri,
            display_name,
        } => Ok(Some(PickedTree {
            tree_uri,
            display_name,
        })),
        PickDirectoryResponse::Cancelled => Ok(None),
        PickDirectoryResponse::Error { reason } => Err(map_reason(&reason)),
    }
}

pub fn pick_archive_document<R: Runtime>(
    handle: &PluginHandle<R>,
) -> AcquisitionResult<Option<PickedDocument>> {
    let response: PickArchiveResponse = invoke(handle, "pickArchiveDocument", ())?;
    match response {
        PickArchiveResponse::Selected {
            document_uri,
            display_name,
        } => Ok(Some(PickedDocument {
            document_uri,
            display_name,
        })),
        PickArchiveResponse::Cancelled => Ok(None),
        PickArchiveResponse::Error { reason } => Err(map_reason(&reason)),
    }
}

/// Copies one SAF document's bytes into an app-private staging file via the
/// Kotlin plugin and returns that file's path, ready to be opened with
/// ordinary `std::fs`. Used both by `AndroidSafSource::open_file` (per-entry,
/// during a directory import) and directly by the archive-import command
/// (the whole picked archive is one document).
///
/// # Why a staging file, not a stream or fd, crosses the Rust/Kotlin
/// boundary (WI065 Checkpoint B §9)
///
/// Tauri's mobile-plugin invoke channel is JSON-in/JSON-out
/// (`run_mobile_plugin<T: DeserializeOwned>`); there is no supported way in
/// this Tauri/plugin version to hand a live Kotlin `InputStream` or a raw
/// POSIX file descriptor across that boundary as a value. The two
/// alternatives this crate deliberately avoids are: marshaling file bytes
/// through the JSON payload itself (base64-inflated, and a large file would
/// have to be held in memory on both sides at once -- exactly what §9
/// prohibits), or reimplementing a chunked pull protocol (multiple
/// round-trips per file, each carrying a bounded byte range) purely to avoid
/// one temp file per entry. A native, app-private (Android
/// `cacheDir`-rooted) staging file is the narrowest mechanism that is both
/// reliable on every provider and keeps the actual byte content off the
/// JSON channel entirely: Kotlin copies `ContentResolver.openInputStream`
/// straight to that file with its own generous byte cap (a safety net, not
/// the authority), Rust reads it back with an ordinary `std::fs::File`, and
/// the bounded importer's own byte-counting copy loop (Checkpoint A) remains
/// the actual resource-accounting authority regardless of what either side
/// of the bridge believed the size to be.
fn open_document_to_staging<R: Runtime>(
    handle: &PluginHandle<R>,
    uri: &str,
) -> AcquisitionResult<(std::path::PathBuf, u64)> {
    let response: OpenDocumentResponse =
        invoke(handle, "openDocument", OpenDocumentRequest { uri })?;
    match response {
        OpenDocumentResponse::Ok {
            staging_path,
            byte_count,
        } => Ok((std::path::PathBuf::from(staging_path), byte_count)),
        OpenDocumentResponse::Error { reason } => Err(map_reason(&reason)),
    }
}

#[derive(serde::Serialize)]
struct OpenDocumentRequest<'a> {
    uri: &'a str,
}

#[derive(serde::Serialize)]
struct ListChildrenRequest<'a> {
    #[serde(rename = "treeUri")]
    tree_uri: &'a str,
    #[serde(rename = "parentUri")]
    parent_uri: &'a str,
}

/// A file opened from SAF-backed staging that deletes its staging copy the
/// moment the importer is done reading it -- the app-private cache never
/// accumulates one leftover file per imported entry.
struct StagingFile {
    file: fs::File,
    path: std::path::PathBuf,
}

impl Read for StagingFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Drop for StagingFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

struct WalkEntry {
    uri: String,
    relative: String,
    is_directory: bool,
    size_hint: Option<u64>,
}

/// Bridges a real, user-picked SAF directory tree into Checkpoint A's
/// platform-neutral `AcquisitionSource`. Streaming by construction (see
/// the trait's own contract): children of a directory are listed lazily,
/// one `listChildren` plugin call per directory actually visited, never a
/// single upfront tree walk (WI065 Checkpoint B §9).
///
/// This struct performs **zero** path validation, collision detection,
/// bound enforcement, or byte accounting of its own -- all of that remains
/// exclusively Checkpoint A's `import::import_directory` responsibility
/// (WI065 Checkpoint B §8/§27), operating on the `SourceEntry`s this
/// produces exactly as it already does in its own host-side tests.
pub struct AndroidSafSource<R: Runtime> {
    handle: PluginHandle<R>,
    tree_uri: String,
    stack: Vec<WalkEntry>,
    last_opened: Option<(String, String)>, // (relative_path, uri)
}

impl<R: Runtime> AndroidSafSource<R> {
    pub fn new(handle: PluginHandle<R>, tree_uri: String) -> AcquisitionResult<Self> {
        let mut source = Self {
            handle,
            tree_uri: tree_uri.clone(),
            stack: Vec::new(),
            last_opened: None,
        };
        source.push_children(&tree_uri, "")?;
        Ok(source)
    }

    fn push_children(&mut self, parent_uri: &str, relative_dir: &str) -> AcquisitionResult<()> {
        let response: ListChildrenResponse = invoke(
            &self.handle,
            "listChildren",
            ListChildrenRequest {
                tree_uri: &self.tree_uri,
                parent_uri,
            },
        )?;
        let mut entries = match response {
            ListChildrenResponse::Ok { entries } => entries,
            ListChildrenResponse::Error { reason } => return Err(map_reason(&reason)),
        };
        // Deterministic order (matches FilesystemSource's own sort-by-name
        // discipline, and keeps progress reporting stable across runs).
        entries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        for entry in entries.into_iter().rev() {
            // §10/§11: a provider display name is untrusted and may be
            // absent entirely. A missing name cannot be safely defaulted
            // (it would collide with every other unnamed entry under the
            // same collision-guard key) -- surfaced as a typed failure
            // rather than silently substituting a placeholder.
            let Some(name) = entry.display_name else {
                return Err(AcquisitionError::new(
                    ErrorCode::UnsupportedEntry,
                    "SAF provider returned an entry with no display name",
                ));
            };
            let relative = if relative_dir.is_empty() {
                name
            } else {
                format!("{relative_dir}/{name}")
            };
            self.stack.push(WalkEntry {
                uri: entry.uri,
                relative,
                is_directory: entry.is_directory,
                size_hint: entry.size,
            });
        }
        Ok(())
    }
}

impl<R: Runtime> AcquisitionSource for AndroidSafSource<R> {
    fn next_entry(&mut self) -> AcquisitionResult<Option<SourceEntry>> {
        let Some(entry) = self.stack.pop() else {
            return Ok(None);
        };
        if entry.is_directory {
            self.push_children(&entry.uri, &entry.relative)?;
            Ok(Some(SourceEntry {
                relative_path: entry.relative,
                kind: SourceEntryKind::Directory,
                size_hint: None,
            }))
        } else {
            self.last_opened = Some((entry.relative.clone(), entry.uri));
            Ok(Some(SourceEntry {
                relative_path: entry.relative,
                kind: SourceEntryKind::File,
                size_hint: entry.size_hint,
            }))
        }
    }

    fn open_file(&mut self, relative_path: &str) -> AcquisitionResult<Box<dyn Read + '_>> {
        let Some((expected_relative, uri)) = self.last_opened.take() else {
            return Err(AcquisitionError::new(
                ErrorCode::InternalIo,
                "open_file called without a preceding file entry from next_entry",
            ));
        };
        if expected_relative != relative_path {
            return Err(AcquisitionError::new(
                ErrorCode::InternalIo,
                "open_file called for a different entry than the last one produced",
            ));
        }
        let (staging_path, _byte_count) = open_document_to_staging(&self.handle, &uri)?;
        let file = fs::File::open(&staging_path).map_err(|error| {
            let _ = fs::remove_file(&staging_path);
            AcquisitionError::new(
                ErrorCode::InternalIo,
                format!("unable to open SAF staging file: {error}"),
            )
        })?;
        Ok(Box::new(StagingFile {
            file,
            path: staging_path,
        }))
    }
}

/// Copies a picked archive document straight into an app-private staging
/// file and hands back a seekable handle Checkpoint A's ZIP importer can
/// read directly (`zip::ZipArchive` requires `Read + Seek`, which an
/// ordinary `std::fs::File` provides -- unlike the streamed `Read`-only
/// path `AndroidSafSource::open_file` uses for directory-import entries).
pub fn open_archive_document<R: Runtime>(
    handle: &PluginHandle<R>,
    document_uri: &str,
) -> AcquisitionResult<StagingArchiveFile> {
    let (staging_path, _byte_count) = open_document_to_staging(handle, document_uri)?;
    let file = fs::File::open(&staging_path).map_err(|error| {
        let _ = fs::remove_file(&staging_path);
        AcquisitionError::new(
            ErrorCode::InternalIo,
            format!("unable to open SAF staging archive: {error}"),
        )
    })?;
    Ok(StagingArchiveFile {
        file,
        path: staging_path,
    })
}

/// Like [`StagingFile`], but also `Seek` (required by `zip::ZipArchive`),
/// and deletes its staging copy on drop.
pub struct StagingArchiveFile {
    file: fs::File,
    path: std::path::PathBuf,
}

impl Read for StagingArchiveFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl std::io::Seek for StagingArchiveFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl Drop for StagingArchiveFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
