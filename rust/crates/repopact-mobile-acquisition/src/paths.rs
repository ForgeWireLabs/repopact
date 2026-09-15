//! Decision 0057 path safety: every candidate relative path from an
//! external source (a SAF tree walk or a ZIP entry name) is normalized and
//! validated before being joined to a staging/workspace root. This module
//! deliberately reuses `repopact_repository`'s existing containment
//! primitives (`normalize_path`/`resolve_within_root`) rather than
//! maintaining a second, weaker implementation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use repopact_repository::resolve_within_root;

use crate::error::{AcquisitionError, AcquisitionResult, ErrorCode};

/// Rejects a raw relative-path string outright before any filesystem join
/// is attempted: `..` components, absolute paths, drive prefixes, UNC
/// prefixes, and embedded NUL bytes. This is a pre-filter; the final
/// containment check still happens in [`safe_join`] via
/// `resolve_within_root`, so a path that slips past this filter (there
/// should be none) is still caught.
pub fn reject_unsafe_relative_path(raw: &str) -> AcquisitionResult<()> {
    if raw.is_empty() {
        return Err(AcquisitionError::new(
            ErrorCode::PathEscape,
            "empty relative path",
        ));
    }
    if raw.contains('\0') {
        return Err(AcquisitionError::new(
            ErrorCode::PathEscape,
            "relative path contains a NUL byte",
        ));
    }
    if raw.starts_with('/') || raw.starts_with('\\') {
        return Err(AcquisitionError::new(
            ErrorCode::PathEscape,
            format!("relative path '{raw}' is absolute"),
        ));
    }
    // Drive prefix (`C:`) or UNC prefix (`\\server\share`, `//server/share`).
    if raw.starts_with("\\\\") || raw.starts_with("//") {
        return Err(AcquisitionError::new(
            ErrorCode::PathEscape,
            format!("relative path '{raw}' is a UNC prefix"),
        ));
    }
    let mut chars = raw.chars();
    if let (Some(first), Some(':')) = (chars.next(), chars.next()) {
        if first.is_ascii_alphabetic() {
            return Err(AcquisitionError::new(
                ErrorCode::PathEscape,
                format!("relative path '{raw}' has a drive prefix"),
            ));
        }
    }
    for component in raw.split(['/', '\\']) {
        if component == ".." {
            return Err(AcquisitionError::new(
                ErrorCode::PathEscape,
                format!("relative path '{raw}' contains a '..' component"),
            ));
        }
    }
    Ok(())
}

/// Joins `relative` beneath `root`, rejecting anything that resolves
/// outside it. `root` is expected to already exist (every caller in this
/// crate creates its staging/workspace root before importing into it).
///
/// `root` is canonicalized here before delegating to
/// `resolve_within_root`. Without this, `resolve_within_root`'s own
/// `normalize_path(root)` canonicalizes an *existing* root (which, on
/// Windows, produces an extended-length `\\?\`-prefixed path) while
/// `normalize_path(candidate)` falls back to lexical normalization for a
/// *not-yet-created* destination file/directory (the overwhelmingly common
/// case here, since we are always about to create the entry) -- the two
/// prefixes then never match and every legitimate join is misreported as
/// escaping the root. Canonicalizing `root` first makes both sides agree.
pub fn safe_join(root: &Path, relative: &str) -> AcquisitionResult<PathBuf> {
    reject_unsafe_relative_path(relative)?;
    let canonical_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    resolve_within_root(&canonical_root, relative).ok_or_else(|| {
        AcquisitionError::new(
            ErrorCode::PathEscape,
            format!("relative path '{relative}' escapes the destination root"),
        )
    })
}

/// A canonical comparison key for duplicate/case-collision detection
/// (Decision 0057): Unicode-NFC-ish (best-effort, without pulling in a
/// normalization crate: we fold via `to_lowercase`, which already collapses
/// the common Android/desktop-import case-collision shapes this policy
/// exists to catch) and case-folded, per path component, joined with `/`.
fn collision_key(relative: &str) -> String {
    relative
        .split(['/', '\\'])
        .map(|segment| segment.to_lowercase())
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
}

/// Tracks every relative path admitted into one import operation and fails
/// closed on any collision under Decision 0057's comparison: an exact
/// duplicate, a case-only collision, or a file-vs-directory collision at
/// the same normalized path.
#[derive(Debug, Default)]
pub struct CollisionGuard {
    seen: BTreeMap<String, (String, EntryKind)>,
}

impl CollisionGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn admit(&mut self, relative: &str, kind: EntryKind) -> AcquisitionResult<()> {
        let key = collision_key(relative);
        if let Some((existing_path, existing_kind)) = self.seen.get(&key) {
            if existing_path == relative && *existing_kind == kind {
                return Err(AcquisitionError::new(
                    ErrorCode::DuplicatePath,
                    format!("duplicate path '{relative}'"),
                ));
            }
            if existing_kind != &kind {
                return Err(AcquisitionError::new(
                    ErrorCode::CaseConflict,
                    format!(
                        "'{relative}' collides with '{existing_path}' as a file/directory type mismatch"
                    ),
                ));
            }
            return Err(AcquisitionError::new(
                ErrorCode::CaseConflict,
                format!("'{relative}' collides with '{existing_path}' under case-insensitive comparison"),
            ));
        }
        self.seen.insert(key, (relative.to_owned(), kind));
        Ok(())
    }
}

/// WI065 Checkpoint D §6: derives a deterministic, sanitized export-root
/// directory/document name from a workspace's user-facing display name.
/// Never used for anything path-safety-authoritative on its own -- the
/// resulting name still passes through the same collision/creation checks
/// as any other SAF document name -- but a raw display name may contain
/// path separators, control characters, or be empty, none of which are
/// valid single path segments.
pub fn sanitize_export_root_name(display_name: &str) -> String {
    let mut sanitized: String = display_name
        .trim()
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect();
    sanitized = sanitized.trim_matches(['.', ' ', '_']).to_owned();
    if sanitized.is_empty() {
        sanitized = "repopact-export".to_owned();
    }
    // A generous but bounded length -- this is a single path segment name,
    // not the deep-path-length bound `ExportBounds`/`ImportBounds` enforce
    // during traversal.
    const MAX_NAME_LENGTH: usize = 128;
    if sanitized.chars().count() > MAX_NAME_LENGTH {
        sanitized = sanitized.chars().take(MAX_NAME_LENGTH).collect();
    }
    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_traversal() {
        assert_eq!(
            reject_unsafe_relative_path("a/../../etc/passwd")
                .unwrap_err()
                .code,
            ErrorCode::PathEscape
        );
    }

    #[test]
    fn rejects_absolute_path() {
        assert_eq!(
            reject_unsafe_relative_path("/etc/passwd").unwrap_err().code,
            ErrorCode::PathEscape
        );
        assert_eq!(
            reject_unsafe_relative_path("\\Windows\\System32")
                .unwrap_err()
                .code,
            ErrorCode::PathEscape
        );
    }

    #[test]
    fn rejects_drive_and_unc_prefix() {
        assert_eq!(
            reject_unsafe_relative_path("C:\\evil").unwrap_err().code,
            ErrorCode::PathEscape
        );
        assert_eq!(
            reject_unsafe_relative_path("\\\\server\\share\\evil")
                .unwrap_err()
                .code,
            ErrorCode::PathEscape
        );
    }

    #[test]
    fn rejects_nul_byte() {
        assert_eq!(
            reject_unsafe_relative_path("a\0b").unwrap_err().code,
            ErrorCode::PathEscape
        );
    }

    #[test]
    fn accepts_ordinary_relative_path() {
        reject_unsafe_relative_path("src/lib.rs").unwrap();
    }

    #[test]
    fn safe_join_rejects_escape_via_resolve_within_root() {
        let dir = tempfile::tempdir().unwrap();
        let err = safe_join(dir.path(), "a/../../../escaped").unwrap_err();
        assert_eq!(err.code, ErrorCode::PathEscape);
    }

    #[test]
    fn safe_join_accepts_nested_path() {
        let dir = tempfile::tempdir().unwrap();
        let joined = safe_join(dir.path(), "a/b/c.txt").unwrap();
        // Compare against a canonicalized root, not `dir.path()` directly:
        // `safe_join` canonicalizes its root internally (see its doc
        // comment for why), so on Windows the returned path carries an
        // extended-length `\\?\` prefix `dir.path()` itself does not have.
        let canonical_root = std::fs::canonicalize(dir.path()).unwrap();
        assert!(joined.starts_with(&canonical_root));
        assert!(joined.ends_with("a/b/c.txt") || joined.ends_with("a\\b\\c.txt"));
    }

    #[test]
    fn collision_guard_rejects_exact_duplicate() {
        let mut guard = CollisionGuard::new();
        guard.admit("a/b.txt", EntryKind::File).unwrap();
        let err = guard.admit("a/b.txt", EntryKind::File).unwrap_err();
        assert_eq!(err.code, ErrorCode::DuplicatePath);
    }

    #[test]
    fn collision_guard_rejects_case_only_collision() {
        let mut guard = CollisionGuard::new();
        guard.admit("A.txt", EntryKind::File).unwrap();
        let err = guard.admit("a.txt", EntryKind::File).unwrap_err();
        assert_eq!(err.code, ErrorCode::CaseConflict);
    }

    #[test]
    fn collision_guard_rejects_file_vs_directory() {
        let mut guard = CollisionGuard::new();
        guard.admit("thing", EntryKind::File).unwrap();
        let err = guard.admit("thing", EntryKind::Directory).unwrap_err();
        assert_eq!(err.code, ErrorCode::CaseConflict);
    }

    #[test]
    fn collision_guard_allows_distinct_paths() {
        let mut guard = CollisionGuard::new();
        guard.admit("a.txt", EntryKind::File).unwrap();
        guard.admit("b.txt", EntryKind::File).unwrap();
        guard.admit("dir/a.txt", EntryKind::File).unwrap();
    }

    #[test]
    fn sanitizes_path_separators_and_control_characters() {
        assert_eq!(
            sanitize_export_root_name("My/Project\\Name"),
            "My_Project_Name"
        );
    }

    #[test]
    fn sanitizes_empty_name_to_a_fallback() {
        assert_eq!(sanitize_export_root_name("   "), "repopact-export");
    }

    #[test]
    fn preserves_an_ordinary_display_name() {
        assert_eq!(
            sanitize_export_root_name("repo-dir-fixture"),
            "repo-dir-fixture"
        );
    }
}
