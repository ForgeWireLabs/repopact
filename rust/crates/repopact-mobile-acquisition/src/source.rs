//! `AcquisitionSource` is the platform-neutral boundary between "however a
//! tree of bytes was picked" and the bounded Rust-owned importer (Decision
//! 0057 §"SAF native boundary" / §61 layering). A real Android SAF bridge
//! and this crate's own `FilesystemSource` (used for desktop regression and
//! for host-side adversarial testing without any Android dependency) both
//! implement the same trait, so the bounded import algorithm in
//! [`crate::import`] never needs to know which one it is walking.
//!
//! Streaming by construction: entries are produced one at a time and a
//! file's bytes are only opened for reading when the importer actually
//! copies it, so a source is never required to materialize its entire tree
//! in memory before copying begins.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::{AcquisitionError, AcquisitionResult, ErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceEntryKind {
    Directory,
    File,
    /// A symlink or other special entry type this source cannot express as
    /// an ordinary file/directory (Decision 0057's symlink policy: never
    /// materialized, never followed).
    Unsupported,
}

pub struct SourceEntry {
    /// `/`-separated, relative to the source root. Not yet validated —
    /// callers must run it through [`crate::paths::reject_unsafe_relative_path`]
    /// before using it.
    pub relative_path: String,
    pub kind: SourceEntryKind,
    /// Source-reported size, when cheaply available. Never trusted as an
    /// upper bound by the importer — actual bytes copied are always counted
    /// at runtime.
    pub size_hint: Option<u64>,
}

pub trait AcquisitionSource {
    /// Returns the next entry, or `Ok(None)` when the source is exhausted.
    fn next_entry(&mut self) -> AcquisitionResult<Option<SourceEntry>>;

    /// Opens the current file entry for reading. Only ever called
    /// immediately after `next_entry` returned a `SourceEntryKind::File`
    /// entry with that entry's `relative_path`, and before the next call to
    /// `next_entry`.
    fn open_file(&mut self, relative_path: &str) -> AcquisitionResult<Box<dyn Read + '_>>;
}

/// A source backed by an ordinary local directory. Used for desktop's own
/// "native picker -> PathBuf -> DesktopService" regression path (which does
/// not go through the mobile importer at all, per Decision 0057) and, here,
/// as the concrete implementation this crate's adversarial tests exercise
/// without any Android dependency. A future Android SAF bridge implements
/// the same trait against `DocumentsContract`/`DocumentFile` instead.
pub struct FilesystemSource {
    root: PathBuf,
    stack: Vec<walk::WalkEntry>,
}

mod walk {
    use std::path::PathBuf;

    pub struct WalkEntry {
        pub absolute: PathBuf,
        pub relative: String,
    }
}

impl FilesystemSource {
    pub fn new(root: impl Into<PathBuf>) -> AcquisitionResult<Self> {
        let root = root.into();
        if !root.is_dir() {
            return Err(AcquisitionError::new(
                ErrorCode::SourceUnavailable,
                format!("'{}' is not a directory", root.display()),
            ));
        }
        // Seed with the root's immediate children; each directory
        // encountered is pushed onto the stack lazily as we go, so the
        // whole tree is never listed upfront.
        let mut stack = Vec::new();
        push_children(&root, "", &mut stack)?;
        Ok(Self { root, stack })
    }
}

fn push_children(
    absolute_dir: &Path,
    relative_dir: &str,
    stack: &mut Vec<walk::WalkEntry>,
) -> AcquisitionResult<()> {
    let mut children: Vec<_> = fs::read_dir(absolute_dir)
        .map_err(|error| {
            AcquisitionError::new(
                ErrorCode::SourceUnavailable,
                format!(
                    "unable to read directory '{}': {error}",
                    absolute_dir.display()
                ),
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AcquisitionError::new(ErrorCode::SourceUnavailable, error.to_string()))?;
    // Deterministic order (stable tests, stable progress reporting).
    children.sort_by_key(|entry| entry.file_name());
    // Push in reverse so popping the stack yields ascending order.
    for entry in children.into_iter().rev() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = if relative_dir.is_empty() {
            name
        } else {
            format!("{relative_dir}/{name}")
        };
        stack.push(walk::WalkEntry {
            absolute: entry.path(),
            relative,
        });
    }
    Ok(())
}

impl AcquisitionSource for FilesystemSource {
    fn next_entry(&mut self) -> AcquisitionResult<Option<SourceEntry>> {
        let Some(entry) = self.stack.pop() else {
            return Ok(None);
        };
        let metadata = fs::symlink_metadata(&entry.absolute).map_err(|error| {
            AcquisitionError::new(
                ErrorCode::SourceUnavailable,
                format!("unable to stat '{}': {error}", entry.absolute.display()),
            )
        })?;
        if metadata.is_symlink() {
            return Ok(Some(SourceEntry {
                relative_path: entry.relative,
                kind: SourceEntryKind::Unsupported,
                size_hint: None,
            }));
        }
        if metadata.is_dir() {
            push_children(&entry.absolute, &entry.relative, &mut self.stack)?;
            return Ok(Some(SourceEntry {
                relative_path: entry.relative,
                kind: SourceEntryKind::Directory,
                size_hint: None,
            }));
        }
        if metadata.is_file() {
            return Ok(Some(SourceEntry {
                relative_path: entry.relative,
                kind: SourceEntryKind::File,
                size_hint: Some(metadata.len()),
            }));
        }
        Ok(Some(SourceEntry {
            relative_path: entry.relative,
            kind: SourceEntryKind::Unsupported,
            size_hint: None,
        }))
    }

    fn open_file(&mut self, relative_path: &str) -> AcquisitionResult<Box<dyn Read + '_>> {
        let absolute = self.root.join(relative_path);
        let file = fs::File::open(&absolute).map_err(|error| {
            AcquisitionError::new(
                ErrorCode::SourceUnavailable,
                format!("unable to open '{}': {error}", absolute.display()),
            )
        })?;
        Ok(Box::new(file))
    }
}
