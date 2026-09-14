//! ROG-016 clean-clone proof (WI063 adoption/backfill/clean-clone
//! checkpoint). Uses a real `git clone` subprocess -- never a directory
//! copy -- as the primary proof, per this checkpoint's own directive.
//! Every test here is skipped (not failed) when the `git` binary is
//! unavailable in this environment, matching the existing best-effort
//! precedent already established for `.gitignore`-swallowed-record
//! detection.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use repopact_repository::{CountingGitRunner, Repository};

use crate::query::{
    open_durable_graph, GraphQueryEngine, NodeSelector, OrientOutcome, QueryBounds,
    ResolutionOutcome,
};

fn temp_root(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("repopact-clean-clone-{name}-{suffix}"));
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// Returns `false` (and the caller should skip) if a real `git` binary
/// is not usable in this environment.
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

/// Build a real source repository with governance content, a Cargo
/// crate, a test target, and commit it under Git.
fn build_source_repo(name: &str) -> Option<PathBuf> {
    if !git_available() {
        return None;
    }
    let root = temp_root(name);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::create_dir_all(root.join("work/active/100")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"clean-clone-fixture\"\n\n[[bin]]\nname = \"clean-clone-fixture\"\npath = \"src/main.rs\"\n\n[[test]]\nname = \"integration\"\npath = \"tests/integration.rs\"\n",
    )
    .unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn hello() {}\n").unwrap();
    std::fs::write(
        root.join("tests/integration.rs"),
        "#[test]\nfn it_works() {}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("work/active/100/work-item.json"),
        r#"{"id":"100","title":"Fixture","status":"active","owner_scope":"work","affected_scopes":[],"depends_on":[],"acceptance_criteria":[{"id":"AC-1","text":"prove","state":"pending","evidence":[]}],"created":"2026-01-01","updated":"2026-01-01"}"#,
    )
    .unwrap();

    assert!(git(&root, &["init", "--quiet"]));
    assert!(git(
        &root,
        &["config", "user.email", "test@example.invalid"]
    ));
    assert!(git(&root, &["config", "user.name", "Test"]));
    assert!(git(&root, &["add", "-A"]));
    assert!(git(&root, &["commit", "--quiet", "-m", "source"]));
    Some(root)
}

/// Build the durable graph in `source_root`, commit it, and return the
/// commit is expected to have succeeded.
fn enable_and_commit_graph(source_root: &Path) {
    let repository = Repository::open(source_root);
    let snapshot = repository.session().snapshot();
    crate::build_and_write(&snapshot).expect("build");
    assert!(git(source_root, &["add", "-A"]));
    assert!(git(source_root, &["commit", "--quiet", "-m", "enable rog"]));
}

fn clone_repo(source_root: &Path, name: &str) -> PathBuf {
    let clone_root = temp_root(name);
    std::fs::remove_dir_all(&clone_root).ok();
    let status = Command::new("git")
        .args([
            "clone",
            "--quiet",
            source_root.to_str().unwrap(),
            clone_root.to_str().unwrap(),
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .status()
        .expect("git clone must run");
    assert!(status.success(), "git clone failed");
    clone_root
}

fn cleanup(paths: &[&Path]) {
    for path in paths {
        std::fs::remove_dir_all(path).ok();
    }
}

// ---- ROG-016 steps 26-29: the clean-clone success path -------------------

#[test]
fn clean_clone_carries_capability_and_durable_graph_byte_identical() {
    let Some(source_root) = build_source_repo("carries-state") else {
        eprintln!("skipping: git unavailable");
        return;
    };
    enable_and_commit_graph(&source_root);
    let clone_root = clone_repo(&source_root, "carries-state-clone");

    // step 27: the clone carries all durable state.
    assert!(clone_root.join("governance/rog-capability.json").is_file());
    assert!(clone_root.join("rog/manifest.json").is_file());
    let source_manifest = std::fs::read_to_string(source_root.join("rog/manifest.json")).unwrap();
    let clone_manifest = std::fs::read_to_string(clone_root.join("rog/manifest.json")).unwrap();
    assert_eq!(
        source_manifest, clone_manifest,
        "the cloned manifest must be byte-identical to the source"
    );
    let source_shards: Vec<_> = std::fs::read_dir(source_root.join("rog/nodes"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    for shard_name in &source_shards {
        let source_bytes = std::fs::read(source_root.join("rog/nodes").join(shard_name)).unwrap();
        let clone_bytes = std::fs::read(clone_root.join("rog/nodes").join(shard_name)).unwrap();
        assert_eq!(
            source_bytes, clone_bytes,
            "node shard {shard_name:?} must be byte-identical between source and clone"
        );
    }

    cleanup(&[&source_root, &clone_root]);
}

#[test]
fn clean_clone_validates_and_reports_fresh_without_a_rebuild() {
    let Some(source_root) = build_source_repo("validate-first") else {
        eprintln!("skipping: git unavailable");
        return;
    };
    enable_and_commit_graph(&source_root);
    let clone_root = clone_repo(&source_root, "validate-first-clone");

    // step 28: first action is validation, not rebuild -- no
    // graph.build call happens anywhere in this test.
    let repository = Repository::open(&clone_root);
    let result = crate::status::status(&repository);
    assert_eq!(
        result.capability_state,
        crate::capability::CapabilityState::ExplicitEnabled
    );
    assert_eq!(result.freshness, crate::status::Freshness::Fresh);
    assert!(result.diagnostics.is_empty());

    cleanup(&[&source_root, &clone_root]);
}

#[test]
fn clean_clone_answers_bounded_queries_from_the_committed_graph() {
    let Some(source_root) = build_source_repo("query-before-rebuild") else {
        eprintln!("skipping: git unavailable");
        return;
    };
    enable_and_commit_graph(&source_root);
    let clone_root = clone_repo(&source_root, "query-before-rebuild-clone");

    // step 29: the load-bearing proof -- resolve/orient/dependencies/
    // tests all succeed from the committed durable graph, never
    // rebuilt in this test.
    let loaded = open_durable_graph(&clone_root, false).expect("clean clone should open fresh");
    let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
    let bounds = QueryBounds::default();

    let resolved = engine.resolve(&NodeSelector::WorkItemId("100".to_owned()), &bounds);
    assert!(matches!(resolved.result, ResolutionOutcome::Exact { .. }));

    let oriented = engine.orient(
        &NodeSelector::Package("clean-clone-fixture".to_owned()),
        &bounds,
    );
    assert!(matches!(oriented.result, OrientOutcome::Resolved(_)));

    let deps = engine.dependencies("work:100", true, &bounds);
    assert!(deps.result.is_some());

    let tests_result = engine.tests("manifest:Cargo.toml", &bounds);
    let tests_result = tests_result.result.expect("tests query should succeed");
    assert!(!tests_result.test_targets.is_empty());

    cleanup(&[&source_root, &clone_root]);
}

/// step 32: after clone, graph query operations (including opening the
/// durable graph and constructing the query index) invoke no Git beyond
/// the bounded snapshot cost WI057 already allows.
#[test]
fn clean_clone_queries_invoke_no_additional_git() {
    let Some(source_root) = build_source_repo("no-git-on-clone-query") else {
        eprintln!("skipping: git unavailable");
        return;
    };
    enable_and_commit_graph(&source_root);
    let clone_root = clone_repo(&source_root, "no-git-on-clone-query-clone");

    let runner = CountingGitRunner::native();
    let repository = Repository::with_git_runner(&clone_root, runner.clone());
    // status::status() is the one call in this path that legitimately
    // walks source via RepositorySession-adjacent machinery; measure
    // git cost starting from a fresh runner immediately before it, the
    // same idiom the existing WI057 graph-build test uses.
    let status_result = crate::status::status(&repository);
    assert_eq!(status_result.freshness, crate::status::Freshness::Fresh);
    let after_status = runner.count();
    assert!(
        after_status <= 4,
        "status on a clean clone must stay within the WI057 bound, got {after_status}"
    );

    let loaded = open_durable_graph(&clone_root, false).expect("clean clone should open fresh");
    let engine = GraphQueryEngine::new(&loaded.graph, loaded.context);
    let bounds = QueryBounds::default();
    let _ = engine.orient(
        &NodeSelector::Package("clean-clone-fixture".to_owned()),
        &bounds,
    );
    let _ = engine.dependencies("work:100", true, &bounds);
    let _ = engine.tests("manifest:Cargo.toml", &bounds);
    assert_eq!(
        runner.count(),
        after_status,
        "query-index construction and query operations must invoke zero additional Git \
         beyond what opening/validating the durable graph already cost"
    );

    cleanup(&[&source_root, &clone_root]);
}

// ---- step 33: damaged clone -----------------------------------------------

#[test]
fn damaged_clone_with_rog_removed_is_a_hard_failure_not_absent() {
    let Some(source_root) = build_source_repo("damaged-clone") else {
        eprintln!("skipping: git unavailable");
        return;
    };
    enable_and_commit_graph(&source_root);
    let clone_root = clone_repo(&source_root, "damaged-clone-clone");

    // Deliberately damage the clone: remove rog/ (e.g. a partial
    // checkout, a manual deletion) while the capability declaration
    // still says enabled.
    std::fs::remove_dir_all(clone_root.join("rog")).unwrap();

    let repository = Repository::open(&clone_root);
    let result = crate::status::status(&repository);
    assert_eq!(
        result.capability_state,
        crate::capability::CapabilityState::EnabledMissing
    );
    assert_ne!(
        result.freshness,
        crate::status::Freshness::Absent,
        "a damaged enabled clone must never be reported as plain absent/valid"
    );
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "graph.capability-enabled-but-missing"));

    let query_result = open_durable_graph(&clone_root, true);
    assert!(
        query_result.is_err(),
        "queries against a damaged enabled clone must fail closed even with allow_stale"
    );

    cleanup(&[&source_root, &clone_root]);
}

// ---- step 34: ignored-artifact clone proof --------------------------------

#[test]
fn a_gitignored_rog_directory_never_survives_to_a_clone() {
    // This is the upstream half of the ROG-016/ignored-artifact
    // guarantee: `check_enablement_not_ignored` (proven directly in
    // `lib.rs`'s `ignored_durable_graph_refuses_to_report_enablement_
    // success`) refuses to let `rog/` be committed as "enabled" in the
    // first place when `.gitignore` would swallow it -- so there is
    // nothing for a later clone to lose. Proven here end-to-end: build
    // fails, nothing is committed, and a clone of the source repository
    // (with no rog/ ever added) is legacy-absent, not a broken enabled
    // state.
    let Some(source_root) = build_source_repo("ignored-artifact") else {
        eprintln!("skipping: git unavailable");
        return;
    };
    std::fs::write(source_root.join(".gitignore"), "rog/\n").unwrap();
    assert!(git(&source_root, &["add", "-A"]));
    assert!(git(&source_root, &["commit", "--quiet", "-m", "gitignore"]));

    let repository = Repository::open(&source_root);
    let snapshot = repository.session().snapshot();
    let build_result = crate::build_and_write(&snapshot);
    assert!(
        build_result.is_err(),
        "build must refuse to report enablement success when .gitignore swallows rog/"
    );

    let clone_root = clone_repo(&source_root, "ignored-artifact-clone");
    let clone_repository = Repository::open(&clone_root);
    let result = crate::status::status(&clone_repository);
    assert_eq!(
        result.capability_state,
        crate::capability::CapabilityState::LegacyAbsent
    );
    assert_eq!(result.freshness, crate::status::Freshness::Absent);

    cleanup(&[&source_root, &clone_root]);
}
