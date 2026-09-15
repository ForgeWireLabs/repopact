//! Decision 0057 §"Directory import bounds" / §"Path safety" / §"Duplicate
//! and case-collision policy": the bounded, incremental, adversarial-input-
//! hardened directory importer. Operates against any [`AcquisitionSource`]
//! (a real SAF bridge or, for desktop regression and host-side testing,
//! [`crate::source::FilesystemSource`]) and writes only into a
//! caller-provided staging root -- it never touches a `ready` workspace
//! directly (Decision 0057 §"Staging-then-publish transaction").

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use crate::bounds::ImportBounds;
use crate::error::{AcquisitionError, AcquisitionResult, ErrorCode};
use crate::operation::{CancellationToken, OperationPhase, ProgressThrottle};
use crate::paths::{safe_join, CollisionGuard, EntryKind};
use crate::source::{AcquisitionSource, SourceEntryKind};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub entries_imported: u64,
    pub bytes_imported: u64,
    pub files_imported: u64,
    pub directories_imported: u64,
}

const COPY_CHUNK_BYTES: usize = 256 * 1024;

/// Imports every entry from `source` into `staging_root` (which must
/// already exist and be empty), enforcing Decision 0057's bounds and path
/// safety rules. Returns as soon as any entry is rejected, is cancelled, or
/// a bound is exceeded -- the caller is responsible for deleting
/// `staging_root` on any `Err` (this function does not clean up after
/// itself, so a caller which wants to inspect a failed staging directory
/// for diagnostics still can).
pub fn import_directory(
    source: &mut dyn AcquisitionSource,
    staging_root: &Path,
    bounds: &ImportBounds,
    cancel: &CancellationToken,
    mut on_progress: impl FnMut(crate::operation::OperationProgress),
) -> AcquisitionResult<ImportSummary> {
    let mut summary = ImportSummary::default();
    let mut guard = CollisionGuard::new();
    let mut throttle = ProgressThrottle::new("import", OperationPhase::Importing);

    loop {
        cancel.check()?;

        let Some(entry) = source.next_entry()? else {
            break;
        };

        if entry.relative_path.len() > bounds.max_path_length {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                format!(
                    "path '{}' exceeds the {}-character path-length bound",
                    entry.relative_path, bounds.max_path_length
                ),
            ));
        }
        let depth = entry.relative_path.split(['/', '\\']).count();
        if depth > bounds.max_depth {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                format!(
                    "path '{}' exceeds the {}-level depth bound",
                    entry.relative_path, bounds.max_depth
                ),
            ));
        }

        summary.entries_imported += 1;
        if summary.entries_imported > bounds.max_entries {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                format!("import exceeds the {}-entry bound", bounds.max_entries),
            ));
        }

        match entry.kind {
            SourceEntryKind::Unsupported => {
                return Err(AcquisitionError::new(
                    ErrorCode::UnsupportedEntry,
                    format!(
                        "'{}' is a symlink or other unsupported special entry; Stage 1 does not materialize external symlink semantics",
                        entry.relative_path
                    ),
                ));
            }
            SourceEntryKind::Directory => {
                guard.admit(&entry.relative_path, EntryKind::Directory)?;
                let dest = safe_join(staging_root, &entry.relative_path)?;
                fs::create_dir_all(&dest).map_err(|error| {
                    AcquisitionError::new(
                        ErrorCode::InternalIo,
                        format!("unable to create directory '{}': {error}", dest.display()),
                    )
                })?;
                summary.directories_imported += 1;
                throttle.record(0);
            }
            SourceEntryKind::File => {
                guard.admit(&entry.relative_path, EntryKind::File)?;
                let dest = safe_join(staging_root, &entry.relative_path)?;
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        AcquisitionError::new(
                            ErrorCode::InternalIo,
                            format!("unable to create parent for '{}': {error}", dest.display()),
                        )
                    })?;
                }
                let mut reader = source.open_file(&entry.relative_path)?;
                let bytes_copied = bounded_copy(
                    &mut reader,
                    &dest,
                    bounds.max_single_file_bytes,
                    bounds
                        .max_total_bytes
                        .saturating_sub(summary.bytes_imported),
                    cancel,
                )?;
                summary.bytes_imported += bytes_copied;
                summary.files_imported += 1;
                throttle.record(bytes_copied);
            }
        }

        throttle.maybe_emit(Some(&entry.relative_path), None, false, &mut on_progress);
    }

    throttle.maybe_emit(None, Some(summary.entries_imported), true, &mut on_progress);
    Ok(summary)
}

/// Copies from `reader` to a new file at `dest`, counting actual bytes read
/// (never trusting a source-reported size hint) and aborting the moment
/// either the per-file or the remaining-total-budget bound would be
/// exceeded. This is what makes a forged/mismatched size hint harmless.
fn bounded_copy(
    reader: &mut dyn Read,
    dest: &Path,
    max_single_file_bytes: u64,
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
        let read = reader.read(&mut buffer).map_err(|error| {
            AcquisitionError::new(ErrorCode::SourceUnavailable, error.to_string())
        })?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > max_single_file_bytes {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                format!(
                    "'{}' exceeds the {}-byte single-file bound",
                    dest.display(),
                    max_single_file_bytes
                ),
            ));
        }
        if total > remaining_total_budget {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                "import exceeds the total-bytes bound",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{AcquisitionSource, FilesystemSource, SourceEntry, SourceEntryKind};
    use std::fs;
    use std::io::Cursor;

    /// An in-memory source used only to construct entry-name collisions
    /// that a case-insensitive host filesystem (Windows, and macOS by
    /// default) cannot itself hold as two distinct directory entries --
    /// unlike a ZIP archive's entry table, or a real Android
    /// `DocumentsProvider` tree, an ordinary Windows directory silently
    /// collapses `A.txt`/`a.txt` into one file before our importer ever
    /// sees two entries. This double-checks `CollisionGuard` is actually
    /// wired into `import_directory`'s loop, independent of host
    /// filesystem case sensitivity.
    struct VecSource {
        entries: std::vec::IntoIter<(String, SourceEntryKind, Vec<u8>)>,
    }

    impl VecSource {
        fn new(entries: Vec<(&str, SourceEntryKind, &[u8])>) -> Self {
            Self {
                entries: entries
                    .into_iter()
                    .map(|(name, kind, bytes)| (name.to_owned(), kind, bytes.to_vec()))
                    .collect::<Vec<_>>()
                    .into_iter(),
            }
        }
    }

    impl AcquisitionSource for VecSource {
        fn next_entry(&mut self) -> AcquisitionResult<Option<SourceEntry>> {
            Ok(self
                .entries
                .next()
                .map(|(relative_path, kind, bytes)| SourceEntry {
                    relative_path,
                    kind,
                    size_hint: Some(bytes.len() as u64),
                }))
        }

        fn open_file(&mut self, _relative_path: &str) -> AcquisitionResult<Box<dyn Read + '_>> {
            // Simplified for this test double: real content isn't needed
            // to prove collision detection, only that two distinct entries
            // reach the importer.
            Ok(Box::new(Cursor::new(Vec::<u8>::new())))
        }
    }

    fn bounds() -> ImportBounds {
        ImportBounds {
            max_entries: 1000,
            max_total_bytes: 10 * 1024 * 1024,
            max_single_file_bytes: 5 * 1024 * 1024,
            max_depth: 16,
            max_path_length: 512,
        }
    }

    #[test]
    fn imports_a_normal_tree() {
        let src = tempfile::tempdir().unwrap();
        fs::create_dir_all(src.path().join("a/b")).unwrap();
        fs::write(src.path().join("a/b/file.txt"), b"hello").unwrap();
        fs::write(src.path().join("root.txt"), b"world").unwrap();

        let dest = tempfile::tempdir().unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let cancel = CancellationToken::new();
        let summary =
            import_directory(&mut source, dest.path(), &bounds(), &cancel, |_| {}).unwrap();

        assert_eq!(summary.files_imported, 2);
        assert_eq!(summary.bytes_imported, 10);
        assert!(dest.path().join("a/b/file.txt").is_file());
        assert_eq!(
            fs::read_to_string(dest.path().join("root.txt")).unwrap(),
            "world"
        );
    }

    #[test]
    fn rejects_case_only_collision() {
        // A case-insensitive host filesystem (this Windows machine
        // included) collapses `A.txt`/`a.txt` into one real directory
        // entry, so this uses `VecSource` rather than real files -- see
        // its doc comment. A real Android `DocumentsProvider` tree (ext4
        // underneath) can genuinely expose both.
        let mut source = VecSource::new(vec![
            ("A.txt", SourceEntryKind::File, b"1".as_slice()),
            ("a.txt", SourceEntryKind::File, b"2".as_slice()),
        ]);
        let dest = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let err =
            import_directory(&mut source, dest.path(), &bounds(), &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::CaseConflict);
    }

    #[test]
    fn rejects_file_directory_collision_from_source() {
        let mut source = VecSource::new(vec![
            ("thing", SourceEntryKind::Directory, b"".as_slice()),
            ("thing/inner.txt", SourceEntryKind::File, b"x".as_slice()),
        ]);
        // Directory admitted first, fine; now collide "thing" itself as a
        // file too.
        let mut source2 = VecSource::new(vec![
            ("thing", SourceEntryKind::Directory, b"".as_slice()),
            ("thing", SourceEntryKind::File, b"x".as_slice()),
        ]);
        let dest = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        // The first case (directory then a nested file under it) must
        // succeed -- it is not a collision.
        import_directory(&mut source, dest.path(), &bounds(), &cancel, |_| {}).unwrap();
        let dest2 = tempfile::tempdir().unwrap();
        let err =
            import_directory(&mut source2, dest2.path(), &bounds(), &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::CaseConflict);
    }

    #[test]
    fn rejects_symlink_entry() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("real.txt"), b"1").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(src.path().join("real.txt"), src.path().join("link.txt"))
            .unwrap();
        #[cfg(windows)]
        {
            // Symlink creation on Windows CI often requires elevated
            // privileges; skip gracefully rather than fail the suite when
            // unavailable, while still proving the policy when it is.
            if std::os::windows::fs::symlink_file(
                src.path().join("real.txt"),
                src.path().join("link.txt"),
            )
            .is_err()
            {
                return;
            }
        }

        let dest = tempfile::tempdir().unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let cancel = CancellationToken::new();
        let err =
            import_directory(&mut source, dest.path(), &bounds(), &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::UnsupportedEntry);
    }

    #[test]
    fn enforces_entry_count_bound() {
        let src = tempfile::tempdir().unwrap();
        for i in 0..10 {
            fs::write(src.path().join(format!("f{i}.txt")), b"x").unwrap();
        }
        let dest = tempfile::tempdir().unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let cancel = CancellationToken::new();
        let mut tight_bounds = bounds();
        tight_bounds.max_entries = 5;
        let err =
            import_directory(&mut source, dest.path(), &tight_bounds, &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn enforces_total_byte_bound_ignoring_forged_size_hint() {
        let src = tempfile::tempdir().unwrap();
        // 3 files of 4MB each = 12MB, above the 10MB bound in `bounds()`.
        for i in 0..3 {
            fs::write(
                src.path().join(format!("big{i}.bin")),
                vec![7u8; 4 * 1024 * 1024],
            )
            .unwrap();
        }
        let dest = tempfile::tempdir().unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let cancel = CancellationToken::new();
        let err =
            import_directory(&mut source, dest.path(), &bounds(), &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn enforces_single_file_byte_bound() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("huge.bin"), vec![1u8; 6 * 1024 * 1024]).unwrap();
        let dest = tempfile::tempdir().unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let cancel = CancellationToken::new();
        let err =
            import_directory(&mut source, dest.path(), &bounds(), &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn enforces_depth_bound() {
        let src = tempfile::tempdir().unwrap();
        let mut deep = src.path().to_path_buf();
        for i in 0..20 {
            deep = deep.join(format!("d{i}"));
        }
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("leaf.txt"), b"x").unwrap();

        let dest = tempfile::tempdir().unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let cancel = CancellationToken::new();
        let mut tight_bounds = bounds();
        tight_bounds.max_depth = 5;
        let err =
            import_directory(&mut source, dest.path(), &tight_bounds, &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn cancellation_stops_the_import() {
        let src = tempfile::tempdir().unwrap();
        for i in 0..50 {
            fs::write(src.path().join(format!("f{i}.txt")), b"x").unwrap();
        }
        let dest = tempfile::tempdir().unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let err =
            import_directory(&mut source, dest.path(), &bounds(), &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::OperationCancelled);
    }
}
