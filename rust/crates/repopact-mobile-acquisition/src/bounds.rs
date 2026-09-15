//! Decision 0057 §"Directory import bounds" / §"Archive bomb bounds":
//! centrally defined resource limits, enforced during traversal/extraction,
//! never only after the fact, and never trusted from source-reported
//! metadata alone (a forged/mismatched size must not bypass the runtime
//! byte counter).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportBounds {
    pub max_entries: u64,
    pub max_total_bytes: u64,
    pub max_single_file_bytes: u64,
    pub max_depth: usize,
    pub max_path_length: usize,
}

impl Default for ImportBounds {
    /// Practical v1 values (engineering evidence, not a hard product
    /// promise): generous enough for an ordinary small-to-medium project
    /// import, bounded enough that a pathological or adversarial source
    /// cannot exhaust device storage/memory during one operation.
    fn default() -> Self {
        Self {
            max_entries: 200_000,
            max_total_bytes: 2 * 1024 * 1024 * 1024,  // 2 GiB
            max_single_file_bytes: 512 * 1024 * 1024, // 512 MiB
            max_depth: 64,
            max_path_length: 1024,
        }
    }
}

/// Decision 0057 §19 (Checkpoint D): export must remain bounded even though
/// its source (the app-private workspace) is more trusted than an external
/// import source. Deliberately mirrors [`ImportBounds`]'s values -- an
/// export can never legitimately need to move more data than the same
/// workspace was allowed to import in the first place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportBounds {
    pub max_entries: u64,
    pub max_total_bytes: u64,
    pub max_single_file_bytes: u64,
    pub max_depth: usize,
    pub max_path_length: usize,
}

impl Default for ExportBounds {
    fn default() -> Self {
        let import = ImportBounds::default();
        Self {
            max_entries: import.max_entries,
            max_total_bytes: import.max_total_bytes,
            max_single_file_bytes: import.max_single_file_bytes,
            max_depth: import.max_depth,
            max_path_length: import.max_path_length,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveBounds {
    pub max_entries: u64,
    pub max_expanded_bytes: u64,
    pub max_single_entry_bytes: u64,
    pub max_depth: usize,
    pub max_path_length: usize,
    /// Reject an entry whose expanded size divided by its compressed size
    /// exceeds this ratio -- a zip-bomb heuristic independent of the
    /// absolute byte bounds above.
    pub max_compression_ratio: u64,
}

impl Default for ArchiveBounds {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            max_expanded_bytes: 1024 * 1024 * 1024,    // 1 GiB
            max_single_entry_bytes: 256 * 1024 * 1024, // 256 MiB
            max_depth: 64,
            max_path_length: 1024,
            max_compression_ratio: 1000,
        }
    }
}
