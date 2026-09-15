//! WI065 Checkpoint D: the bounded, cancellable directory exporter. Mirrors
//! [`crate::import::import_directory`]'s shape exactly, but walks a
//! platform-neutral [`crate::source::AcquisitionSource`] (in practice
//! always [`crate::source::FilesystemSource`] rooted at the app-private
//! workspace's `repository/` directory -- Decision 0056's canonical
//! workspace, already trusted RepoPact content) and writes through a
//! platform-neutral [`crate::sink::ExportSink`] (a real Android SAF sink or,
//! for desktop/tests, [`crate::sink::FilesystemSink`]).
//!
//! Even though the source here is app-owned rather than external/untrusted,
//! every relative path is still run through
//! [`crate::paths::reject_unsafe_relative_path`] and bounds are still
//! enforced during traversal (Decision 0057 §19/§AC-6): defense in depth,
//! and a guard against a workspace that somehow accumulated a pathological
//! path after import (e.g. through a future local-mutation surface).

use std::io::{Read, Write};

use crate::bounds::ExportBounds;
use crate::error::{AcquisitionError, AcquisitionResult, ErrorCode};
use crate::operation::{CancellationToken, OperationPhase, ProgressThrottle};
use crate::paths::reject_unsafe_relative_path;
use crate::registry::SourceFingerprint;
use crate::sink::ExportSink;
use crate::source::{AcquisitionSource, SourceEntryKind};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportSummary {
    pub entries_exported: u64,
    pub bytes_exported: u64,
    pub files_exported: u64,
    pub directories_exported: u64,
}

const COPY_CHUNK_BYTES: usize = 256 * 1024;

/// Exports every entry from `source` into `sink`, enforcing the same class
/// of bounds/path-safety discipline Checkpoint A's importer applies on the
/// way in. Returns as soon as any entry is rejected, cancelled, or a bound
/// is exceeded -- cleanup of a partially-written destination (e.g. deleting
/// an app-created export root) is the caller's responsibility, exactly as
/// staging cleanup is the caller's responsibility on the import side.
pub fn export_tree(
    source: &mut dyn AcquisitionSource,
    sink: &mut dyn ExportSink,
    bounds: &ExportBounds,
    cancel: &CancellationToken,
    mut on_progress: impl FnMut(crate::operation::OperationProgress),
) -> AcquisitionResult<ExportSummary> {
    let mut summary = ExportSummary::default();
    let mut throttle = ProgressThrottle::new("export", OperationPhase::Exporting);

    loop {
        cancel.check()?;

        let Some(entry) = source.next_entry()? else {
            break;
        };

        reject_unsafe_relative_path(&entry.relative_path)?;

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

        summary.entries_exported += 1;
        if summary.entries_exported > bounds.max_entries {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                format!("export exceeds the {}-entry bound", bounds.max_entries),
            ));
        }

        match entry.kind {
            SourceEntryKind::Unsupported => {
                // Decision 0057 §16 (Checkpoint D outbound symlink policy):
                // a workspace should never contain one (Stage 1 rejects
                // symlinks on import), but if a later local-mutation surface
                // somehow produced one, export fails closed rather than
                // dereferencing or silently skipping it.
                return Err(AcquisitionError::new(
                    ErrorCode::UnsupportedEntry,
                    format!(
                        "'{}' is a symlink or other unsupported special entry; export does not follow or materialize it",
                        entry.relative_path
                    ),
                ));
            }
            SourceEntryKind::Directory => {
                sink.create_directory(&entry.relative_path)?;
                summary.directories_exported += 1;
                throttle.record(0);
            }
            SourceEntryKind::File => {
                let mut reader = source.open_file(&entry.relative_path)?;
                let mut writer = sink.create_file(&entry.relative_path)?;
                let bytes_copied = bounded_copy(
                    &mut reader,
                    writer.as_mut(),
                    bounds.max_single_file_bytes,
                    bounds
                        .max_total_bytes
                        .saturating_sub(summary.bytes_exported),
                    cancel,
                )?;
                writer.finish()?;
                summary.bytes_exported += bytes_copied;
                summary.files_exported += 1;
                throttle.record(bytes_copied);
            }
        }

        throttle.maybe_emit(Some(&entry.relative_path), None, false, &mut on_progress);
    }

    throttle.maybe_emit(None, Some(summary.entries_exported), true, &mut on_progress);
    Ok(summary)
}

fn bounded_copy(
    reader: &mut dyn Read,
    writer: &mut dyn Write,
    max_single_file_bytes: u64,
    remaining_total_budget: u64,
    cancel: &CancellationToken,
) -> AcquisitionResult<u64> {
    let mut buffer = [0u8; COPY_CHUNK_BYTES];
    let mut total: u64 = 0;
    loop {
        cancel.check()?;
        let read = reader
            .read(&mut buffer)
            .map_err(|error| AcquisitionError::new(ErrorCode::InternalIo, error.to_string()))?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > max_single_file_bytes {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                format!("export entry exceeds the {max_single_file_bytes}-byte single-file bound"),
            ));
        }
        if total > remaining_total_budget {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                "export exceeds the total-bytes bound",
            ));
        }
        writer
            .write_all(&buffer[..read])
            .map_err(|error| AcquisitionError::new(ErrorCode::InternalIo, error.to_string()))?;
    }
    Ok(total)
}

/// A read-only, non-writing walk of `source` that only accumulates counts
/// (never opens file contents) -- used for the source-divergence "obvious
/// change" check (Decision 0057 §"Source-divergence detection"; WI065
/// Checkpoint D §8/§9). Produces exactly the same bounded
/// [`SourceFingerprint`] shape captured at import time, so the two are
/// directly comparable. This is deliberately not a cryptographic proof:
/// entry count and aggregate declared size are the only facts a SAF
/// provider listing can honestly establish without reading every byte
/// again.
pub fn scan_source_fingerprint(
    source: &mut dyn AcquisitionSource,
    cancel: &CancellationToken,
) -> AcquisitionResult<SourceFingerprint> {
    let mut relative_path_count = 0u64;
    let mut aggregate_bytes = 0u64;
    loop {
        cancel.check()?;
        let Some(entry) = source.next_entry()? else {
            break;
        };
        relative_path_count += 1;
        if let Some(size) = entry.size_hint {
            aggregate_bytes = aggregate_bytes.saturating_add(size);
        }
    }
    Ok(SourceFingerprint {
        relative_path_count,
        aggregate_bytes,
        provider_markers: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sink::FilesystemSink;
    use crate::source::FilesystemSource;
    use std::fs;

    fn bounds() -> ExportBounds {
        ExportBounds {
            max_entries: 1000,
            max_total_bytes: 10 * 1024 * 1024,
            max_single_file_bytes: 5 * 1024 * 1024,
            max_depth: 16,
            max_path_length: 512,
        }
    }

    #[test]
    fn exports_a_normal_tree() {
        let src = tempfile::tempdir().unwrap();
        fs::create_dir_all(src.path().join("a/b")).unwrap();
        fs::write(src.path().join("a/b/file.txt"), b"hello").unwrap();
        fs::write(src.path().join("root.txt"), b"world").unwrap();

        let dest = tempfile::tempdir().unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let mut sink = FilesystemSink::new(dest.path());
        let cancel = CancellationToken::new();
        let summary = export_tree(&mut source, &mut sink, &bounds(), &cancel, |_| {}).unwrap();

        assert_eq!(summary.files_exported, 2);
        assert_eq!(summary.bytes_exported, 10);
        assert_eq!(
            fs::read_to_string(dest.path().join("a/b/file.txt")).unwrap(),
            "hello"
        );
        assert_eq!(
            fs::read_to_string(dest.path().join("root.txt")).unwrap(),
            "world"
        );
    }

    #[test]
    fn cancellation_stops_the_export() {
        let src = tempfile::tempdir().unwrap();
        for i in 0..50 {
            fs::write(src.path().join(format!("f{i}.txt")), b"x").unwrap();
        }
        let dest = tempfile::tempdir().unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let mut sink = FilesystemSink::new(dest.path());
        let cancel = CancellationToken::new();
        cancel.cancel();
        let err = export_tree(&mut source, &mut sink, &bounds(), &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::OperationCancelled);
    }

    #[test]
    fn enforces_single_file_bound() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("huge.bin"), vec![1u8; 6 * 1024 * 1024]).unwrap();
        let dest = tempfile::tempdir().unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let mut sink = FilesystemSink::new(dest.path());
        let cancel = CancellationToken::new();
        let err = export_tree(&mut source, &mut sink, &bounds(), &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn enforces_entry_count_bound() {
        let src = tempfile::tempdir().unwrap();
        for i in 0..10 {
            fs::write(src.path().join(format!("f{i}.txt")), b"x").unwrap();
        }
        let dest = tempfile::tempdir().unwrap();
        let mut source = FilesystemSource::new(src.path()).unwrap();
        let mut sink = FilesystemSink::new(dest.path());
        let cancel = CancellationToken::new();
        let mut tight = bounds();
        tight.max_entries = 5;
        let err = export_tree(&mut source, &mut sink, &tight, &cancel, |_| {}).unwrap_err();
        assert_eq!(err.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn scan_fingerprint_counts_without_reading_file_bytes() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir_all(src.path().join("dir")).unwrap();
        fs::write(src.path().join("dir/b.txt"), b"world!").unwrap();

        let mut source = FilesystemSource::new(src.path()).unwrap();
        let cancel = CancellationToken::new();
        let fingerprint = scan_source_fingerprint(&mut source, &cancel).unwrap();
        // 3 entries: a.txt, dir, dir/b.txt.
        assert_eq!(fingerprint.relative_path_count, 3);
        assert_eq!(fingerprint.aggregate_bytes, 11);
    }

    #[test]
    fn scan_fingerprint_matches_a_second_scan_of_the_unchanged_tree() {
        let src = tempfile::tempdir().unwrap();
        fs::write(src.path().join("a.txt"), b"hello").unwrap();

        let cancel = CancellationToken::new();
        let mut source1 = FilesystemSource::new(src.path()).unwrap();
        let first = scan_source_fingerprint(&mut source1, &cancel).unwrap();
        let mut source2 = FilesystemSource::new(src.path()).unwrap();
        let second = scan_source_fingerprint(&mut source2, &cancel).unwrap();
        assert_eq!(first, second);

        fs::write(src.path().join("b.txt"), b"new file").unwrap();
        let mut source3 = FilesystemSource::new(src.path()).unwrap();
        let third = scan_source_fingerprint(&mut source3, &cancel).unwrap();
        assert_ne!(first, third);
    }
}
