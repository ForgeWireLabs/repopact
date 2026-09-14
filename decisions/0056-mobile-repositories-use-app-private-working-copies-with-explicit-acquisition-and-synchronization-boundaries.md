---
id: 0056
title: Mobile repositories use app-private working copies with explicit acquisition and synchronization boundaries
status: accepted
date: 2026-09-14
supersedes: []
---

# 0056: Mobile repositories use app-private working copies with explicit acquisition and synchronization boundaries

## Context

WI060 brought the RepoPact Workbench up on Android end-to-end — the shared
desktop/mobile Tauri crate builds and launches on a real device, the compact
IA renders correctly, and a real Rust-owned mutation plan/review/apply flow
was exercised on-device with zero Git-process dependency — but deliberately
left production Android repository *acquisition* unresolved.
`select_repository` on a normal Android build returns a typed
`repository.mobile-selection-unavailable` error, because:

- Tauri's Android dialog implementation cannot pick a folder the way its
  desktop implementation can;
- an Android `content://` URI returned by the Storage Access Framework
  (`ACTION_OPEN_DOCUMENT_TREE`) is not a filesystem path
  `DesktopService::open_repository` can consume — `DesktopService`,
  `RepositorySession`, and `Repository` are all `PathBuf`-based
  (`rust/crates/repopact-desktop-api/src/lib.rs`,
  `rust/crates/repopact-repository/src/lib.rs`);
- `NativeGitRunner` (`rust/crates/repopact-repository/src/git.rs`) shells out
  to a system `git` executable that an ordinary Android application package
  cannot assume exists.

WI061 (`work/active/061-mobile-repository-acquisition-app-private-workspace-and-git-strategy/`)
was opened to decide this architecture. Its comparison document
(`mobile-repository-strategy-comparison.md`) evaluated five required options
— (A) SAF import/export into an app-private workspace, (B) a live SAF
document tree used directly as the working repository, (C) archive import,
(D) an embedded Git library (`git2-rs`/libgit2 or `gix`/gitoxide), and (E)
remote Git clone/sync as a primary acquisition path — against
`DesktopService`'s filesystem-directory assumption, `NativeGitRunner`'s
system-git assumption, `RepositoryTopology`'s `.git`-presence gating,
mutation-plan/apply and opaque-plan-handle semantics (Decision 0041), and the
offline/no-Git-binary constraint WI060 proved on-device. This decision
records that comparison's conclusion.

## Decision

**On mobile, RepoPact operates only on an app-private ordinary filesystem
workspace.** External document trees, archives, and Git remotes are
acquisition/synchronization *transports*. They are not alternate `Repository`
implementations.

This preserves, without modification, everything WI060 already proved works
on Android: `DesktopService`, `Repository`/`RepositorySession`, the
`MutationRequest -> MutationPlan -> MutationResult` pipeline, opaque
mutation-plan handles (Decision 0041), ordinary Rust filesystem semantics,
ROG/indexing semantics (WI063), and mutation-plan path containment. It
rejects introducing a second, document-provider-backed repository model.

### Canonical mobile workspace location

The canonical mobile workspace is **app-private internal storage** — the
Tauri/OS app-data equivalent already proven reachable and writable on
Android in WI060's `android_validation.rs`
(`app.path().app_data_dir().join("files").join(<workspace>)`). Rationale:
ordinary filesystem semantics; no broad storage permission required; stable
lifetime while the app is installed; private to the app; directly compatible
with `DesktopService`/`RepositorySession`/ROG/mutation semantics as already
proven on-device. Removable/external app-specific storage is not adopted as
the sole canonical location, because its availability is not guaranteed and
loss of it would break core function. Large-repository storage policy (e.g.
supporting workspaces that approach device storage limits) is explicit
future work, not decided here.

### Workspace identity

The implementation follow-up must give each acquired workspace a stable,
app-owned identity recording at minimum: a workspace id, a display name, its
acquisition kind (SAF import, archive import, or embedded-Git clone), a
source reference, created/imported time, last export/sync state, and whether
it is Git-backed. This is local product state describing what the app has
acquired, not a new global governance authority, and it must never contain
credential material.

### Staged product sequence

**Stage 1 — local acquisition, no embedded Git required.**

- **(A) SAF import/export**: the user picks a directory tree
  (`ACTION_OPEN_DOCUMENT_TREE`) or a single document
  (`ACTION_OPEN_DOCUMENT`) via SAF; RepoPact performs a safe recursive copy
  into the app-private workspace; the Workbench operates on that copy
  exactly as it already does against `android_validation.rs`'s validation
  tree.
- **(C) Archive import**: the user picks a `.zip` (or equivalent) via SAF;
  it is validated and safely extracted into the app-private workspace; the
  Workbench operates on the extracted files.

Both are useful, complementary acquisition modes — not every user's project
is a live folder, and not every user's project is an archive. Neither
requires a system `git` binary, and neither expands the Android permission
surface beyond a one-time SAF pick.

**Stage 1 write-back (resolves AC-2):** mutation occurs only in the
app-owned copy. The user explicitly invokes an export/replace/share-back
action; RepoPact never silently writes back to the original SAF tree or
archive, and there is no background or automatic synchronization. Where
persistent SAF write permission is retained for a directory import,
write-back remains an explicit user-invoked operation, never background
mutation. Before replacing/exporting, the implementation should attempt to
detect obvious source divergence where feasible, rather than silently
clobbering a source that changed since import. For an archive-acquired
workspace, v1 write-back is an explicit "Export Repository" action that
produces a new archive/document through a user-selected SAF destination; it
does not silently overwrite the original archive unless a later, explicit UX
decision supports safe replacement.

**Stage 2 — embedded Git backend.** After Stage 1 ships, an embedded Git
backend enables real clone/fetch/pull/push into and from the same
app-private workspace model. Git and non-Git mobile workspaces then share
every layer above the acquisition/sync adapter: `DesktopService`,
`RepositorySession`, Workbench, the mutation engine, ROG, and validation.
Only the acquisition/sync adapter changes; RepoPact's repository model does
not.

**Embedded Git library choice:** `git2-rs`/libgit2 is the preferred
implementation candidate for the first complete mobile clone/fetch/push
backend, because push and full remote-write support are mature today and
Stage 2's entire purpose is enabling real write-back to a remote. `gix` is
Rust-native and architecturally attractive, but as of this decision date its
push/high-level porcelain workflow coverage is not yet mature enough for
this role. This is a same-day, current-state comparison, not a permanent
ban: the implementation work item must prototype/build-gate `git2-rs` on
Android before treating the selection as irreversible, and `gix` remains the
correct reevaluation candidate once its required remote-write capabilities
mature. See `mobile-repository-strategy-comparison.md`'s library comparison
table for the full trade-off (native dependency/build/security surface vs.
push maturity vs. binary size vs. Rust-native ergonomics).

**Backend seam:** the existing `GitRunner` trait
(`rust/crates/repopact-repository/src/git.rs:62-64`) is a shell-command-
shaped seam — `run(root, args: &[&str], label) -> Result<GitOutput,
GitError>` — that `NativeGitRunner` implements today. It is not directly
reusable as an embedded-Git seam, because `git2`/`gix` expose an
object/reference/index API, not a CLI-argument-and-captured-stdout contract.
The implementation follow-up must not scatter `#[cfg(android)] use git2`
through repository logic; it must define a proper backend abstraction
(conceptually `GitBackend`, alongside the existing `GitRunner`, with
`NativeGitRunner` remaining the desktop/system-git implementation and a new
`EmbeddedGitRunner`/`GitBackend` implementation added for mobile) so Git
implementation stays behind repository-owned semantics rather than leaking
into call sites.

**Remote clone/sync sequencing (Option E):** once Stage 2's embedded Git
backend exists, remote clone becomes the preferred acquisition path for
repositories already hosted on a Git host (HTTPS remotes first; SSH is
deferred — see below). Remote Git acquisition does not make the remote
runtime authority: the checked-out app-private working tree remains the
Workbench repository `DesktopService`/`RepositorySession` operate on.
Stage 1's SAF/archive import paths are retained even after Stage 2 ships,
because not every repository is remote, not every user has connectivity,
air-gapped/local projects exist, and users may receive repository bundles.

### Credential boundary

Credentials are not implemented by WI061. The implementation follow-up must
use OS-protected credential handling (e.g., a keystore-backed store on
Android; the exact mechanism is an implementation-time decision). No PAT,
password, or SSH private key may appear in repository records, frontend JS
state longer than strictly needed for one operation, logs, evidence, or
workspace-identity metadata. No credential-bearing remote URL may be
persisted.

### Network authority boundary

Embedded remote Git adds real Android network authority. That authority is
scoped to explicit, typed clone/fetch/pull/push commands only. It must not
become a generic HTTP client exposed to the frontend: no arbitrary-URL-fetch
command, no web proxy. The Android manifest's existing `INTERNET` permission
(present for the WebView) is not evidence this scoping problem is already
solved and must not be read as such.

### SSH sequencing

SSH support is not required for mobile Git v1. The recommended sequence is
HTTPS remotes first, with credential/token auth through protected storage,
and SSH later if warranted — this reduces native mobile dependency
complexity for the first shipped backend. This is a sequencing choice, not a
claim that HTTPS-only serves every adopter; SSH-only or SSH-preferring
adopters remain a known gap until SSH is added.

### `.git` behavior

Two states are distinguished and must not be conflated:

- **Imported non-Git tree** (Stage 1 SAF/archive import without a usable
  `.git`): no `.git` is present; `RepositoryTopology` uses its existing,
  already-shipped filesystem-only fallback (gated on
  `repository.root.join(".git").exists()`,
  `rust/crates/repopact-repository/src/lib.rs:44-107`); RepoPact validation
  and mutation still work; Git operations are simply unavailable. RepoPact
  must never synthesize fake `.git` metadata for an imported tree that
  lacks one.
- **Embedded-Git clone** (Stage 2): a real `.git` produced by the embedded
  backend is present in the app-private workspace; `RepositoryTopology` sees
  a genuine Git repository through the same code path already proven for
  desktop; Git operations are supplied through the embedded backend.

**Importing an existing `.git` via SAF directory import** is treated
conservatively for v1: a SAF directory import is a filesystem-project
import, and Git capability is recognized only if a valid, safely usable
copied `.git` structure is actually present after the copy. There is no
guarantee that every Android `DocumentsProvider` reliably exposes dotfiles
or preserves Git's internal structure (loose objects, packed-refs, index) on
copy. Where Git fidelity matters, remote clone (once Stage 2 exists) or
archive import of a bundle that includes the repository is the recommended
path, not relying on SAF directory-copy fidelity for `.git`.

### Copy and archive safety (recorded for the implementation follow-up)

Safe recursive directory import must specify: no path traversal, no writes
outside the allocated workspace root, bounded resource handling, a defined
symlink policy, defined duplicate-name behavior, cancellation, partial-import
cleanup, and atomic/publish-after-success where practical.

Safe archive import must specify: zip-slip protection, rejection of absolute
paths and `..` traversal, symlink-escape protection, archive-bomb bounds
(entry-count bound, expanded-byte bound, depth bound), duplicate/case-
conflicting path handling (Android's filesystem case-sensitivity may not
match a source archive's assumptions), and cleanup after a failed extraction.

WI061 records these requirements; it does not implement them.

### Export/write-back authority surface

The future mobile Tauri command surface should conceptually expose typed
operations only: import repository, export workspace, clone repository,
pull/sync, push. It must not expose an arbitrary read URI, an arbitrary
write URI, an arbitrary filesystem path, or a generic shell — preserving
Decision 0041's typed-command authority boundary on mobile exactly as it
already applies on desktop.

### iOS portability

This decision is triggered by, and its acceptance criteria are written in
terms of, Android — the only mobile platform WI060 actually proved. The
architecture generalizes naturally to iOS where it matters: an app-private
workspace, a document-picker-based import, archive import, an embedded Git
backend, and explicit export/share are all iOS-compatible concepts (iOS has
its own document-picker/document-provider model with a similar authority
shape to SAF). This decision does not implement or validate anything on
iOS; the architecture is recorded as mobile-general where possible, with
Android as the only platform this decision's evidence actually proves.

## Alternatives considered (rejected or deferred)

- **Live SAF document tree as the canonical Repository (Option B):**
  rejected. `content://` URIs are not `PathBuf` values `DesktopService`
  understands; Storage Access Framework provider semantics (directory
  walking, rename/atomic-replace/locking, symlink representation) do not
  reliably match the POSIX/filesystem assumptions `RepositoryTopology`,
  ROG/indexing, and mutation-plan path containment all depend on; making
  this work would require either rewriting `DesktopService` around a
  virtual-filesystem abstraction or building a second, document-provider-
  backed `Repository` implementation, both a far larger kernel change than
  mobile acquisition requires and both reintroducing the "second authority"
  problem Decision 0041 exists to prevent on desktop. SAF itself is not
  rejected — it remains the mechanism for acquisition and export (Stage 1).
- **Broad external-storage filesystem access** (e.g. legacy
  `MANAGE_EXTERNAL_STORAGE`-style access): rejected. No option this decision
  adopts requires it.
- **Bundling a `git` executable and shelling out on mobile:** rejected for
  mobile product architecture. Ordinary Android application packages cannot
  assume a system `git` binary is present or reachable, and bundling one
  does not resolve `NativeGitRunner`'s underlying architectural assumption
  so much as work around it fragilely per-platform.
- **Remote-only product** (no local SAF/archive import path): rejected as
  the sole acquisition mode. Not every repository is remote, not every user
  has network connectivity, air-gapped/local projects exist, and users may
  receive repository bundles rather than URLs.
- **A hand-rolled Git protocol implementation:** rejected. Reimplementing
  smart-HTTP/SSH transport and pack/object handling is unjustifiable when
  mature embeddable implementations (`git2-rs`/libgit2, `gix`) already
  exist.
- **`gix` as the first complete mobile Git backend today:** deferred, not
  banned. Reevaluate once its push and high-level porcelain workflow
  coverage matures to match what Stage 2 needs.
- **Implementing the selected architecture inside WI061 itself:** rejected
  by WI061's own AC-5. This decision, its comparison document, and the
  follow-up implementation work item are WI061's product; no `DesktopService`,
  `RepositorySession`, `NativeGitRunner`, `RepositoryTopology`, Tauri Android
  command, Android manifest/capability, Gradle, mobile plugin, or frontend
  repository-selection code changes with this decision's acceptance.

## Consequences

Mobile acquisition and synchronization become an adapter problem layered on
top of RepoPact's existing, already-proven repository kernel, rather than a
reason to build a second repository model. The Workbench, mutation engine,
ROG, and validation all continue to operate exactly as they do on desktop
once a workspace exists in `app_data_dir()`, regardless of whether that
workspace arrived via SAF import, archive import, or (from Stage 2 onward)
an embedded-Git clone.

The cost is that mobile users do not get "point at my folder and it's always
live" for free — Stage 1 is explicitly a copy-then-export model, which is an
honest but less convenient mental model than live synchronization. That cost
is accepted because the alternative (Option B) would require rebuilding
`DesktopService`'s filesystem assumption around unproven document-provider
semantics for every future feature that reads or writes repository content,
not just acquisition.

A second cost is that real write-back and synchronization (what most users
actually want) is deferred to Stage 2's embedded Git backend, which itself
carries real native-dependency, credential-handling, and network-authority
work that the implementation follow-up must scope carefully, prototype on
Android before treating `git2-rs` as irreversible, and never let expand into
a general filesystem/shell/network capability exposed to the frontend.

## Follow-up work

`work/active/061-mobile-repository-acquisition-app-private-workspace-and-git-strategy/`
creates a Stage-1 implementation work item at its closeout, scoped to a
production mobile workspace registry, SAF directory/document acquisition,
safe recursive directory import, safe archive import, the app-private
repository workspace, normal Workbench operation against it, explicit
export/share-back, typed mobile errors/progress/cancellation, and Android
runtime + security/permission proof. It establishes a clean seam for
embedded Git without necessarily implementing remote Git in the same item.
A likely later item, **Embedded Mobile Git Backend and Remote Repository
Synchronization**, is anticipated (a `GitBackend` abstraction, a
`git2-rs`/libgit2 Android prototype, HTTPS clone/fetch/push, protected
credential storage, pull/update/conflict handling, offline state, remote
tracking, sync UX, network authority scoping, and Android-then-iOS build
proof) but is not created by WI061 itself; it should be created or activated
at the Stage-1 implementation item's own closeout if still warranted then.
