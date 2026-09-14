//! Decision 0044 source projection `P(repo)` and its deterministic
//! fingerprint.
//!
//! The projection is the ordered set of repository-relative files
//! considered as durable-graph input. It excludes the graph's own output
//! directory (self-hashing prevention), the shared
//! [`repopact_repository::IGNORED_PARTS`] exclusions, and `target`
//! (folded into that shared list by Decision 0044 rather than forked
//! here). Fingerprinting uses `Repository::path_state`'s existing SHA-256
//! content digest, not Git blob identity (see Decision 0044 section 8 for
//! why).

use std::path::Path;

use repopact_repository::{Repository, RepositoryTopology};
use repopact_types::PathState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Repository-relative directory name that holds the durable ROG output.
/// Excluded from every source projection so the graph never hashes itself.
pub const ROG_ROOT: &str = "rog";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedFile {
    pub relative_path: String,
    pub digest: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceProjection {
    /// Sorted by `relative_path`; the sort itself is part of the
    /// determinism contract, not an implementation detail.
    pub files: Vec<ProjectedFile>,
}

impl SourceProjection {
    /// Build the projection for `repository` using an already-computed
    /// `topology` (from `RepositorySnapshot::topology()`), not a fresh
    /// `Repository::all_files()` call. `all_files()`/`files_under()`
    /// recompute `RepositoryTopology` (fresh Git invocations) on every
    /// call; reusing the snapshot's topology is required to keep the
    /// WI057 bounded-git-invocation guarantee intact when a graph build
    /// runs on top of an already-open snapshot. The walk itself still
    /// applies the shared `IGNORED_PARTS` exclusions, linked-worktree
    /// exclusion, and structural symlink exclusion, and additionally
    /// excludes anything under the durable ROG output directory.
    pub fn build(repository: &Repository, topology: &RepositoryTopology) -> Self {
        let mut files: Vec<ProjectedFile> = repository
            .files_under_with_topology(repository.root(), topology)
            .into_iter()
            .filter_map(|path| {
                let relative = repository.relative_path(&path);
                if is_under_rog_root(&relative) || is_capability_declaration(&relative) {
                    return None;
                }
                let digest = match repository.path_state(&path) {
                    PathState::Present { digest, .. } => digest,
                    PathState::Absent => return None,
                };
                Some(ProjectedFile {
                    relative_path: relative,
                    digest,
                })
            })
            .collect();
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        files.dedup_by(|left, right| left.relative_path == right.relative_path);
        Self { files }
    }

    /// `sha256(join("\n", sorted("{path}\0{digest}")))`, hex-encoded. See
    /// Decision 0044 section 8.
    pub fn fingerprint(&self) -> String {
        let mut hasher = Sha256::new();
        for (index, file) in self.files.iter().enumerate() {
            if index > 0 {
                hasher.update(b"\n");
            }
            hasher.update(file.relative_path.as_bytes());
            hasher.update(b"\0");
            hasher.update(file.digest.as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

fn is_under_rog_root(relative_path: &str) -> bool {
    relative_path == ROG_ROOT || relative_path.starts_with(&format!("{ROG_ROOT}/"))
}

/// The ROG capability declaration (Decision 0051) is graph-lifecycle
/// metadata, not semantic source content -- excluded from the source
/// projection for exactly the same self-referential-churn reason `rog/`
/// itself is excluded. Without this, a fresh `graph build` on a
/// legacy-absent repository would write the capability record *after*
/// computing the fingerprint the manifest records, making the very next
/// `graph status`/`verify` call see a "new" file the manifest's own
/// fingerprint never accounted for and falsely report `stale`
/// immediately after a successful build.
fn is_capability_declaration(relative_path: &str) -> bool {
    relative_path == crate::capability::CAPABILITY_RECORD_RELATIVE_PATH
}

/// Minimal hex encoder so this crate does not take on a dependency purely
/// for hex formatting; `sha2::Sha256::finalize()` already returns bytes.
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

pub fn normalized_relative(repository: &Repository, path: &Path) -> String {
    repository.relative_path(path)
}
