# 065 — Mobile Repository Acquisition, App-Private Workspace, and Explicit Import/Export Implementation

> **Status**: 📋 Planning
> **Owners**: tooling (lead).
> **Depends on**: Decision 0041, WI060, WI061 (Decision 0056).

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

The governing architectural decision (Decision 0056) and its rationale
already exist and are not re-litigated here. Any additional hard-to-reverse
choice this item makes during implementation (e.g. the exact workspace-
identity schema, the exact safe-extraction bound constants) should be
recorded in this README first and promoted to a `decisions/` record only if
it needs to outlive this work item.

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
