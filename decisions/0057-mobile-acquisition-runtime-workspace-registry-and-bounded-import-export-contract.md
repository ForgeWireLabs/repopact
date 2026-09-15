---
id: 0057
title: Mobile acquisition runtime, workspace registry, and bounded import/export contract
status: accepted
date: 2026-09-14
supersedes: []
---

# 0057: Mobile acquisition runtime, workspace registry, and bounded import/export contract

## Context

Decision 0056 settled the architecture: mobile repositories are app-private
ordinary filesystem workspaces; SAF, archives, and (later) Git remotes are
acquisition/synchronization transports, never alternate `Repository`
implementations. It deliberately left implementation-level details open for
WI065 to settle before code is final. This decision records those details.

## Decision

### Workspace root and layout

```
<app_data_dir>/repositories/
    registry.json              # atomic, native-owned workspace registry
    registry.json.tmp-<rand>   # transient write-ahead file, never left published
    workspaces/
        <workspace-id>/
            repository/         # the actual imported/exported repository tree
            local-metadata/      # acquisition-adapter-owned state (staging leftovers guard, source fingerprint), never part of the repository content
    staging/
        <operation-id>/         # transient import target; only ever promoted into workspaces/<id>/repository via an atomic rename, never written to directly
```

`repository/` is kept structurally isolated from `local-metadata/` so that
nothing the acquisition adapter writes for its own bookkeeping is ever
mistaken for repository content by `DesktopService`/`RepositoryTopology` —
`DesktopService::open_repository` is always pointed at exactly
`workspaces/<id>/repository`, never at the workspace directory itself.

### Workspace identity

A workspace id is a randomly generated UUID (v4), generated natively, never
derived from or influenced by a user-supplied display name or any acquired
content. The on-disk directory name is the UUID's canonical hyphenated
lowercase form, which contains no path-separator or traversal-meaningful
characters, so no user input ever participates in constructing the
workspace's filesystem path.

### Registry model

The registry is a single JSON document, native-owned, containing an array
of workspace records:

```
workspace_id        (UUID string)
display_name        (user-facing, editable, authority-free)
acquisition_kind     ("saf_directory" | "saf_archive" | "remote_git" [reserved, Stage 2])
source_reference     (opaque descriptor: for SAF, a provider-scoped identifier only meaningful for divergence comparison and, when persisted, a permission-scoped tree/document identifier -- never a raw credential; for remote_git [reserved], a URL without embedded credentials)
git_state            ("non_git" | "git_metadata_present" | "embedded_git_managed" [reserved, Stage 2])
lifecycle_state      ("allocating" | "importing" | "ready" | "exporting" | "failed")
created_at / imported_at  (RFC 3339 timestamps)
last_export_state    ("never_exported" | "exported" | "changed_since_export" | "divergence_unknown" | "diverged")
source_fingerprint    (bounded metadata captured at import time: relative-path set digest, aggregate size, provider-reported identifiers where available -- never a claim of cryptographic proof)
```

No credential, token, private key, or raw secret is ever a field of this
model. This is local product state describing what the app has acquired; it
is not RepoPact governance, not a repository source of truth, and not Git
metadata — Decision 0056's boundary applies unchanged.

### Atomicity and crash safety

Every registry mutation is: serialize the full registry to a temp file
inside the same directory as `registry.json` (so the later rename is same-
filesystem and atomic), flush/sync it, then atomically rename it over
`registry.json`. All registry reads/writes are serialized through one
native in-process coordinator (a `Mutex`-guarded owner), never touched
directly by the frontend. A registry that fails to parse is treated as a
loud, typed failure (`internal_io`/registry-corrupt), never silently
replaced with an empty registry — silent replacement would orphan every
already-imported workspace's identity. Repair/rebuild-from-workspace-
metadata tooling is left as explicit future work, not implemented here.

On startup, the coordinator scans `staging/` for leftover operation
directories from a prior crash or forced-kill and deletes them (they are, by
construction, never referenced by any `ready` registry entry, so deleting
them can never affect a valid workspace). It does not delete anything under
`workspaces/<id>/` merely because a workspace's `lifecycle_state` is not
`ready` in the registry unless that same id has no corresponding directory
integrity — recovery removes orphaned staging, not registered records.

### Concurrency

Exactly one acquisition or export operation may be active at a time for
v1. The operation coordinator holds this as an in-process invariant (an
`Option<OperationHandle>` guarded by the same coordinator lock that owns
registry writes); a second concurrent request is rejected with a typed
error rather than queued or interleaved. No database is introduced for the
registry; a single JSON document under one native writer is sufficient at
this scale, and SQLite is explicitly not justified by anything in this
work item's evidence.

### Lifecycle states

`allocating` (workspace id/registry entry reserved, no content yet) →
`importing` (bounded copy/extraction in progress in `staging/`) → `ready`
(published; safe for `DesktopService::open_repository`) or `failed`
(import did not complete; the registry entry is retained only long enough
to report the failure to the user, then removed — a `failed` workspace is
never treated as open-able) . `exporting` is a transient overlay on a
`ready` workspace during write-back, not a replacement for `ready`.
Transient per-operation progress (entries/bytes processed, current phase)
is native in-memory state scoped to the operation id, not persisted to the
registry — only the terminal state transition is durable.

### Staging-then-publish transaction

Both directory and archive import always: allocate a fresh staging
directory under `staging/<operation-id>/`; perform the entire bounded,
adversarial-input-hardened copy/extraction into that staging directory and
nowhere else; run a structural validation pass over the staged result; and
only then atomically rename/move the staged `repository/` content into
`workspaces/<workspace-id>/repository/` and flip the registry entry to
`ready` in the same coordinator-owned transaction. Any failure or explicit
cancellation at any point before publish deletes the staging directory and
leaves no `ready` registry entry — a partially imported tree is never
reachable through `DesktopService`.

### SAF native boundary

The Android-native surface is a narrow, typed acquisition boundary, not a
generic URI I/O API. Required conceptual operations: `PickDirectoryTree`,
`PickArchiveDocument`, `PickExportDirectory`, `CreateExportArchive`. Rust
receives only what it needs to continue the operation (an opaque,
process-scoped handle to the picked tree/document, plus enough metadata to
begin a bounded, streaming copy through the platform's content-resolver/
document APIs) — never a raw `content://` string exposed to JavaScript, and
never a generic `read_uri`/`write_uri`/`list_uri` command. Kotlin/Java owns
only what the Android platform forces it to own: `ActivityResult` handling,
`DocumentsContract`/`DocumentFile` traversal, `ContentResolver` streaming,
and URI-permission lifecycle. Rust owns workspace lifecycle, path
normalization, resource accounting, ZIP safety, the registry, operation
state, repository opening, and the export model.

### Persistable URI permissions

`takePersistableUriPermission` is called only for a SAF directory-tree
grant that a workspace's future export/divergence-check flow may need to
re-open without prompting the user again — i.e., only when the resulting
workspace record's `acquisition_kind` is `saf_directory` and the workspace
survives past the initial import. One-shot archive-document picks
(`saf_archive`) do not retain a persisted grant after import completes,
because Stage 1 archive export always creates a new document through a
fresh `CreateExportArchive` pick rather than writing back to the original
archive. A persisted grant is released (`releasePersistableUriPermission`)
when its owning workspace is removed via the typed removal command.

### No broad storage permission

`MANAGE_EXTERNAL_STORAGE` and legacy broad `READ_EXTERNAL_STORAGE`/
`WRITE_EXTERNAL_STORAGE` are not added. SAF grants are the entire external-
storage authority surface this work item introduces. If a future Android
target-level requirement is discovered to force something beyond SAF, that
is a stop-and-reassess event, not a quiet manifest edit.

### Directory import bounds

Centrally defined constants (implementation may tune exact values, recorded
in code comments and evidence, not restated here as unchangeable): a
maximum entry count, a maximum total copied-byte count, a maximum single-
file byte count, a maximum relative-path depth, and a maximum relative-path
length. Traversal is bounded and incremental — the source tree is never
fully materialized in memory before copying — and tracks running entry
count, byte count, depth, and cancellation state as it advances, so bounds
are enforced during traversal, not only after the fact.

### Path safety

Every candidate relative path is normalized and validated before being
joined to a workspace/staging root, reusing
`repopact_repository::{normalize_path, resolve_within_root}` rather than a
second, weaker normalization implementation. Rejected unconditionally:
`..` components, absolute paths, drive prefixes, UNC prefixes, embedded NUL
bytes, and any resolved path that escapes the intended root after
normalization.

### Symlink policy

Stage 1 does not materialize external symlink semantics from a SAF source:
Android `DocumentsProvider` trees have no portable POSIX symlink contract.
An entry that a provider exposes as a symlink-like or otherwise unsupported
special file type is rejected (or skipped, at the operation's choice) with
an explicit typed diagnostic — never followed, and never used to construct
a link that could escape the destination workspace. Archive imports use an
independent, stricter rule (below): archive symlink entries are rejected
outright for Stage 1.

### Duplicate and case-collision policy

Within one import operation, relative paths are compared under Unicode
NFC-normalized, platform-neutral case-insensitive comparison (matching the
common denominator of Android's case-sensitive filesystem underneath a
frequently case-preserving-but-not-case-sensitive source expectation, and
of ZIP's lack of case semantics). A collision under that comparison —
including an exact duplicate, a case-only collision, or a file-vs-directory
collision at the same normalized path — fails the entire import
(fail-closed), rather than silently overwriting one of the colliding
entries.

### Cancellation and progress

Cancellation is a native-owned, cooperative flag checked between bounded
units of work (e.g., after each entry or after each fixed-size chunk of
bytes), never inferred from the frontend disappearing. A cancelled
operation is reported as a distinct typed outcome from a failure, and
always triggers the same staging cleanup as a failure. Progress is a typed,
throttled event stream — `operation_id`, `phase`, `entries_processed`,
`bytes_processed`, an optional `total` when knowable, and a workspace-
relative current path (never a raw external URI, and never file content).
Events are coalesced (time- or count-based) so a large source does not
flood the WebView IPC channel with one event per file.

### Export semantics

Directory-workspace export writes the current workspace repository content
to a user-picked SAF destination through the same narrow native boundary,
only on explicit user action — never automatically, never merely because a
persisted grant exists. Archive-workspace export creates a new archive
through a user-picked `CreateExportArchive` destination rather than
mutating the original archive in place. Both exports include only
repository content — the registry, operation journals, and any
`local-metadata/` bookkeeping are excluded, unless such files were
genuinely part of the originally imported repository content itself.
Destination collision handling for v1 is the simplest safe mode: a
new/empty destination is required by default; an explicit "replace"
confirmation is a distinct, user-intent-gated mode, never a silent
overwrite.

### Source-divergence detection

At import time, a bounded `source_fingerprint` is captured (the relative-
path set, aggregate sizes, and any provider-reported identifiers available
through the SAF surface — not a cryptographic proof, and provider
modification timestamps are not treated as ground truth). Before an
export/write-back to the original logical source, the same bounded
comparison is attempted against the source's current observable state:
confident divergence blocks/warns pending explicit user choice; an
inability to compare (provider does not expose enough to compare) is
reported honestly as divergence-unknown, never asserted as "unchanged."

### Typed error taxonomy

`selection_cancelled`, `permission_denied`, `source_unavailable`,
`unsupported_entry`, `path_escape`, `duplicate_path`, `case_conflict`,
`resource_limit`, `archive_invalid`, `archive_symlink`,
`operation_cancelled`, `workspace_not_found`, `workspace_not_ready`,
`export_conflict`, `source_diverged`, `internal_io`. The frontend switches
on these typed codes; it never parses free-text error prose to determine
error class.

### Typed command surface

`list_mobile_workspaces`, `import_repository_directory`,
`import_repository_archive`, `cancel_mobile_operation`,
`export_workspace_directory`, `export_workspace_archive`,
`open_mobile_workspace`, `remove_mobile_workspace`. No command accepts an
arbitrary URI, arbitrary filesystem path, or generic read/write/list-by-URI
parameter; no command executes a process or shell; no command performs a
generic HTTP fetch. Stage-2-reserved capabilities (clone/fetch/pull/push)
are exposed, if at all, as a typed capability/status descriptor
(`stage2_not_available`) rather than as callable commands that would need
to fake success.

### Future Git seam

The workspace/registry model already reserves `acquisition_kind =
remote_git` and a `git_state` value for an embedded-Git-managed workspace,
and the command surface's shape (workspace-identity-first, operation-
coordinator-mediated) is designed so that adding real
`clone`/`fetch`/`pull`/`push` operations in Stage 2 extends this model
rather than redesigning workspace identity. No embedded Git dependency
(`git2`, `libgit2`, `gix`) is added in this work item.

## Alternatives considered

- **SQLite-backed registry:** rejected for v1; a single JSON document under
  one native writer with atomic rename is sufficient at this scale, and
  nothing in this work item's evidence demonstrates a need for relational
  queries, migrations, or concurrent-writer support SQLite would justify.
- **Case-sensitive-only duplicate detection:** rejected; it would let an
  import silently succeed with `A.txt`/`a.txt` both present, which then
  fails unpredictably on a case-insensitive/case-preserving export
  destination or a future embedded-Git checkout. Fail-closed at import time
  is safer than fail-unpredictably later.
- **Following/preserving symlinks from SAF sources:** rejected for Stage 1;
  SAF has no portable symlink contract to preserve faithfully or contain
  safely.
- **In-place writes to staging without a publish step:** rejected; a reader
  could observe a partially-copied tree, and a crash mid-import would leave
  an ambiguous, possibly-openable partial workspace.

## Consequences

Implementation has a fixed, reviewable contract for exactly the details
Decision 0056 left open, so the Rust-owned safe-import/export primitives,
the registry, and the native command surface can be built and tested
without re-deciding architecture mid-implementation. The cost is more
up-front schema and state-machine design than a minimal "just copy the
files" implementation would need; that cost is intentional, matching the
same reviewability standard Decision 0041 set for desktop mutation
handling.
