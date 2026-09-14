# WI061 Architecture Review

```
coding agent = Claude Code
architecture reviewer = GPT-5.6 Sol High
```

This review is written before any production source file is modified. WI061
is architecture-only: it produces a decision, a comparison, and a follow-up
work item, and touches no code in `DesktopService`, `RepositorySession`,
`NativeGitRunner`, `RepositoryTopology`, the Tauri Android command surface,
Android capability/manifest configuration, Gradle, mobile plugins, or the
frontend repository-selection flow.

## Facts inherited from WI060 (re-verified against current source, not memory)

WI060's own architecture review and closeout recorded the following facts.
Each has been re-read against the current repository state as part of this
review, and none is contradicted by what is actually in the tree today:

1. **`DesktopService` consumes `PathBuf`/filesystem directories.**
   Verified: `DesktopService::open_repository(&self, root: impl AsRef<Path>)`
   and `open_repository_with_git_runner` in
   `rust/crates/repopact-desktop-api/src/lib.rs:511-524` both take a
   filesystem path. There is no document-tree, URI, or virtual-filesystem
   overload anywhere in `repopact-desktop-api`.
2. **`RepositorySession` assumes an ordinary filesystem working tree.**
   Verified: `RepositorySession` (`rust/crates/repopact-repository/src/lib.rs:541-568`)
   is `{ repository: Repository }`, and `Repository` itself is
   `{ root: PathBuf, git_runner: Arc<dyn GitRunner> }` (`lib.rs:29-33`).
   Every accessor and query on `Repository`/`RepositorySession` operates on
   that `PathBuf` root using ordinary `std::fs`/`std::path` semantics.
3. **Mutation plans remain Rust-owned opaque handles.**
   Verified against Decision 0041 (`decisions/0041-tauri-desktop-session-and-opaque-plan-boundary.md`),
   which is unchanged and still the governing contract: the frontend receives
   a `MutationPlanView` with an opaque handle/token, never the underlying
   `MutationPlan`, and `apply_mutation` re-resolves the handle against the
   current session server-side. WI061 does not touch this contract, and no
   mobile-specific reasoning in this review requires relaxing it.
4. **`RepositoryTopology` already tolerates a non-Git filesystem tree.**
   Verified: `RepositoryTopology::build` (`rust/crates/repopact-repository/src/lib.rs:44-107`)
   gates every Git-backed query (`tracked_paths`, `recording_commits`,
   worktree registration) behind `repository.root.join(".git").exists()`
   first, via `.then(|| ...)` combinators, and falls back to
   `RecordIndex`'s plain filesystem discovery otherwise. This is a real,
   already-shipped, already-tested property of the repository model — not
   something that would need to be built for mobile.
5. **`NativeGitRunner` shells out to a system `git` executable.**
   Verified: `NativeGitRunner::run` (`rust/crates/repopact-repository/src/git.rs:278-291`)
   constructs `std::process::Command::new("git")`, spawns it, and waits on it
   with OS-level process-group/job-object containment. There is no
   in-process Git implementation anywhere in `repopact-repository`.
6. **Normal Android packages do not assume a system `git` executable.**
   Unchanged fact; nothing in Tauri, the generated Android Gradle project, or
   RepoPact's own dependencies bundles or locates a `git` binary on-device.
7. **Android SAF returns a URI/document-provider authority, not an ordinary
   recursive filesystem path.**
   Unchanged fact about the Android platform itself (`ACTION_OPEN_DOCUMENT_TREE`
   yields a `content://` tree URI resolved through `DocumentsContract`/
   `DocumentFile`, not a POSIX path).
8. **Production Android `select_repository` currently fails honestly rather
   than pretending a `content://` URI is a filesystem checkout.**
   Verified: `select_repository` in
   `rust/apps/repopact-desktop/src-tauri/src/lib.rs:47-78` is platform-gated.
   On a normal (non-debug, or debug without the `android-debug-validation`
   feature) Android build it returns
   `DesktopError { code: "repository.mobile-selection-unavailable", .. }`.
   The only Android path that opens a repository at all is the debug-only,
   feature-gated `android_validation::debug_validation_repository_path`
   helper, unreachable from a release build.

None of these eight conclusions is reopened by this review. Repository
inspection did not produce a decisive contradiction of any of them — if
anything, direct reading of the current source strengthens several of them
with exact file/line evidence WI060's own review stated more generally.

## Additional facts this review adds (not previously recorded)

9. **`app_data_dir()` app-private internal storage is already a proven,
   working mobile workspace root.** `android_validation.rs` (lines 44-61)
   already resolves and writes into
   `app.path().app_data_dir().join("files").join("wi060-validation-repo")`
   on a real Android device, and WI060's on-device evidence proved the
   Workbench's IA, Rust-backed reads, and mutation plan/apply cycle all work
   correctly against that exact location. This is direct, already-executed
   evidence for this decision's canonical-workspace choice (§14 of the
   drafting brief), not merely a plausible inference.
10. **`GitRunner` is a shell-command-shaped seam, not a Git-porcelain-shaped
    seam.** `pub trait GitRunner: Send + Sync + fmt::Debug { fn run(&self,
    root: &Path, args: &[&str], label: &str) -> Result<GitOutput, GitError>;
    }` (`rust/crates/repopact-repository/src/git.rs:62-64`) takes raw CLI
    argument vectors and returns raw captured stdout/stderr/exit status. It
    is already an abstraction over *how a git command line gets executed*
    (there is also a `CountingGitRunner` test/audit decorator at
    `git.rs:324-368`, and a `GitGate` decorator in
    `repopact-desktop-api/src/lib.rs:1590-1632`), but it is not an
    abstraction over Git *operations* (clone/fetch/push/checkout as
    porcelain calls). An embedded Git library (`git2-rs`/`gix`) exposes an
    object/reference/index API, not a CLI-argument-and-stdout contract, so a
    future `EmbeddedGitRunner` cannot simply be "another `impl GitRunner`"
    that translates `&["clone", url]` into `git2::Repository::clone(url,
    path)` — the argument-vector shape does not generalize to an embedded
    library's calling convention for the operations RepoPact's own topology
    /mutation code invokes today (`ls-files`, `log --diff-filter=A ...`,
    worktree queries). This is an honest architectural gap, not a solved
    seam: §13 of the drafting brief asks for a `GitRunner`/`GitBackend`
    split; the existing `GitRunner` trait is real and reusable for
    `NativeGitRunner`'s existing desktop/mobile-with-system-git callers, but
    the *new* embedded-Git responsibilities (clone into an empty directory,
    remote credential handling, push) do not fit its `run(args: &[&str])`
    signature and will need either a new trait alongside it or a
    higher-level `GitBackend` that `RepositoryTopology`/mutation code calls
    instead of reaching for `GitRunner` directly for those operations. The
    implementation follow-up must design this seam; WI061 records that it
    does not yet exist in a directly reusable form.
11. **No SAF, embedded-Git, or archive-import code exists anywhere in the
    tree today.** A repository-wide search for `content://`,
    `DocumentsContract`, `ACTION_OPEN_DOCUMENT`, `git2`, and `gix` inside
    `rust/` and the Android Gradle/Kotlin sources returns no matches outside
    this review and the WI061/WI060 prose describing the *absence* of such
    code. There is nothing partially built to preserve or reconcile.
12. **The Android manifest currently grants `INTERNET` and a
    dynamic-receiver-not-exported permission only** (`rust/apps/repopact-desktop/src-tauri/gen/android/app/src/main/AndroidManifest.xml`
    and its per-flavor copies), and the single Tauri capability file
    (`capabilities/main-window.json`) grants only the custom
    `workbench-commands` permission plus narrow `core:event`/`core:app`
    listener permissions — no `core:dialog`, `core:fs`, `core:shell`, or
    generic filesystem/process plugin capability is exposed to the frontend
    on any platform. `INTERNET` is already present for the WebView itself,
    not for any Git operation; it is not evidence that network-authority
    scoping for a future embedded-Git remote operation has already been
    solved.

## What this means for WI061's scope

Given facts 1-12, the correct WI061 posture mirrors WI060's own reasoning:

- The kernel invariants that make RepoPact reviewable and safe today
  (PathBuf-based `DesktopService`/`RepositorySession`, opaque mutation-plan
  handles, `.git`-optional `RepositoryTopology`, narrow typed Tauri commands)
  are not mobile-hostile. They are exactly what an app-private working-copy
  architecture needs and gets for free.
- The two facts that *are* mobile-hostile — SAF's `content://` authority
  model, and the absence of a system `git` binary — are acquisition/
  synchronization-layer problems, not repository-model problems. Solving them
  by adding a second, document-provider-backed repository implementation
  would duplicate `DesktopService`/`RepositorySession`'s responsibilities
  rather than extend them.
- Therefore the architectural center of this decision (see
  `mobile-repository-strategy-comparison.md` and Decision 0056) is: mobile
  repositories are ordinary app-private filesystem trees; SAF, archives, and
  Git remotes are how content gets into and out of that tree, not alternate
  ways of being that tree.
