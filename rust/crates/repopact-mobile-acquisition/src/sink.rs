//! `ExportSink` is the write-side mirror of [`crate::source::AcquisitionSource`]
//! (WI065 Checkpoint D): the platform-neutral boundary between "the bounded
//! Rust-owned exporter knows what to write" and "however those bytes
//! actually land on a destination." A real Android SAF sink
//! (`AndroidSafExportSink` in `repopact-mobile-saf`) and this crate's own
//! [`FilesystemSink`] (desktop export, and host-side adversarial/unit
//! testing without any Android dependency) both implement the same trait,
//! so [`crate::export::export_tree`] never needs to know which one it is
//! writing to -- exactly the same layering [`crate::import::import_directory`]
//! already established for reading.
//!
//! Rust owns traversal order, relative-path validation, resource
//! accounting, collision/bounds policy, progress, and cancellation. A sink
//! implementation owns only "how do I actually create this directory / this
//! file, and accept these bytes" -- it performs no path-safety validation
//! of its own (Decision 0057 §12).

use std::io::Write;

use crate::error::AcquisitionResult;

/// A file write in progress. Implementations return this from
/// [`ExportSink::create_file`]; the exporter calls [`Write`] as many times
/// as needed, then calls [`ExportFileWriter::finish`] exactly once when the
/// entry's bytes are fully written. `finish` -- not `Drop` -- is the point
/// at which a sink is allowed to do anything fallible (e.g. Android's real
/// sink uploads a completed local staging file to its SAF destination
/// document only in `finish`, so an upload failure surfaces as a real
/// `AcquisitionResult::Err` rather than being swallowed in a `Drop` impl).
pub trait ExportFileWriter: Write {
    fn finish(self: Box<Self>) -> AcquisitionResult<()>;
}

pub trait ExportSink {
    /// Creates a directory at `relative_path` (`/`-separated, already
    /// validated by the caller). The immediate parent directory is always
    /// created first (the exporter walks in that order), so a sink may
    /// assume its parent already exists.
    fn create_directory(&mut self, relative_path: &str) -> AcquisitionResult<()>;

    /// Begins a new file at `relative_path`. The immediate parent directory
    /// is always created first. The caller must write the entry's full
    /// contents and then call [`ExportFileWriter::finish`] before moving on
    /// to the next entry -- a sink may assume at most one file is being
    /// written at a time.
    fn create_file(
        &mut self,
        relative_path: &str,
    ) -> AcquisitionResult<Box<dyn ExportFileWriter + '_>>;
}

/// An export sink backed by an ordinary local directory. Used for desktop's
/// own export path and for this crate's host-side export tests without any
/// Android dependency -- a future Android SAF sink implements the same
/// trait against `DocumentsContract`/`ContentResolver` instead.
pub struct FilesystemSink {
    root: std::path::PathBuf,
}

impl FilesystemSink {
    /// `root` must already exist (the caller is responsible for creating
    /// the destination root itself -- see Decision 0057 §"Export root"
    /// semantics in `crate::export`).
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

struct FilesystemFileWriter {
    file: std::fs::File,
}

impl Write for FilesystemFileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl ExportFileWriter for FilesystemFileWriter {
    fn finish(self: Box<Self>) -> AcquisitionResult<()> {
        self.file.sync_all().map_err(|error| {
            crate::error::AcquisitionError::new(
                crate::error::ErrorCode::InternalIo,
                error.to_string(),
            )
        })
    }
}

impl ExportSink for FilesystemSink {
    fn create_directory(&mut self, relative_path: &str) -> AcquisitionResult<()> {
        let dest = crate::paths::safe_join(&self.root, relative_path)?;
        std::fs::create_dir_all(&dest).map_err(|error| {
            crate::error::AcquisitionError::new(
                crate::error::ErrorCode::InternalIo,
                format!("unable to create directory '{}': {error}", dest.display()),
            )
        })
    }

    fn create_file(
        &mut self,
        relative_path: &str,
    ) -> AcquisitionResult<Box<dyn ExportFileWriter + '_>> {
        let dest = crate::paths::safe_join(&self.root, relative_path)?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                crate::error::AcquisitionError::new(
                    crate::error::ErrorCode::InternalIo,
                    format!("unable to create parent for '{}': {error}", dest.display()),
                )
            })?;
        }
        let file = std::fs::File::create(&dest).map_err(|error| {
            crate::error::AcquisitionError::new(
                crate::error::ErrorCode::InternalIo,
                format!("unable to create '{}': {error}", dest.display()),
            )
        })?;
        Ok(Box::new(FilesystemFileWriter { file }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_sink_creates_directories_and_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut sink = FilesystemSink::new(dir.path());
        sink.create_directory("nested").unwrap();
        let mut writer = sink.create_file("nested/file.txt").unwrap();
        writer.write_all(b"hello").unwrap();
        writer.finish().unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("nested/file.txt")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn filesystem_sink_rejects_path_escape() {
        let dir = tempfile::tempdir().unwrap();
        let mut sink = FilesystemSink::new(dir.path());
        let err = sink.create_directory("../escape").unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::PathEscape);
    }
}
