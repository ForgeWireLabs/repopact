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

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};
use tauri::{
    plugin::{mobile::PluginInvokeError, PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use repopact_mobile_acquisition::error::{AcquisitionError, AcquisitionResult, ErrorCode};
use repopact_mobile_acquisition::sink::{ExportFileWriter, ExportSink};
use repopact_mobile_acquisition::source::{AcquisitionSource, SourceEntry, SourceEntryKind};

use crate::models::{
    CreateChildDocumentResponse, CreateExportArchiveResponse, CreateExportRootResponse,
    DeleteDocumentResponse, ListChildrenResponse, OpenDocumentResponse, PickArchiveResponse,
    PickDirectoryResponse, WriteDocumentResponse,
};
use crate::{ExportRootOutcome, PickedDocument, PickedTree};

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

// ---------------------------------------------------------------------
// WI065 Checkpoint D: explicit SAF export/share-back.
//
// Mirrors the import side's layering exactly: Kotlin owns only
// `DocumentsContract`/`ContentResolver` mechanics (create a child document,
// stream bytes into it, delete a document); this module and
// `repopact_mobile_acquisition::export`/`sink` own traversal order, path
// safety, bounds, cancellation, and progress. No generic `write_uri`/
// `create_document` capability is exposed past this module -- the frontend
// never sees a `content://` string (Decision 0056 §"Export/write-back
// authority surface").
// ---------------------------------------------------------------------

#[derive(serde::Serialize)]
struct CreateExportRootRequest<'a> {
    #[serde(rename = "treeUri")]
    tree_uri: &'a str,
    name: &'a str,
}

#[derive(serde::Serialize)]
struct CreateChildDocumentRequest<'a> {
    #[serde(rename = "parentUri")]
    parent_uri: &'a str,
    name: &'a str,
    #[serde(rename = "isDirectory")]
    is_directory: bool,
}

#[derive(serde::Serialize)]
struct WriteDocumentRequest<'a> {
    uri: &'a str,
    #[serde(rename = "stagingPath")]
    staging_path: &'a str,
}

#[derive(serde::Serialize)]
struct DeleteDocumentRequest<'a> {
    uri: &'a str,
}

#[derive(serde::Serialize)]
struct CreateExportArchiveRequest<'a> {
    #[serde(rename = "suggestedName")]
    suggested_name: &'a str,
}

/// Picks a SAF destination *parent* tree for a directory export (Decision
/// 0057 §6/§12). The same picker shape as import's `pickDirectoryTree` --
/// reused deliberately rather than duplicating a second directory-tree
/// picker command.
pub fn pick_export_directory<R: Runtime>(
    handle: &PluginHandle<R>,
) -> AcquisitionResult<Option<PickedTree>> {
    let response: PickDirectoryResponse = invoke(handle, "pickExportDirectory", ())?;
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

/// Creates exactly one new export root beneath the picked parent tree
/// (WI065 Checkpoint D §6/§7: RepoPact never writes into an arbitrary
/// pre-existing destination). A same-named child that already exists is
/// reported as [`ExportRootOutcome::Conflict`], never silently merged into.
pub fn create_export_root<R: Runtime>(
    handle: &PluginHandle<R>,
    tree_uri: &str,
    name: &str,
) -> AcquisitionResult<ExportRootOutcome> {
    let response: CreateExportRootResponse = invoke(
        handle,
        "createExportRoot",
        CreateExportRootRequest { tree_uri, name },
    )?;
    match response {
        CreateExportRootResponse::Ok { root_uri } => Ok(ExportRootOutcome::Created { root_uri }),
        CreateExportRootResponse::Conflict => Ok(ExportRootOutcome::Conflict),
        CreateExportRootResponse::Error { reason } => Err(map_reason(&reason)),
    }
}

/// Deletes a document (used to clean up an app-created export root after a
/// failed or cancelled directory export -- WI065 Checkpoint D §17). Best
/// effort: the caller decides how to report a cleanup failure (a typed
/// `partial_external_export`-shaped outcome), never silently claims success
/// regardless of this call's own result.
pub fn delete_document<R: Runtime>(handle: &PluginHandle<R>, uri: &str) -> AcquisitionResult<()> {
    let response: DeleteDocumentResponse =
        invoke(handle, "deleteDocument", DeleteDocumentRequest { uri })?;
    match response {
        DeleteDocumentResponse::Ok => Ok(()),
        DeleteDocumentResponse::Error { reason } => Err(map_reason(&reason)),
    }
}

/// Picks a `CreateDocument` destination for a brand-new archive (WI065
/// Checkpoint D §10). Always a new document -- there is no "pick an
/// existing archive to overwrite" path in Stage 1.
pub fn pick_export_archive_destination<R: Runtime>(
    handle: &PluginHandle<R>,
    suggested_name: &str,
) -> AcquisitionResult<Option<PickedDocument>> {
    let response: CreateExportArchiveResponse = invoke(
        handle,
        "createExportArchive",
        CreateExportArchiveRequest { suggested_name },
    )?;
    match response {
        CreateExportArchiveResponse::Selected {
            document_uri,
            display_name,
        } => Ok(Some(PickedDocument {
            document_uri,
            display_name,
        })),
        CreateExportArchiveResponse::Cancelled => Ok(None),
        CreateExportArchiveResponse::Error { reason } => Err(map_reason(&reason)),
    }
}

/// Uploads a completed local file's bytes into an already-created SAF
/// destination document. Used both by [`AndroidFileWriter::finish`] (one
/// call per exported file, directory export) and directly by archive export
/// (one call for the whole completed ZIP -- Decision 0057 §10: stage the
/// complete archive locally, then copy it out in one step).
fn upload_staging_to_document<R: Runtime>(
    handle: &PluginHandle<R>,
    document_uri: &str,
    staging_path: &Path,
) -> AcquisitionResult<u64> {
    let response: WriteDocumentResponse = invoke(
        handle,
        "writeDocumentFromStagingFile",
        WriteDocumentRequest {
            uri: document_uri,
            staging_path: &staging_path.to_string_lossy(),
        },
    )?;
    match response {
        WriteDocumentResponse::Ok { byte_count } => Ok(byte_count),
        WriteDocumentResponse::Error { reason } => Err(map_reason(&reason)),
    }
}

/// Uploads an already-completed local archive file straight to its SAF
/// destination (WI065 Checkpoint D §10/§18). Public because
/// `mobile_acquisition`'s archive-export command builds the ZIP into a
/// local staging file with Checkpoint A/D's own bounded `create_archive`
/// before calling this -- it never streams archive bytes through this
/// bridge chunk-by-chunk.
pub fn upload_completed_archive<R: Runtime>(
    handle: &PluginHandle<R>,
    document_uri: &str,
    local_zip_path: &Path,
) -> AcquisitionResult<u64> {
    upload_staging_to_document(handle, document_uri, local_zip_path)
}

fn split_parent(relative_path: &str) -> (&str, &str) {
    match relative_path.rsplit_once('/') {
        Some((parent, name)) => (parent, name),
        None => ("", relative_path),
    }
}

/// A real Android SAF export sink (WI065 Checkpoint D §12): implements
/// [`ExportSink`] against `DocumentsContract`/`ContentResolver` exactly the
/// way [`AndroidSafSource`] implements [`AcquisitionSource`] for import --
/// this struct performs zero path validation, zero bounds enforcement, and
/// zero collision detection of its own; all of that remains
/// `repopact_mobile_acquisition::export::export_tree`'s responsibility.
/// Tracks each created directory's document URI by relative path so a
/// nested entry's immediate parent URI is always known (the exporter always
/// creates a directory before any entry beneath it, matching
/// `FilesystemSource`'s own traversal order).
pub struct AndroidExportSink<R: Runtime> {
    handle: PluginHandle<R>,
    dir_uris: HashMap<String, String>,
    staging_dir: PathBuf,
}

impl<R: Runtime> AndroidExportSink<R> {
    pub fn new(
        handle: PluginHandle<R>,
        root_uri: String,
        staging_dir: PathBuf,
    ) -> AcquisitionResult<Self> {
        fs::create_dir_all(&staging_dir).map_err(|error| {
            AcquisitionError::new(
                ErrorCode::InternalIo,
                format!("unable to create export staging directory: {error}"),
            )
        })?;
        let mut dir_uris = HashMap::new();
        dir_uris.insert(String::new(), root_uri);
        Ok(Self {
            handle,
            dir_uris,
            staging_dir,
        })
    }

    fn parent_uri(&self, relative_path: &str) -> AcquisitionResult<String> {
        let (parent, _name) = split_parent(relative_path);
        self.dir_uris.get(parent).cloned().ok_or_else(|| {
            AcquisitionError::new(
                ErrorCode::InternalIo,
                format!(
                    "export sink asked to create '{relative_path}' before its parent directory"
                ),
            )
        })
    }
}

impl<R: Runtime> ExportSink for AndroidExportSink<R> {
    fn create_directory(&mut self, relative_path: &str) -> AcquisitionResult<()> {
        let parent_uri = self.parent_uri(relative_path)?;
        let (_parent, name) = split_parent(relative_path);
        let response: CreateChildDocumentResponse = invoke(
            &self.handle,
            "createChildDocument",
            CreateChildDocumentRequest {
                parent_uri: &parent_uri,
                name,
                is_directory: true,
            },
        )?;
        match response {
            CreateChildDocumentResponse::Ok { uri } => {
                self.dir_uris.insert(relative_path.to_owned(), uri);
                Ok(())
            }
            CreateChildDocumentResponse::Error { reason } => Err(map_reason(&reason)),
        }
    }

    fn create_file(
        &mut self,
        relative_path: &str,
    ) -> AcquisitionResult<Box<dyn ExportFileWriter + '_>> {
        let parent_uri = self.parent_uri(relative_path)?;
        let (_parent, name) = split_parent(relative_path);
        let response: CreateChildDocumentResponse = invoke(
            &self.handle,
            "createChildDocument",
            CreateChildDocumentRequest {
                parent_uri: &parent_uri,
                name,
                is_directory: false,
            },
        )?;
        let uri = match response {
            CreateChildDocumentResponse::Ok { uri } => uri,
            CreateChildDocumentResponse::Error { reason } => return Err(map_reason(&reason)),
        };
        let staging_path = self
            .staging_dir
            .join(format!("{}.bin", uuid::Uuid::new_v4()));
        let file = fs::File::create(&staging_path).map_err(|error| {
            AcquisitionError::new(
                ErrorCode::InternalIo,
                format!("unable to create export staging file: {error}"),
            )
        })?;
        Ok(Box::new(AndroidFileWriter {
            handle: self.handle.clone(),
            uri,
            staging_path,
            file: Some(file),
        }))
    }
}

/// One file's worth of bytes, written to an app-private staging file first
/// and uploaded to its real SAF destination only in [`ExportFileWriter::finish`]
/// (Decision 0057 §10: stage-then-copy, never a byte-by-byte streamed write
/// straight to `ContentResolver`, which the JSON-only plugin channel cannot
/// carry anyway -- see `open_document_to_staging`'s doc comment for the
/// same constraint on the import side).
struct AndroidFileWriter<R: Runtime> {
    handle: PluginHandle<R>,
    uri: String,
    staging_path: PathBuf,
    file: Option<fs::File>,
}

impl<R: Runtime> Write for AndroidFileWriter<R> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file
            .as_mut()
            .expect("write called after finish")
            .write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file
            .as_mut()
            .expect("write called after finish")
            .flush()
    }
}

impl<R: Runtime> ExportFileWriter for AndroidFileWriter<R> {
    fn finish(mut self: Box<Self>) -> AcquisitionResult<()> {
        if let Some(file) = self.file.take() {
            file.sync_all()
                .map_err(|error| AcquisitionError::new(ErrorCode::InternalIo, error.to_string()))?;
        }
        let result = upload_staging_to_document(&self.handle, &self.uri, &self.staging_path);
        let _ = fs::remove_file(&self.staging_path);
        result.map(|_bytes| ())
    }
}

impl<R: Runtime> Drop for AndroidFileWriter<R> {
    fn drop(&mut self) {
        // If `finish` was never called (an early `?` return elsewhere in
        // the exporter, e.g. cancellation mid-write), the staging file is
        // still cleaned up here so app-private cache never accumulates a
        // leftover per aborted entry -- but no upload is attempted, since a
        // partially-written entry must never reach the real destination.
        let _ = fs::remove_file(&self.staging_path);
    }
}
