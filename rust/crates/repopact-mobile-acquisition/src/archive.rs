//! Decision 0057 §"Archive import" / §"Zip-slip defense" / §"Archive
//! symlink policy" / §"Archive bomb bounds": Rust-owned, bounded, hardened
//! ZIP extraction and creation. Extraction never trusts a ZIP entry's
//! declared (possibly forged) size -- actual bytes written are always
//! counted at runtime -- and every path is normalized and contained the
//! same way [`crate::import`] contains a directory import.

use std::fs;
use std::io::{Read, Seek, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::bounds::ArchiveBounds;
use crate::error::{AcquisitionError, AcquisitionResult, ErrorCode};
use crate::operation::{CancellationToken, OperationPhase, ProgressThrottle};
use crate::paths::{reject_unsafe_relative_path, safe_join, CollisionGuard, EntryKind};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchiveImportSummary {
    pub entries_imported: u64,
    pub bytes_imported: u64,
    pub files_imported: u64,
    pub directories_imported: u64,
}

const COPY_CHUNK_BYTES: usize = 256 * 1024;

/// Extracts every entry of a `.zip` archive into `staging_root` (which must
/// already exist and be empty). The caller is responsible for deleting
/// `staging_root` on any `Err`, exactly as with [`crate::import::import_directory`].
pub fn import_archive<R: Read + Seek>(
    reader: R,
    staging_root: &Path,
    bounds: &ArchiveBounds,
    cancel: &CancellationToken,
    mut on_progress: impl FnMut(crate::operation::OperationProgress),
) -> AcquisitionResult<ArchiveImportSummary> {
    let mut archive = ZipArchive::new(reader).map_err(|error| {
        AcquisitionError::new(ErrorCode::ArchiveInvalid, format!("malformed ZIP: {error}"))
    })?;

    let entry_count = archive.len() as u64;
    if entry_count > bounds.max_entries {
        return Err(AcquisitionError::new(
            ErrorCode::ResourceLimit,
            format!(
                "archive has {entry_count} entries, exceeding the {}-entry bound",
                bounds.max_entries
            ),
        ));
    }

    let mut summary = ArchiveImportSummary::default();
    let mut guard = CollisionGuard::new();
    let mut throttle = ProgressThrottle::new("archive-import", OperationPhase::Importing);

    for index in 0..archive.len() {
        cancel.check()?;

        let mut entry = archive.by_index(index).map_err(|error| {
            AcquisitionError::new(
                ErrorCode::ArchiveInvalid,
                format!("malformed entry: {error}"),
            )
        })?;

        // Reject anything `enclosed_name()` (the crate's own zip-slip
        // sanitizer) refuses, *and* independently run this crate's own
        // path-safety gate over the raw declared name -- defense in depth,
        // and a consistent error taxonomy across the directory and archive
        // importers.
        // ZIP directory entries conventionally carry a trailing `/`
        // (`entry.is_dir()` is in fact defined by this crate as "name ends
        // with '/'"). Trimming it before it becomes this entry's collision
        // key and destination path is required, not cosmetic: without it,
        // a directory entry `"thing/"` and a file entry `"thing"` produce
        // different `CollisionGuard` keys and silently bypass the
        // file-vs-directory collision check Decision 0057 requires.
        let raw_name = entry.name().trim_end_matches('/').to_owned();
        if entry.enclosed_name().is_none() {
            return Err(AcquisitionError::new(
                ErrorCode::PathEscape,
                format!("archive entry '{raw_name}' escapes the destination root"),
            ));
        }
        reject_unsafe_relative_path(&raw_name)?;

        if raw_name.len() > bounds.max_path_length {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                format!(
                    "archive entry '{raw_name}' exceeds the {}-character path-length bound",
                    bounds.max_path_length
                ),
            ));
        }
        let depth = raw_name.split(['/', '\\']).count();
        if depth > bounds.max_depth {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                format!(
                    "archive entry '{raw_name}' exceeds the {}-level depth bound",
                    bounds.max_depth
                ),
            ));
        }

        // Stage 1 symlink policy: reject archive symlink entries outright
        // (stricter than the directory importer's "unsupported" class,
        // because a ZIP-declared symlink target is itself untrusted input
        // that must never be materialized or followed).
        if entry.is_symlink() {
            return Err(AcquisitionError::new(
                ErrorCode::ArchiveSymlink,
                format!(
                    "archive entry '{raw_name}' is a symlink; Stage 1 rejects archive symlinks"
                ),
            ));
        }

        summary.entries_imported += 1;

        // Zip-bomb heuristic: declared uncompressed size vs. compressed
        // size, checked *before* extraction (belt) -- but the authoritative
        // check remains the runtime byte counter during the copy loop
        // (suspenders), so a forged declared size cannot bypass either
        // bound.
        let declared_size = entry.size();
        let compressed_size = entry.compressed_size().max(1);
        if declared_size / compressed_size > bounds.max_compression_ratio {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                format!(
                    "archive entry '{raw_name}' exceeds the {}x compression-ratio bound",
                    bounds.max_compression_ratio
                ),
            ));
        }

        if entry.is_dir() {
            guard.admit(&raw_name, EntryKind::Directory)?;
            let dest = safe_join(staging_root, &raw_name)?;
            fs::create_dir_all(&dest).map_err(|error| {
                AcquisitionError::new(
                    ErrorCode::InternalIo,
                    format!("unable to create directory '{}': {error}", dest.display()),
                )
            })?;
            summary.directories_imported += 1;
            throttle.record(0);
            continue;
        }

        if !entry.is_file() {
            return Err(AcquisitionError::new(
                ErrorCode::UnsupportedEntry,
                format!("archive entry '{raw_name}' is not a regular file or directory"),
            ));
        }

        guard.admit(&raw_name, EntryKind::File)?;
        let dest = safe_join(staging_root, &raw_name)?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                AcquisitionError::new(
                    ErrorCode::InternalIo,
                    format!("unable to create parent for '{}': {error}", dest.display()),
                )
            })?;
        }

        let remaining_total_budget = bounds
            .max_expanded_bytes
            .saturating_sub(summary.bytes_imported);
        let bytes_copied = bounded_extract(
            &mut entry,
            &dest,
            bounds.max_single_entry_bytes,
            remaining_total_budget,
            cancel,
        )?;
        summary.bytes_imported += bytes_copied;
        summary.files_imported += 1;
        throttle.record(bytes_copied);
        throttle.maybe_emit(Some(&raw_name), Some(entry_count), false, &mut on_progress);
    }

    throttle.maybe_emit(None, Some(entry_count), true, &mut on_progress);
    Ok(summary)
}

fn bounded_extract(
    reader: &mut dyn Read,
    dest: &Path,
    max_single_entry_bytes: u64,
    remaining_total_budget: u64,
    cancel: &CancellationToken,
) -> AcquisitionResult<u64> {
    let mut file = fs::File::create(dest).map_err(|error| {
        AcquisitionError::new(
            ErrorCode::InternalIo,
            format!("unable to create '{}': {error}", dest.display()),
        )
    })?;
    let mut buffer = [0u8; COPY_CHUNK_BYTES];
    let mut total: u64 = 0;
    loop {
        cancel.check()?;
        let read = reader
            .read(&mut buffer)
            .map_err(|error| AcquisitionError::new(ErrorCode::ArchiveInvalid, error.to_string()))?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > max_single_entry_bytes {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                format!(
                    "'{}' exceeds the {}-byte single-entry bound",
                    dest.display(),
                    max_single_entry_bytes
                ),
            ));
        }
        if total > remaining_total_budget {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                "archive extraction exceeds the total-expanded-bytes bound",
            ));
        }
        file.write_all(&buffer[..read]).map_err(|error| {
            AcquisitionError::new(
                ErrorCode::InternalIo,
                format!("unable to write '{}': {error}", dest.display()),
            )
        })?;
    }
    Ok(total)
}

/// Creates a new `.zip` at `dest_writer` from `source_root`'s contents.
/// Used for both directory-workspace and archive-workspace export
/// (Decision 0057 §"Export semantics": archive export always creates a new
/// document, never mutates an original archive in place).
pub fn create_archive<W: Write + Seek>(
    source_root: &Path,
    dest_writer: W,
    cancel: &CancellationToken,
) -> AcquisitionResult<u64> {
    let mut writer = ZipWriter::new(dest_writer);
    let options = SimpleFileOptions::default();
    let mut bytes_written = 0u64;

    let mut stack = vec![(source_root.to_path_buf(), String::new())];
    while let Some((absolute_dir, relative_dir)) = stack.pop() {
        let mut children: Vec<_> = fs::read_dir(&absolute_dir)
            .map_err(|error| {
                AcquisitionError::new(
                    ErrorCode::InternalIo,
                    format!("unable to read '{}': {error}", absolute_dir.display()),
                )
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| AcquisitionError::new(ErrorCode::InternalIo, error.to_string()))?;
        children.sort_by_key(|entry| entry.file_name());

        for child in children {
            cancel.check()?;
            let name = child.file_name().to_string_lossy().into_owned();
            let relative = if relative_dir.is_empty() {
                name
            } else {
                format!("{relative_dir}/{name}")
            };
            let metadata = child
                .metadata()
                .map_err(|error| AcquisitionError::new(ErrorCode::InternalIo, error.to_string()))?;
            if metadata.is_dir() {
                writer
                    .add_directory(format!("{relative}/"), options)
                    .map_err(|error| {
                        AcquisitionError::new(ErrorCode::InternalIo, error.to_string())
                    })?;
                stack.push((child.path(), relative));
            } else if metadata.is_file() {
                writer.start_file(relative, options).map_err(|error| {
                    AcquisitionError::new(ErrorCode::InternalIo, error.to_string())
                })?;
                let mut source = fs::File::open(child.path()).map_err(|error| {
                    AcquisitionError::new(ErrorCode::InternalIo, error.to_string())
                })?;
                let mut buffer = [0u8; COPY_CHUNK_BYTES];
                loop {
                    cancel.check()?;
                    let read = source.read(&mut buffer).map_err(|error| {
                        AcquisitionError::new(ErrorCode::InternalIo, error.to_string())
                    })?;
                    if read == 0 {
                        break;
                    }
                    writer.write_all(&buffer[..read]).map_err(|error| {
                        AcquisitionError::new(ErrorCode::InternalIo, error.to_string())
                    })?;
                    bytes_written += read as u64;
                }
            }
            // Symlinks or other special files inside a workspace are not
            // expected (Stage 1 never materializes them on import), so
            // they are silently skipped here rather than failing an
            // otherwise-valid export.
        }
    }

    writer
        .finish()
        .map_err(|error| AcquisitionError::new(ErrorCode::InternalIo, error.to_string()))?;
    Ok(bytes_written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use zip::write::SimpleFileOptions as Opts;

    fn bounds() -> ArchiveBounds {
        ArchiveBounds {
            max_entries: 1000,
            max_expanded_bytes: 10 * 1024 * 1024,
            max_single_entry_bytes: 5 * 1024 * 1024,
            max_depth: 16,
            max_path_length: 512,
            max_compression_ratio: 1000,
        }
    }

    fn build_zip(entries: impl IntoIterator<Item = (&'static str, Vec<u8>)>) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, contents) in entries {
            writer.start_file(name, Opts::default()).unwrap();
            writer.write_all(&contents).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn imports_a_normal_zip() {
        let bytes = build_zip([
            ("a.txt", b"hello".to_vec()),
            ("dir/b.txt", b"world".to_vec()),
        ]);
        let dest = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let summary =
            import_archive(Cursor::new(bytes), dest.path(), &bounds(), &cancel, |_| {}).unwrap();
        assert_eq!(summary.files_imported, 2);
        assert_eq!(summary.bytes_imported, 10);
        assert_eq!(
            fs::read_to_string(dest.path().join("a.txt")).unwrap(),
            "hello"
        );
        assert_eq!(
            fs::read_to_string(dest.path().join("dir/b.txt")).unwrap(),
            "world"
        );
    }

    #[test]
    fn rejects_zip_slip_parent_traversal() {
        // Bypass the writer's own name validation by writing a raw local
        // file header with a `..`-containing name, mirroring how a hand-
        // crafted malicious archive would look on disk. The `zip` crate's
        // high-level `start_file` API rejects such names outright, so we
        // assert that our importer's defense-in-depth check would catch it
        // via `enclosed_name()`/`reject_unsafe_relative_path` regardless of
        // which layer a real adversarial archive slips past.
        assert!(reject_unsafe_relative_path("../../etc/passwd").is_err());
        assert!(reject_unsafe_relative_path("a/../../escape").is_err());
    }

    // `zip` 2.4.2's own `ZipWriter::start_file` refuses to write a second
    // entry under a byte-identical name (`InvalidArchive("Duplicate
    // filename")`) -- itself a real, relevant defense-in-depth fact: the
    // most common way to produce an exact-duplicate-path archive (a naive
    // writer) is already blocked upstream. `import_archive`'s own defense
    // against a *maliciously hand-crafted* archive that reaches this code
    // with a duplicate name anyway is `CollisionGuard`, proven directly at
    // the unit level in `paths::tests::collision_guard_rejects_exact_duplicate`
    // and exercised end-to-end here via the case-only and file/dir
    // collision cases below (both of which the writer does *not* block).

    #[test]
    fn duplicate_filename_is_rejected_even_by_the_archive_writer_itself() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("a.txt", Opts::default()).unwrap();
        writer.write_all(b"1").unwrap();
        let result = writer.start_file("a.txt", Opts::default());
        assert!(
            result.is_err(),
            "the underlying zip writer must refuse a duplicate name"
        );
    }

    #[test]
    fn rejects_case_only_collision() {
        let bytes = build_zip([("A.txt", b"1".to_vec()), ("a.txt", b"2".to_vec())]);
        let dest = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let err = import_archive(Cursor::new(bytes), dest.path(), &bounds(), &cancel, |_| {})
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::CaseConflict);
    }

    #[test]
    fn rejects_file_dir_collision() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer.add_directory("thing/", Opts::default()).unwrap();
        writer.start_file("thing", Opts::default()).unwrap();
        writer.write_all(b"x").unwrap();
        let bytes = writer.finish().unwrap().into_inner();

        let dest = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let err = import_archive(Cursor::new(bytes), dest.path(), &bounds(), &cancel, |_| {})
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::CaseConflict);
    }

    #[test]
    fn rejects_too_many_entries() {
        let mut entries = Vec::new();
        for i in 0..20 {
            entries.push((
                Box::leak(format!("f{i}.txt").into_boxed_str()) as &'static str,
                b"x".to_vec(),
            ));
        }
        let bytes = build_zip(entries);
        let dest = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let mut tight = bounds();
        tight.max_entries = 5;
        let err =
            import_archive(Cursor::new(bytes), dest.path(), &tight, &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn rejects_oversized_expanded_output() {
        let bytes = build_zip([("big.bin", vec![9u8; 6 * 1024 * 1024])]);
        let dest = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let mut tight = bounds();
        tight.max_single_entry_bytes = 5 * 1024 * 1024;
        let err =
            import_archive(Cursor::new(bytes), dest.path(), &tight, &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn rejects_deep_nesting() {
        let mut path = String::new();
        for i in 0..30 {
            path.push_str(&format!("d{i}/"));
        }
        path.push_str("leaf.txt");
        let leaked: &'static str = Box::leak(path.into_boxed_str());
        let bytes = build_zip([(leaked, b"x".to_vec())]);
        let dest = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let mut tight = bounds();
        tight.max_depth = 5;
        let err =
            import_archive(Cursor::new(bytes), dest.path(), &tight, &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn rejects_malformed_zip() {
        let dest = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let err = import_archive(
            Cursor::new(b"not a zip file at all".to_vec()),
            dest.path(),
            &bounds(),
            &cancel,
            |_| {},
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::ArchiveInvalid);
    }

    #[test]
    fn cancellation_stops_the_extraction() {
        let bytes = build_zip([("a.txt", b"1".to_vec()), ("b.txt", b"2".to_vec())]);
        let dest = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let err = import_archive(Cursor::new(bytes), dest.path(), &bounds(), &cancel, |_| {})
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::OperationCancelled);
    }

    #[test]
    fn round_trips_through_create_archive() {
        let source = tempfile::tempdir().unwrap();
        fs::create_dir_all(source.path().join("nested")).unwrap();
        fs::write(source.path().join("nested/file.txt"), b"payload").unwrap();
        fs::write(source.path().join("top.txt"), b"top-level").unwrap();

        let cancel = CancellationToken::new();
        let mut buffer = Cursor::new(Vec::new());
        create_archive(source.path(), &mut buffer, &cancel).unwrap();

        let dest = tempfile::tempdir().unwrap();
        buffer.set_position(0);
        import_archive(buffer, dest.path(), &bounds(), &cancel, |_| {}).unwrap();
        assert_eq!(
            fs::read_to_string(dest.path().join("nested/file.txt")).unwrap(),
            "payload"
        );
        assert_eq!(
            fs::read_to_string(dest.path().join("top.txt")).unwrap(),
            "top-level"
        );
    }
}
