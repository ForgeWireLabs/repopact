//! `repopact graph reconcile-merge` (WI063 ROG-029, Decision 0052 section
//! 4): the explicit, never-automatic derived-graph merge repair command.
//!
//! Source/configuration content is authoritative for Git merge purposes;
//! ordinary Git conflict resolution governs it exactly as it always has,
//! and this module never runs graph semantics against conflict-marker
//! source. `rog/**` is derived: a Git conflict confined to it is repaired
//! by deterministic regeneration from the already-merged authoritative
//! source, never by hand-merging JSONL graph semantics and never by a
//! `merge=union` driver. `governance/rog-capability.json` is
//! authoritative *configuration*, not derived content, despite living
//! outside `rog/` -- an enabled-vs-disabled conflict in it is a real
//! policy choice this module never auto-resolves in either direction.
//!
//! Bounded by construction: one Git invocation to enumerate unresolved
//! paths, and (only on the derived-only repair path) one scoped Git
//! invocation to stage the regenerated result -- never a per-file
//! subprocess.

use repopact_repository::{Repository, RepositorySnapshot};

use crate::capability::{CapabilityError, RogSetting, CAPABILITY_RECORD_RELATIVE_PATH};
use crate::durable::{DurableError, Manifest, ROG_DIR_NAME};

/// A Git path this repository currently reports as unmerged (`git diff
/// --diff-filter=U`), classified by which side of the source/derived
/// boundary (Decision 0052 section 3) it falls on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedPath {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileOutcome {
    /// Nothing was unresolved; there was no merge conflict for this
    /// command to repair.
    NothingToReconcile,
    /// Every unresolved path was confined to `rog/**`, and the graph was
    /// successfully regenerated from the merged authoritative source,
    /// verified, and staged.
    Repaired {
        staged_paths: Vec<String>,
        manifest: Option<Manifest>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileError {
    /// One or more unresolved paths are authoritative source/
    /// configuration, not derived `rog/**` content. RepoPact never
    /// decides which branch's source is correct; the operator must
    /// resolve these through normal Git conflict resolution first.
    AuthoritativeConflict { paths: Vec<String> },
    /// `governance/rog-capability.json` itself is unresolved. An
    /// enabled-vs-disabled conflict is a real policy choice; RepoPact
    /// never infers "enabled beats disabled" or its converse.
    CapabilityConflict,
    /// The unresolved-path query itself failed (not a Git repository,
    /// Git unavailable, or another Git error).
    GitQueryFailed(String),
    /// A capability declaration existed but could not be parsed.
    CapabilityUnreadable(CapabilityError),
    /// The derived-only repair path ran, but the regenerated graph
    /// failed to build or verify. The merge is left unresolved; nothing
    /// under `rog/**` is marked resolved.
    RebuildFailed(DurableError),
    /// The regenerated graph built successfully but could not be staged
    /// (e.g. `git add` itself failed).
    StageFailed(String),
}

impl std::fmt::Display for ReconcileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthoritativeConflict { paths } => write!(
                formatter,
                "unresolved authoritative source/configuration conflicts remain: {}",
                paths.join(", ")
            ),
            Self::CapabilityConflict => write!(
                formatter,
                "{CAPABILITY_RECORD_RELATIVE_PATH} is unresolved; resolve the enabled/disabled \
                 policy choice before reconciling the derived graph"
            ),
            Self::GitQueryFailed(message) => {
                write!(
                    formatter,
                    "could not enumerate unresolved Git paths: {message}"
                )
            }
            Self::CapabilityUnreadable(error) => {
                write!(formatter, "capability declaration is unreadable: {error}")
            }
            Self::RebuildFailed(error) => write!(formatter, "graph rebuild failed: {error}"),
            Self::StageFailed(message) => write!(formatter, "could not stage rog/**: {message}"),
        }
    }
}

fn is_capability_path(path: &str) -> bool {
    path == CAPABILITY_RECORD_RELATIVE_PATH
}

fn is_derived_path(path: &str) -> bool {
    path == ROG_DIR_NAME || path.starts_with(&format!("{ROG_DIR_NAME}/"))
}

/// One bounded `git diff --name-only --diff-filter=U` invocation.
fn unresolved_paths(repository: &Repository) -> Result<Vec<UnresolvedPath>, ReconcileError> {
    let runner = repository.git_runner();
    let output = runner
        .run(
            repository.root(),
            &["diff", "--name-only", "--diff-filter=U"],
            "graph.reconcile-merge.unresolved",
        )
        .map_err(|error| ReconcileError::GitQueryFailed(error.to_string()))?;
    if !output.status.success() {
        return Err(ReconcileError::GitQueryFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| UnresolvedPath {
            path: line.trim().replace('\\', "/"),
        })
        .collect())
}

/// One scoped `git add -A -- rog/` invocation: stages every current
/// `rog/**` path (additions, modifications, and removals alike) as
/// resolved. Never a per-shard `git add`.
fn stage_derived_path(repository: &Repository) -> Result<(), ReconcileError> {
    let runner = repository.git_runner();
    let pathspec = format!("{ROG_DIR_NAME}/");
    let output = runner
        .run(
            repository.root(),
            &["add", "-A", "--", pathspec.as_str()],
            "graph.reconcile-merge.stage",
        )
        .map_err(|error| ReconcileError::StageFailed(error.to_string()))?;
    if !output.status.success() {
        return Err(ReconcileError::StageFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    Ok(())
}

/// Reconcile a Git merge that left derived `rog/**` conflicts, per
/// Decision 0052 section 4. Never invoked automatically by `git merge`
/// itself -- this is always an explicit operator action.
pub fn reconcile_merge(
    repository: &Repository,
    snapshot: &RepositorySnapshot,
) -> Result<ReconcileOutcome, ReconcileError> {
    let unresolved = unresolved_paths(repository)?;
    if unresolved.is_empty() {
        return Ok(ReconcileOutcome::NothingToReconcile);
    }

    if unresolved
        .iter()
        .any(|entry| is_capability_path(&entry.path))
    {
        return Err(ReconcileError::CapabilityConflict);
    }

    let authoritative: Vec<String> = unresolved
        .iter()
        .filter(|entry| !is_derived_path(&entry.path))
        .map(|entry| entry.path.clone())
        .collect();
    if !authoritative.is_empty() {
        return Err(ReconcileError::AuthoritativeConflict {
            paths: authoritative,
        });
    }

    // Every unresolved path is confined to rog/** and the capability
    // record itself is not conflicted -- read its already-merged,
    // resolved value to decide the canonical derived state.
    let declaration = crate::capability::read_declaration(repository.root())
        .map_err(ReconcileError::CapabilityUnreadable)?;

    match declaration {
        Some(RogSetting::Disabled) | None => {
            // Explicit disabled (or legacy-absent, which has never had a
            // graph either) -- the canonical derived state is "no rog/".
            // Never re-enable as a side effect of reconciliation.
            let rog_root = repository.root().join(ROG_DIR_NAME);
            if rog_root.exists() {
                std::fs::remove_dir_all(&rog_root).map_err(|error| {
                    ReconcileError::RebuildFailed(DurableError {
                        code: "graph.io".to_owned(),
                        message: error.to_string(),
                    })
                })?;
            }
            stage_derived_path(repository)?;
            Ok(ReconcileOutcome::Repaired {
                staged_paths: unresolved.into_iter().map(|entry| entry.path).collect(),
                manifest: None,
            })
        }
        Some(RogSetting::Enabled) => {
            // Explicit enabled (or a legacy-enabled repository reaching
            // reconciliation, which this migrates to Decision 0051's
            // explicit-enabled declaration through the identical
            // enable-after-proof-good path every other build uses --
            // disclosed here, not a separate hidden migration).
            let manifest =
                crate::build_and_write(snapshot).map_err(ReconcileError::RebuildFailed)?;
            stage_derived_path(repository)?;
            Ok(ReconcileOutcome::Repaired {
                staged_paths: unresolved.into_iter().map(|entry| entry.path).collect(),
                manifest: Some(manifest),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use repopact_repository::Repository;

    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("repopact-reconcile-{name}-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn git(root: &Path, args: &[&str]) -> bool {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn init_source(root: &Path) {
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("work/active/100")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"reconcile-fixture\"\n\n[[bin]]\nname = \"reconcile-fixture\"\npath = \"src/main.rs\"\n",
        )
        .unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(
            root.join("work/active/100/work-item.json"),
            r#"{"id":"100","title":"Fixture","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-01"}"#,
        )
        .unwrap();
    }

    fn commit_all(root: &Path, message: &str) {
        assert!(git(root, &["add", "-A"]));
        assert!(git(root, &["commit", "--quiet", "-m", message]));
    }

    fn build_and_commit_graph(root: &Path) {
        let repository = Repository::open(root);
        let snapshot = repository.session().snapshot();
        crate::build_and_write(&snapshot).expect("build");
        commit_all(root, "enable rog");
    }

    fn init_git(root: &Path) {
        assert!(git(root, &["init", "--quiet"]));
        assert!(git(root, &["config", "user.email", "test@example.invalid"]));
        assert!(git(root, &["config", "user.name", "Test"]));
    }

    #[test]
    fn no_unresolved_paths_reports_nothing_to_reconcile() {
        let Some(root) = git_available().then(|| temp_root("clean")) else {
            eprintln!("skipping: git unavailable");
            return;
        };
        init_source(&root);
        init_git(&root);
        commit_all(&root, "source");

        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let outcome = reconcile_merge(&repository, &snapshot).unwrap();
        assert_eq!(outcome, ReconcileOutcome::NothingToReconcile);
        std::fs::remove_dir_all(&root).ok();
    }

    /// A derived-only `rog/**` conflict (branch A and branch B each built
    /// the graph independently on top of different source, then merge):
    /// `reconcile_merge` must regenerate from the merged authoritative
    /// source, verify, and stage it, leaving no unresolved path.
    #[test]
    fn derived_only_conflict_is_repaired_by_regeneration() {
        let Some(root) = git_available().then(|| temp_root("derived-conflict")) else {
            eprintln!("skipping: git unavailable");
            return;
        };
        init_source(&root);
        init_git(&root);
        commit_all(&root, "source");
        build_and_commit_graph(&root);
        assert!(git(&root, &["checkout", "-b", "branch-a"]));
        std::fs::write(root.join("src/a.rs"), "pub fn a() {}\n").unwrap();
        build_and_commit_graph(&root);

        assert!(git(&root, &["checkout", "main"]) || git(&root, &["checkout", "master"]));
        assert!(git(&root, &["checkout", "-b", "branch-b"]));
        std::fs::write(root.join("src/b.rs"), "pub fn b() {}\n").unwrap();
        build_and_commit_graph(&root);

        assert!(git(&root, &["checkout", "branch-a"]));
        // A real merge conflict: Git cannot auto-merge the sharded JSONL
        // content, so `rog/**` is left conflicted. The merge command
        // itself is allowed to report failure (exit != 0) -- what
        // matters is that unresolved rog/** paths exist afterward.
        let _ = git(&root, &["merge", "--no-edit", "branch-b"]);

        let repository = Repository::open(&root);
        let unresolved = unresolved_paths(&repository).unwrap();
        assert!(
            !unresolved.is_empty(),
            "expected the merge to leave rog/** conflicted"
        );
        assert!(unresolved.iter().all(|entry| is_derived_path(&entry.path)));

        let snapshot = repository.session().snapshot();
        let outcome = reconcile_merge(&repository, &snapshot).unwrap();
        match outcome {
            ReconcileOutcome::Repaired { manifest, .. } => assert!(manifest.is_some()),
            other => panic!("expected Repaired, got {other:?}"),
        }
        let after = unresolved_paths(&repository).unwrap();
        assert!(after.is_empty(), "no unresolved rog/** paths should remain");

        // The repaired graph must exactly equal a clean full rebuild of
        // the same merged source -- no manual JSONL editing occurred.
        let repository2 = Repository::open(&root);
        let snapshot2 = repository2.session().snapshot();
        let fresh_manifest = crate::RepositoryGraph::build_with_fingerprint(&snapshot2);
        let committed_manifest: Manifest =
            serde_json::from_str(&std::fs::read_to_string(root.join("rog/manifest.json")).unwrap())
                .unwrap();
        assert_eq!(
            committed_manifest.source_projection_fingerprint,
            fresh_manifest.1
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// A conflict in ordinary authoritative source must block repair
    /// entirely -- `reconcile_merge` never decides which branch's source
    /// is correct.
    #[test]
    fn authoritative_source_conflict_blocks_repair() {
        let Some(root) = git_available().then(|| temp_root("authoritative-conflict")) else {
            eprintln!("skipping: git unavailable");
            return;
        };
        init_source(&root);
        init_git(&root);
        commit_all(&root, "source");

        assert!(git(&root, &["checkout", "-b", "branch-a"]));
        std::fs::write(root.join("src/main.rs"), "fn main() { println!(\"a\"); }\n").unwrap();
        commit_all(&root, "branch a change");

        assert!(git(&root, &["checkout", "main"]) || git(&root, &["checkout", "master"]));
        assert!(git(&root, &["checkout", "-b", "branch-b"]));
        std::fs::write(root.join("src/main.rs"), "fn main() { println!(\"b\"); }\n").unwrap();
        commit_all(&root, "branch b change");

        assert!(git(&root, &["checkout", "branch-a"]));
        let _ = git(&root, &["merge", "--no-edit", "branch-b"]);

        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let error = reconcile_merge(&repository, &snapshot).unwrap_err();
        match error {
            ReconcileError::AuthoritativeConflict { paths } => {
                assert!(paths.iter().any(|path| path == "src/main.rs"));
            }
            other => panic!("expected AuthoritativeConflict, got {other:?}"),
        }

        // Resolve the source conflict responsibly and re-run: now
        // reconciliation may proceed (there is nothing left to
        // reconcile, since no graph was ever built on this branch).
        std::fs::write(
            root.join("src/main.rs"),
            "fn main() { println!(\"resolved\"); }\n",
        )
        .unwrap();
        commit_all(&root, "resolve conflict");
        let repository2 = Repository::open(&root);
        let snapshot2 = repository2.session().snapshot();
        let outcome = reconcile_merge(&repository2, &snapshot2).unwrap();
        assert_eq!(outcome, ReconcileOutcome::NothingToReconcile);
        std::fs::remove_dir_all(&root).ok();
    }

    /// A conflict in the capability record itself must block repair --
    /// RepoPact never infers an enabled/disabled policy choice.
    #[test]
    fn capability_conflict_blocks_repair() {
        let Some(root) = git_available().then(|| temp_root("capability-conflict")) else {
            eprintln!("skipping: git unavailable");
            return;
        };
        init_source(&root);
        init_git(&root);
        commit_all(&root, "source");

        assert!(git(&root, &["checkout", "-b", "branch-enabled"]));
        build_and_commit_graph(&root);

        assert!(git(&root, &["checkout", "main"]) || git(&root, &["checkout", "master"]));
        assert!(git(&root, &["checkout", "-b", "branch-disabled"]));
        std::fs::create_dir_all(root.join("governance")).unwrap();
        std::fs::write(
            root.join("governance/rog-capability.json"),
            r#"{"$schema":"../repopact/schemas/rog-capability.schema.json","version":1,"capabilities":{"rog":"disabled"}}"#,
        )
        .unwrap();
        commit_all(&root, "explicit disable");

        assert!(git(&root, &["checkout", "branch-enabled"]));
        let _ = git(&root, &["merge", "--no-edit", "branch-disabled"]);

        let repository = Repository::open(&root);
        let unresolved = unresolved_paths(&repository).unwrap();
        assert!(
            unresolved
                .iter()
                .any(|entry| is_capability_path(&entry.path)),
            "expected the capability record to be part of the conflict"
        );

        let snapshot = repository.session().snapshot();
        let error = reconcile_merge(&repository, &snapshot).unwrap_err();
        assert_eq!(error, ReconcileError::CapabilityConflict);
        std::fs::remove_dir_all(&root).ok();
    }

    /// If the resolved capability is explicit-disabled, reconciliation
    /// removes the conflicting rog/** paths without re-enabling the
    /// graph.
    #[test]
    fn disabled_capability_removes_derived_conflicts_without_reenabling() {
        let Some(root) = git_available().then(|| temp_root("disabled-repair")) else {
            eprintln!("skipping: git unavailable");
            return;
        };
        init_source(&root);
        init_git(&root);
        commit_all(&root, "source");
        build_and_commit_graph(&root);

        assert!(git(&root, &["checkout", "-b", "branch-a"]));
        std::fs::write(root.join("src/a.rs"), "pub fn a() {}\n").unwrap();
        build_and_commit_graph(&root);

        assert!(git(&root, &["checkout", "main"]) || git(&root, &["checkout", "master"]));
        assert!(git(&root, &["checkout", "-b", "branch-b"]));
        std::fs::write(root.join("src/b.rs"), "pub fn b() {}\n").unwrap();
        build_and_commit_graph(&root);
        // Explicitly disable on branch-b, resolved before the merge (no
        // capability conflict here -- only rog/** conflicts).
        crate::disable_graph(&root).unwrap();
        commit_all(&root, "disable rog");

        assert!(git(&root, &["checkout", "branch-a"]));
        let _ = git(&root, &["merge", "--no-edit", "branch-b"]);

        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        let outcome = reconcile_merge(&repository, &snapshot).unwrap();
        match outcome {
            ReconcileOutcome::Repaired { manifest, .. } => assert!(manifest.is_none()),
            other => panic!("expected Repaired with no manifest, got {other:?}"),
        }
        assert!(!root.join("rog").exists());
        let state = crate::capability::current_state(&root).unwrap();
        assert_eq!(state, crate::capability::CapabilityState::ExplicitDisabled);
        std::fs::remove_dir_all(&root).ok();
    }
}
