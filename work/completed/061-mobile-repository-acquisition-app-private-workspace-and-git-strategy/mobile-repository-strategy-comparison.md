# WI061 Mobile Repository Strategy Comparison

```
coding agent = Claude Code
architecture reviewer = GPT-5.6 Sol High
```

This document satisfies AC-1, AC-2, and AC-3. It compares the five required
architectures for getting a real, RepoPact-governed tree onto an Android
device (and, where relevant, back off it) against `architecture-review.md`'s
verified facts, and is the evidentiary basis for Decision 0056.

## The five options

- **A. SAF import/export into an app-private workspace.** User picks a
  directory tree or a document/archive via SAF; RepoPact copies/imports into
  `app_data_dir()`; the Workbench operates on that private copy; edits are
  explicitly exported or shared back.
- **B. Live SAF document tree as the working repository.** RepoPact uses
  `ACTION_OPEN_DOCUMENT_TREE` / `DocumentsContract` / `DocumentFile` /
  `content://` URIs directly as the live tree `DesktopService` operates on.
- **C. Archive import.** User picks a `.zip` (or equivalent) via SAF; it is
  validated and extracted into `app_data_dir()`; RepoPact operates on the
  extracted normal files; write-back is explicit export/repack/share.
- **D. Embedded Git** (`git2-rs`/libgit2, or `gix`/gitoxide) providing
  clone/fetch/push/checkout without a system `git` binary.
- **E. Remote Git clone/sync as primary acquisition.** RepoPact obtains a
  remote URL and clones directly into its app-private workspace, bypassing
  SAF for repositories that already live on a Git host. The checked-out
  app-private tree remains the Workbench repository; the remote does not
  become runtime authority.

Options A/B/C are acquisition *transports*; D is an execution-layer choice
that stage-2 needs regardless of which of A/C/E supplies the initial tree;
E is a stage-2 acquisition path that depends on D existing.

## Scoring table

Legend: **strong** / **acceptable** / **weak** / **incompatible**.

| Dimension | A. SAF import/export | B. Live SAF tree | C. Archive import | D. Embedded Git (as a backend, not an acquisition mode by itself) | E. Remote clone/sync |
|---|---|---|---|---|---|
| `DesktopService` filesystem-directory compatibility | strong — copy lands in an ordinary `PathBuf` tree | incompatible — no `PathBuf` exists for a `content://` tree; would require a second storage abstraction | strong — extraction target is an ordinary `PathBuf` tree | strong — a clone target is an ordinary `PathBuf` tree | strong — clone destination is `app_data_dir()`, an ordinary `PathBuf` tree |
| `NativeGitRunner` system-git compatibility | acceptable — irrelevant unless the imported tree happens to carry a usable `.git` (fact 4/7 in architecture-review) | weak — even if content is exposed as files, no system `git` exists to operate on them | acceptable — same as A; Git operations unavailable unless the archive's `.git` is intact and trusted | incompatible with `NativeGitRunner` itself, but this is expected: D *replaces* the system-git assumption with `GitBackend`/`EmbeddedGitRunner` rather than satisfying `NativeGitRunner`'s existing contract (see fact 10) | strong once D exists — clone/fetch/push run through the embedded backend, not `NativeGitRunner` |
| `RepositoryTopology` `.git` behavior | strong — no `.git` present is the expected, already-supported case (fact 4) | weak — a synthesized or provider-backed `.git` concept has no defined semantics | strong — same as A unless a real `.git` was archived, in which case treat as D's case | strong — a real `.git` from `git2`/`gix` clone is an ordinary on-disk `.git`, which `RepositoryTopology` already reads correctly | strong — same as D |
| Mutation-plan/apply compatibility | strong — plans run against the private copy exactly as WI060 proved (android_validation.rs) | weak — plan path containment and file operations assume ordinary paths, not document-tree operations | strong — same as A | strong — no change to mutation semantics; only how `.git` got there changes | strong — same as A/C/D |
| Opaque plan-handle compatibility | strong — untouched, Decision 0041 unaffected | weak — a document-tree apply step still needs an ordinary filesystem target underneath, undermining the point of "live" | strong — untouched | strong — untouched | strong — untouched |
| Offline operation | strong — import once, then fully offline | acceptable — offline reads work if the provider caches locally, but this is provider-dependent and unreliable | strong — same as A | strong (backend-only capability; usable offline for local branch/checkout operations after a prior clone) | weak for the *initial* clone (needs network); strong afterward |
| No-system-git operation | strong — no Git required at all for import/export | strong — no Git required, but for the wrong reason (nothing here is Git) | strong — same as A | strong — this is D's entire purpose | strong — depends on D, not on system git |
| Write-back behavior | acceptable — explicit export/share required (see AC-2 resolution below); not automatic | weak — "live" implies no write-back step, but that requires trusting provider write semantics (rename/atomic-replace/locking vary) RepoPact cannot verify | acceptable — explicit export/repack, same shape as A | n/a (D is a capability, not a write-back model) | strong once D exists — `push` is real, first-class write-back |
| Permission surface | strong — one-time `ACTION_OPEN_DOCUMENT[_TREE]` grant, no persistent broad storage permission required | weak — requires persisting a document-tree grant across app restarts, and Android's SAF persistable-permission model is more fragile than a one-shot copy | strong — same as A, often even narrower (`ACTION_OPEN_DOCUMENT` on one file) | acceptable — no storage permission, but adds a compiled native dependency and its own security surface (fact 12; C/OpenSSL/SSH stack for libgit2) | acceptable — adds explicit network authority for the clone/fetch/push operations, scoped to those typed commands only |
| Network requirement | none | none | none | none by itself | required for clone/fetch/push; must not become a generic HTTP capability |
| Credential requirement | none | none (unless the provider itself gates access) | none | none by itself | required for private remotes; must use OS-protected storage, never workspace metadata or logs (see §22 of the drafting brief) |
| Implementation complexity | acceptable — safe recursive copy, path/traversal/symlink/resource bounds (§27) | strong (in the sense of "would be very large") — a second storage backend, `DesktopService` becomes dual-mode, ROG/indexing would need document-tree-aware walking | acceptable — safe extraction, zip-slip/bomb/symlink/entry-count bounds (§28) | weak — native build across Android/iOS, credential plumbing, prototype-gated per architectural-comparison guidance (§12) | acceptable, but only after D exists — clone/push wraps D; the acquisition-specific work (URL entry, credential UX, offline-state messaging) is smaller than D itself |
| Cross-platform portability (iOS) | strong — SAF is Android-specific, but "pick a source, copy into app-private storage" maps directly to the iOS document picker | weak — iOS's own document-provider model has the same category of problems as Android's, doubled work for no benefit | strong — archive import is platform-neutral once a picker exists | acceptable — `git2-rs`/libgit2 and `gix` both have iOS build paths, though neither is proven inside this repository yet | acceptable — same clone/push logic is platform-neutral once D exists on both platforms |
| Performance | acceptable — large recursive imports cost time/IO proportional to tree size, but happen once, not per-session | weak — provider directory walks and per-file document-tree calls are typically much slower than local filesystem syscalls, and would be *repeated*, not one-time | acceptable — extraction cost is bounded by decompression + write, comparable to A | strong once built — libgit2/gix operate as fast as any local Git client | acceptable — one-time clone cost, then local-filesystem speed identical to A/C |
| Failure recovery | strong — a failed/partial import can be cleaned up and retried without having touched the original source (fact: WI060's copy-then-operate pattern) | weak — a failed live write can leave the *source* (not just a disposable copy) in an inconsistent, provider-dependent state | strong — same as A; a corrupt/partial extraction never touches the original archive | acceptable — clone failures are well-understood Git failure modes (partial clone, network interruption); the library itself gives typed errors | acceptable — same as D; a failed clone leaves an empty/partial app-private directory, never the remote |
| User mental model | acceptable — "import a copy, work on it, export/share when done" is an unfamiliar step for a user expecting one live folder, but is honest about what actually happened | strong (if it worked) — "point at my folder, it's just always in sync" is the model users actually want, which is exactly why rejecting it needs to be explicit and well-justified, not just technically convenient | acceptable — "unzip a project" is a familiar, well-understood action | n/a (D is invisible plumbing to the user) | strong — "clone my repo" is the most familiar model for anyone whose project already lives on a Git host |

## AC-1: comparison scope confirmation

All five required options (A-E) are compared above against every named
architectural dimension from the work item's AC-1 text: `DesktopService`
filesystem-directory assumption, `NativeGitRunner` system-git assumption,
`RepositoryTopology`'s `.git`-presence gating, mutation plan/apply and
opaque-plan-handle semantics, and the offline/no-Git-binary constraint WI060
proved. AC-1 is satisfied by this table plus the per-option discussion below.

## Option A — SAF import/export

**Confirmed strengths** (verified against architecture-review.md facts 1-4,
9): preserves the ordinary filesystem core `DesktopService`/
`RepositorySession` already implement; no system `git` required; no broad
storage permission required (a single `ACTION_OPEN_DOCUMENT_TREE` or
`ACTION_OPEN_DOCUMENT` grant is enough to perform the one-time copy); works
fully offline after import; minimal authority expansion over WI060's
already-proven `android_validation.rs` pattern; compatible with mutation
handles and ROG/indexing exactly as WI060 proved on-device.

**Confirmed weaknesses:** the copied workspace can diverge from its SAF
source if the user or another app changes the original after import; write-
back is an explicit step, not automatic synchronization; a large recursive
import (many files, deep trees) costs time and I/O proportional to tree size
at import time; `DocumentsProvider` behavior (metadata availability,
dotfile/`​.git` visibility, symlink representation) varies by provider and is
not something RepoPact controls; Git metadata may or may not survive the
copy depending on the source provider (see §26 disposition below).

This option is **not** Git synchronization, and Decision 0056 must not
describe it as such — it is a one-time (or user-repeated) transport.

## Option B — Live SAF tree as canonical workspace

**This review rejects Option B as the canonical mobile repository
architecture.** Confirmed reasons, verified against the running code rather
than assumed:

- `content://` URIs are not `PathBuf` values. `DesktopService::open_repository`
  (fact 1) has no code path that accepts one, and none of `Repository`,
  `RepositorySession`, `RepositoryTopology`, or the mutation-plan pipeline
  operate on anything but `std::path::Path`/`PathBuf`.
- Provider semantics do not reliably match POSIX/filesystem assumptions:
  directory walking is per-document-call rather than a single `readdir`,
  rename/atomic-replace/locking/symlink semantics vary by provider and are
  not specified by the SAF contract, and `.git` semantics (a plain-file
  worktree pointer, packed-refs, loose objects, index locking) would become
  provider-dependent and unverifiable.
- Making this work would require either rewriting `DesktopService` around a
  virtual-filesystem abstraction, or building a second, document-provider-
  backed `Repository`/`RepositoryTopology` implementation — both are a far
  larger kernel change than mobile acquisition requires, and both introduce
  exactly the "second authority" problem Decision 0041 was written to avoid
  on desktop.
- ROG/source indexing (WI063) assumes ordinary files it can read with
  `std::fs`; mutation-plan path containment assumes normal filesystem paths
  it can canonicalize and verify are inside the repository root. Both
  assumptions would need re-proving under document-tree semantics.

**This review does not reject SAF itself.** SAF remains the correct,
narrow-authority mechanism for *acquisition and export* (Option A) and for
picking an archive to import (Option C). Only "SAF tree as the live,
canonical Repository" is rejected.

## Option C — Archive import

Archive import is a useful **secondary** acquisition mode alongside Option A,
not a replacement for it — some users receive a `.zip` bundle of a project
rather than pointing at a live folder.

**Strengths:** simple, deterministic import; portable across platforms (no
Android-specific provider quirks once the bytes are on-device); no
persistent SAF tree permission is needed after the one-time pick + extract;
easy app-private extraction target (same `app_data_dir()` root as Option A);
fully offline; works without any Git tooling.

**Risks the implementation follow-up must bound** (recorded here, not
implemented — see §28 of the drafting brief and `architecture-review.md`
fact 11 confirming none of this exists yet): zip-slip/absolute-path/`..`
traversal, symlink extraction escaping the workspace root, archive/resource
bombs (entry-count bound, expanded-byte bound, depth bound), duplicate or
case-conflicting paths (notably relevant on Android, where the underlying
filesystem may be case-sensitive while some source archives assume
case-insensitive semantics), executable-bit/permission handling, Git
metadata if the archive happens to include a `.git` directory, and a defined
write-back/repack UX for the exported result.

## Option D — Embedded Git library selection

Two realistic candidates exist for the future full mobile Git backend:
`git2-rs` (Rust bindings over libgit2) and `gix`/gitoxide (a Rust-native
implementation). They are **not equivalent** and must not be scored as
interchangeable.

| | `git2-rs` / libgit2 | `gix` / gitoxide |
|---|---|---|
| Maturity | mature; libgit2 has been an embeddable Git implementation for over a decade, used by GitHub Desktop, GitKraken, and many other embedders | actively developed, Rust-native, but younger as a complete porcelain-level tool |
| clone / fetch | full support | supported, actively maintained |
| pull (fetch+merge) building blocks | full support at the libgit2 level | fetch is solid; merge/pull as a single porcelain operation is less turnkey and may need to be assembled from lower-level pieces |
| push | full support, including credential callbacks | historically incomplete/immature; as of this decision date, `gix`'s push and higher-level porcelain workflow coverage lags libgit2's, which is exactly the capability WI061's Stage 2 needs most (write-back to a remote) |
| checkout / worktree mutation | full support | supported at a lower level; less turnkey porcelain |
| credentials (HTTPS token, SSH key) | full support via libgit2's credential-callback mechanism | supported, though the ecosystem/examples are less mature |
| Android build | proven path: libgit2 cross-compiles to Android's NDK targets, and `git2-rs` links against it; this is a well-trodden path for other embedders | should be buildable as pure(r) Rust with fewer native cross-compilation steps, but not yet proven inside this repository |
| iOS build | proven path, same reasoning as Android | same as Android: plausible, not yet proven here |
| Native dependencies | C libgit2 + its own dependencies (commonly OpenSSL or a TLS backend, libssh2 for SSH) — a real native build/security surface (CVEs in libgit2/OpenSSL/libssh2 become RepoPact's problem to track) | mostly pure Rust; smaller native surface, fewer independent CVE streams to track |
| Binary size | larger, due to the C dependency chain | expected to be smaller |
| Rust integration ergonomics | idiomatic but wraps a C API's error/ownership model | fully idiomatic Rust from the ground up |
| Maintenance risk | low risk of the *library* disappearing (extremely widely used); the risk is native-toolchain maintenance burden | some risk that required porcelain (push, in particular) is still catching up; lower toolchain burden once mature |
| Security history | libgit2/OpenSSL/libssh2 have each had real, patched CVEs over the years — a real, ongoing surface to track | smaller attack surface by construction (memory-safe Rust, fewer native dependencies), but has not had the same duration of adversarial scrutiny that a decade-old, extremely widely embedded C library has had |

**Conclusion for Decision 0056:** prefer `git2-rs`/libgit2 as the preferred
*implementation candidate* for the first complete mobile clone/fetch/push
backend, specifically because push and full remote-write support are mature
today and Stage 2's entire purpose is enabling real write-back to a remote.
This is not a permanent rejection of `gix` — it is a same-day, current-state
comparison. The implementation work item must prototype/build-gate the
choice on Android before treating it as irreversible, and `gix` remains the
right reevaluation candidate once its push/porcelain coverage matures.

## Option E — Remote Git clone/sync as primary acquisition

Once Option D exists, remote clone becomes the natural **preferred**
acquisition path for any repository that already lives on a Git host —
users type or paste a remote URL, RepoPact clones straight into
`app_data_dir()`, and the resulting `.git` gives `RepositoryTopology` (and
`NativeGitRunner`'s architectural successor) exactly the on-disk shape they
already understand (fact 4, fact 8-adjacent reasoning).

**Still must be evaluated on its own terms, not assumed to inherit D's
strengths for free:**

- **Offline use:** the initial clone requires network; every operation
  after that (browsing, mutation planning/apply, local diffing) is fully
  offline exactly like Options A/C, because the result is an ordinary
  on-disk tree.
- **Credentials:** HTTPS token/PAT or SSH key material must never live in
  RepoPact records, frontend JS state longer than needed, logs/evidence, or
  workspace metadata (§22); OS-protected storage (e.g. Android Keystore-
  backed storage) is a plausible mechanism, but the exact implementation is
  explicitly deferred to the follow-up work item.
- **Network permission:** already present in the Android manifest for the
  WebView (`INTERNET`, fact 12), but that fact must not be read as "network
  authority for Git is already solved" — the follow-up must scope network
  use to the specific typed clone/fetch/pull/push commands, never a generic
  HTTP capability exposed to the frontend.
- **Push/write-back:** real, first-class, unlike Options A/C's explicit-
  export model — this is Option E's main advantage once D exists.
- **Self-hosted Git / GitHub/GitLab compatibility:** both libgit2 and `gix`
  speak the standard Git smart-HTTP/SSH transports, so compatibility is a
  property of the embedded library (Option D), not something Option E adds
  independently.

**Remote Git acquisition does not make the remote runtime authority.** The
checked-out app-private working tree remains the Workbench repository that
`DesktopService`/`RepositorySession` operate on; the remote is a
synchronization endpoint the embedded backend talks to on explicit user
action (clone/fetch/pull/push), not a live backing store.

## AC-2: write-back resolution

For any option where the on-device copy is not the canonical checkout
(Options A and C), this comparison explicitly resolves write-back rather
than leaving it open:

> **Resolved for v1: explicit, user-initiated export/share-back. RepoPact
> never silently writes back to the original SAF tree or archive, and there
> is no background or automatic synchronization.**

Rationale: RepoPact does not have enough authority or information to safely
perform an implicit bidirectional merge against an arbitrary SAF tree or
archive it does not control the writer for — a fake synchronization
algorithm would be worse than an honest manual step, because it could
silently clobber changes made outside RepoPact. True versioned
synchronization is Option D/E's job (a real Git merge/rebase against a real
remote), not something Options A/C should simulate. This is exactly the
posture WI060 already modeled: fail/require an explicit step honestly rather
than fake a capability that does not exist.

## AC-3: security/permission matrix

| | broad storage permission? | persisted SAF grant needed? | network permission? | credential storage? | native library added? | system process needed? | shell needed? | unrestricted path/URI to frontend? |
|---|---|---|---|---|---|---|---|---|
| A. SAF import/export | no | no (one-shot pick, then ordinary app-private files) | no | no | no | no | no | no — Rust resolves the copy target; the frontend only ever sees `RepositoryOverview` DTOs, same as desktop |
| B. Live SAF tree (rejected) | no | yes — this is exactly the fragility this review rejects | no | no | no | no | no | would require exposing document-tree identifiers to *something*, widening the DTO surface for no governance benefit |
| C. Archive import | no | no | no | no | no | no | no | no |
| D. Embedded Git (backend only) | no | n/a | no by itself (only when actually cloning/fetching/pushing) | yes, for private-remote auth — must be OS-protected (§22) | yes — libgit2 (C) or gix's Rust dependency tree | no | no | no |
| E. Remote clone/sync | no | n/a | yes, scoped to clone/fetch/pull/push commands only | yes, same requirement as D | inherits D's | no | no | no — the frontend gets a "clone repository" typed command, never a generic URL-fetch capability |

Measured against WI060's narrow-authority baseline (fact 12): no option
selected by this decision requires `MANAGE_EXTERNAL_STORAGE` or any broad
storage permission, a generic shell/process authority, an unrestricted
filesystem command, or a generic `content://` browser exposed to the
frontend. Option D/E's network and credential requirements are real and new
relative to WI060's baseline, but are scoped to specific typed commands, not
general capabilities — this scoping requirement is recorded as binding on
the implementation follow-up in Decision 0056.

## Rejected/deferred alternatives (summary; full reasoning above)

- **Live SAF repository (Option B):** rejected as canonical architecture.
  SAF remains valid as an acquisition/export transport (Options A/C).
- **Broad external-storage filesystem access:** rejected outright; not
  required by any option that satisfies WI060's baseline.
- **Bundling a git executable and shelling out on mobile:** rejected for
  mobile product architecture; ordinary Android packages cannot assume this,
  and it does not solve the underlying problem `NativeGitRunner`'s
  assumption creates.
- **Remote-only product (no local import path):** rejected as the *sole*
  acquisition mode — not every repository is remote, not every user has
  connectivity, air-gapped/local projects exist, and users may receive
  bundles rather than URLs.
- **Hand-rolled Git protocol implementation:** rejected; reinventing smart-
  HTTP/SSH transport and object/pack handling is unjustifiable when mature
  embeddable implementations exist.
- **`gix` as the first complete mobile Git backend today:** deferred, not
  banned — re-evaluate once its push/high-level porcelain workflow coverage
  is mature enough to match what Stage 2 needs.
- **Immediate full implementation inside WI061:** rejected by AC-5; this
  document and Decision 0056 are the product of WI061, not code.
