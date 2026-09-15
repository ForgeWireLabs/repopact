# 065 — Mobile Repository Acquisition, App-Private Workspace, and Explicit Import/Export Implementation

> **Status**: 🔨 Active
> **Owners**: tooling (lead).
> **Depends on**: Decision 0041, WI060, WI061 (Decision 0056).
> **Coding agent**: Claude Code. **Architecture reviewer**: GPT-5.6 Sol High.

## Intent

WI061 decided (Decision 0056) that mobile repositories are ordinary
app-private filesystem workspaces, acquired via SAF import, archive import,
or (Stage 2, not this item) an embedded Git backend — never a live
document-provider-backed repository model. This item implements **Stage 1**
of that decision: production SAF directory/document acquisition, safe
recursive directory import, safe archive import, an app-private repository
workspace registry, normal Workbench operation against the resulting
workspace, and explicit export/share-back. It replaces `select_repository`'s
current unconditional `repository.mobile-selection-unavailable` error on a
normal Android build with a real, narrow-authority acquisition flow.

**In scope:** a production mobile workspace registry; SAF
`ACTION_OPEN_DOCUMENT_TREE`/`ACTION_OPEN_DOCUMENT` acquisition; safe bounded
recursive directory import; safe bounded archive import (zip-slip/bomb/
symlink-hardened); the app-private workspace `DesktopService`/
`RepositorySession` open exactly as on desktop; explicit, user-invoked
export/share-back for both import kinds; typed mobile command surface
(import/export only — no clone/pull/push yet); typed progress/cancellation/
error reporting; real Android on-device runtime and security/permission
proof.

**Out of scope:** Stage 2 embedded Git (`git2-rs`/`gix` integration, real
clone/fetch/push, remote credential storage) — that is a further follow-up
this item's own closeout may create or activate; iOS implementation;
changing `RepositoryTopology`, the mutation engine's repository-model
assumptions, or Decision 0041's opaque-plan-handle boundary; any change to
WI050 admission/guard authority.

## Decisions

The governing architectural decision (Decision 0056) already exists.
**Decision 0057** — *Mobile acquisition runtime, workspace registry, and
bounded import/export contract* — settles the implementation-level details
Decision 0056 deliberately left open (workspace root/layout, workspace
identity, registry model, atomicity/recovery, staging-then-publish,
SAF native boundary shape, import/export bounds, duplicate/case-collision
policy, symlink policy, cancellation/progress, export semantics,
divergence-detection honesty requirement, typed error taxonomy, and the
future Git seam), and is binding on this item's implementation.

## Progress

This item is being implemented in checkpoints (see `evidence/runs/` for the
full record of each):

- **Checkpoint A — done.** `rust/crates/repopact-mobile-acquisition/`: the
  workspace registry (atomic temp-file-then-rename JSON, corrupt-registry-
  fails-loudly, no credential fields, stale-temp-file recovery), the bounded
  Rust-owned safe directory importer and safe ZIP importer/exporter (both
  reuse `repopact_repository`'s existing path-containment primitives; both
  enforce entry/byte/depth/path-length bounds during traversal, never
  trusting a source-declared size; both fail closed on exact-duplicate,
  case-only, and file/directory-type path collisions), the operation
  coordinator (single-active-operation, cooperative cancellation, throttled
  progress), and the `WorkspaceManager` orchestration layer implementing
  the staging-then-publish transaction and safe workspace removal. 46
  passing tests, including the adversarial cases (traversal, absolute path,
  duplicate/case/type collisions, resource-limit overflow with a forged
  size hint, cancellation, cleanup, zip-slip, archive symlink rejection,
  malformed ZIP, crash-recovery of orphaned staging). Two real defects this
  work surfaced (a Windows-specific path-canonicalization asymmetry in the
  reused containment primitive, and a ZIP directory-entry trailing-slash
  collision-key gap) are documented and fixed in place, not worked around.
  See `evidence/runs/20260915-065-checkpoint-a-workspace-registry-and-safe-import-export.json`.
- **Checkpoint B — not started.** The Android-native SAF adapter (a real
  Tauri mobile plugin: Kotlin `@TauriPlugin`/`ActivityResult` handling for
  `ACTION_OPEN_DOCUMENT_TREE`/`ACTION_OPEN_DOCUMENT`/`ACTION_CREATE_DOCUMENT`,
  bridged to an `AcquisitionSource` implementation), the typed Tauri command
  surface wiring `repopact-mobile-acquisition` into `repopact-desktop`, and
  the Android Gradle/capability changes this requires. `repopact-mobile-
  acquisition` deliberately exists as a standalone, already-tested crate
  precisely so this checkpoint only has to bridge to it, not redesign it.
- **Checkpoint C (Workbench open + mutation cycle), Checkpoint D (export/
  share-back wiring), Checkpoint E (adversarial device tests + Android
  runtime proof + closeout) — not started.**

AC-1 through AC-9 therefore remain `pending` in `work-item.json`: Checkpoint
A is real, tested, host-verified progress, but AC-1 ("a production mobile
workspace registry exists") and the rest are written in terms of the
running application and real Android runtime evidence, which this
checkpoint does not yet provide. This work item stays `active` rather than
being closed against partial evidence.

## Scope

- `rust/apps/repopact-desktop/src-tauri/src/lib.rs` (`select_repository` and
  new import/export commands)
- a new mobile-acquisition module/crate (workspace registry, safe import,
  safe extraction)
- `rust/apps/repopact-desktop/src-tauri/gen/android/` (SAF-related
  Kotlin/plugin glue if Tauri's own plugins do not cover
  `ACTION_OPEN_DOCUMENT_TREE`/`ACTION_OPEN_DOCUMENT`)
- `rust/apps/repopact-desktop/src-tauri/capabilities/`,
  `permissions/` (narrow additions only — no generic fs/shell)
- frontend workspace-selection/import/export UI
- `evidence/runs/` (Android on-device import/export/mutation/export-back
  proof)

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are
satisfied, move this directory to `work/completed/`, regenerate the
dashboard, and — if embedded Git is still warranted and not already
covered — create or activate the Stage 2 follow-up ("Embedded Mobile Git
Backend and Remote Repository Synchronization") per Decision 0056.
