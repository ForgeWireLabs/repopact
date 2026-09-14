# 061 — Mobile Repository Acquisition, App-Private Workspace, and Git Strategy

> **Status**: ✅ Completed
> **Owners**: tooling (lead).
> **Depends on**: 060.
> **Coding agent**: Claude Code. **Architecture reviewer**: GPT-5.6 Sol High.

## Intent

WI060 brought the RepoPact Workbench up on Android end-to-end: the shared
desktop/mobile Tauri crate builds and launches on a real device, the compact
IA renders correctly across all eight sections, Android system Back/orientation/
lifecycle/IME behavior is proven, and a real Rust-owned mutation plan/review/
apply flow was exercised on-device with zero Git-process dependency. All of
that ran against a debug-only, feature-gated, app-private *validation*
repository staged by the operator via `adb`/`run-as` — deliberately not a
production repository-selection mechanism.

WI060 explicitly declined to solve production Android repository
*acquisition*: `select_repository` on a normal Android build returns a typed
`repository.mobile-selection-unavailable` error, because Tauri's Android
dialog plugin cannot pick a folder, an Android `content://` URI is not a
filesystem path `DesktopService` can consume, and no system `git` binary can
be assumed to exist on the device.

This work item's outcome is a **decision**, not code: compare the realistic
architectures for letting a real Android user get a real RepoPact-governed
tree onto their device (and, where relevant, back off it), and record which
one RepoPact adopts — or an explicit staged sequence — as a durable decision.
Implementation is out of scope here and belongs to whatever follow-up work
item this one's closeout creates.

## Decisions

**Decision 0056** — *Mobile repositories use app-private working copies with
explicit acquisition and synchronization boundaries* — is this work item's
main product. See `architecture-review.md` for the re-verified baseline
facts and `mobile-repository-strategy-comparison.md` for the full five-option
comparison, scoring table, and security/write-back matrices that Decision
0056 is built on. Summary of the decision:

- **Selected architecture:** RepoPact operates only on an app-private
  ordinary filesystem workspace on mobile. External document trees,
  archives, and Git remotes are acquisition/synchronization transports, not
  alternate `Repository` implementations. `DesktopService`,
  `RepositorySession`, the mutation pipeline, opaque plan handles, and
  ROG/indexing remain exactly as they already are on desktop.
- **Canonical mobile workspace:** app-private internal storage
  (`app_data_dir()`), already proven reachable and Workbench-compatible by
  WI060's `android_validation.rs`.
- **Live SAF tree (Option B):** rejected as the canonical repository model —
  `content://` URIs are not `PathBuf` values, and provider semantics do not
  reliably match the POSIX assumptions `RepositoryTopology`/ROG/mutation
  path containment depend on. SAF remains valid for acquisition and export.
- **Archive import (Option C):** adopted as a secondary Stage 1 acquisition
  mode alongside SAF directory import.
- **V1 write-back (resolves AC-2):** explicit, user-initiated export/share-
  back only. No silent or background synchronization against the original
  SAF tree or archive.
- **Embedded Git (Option D):** deferred to Stage 2. `git2-rs`/libgit2 is the
  preferred implementation candidate (mature push/remote-write support
  today); `gix` is deferred, not banned, pending its push/porcelain
  maturity. A `GitBackend`-style seam alongside the existing (shell-shaped)
  `GitRunner` trait is required rather than reusing `GitRunner` directly.
- **Remote clone/sync (Option E):** the preferred Stage 2 acquisition path
  once embedded Git exists, but does not replace Stage 1's local import
  paths, and the remote never becomes runtime authority over the checked-
  out app-private tree.
- **Rejected/deferred alternatives:** live SAF repository, broad external-
  storage access, bundling a `git` executable, a remote-only product,
  hand-rolled Git protocol implementation, and `gix` as today's first
  complete backend — each with reasoning recorded in Decision 0056.

## Scope

- `architecture-review.md` — re-verified WI060 baseline facts plus new
  facts this review surfaced (the `GitRunner` shell-shaped seam, the proven
  `app_data_dir()` workspace root, the current Android permission surface).
- `mobile-repository-strategy-comparison.md` — the required five-option
  comparison, scoring table, git2-rs-vs-gix table, and AC-2/AC-3 resolution.
- `decisions/0056-...md` — the durable architectural decision.
- `evidence/runs/20260914-061-mobile-repository-strategy-decision.json` —
  the evidence run tying activation, research, authorship, and closeout
  together.
- `work/proposed/065-mobile-repository-acquisition-app-private-workspace-and-explicit-import-export-implementation/`
  — the Stage 1 implementation follow-up this item's closeout creates.
- No production code changes; no Android Gradle/Kotlin/Rust changes. This
  work item is complete through decision, not code.

## Security boundary

Measured against WI060's narrow-authority baseline: no option this decision
selects requires `MANAGE_EXTERNAL_STORAGE` or any broad storage permission,
a generic shell/process authority, an unrestricted filesystem command, or a
generic `content://` browser exposed to the frontend. Stage 2's network
(clone/fetch/pull/push) and credential requirements are real and new, but
must be scoped to specific typed commands and OS-protected credential
storage, never general capabilities — this is binding on the follow-up.

## Closeout

All five acceptance criteria are satisfied:

- **AC-1** — satisfied by `mobile-repository-strategy-comparison.md`'s
  scoring table, which compares all five required options against every
  named dimension.
- **AC-2** — satisfied by the explicit write-back resolution recorded in
  both the comparison document and Decision 0056 (explicit user-initiated
  export/share-back; no silent synchronization).
- **AC-3** — satisfied by the comparison document's security/permission
  matrix and Decision 0056's credential/network-authority boundary sections,
  measured against WI060's narrow-authority baseline.
- **AC-4** — satisfied by Decision 0056, which durably selects the staged
  architecture and records every rejected/deferred alternative with reasons.
- **AC-5** — satisfied: this work item contains no implementation, and
  `work/proposed/065-.../` exists as the separate implementation follow-up
  at closeout.

See `evidence/runs/20260914-061-mobile-repository-strategy-decision.json`
for the full run record.
