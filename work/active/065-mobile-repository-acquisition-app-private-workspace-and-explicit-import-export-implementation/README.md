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
- **Checkpoint B — mostly done; one gap recorded honestly.** A real Android
  Tauri mobile plugin, `rust/crates/repopact-mobile-saf/`: Kotlin
  `@TauriPlugin`/`ActivityResult` handling for
  `ACTION_OPEN_DOCUMENT_TREE`/`ACTION_OPEN_DOCUMENT`
  (`SafAcquisitionPlugin.kt`), bridged from Rust via `PluginHandle::
  run_mobile_plugin` into `AndroidSafSource` (implements Checkpoint A's
  `AcquisitionSource` by listing one directory's children per SAF call --
  never a single upfront tree walk) and a staging-file bridge for archive
  documents. The production `MobileAcquisitionCoordinator`
  (`rust/apps/repopact-desktop/src-tauri/src/mobile_acquisition.rs`) wraps
  one `WorkspaceManager` rooted at `app_data_dir()/repositories` (never
  WI060's debug validation path), initialized once in `run()`'s `setup`.
  Seven typed commands (`mobile_workspace_list`, `mobile_import_directory`,
  `mobile_import_archive`, `mobile_operation_status`,
  `mobile_operation_cancel`, `mobile_workspace_open`,
  `mobile_git_capabilities`) are registered only on the Android build;
  `select_repository` and desktop's native picker are completely unchanged.
  A minimal frontend entry surface (`MobileAcquisitionPanel.tsx`: Import
  folder / Import ZIP / existing workspaces / Open) renders only where the
  mobile commands actually exist. A real `npx tauri android build --debug
  --target aarch64 --apk` **succeeded**, producing a genuine debug APK with
  the compiled `SafAcquisitionPlugin.class` linked in; the Android manifest
  permission set is unchanged (no new permission of any kind). The
  interactive on-device/emulator smoke test this checkpoint did not have
  time for was completed in Checkpoint B.5 below. See
  `evidence/runs/20260914-065-checkpoint-b-android-saf-acquisition.json`
  for the full record, including a real build failure (an illegal `--`
  inside an XML comment breaking Android's manifest merger) found and fixed
  by the first real build attempt.
- **Checkpoint B.5 — done, real SAF runtime acceptance.** Booted the
  WI060-proven emulator (`forge_moto_one_hyper_lab_api35`, Android 15/API 35,
  x86_64), built and installed a fresh APK from synchronized source, and
  drove the **real** production app (`android-debug-validation` not used)
  through both real SAF pickers via `adb`/`uiautomator` UI automation --
  never by injecting a URI into native code. Both a small controlled
  directory fixture and a controlled ZIP were imported end-to-end through
  every layer (picker → Kotlin plugin → Rust adapter → `AcquisitionSource` →
  Checkpoint A's bounded importer → staging → app-private workspace →
  registry → frontend list), with **on-device SHA-256 digests matching the
  pre-import fixtures byte-for-byte** for every file. A 400-file real-SAF
  import completed successfully too (408 total files across 3 workspaces),
  strengthening AC-3's real-tree requirement. Both workspaces opened via
  `mobile_workspace_open` and showed real Rust-backed reads (repository
  identity, validation) on the actual production app-private path. Registry
  persistence was proven across two full `force-stop` + relaunch cycles for
  all three workspaces. Picker cancellation (directory and archive, via the
  hardware Back key) was clean: no crash, no error, no phantom workspace.
  The installed package's runtime permission audit showed **zero** granted
  permissions beyond pre-existing `INTERNET`, and a full-session log-privacy
  audit found no `content://` leakage from this app's own log tags.
  Two real defects were found and fixed by this session's own real-build/
  real-run attempts (not fabricated): (1) `listChildren`'s root-level SAF
  call passed a tree URI where `DocumentsContract.getDocumentId` requires a
  document URI, throwing `IllegalArgumentException` on every first
  directory import; (2) that same exception's message (and, independently,
  `Logger.error`'s own throwable-argument stack-trace dump) would have
  logged the full picked-tree `content://` URI unredacted. Both fixed,
  rebuilt, reinstalled, and reverified. **Honest gap:** `mobile_operation_cancel`
  is implemented and host-tested, but `MobileAcquisitionPanel.tsx` exposes no
  UI affordance to trigger it mid-import, so a real runtime
  cancel-while-importing scenario was not executed this session -- recorded
  as a real follow-up rather than faked. See
  `evidence/runs/20260915-065-checkpoint-b5-android-saf-runtime.json`.
- **Checkpoint C — done, production-imported Android workspace mutation
  proof.** Proved that an app-private workspace acquired through the real
  production SAF path behaves identically to a desktop-opened repository
  through the existing, unmodified Rust-owned mutation system. On the same
  WI060/B.5-proven emulator, imported a new, richer SAF-directory fixture
  (a minimal but genuinely valid RepoPact repository -- the B.5 fixture was
  deliberately not one) through the real `ACTION_OPEN_DOCUMENT_TREE` picker,
  then drove the identical Workbench mutation UI used on desktop end to end:
  Work tab → `001 · Seed work item` → **Edit typed fields** → typed title
  edit → **Review edit plan** (`plan_mutation`) → real review dialog showing
  the opaque plan handle (`plan-1-1`), the Rust-generated diff preview, and
  generated impacts (no MutationPlan DTO serialized into or reconstructed by
  JavaScript) → **Apply approved plan** (`apply_mutation_plan`) → toast
  "Plan applied and post-validation completed." The on-device file hash
  changed exactly as planned (`430c135a…` → `1b0d7f9e…`), the session
  snapshot token was invalidated and replaced (`124440fdaaad…` →
  `52004f1cdd86…`) with no manual refresh, and the session generation
  counter incremented to `2` -- all without an app restart. A full
  force-stop + relaunch cycle then proved the mutation was durably
  persisted (not merely in-memory), and the original external SAF source
  file remained byte-identical throughout, confirming the app-private copy
  was the sole canonical, edited copy per Decision 0056. A cheap secondary
  read-only open of the pre-existing archive-imported workspace confirmed
  the same open/read path works identically for `saf_archive` acquisitions.
  No source change was required anywhere in `DesktopService`,
  `RepositorySession`, `RepositoryTopology`, or the mutation engine --
  Checkpoint C is a pure proof exercise against the already-landed
  production build, and the post-session permission/log-privacy audits
  stayed clean (no new permissions, no crashes, no leaked URIs). The
  optional negative stale-plan test (Section 17) was not attempted this
  session, honestly recorded as not executed rather than faked, since the
  primary AC-5 requirement -- one complete, valid plan/review/apply cycle
  proven through the real UI -- was already fully satisfied. See
  `evidence/runs/20260915-065-checkpoint-c-android-mutation-cycle.json`.
- **Checkpoint D (export/share-back wiring, including the typed export
  commands AC-7 also requires), Checkpoint E (adversarial device tests +
  final closeout) — not started.**

AC-1, AC-2, AC-3, AC-4, and now AC-5 are `satisfied` in `work-item.json`,
backed by the real on-device evidence above. AC-7 stays `pending`: its
exact wording requires the mobile command surface to expose typed
**export** operations too, and those belong to Checkpoint D, not yet built.
AC-6, AC-8, and AC-9 stay `pending` as Checkpoint D/E's job outright (AC-8
in particular still needs export/share-back runtime proof on top of this
checkpoint's mutation-apply proof). This work item stays `active` rather
than being closed against partial evidence.

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
