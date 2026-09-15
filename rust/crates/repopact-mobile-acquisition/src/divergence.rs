//! WI065 Checkpoint D §8/§9: obvious source-divergence detection, using only
//! the bounded, non-cryptographic facts a SAF provider listing can honestly
//! establish (Decision 0057 §"Source-divergence detection"). This
//! deliberately does not attempt to "prove" a source is unchanged --
//! provider modification timestamps are not treated as ground truth, and no
//! content hash of the external source is computed (that would require
//! reading every byte again, defeating the purpose of a cheap divergence
//! check). Comparison is limited to entry count and aggregate declared
//! size, exactly the same fields [`crate::registry::SourceFingerprint`]
//! already records at import time.

use crate::error::ErrorCode;
use crate::registry::SourceFingerprint;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    /// The current listing matches the persisted import-time fingerprint
    /// exactly. This is "no obvious change detected," never a claim of
    /// cryptographic proof.
    Unchanged,
    /// Entry count or aggregate size differs from the persisted
    /// fingerprint.
    ObviouslyChanged,
    /// The source could not be reached at all (provider gone, tree
    /// unmounted, document deleted).
    Unavailable,
    /// The persisted URI permission for this source is no longer valid.
    PermissionLost,
    /// A comparison was attempted but could not be honestly completed for
    /// some other reason. Never conflated with `Unchanged`.
    Unknown,
}

/// Compares a freshly-scanned fingerprint against the one captured at
/// import time. `Ok` only when both fingerprints could actually be
/// established; callers map a failed re-scan to [`SourceStatus::Unavailable`]/
/// [`SourceStatus::PermissionLost`]/[`SourceStatus::Unknown`] via
/// [`status_from_scan_error`] instead of calling this function at all.
pub fn compare_fingerprints(
    persisted: &SourceFingerprint,
    current: &SourceFingerprint,
) -> SourceStatus {
    if persisted.relative_path_count == current.relative_path_count
        && persisted.aggregate_bytes == current.aggregate_bytes
    {
        SourceStatus::Unchanged
    } else {
        SourceStatus::ObviouslyChanged
    }
}

/// Maps a failed re-scan attempt to the honest typed status -- never
/// asserts `Unchanged` merely because the scan itself failed.
pub fn status_from_scan_error(code: ErrorCode) -> SourceStatus {
    match code {
        ErrorCode::PermissionDenied => SourceStatus::PermissionLost,
        ErrorCode::SourceUnavailable => SourceStatus::Unavailable,
        _ => SourceStatus::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(count: u64, bytes: u64) -> SourceFingerprint {
        SourceFingerprint {
            relative_path_count: count,
            aggregate_bytes: bytes,
            provider_markers: Vec::new(),
        }
    }

    #[test]
    fn identical_fingerprints_are_unchanged() {
        assert_eq!(
            compare_fingerprints(&fp(3, 100), &fp(3, 100)),
            SourceStatus::Unchanged
        );
    }

    #[test]
    fn differing_entry_count_is_obviously_changed() {
        assert_eq!(
            compare_fingerprints(&fp(3, 100), &fp(4, 100)),
            SourceStatus::ObviouslyChanged
        );
    }

    #[test]
    fn differing_byte_count_is_obviously_changed() {
        assert_eq!(
            compare_fingerprints(&fp(3, 100), &fp(3, 999)),
            SourceStatus::ObviouslyChanged
        );
    }

    #[test]
    fn scan_error_never_becomes_unchanged() {
        assert_eq!(
            status_from_scan_error(ErrorCode::PermissionDenied),
            SourceStatus::PermissionLost
        );
        assert_eq!(
            status_from_scan_error(ErrorCode::SourceUnavailable),
            SourceStatus::Unavailable
        );
        assert_eq!(
            status_from_scan_error(ErrorCode::ResourceLimit),
            SourceStatus::Unknown
        );
    }
}
