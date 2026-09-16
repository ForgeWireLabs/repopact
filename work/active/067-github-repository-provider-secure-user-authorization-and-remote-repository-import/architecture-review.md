# WI067 Checkpoint A — Architecture Review

> **Coding agent**: Claude Code.
> **Architecture reviewer**: GPT-5.6 Sol High.

## Method

This review re-read the live repository as authority rather than working
from the operator's prompt alone: WI067's own README and `work-item.json`
(GH-001..GH-015 verbatim, unmodified by this checkpoint); Decision 0041
(Tauri desktop session and opaque-plan boundary); Decision 0056 and
Decision 0057 (mobile acquisition, the Stage-2 Git seam, and the bounded
import/export contract WI067 must reuse); WI065's final closeout evidence
(`evidence/runs/20260915-065-final-closeout.json`) and its completed
work item; WI068's proposed scope (Embedded Mobile Git Backend); the
current `DesktopService`/`RepositorySession` (`rust/crates/
repopact-desktop-api/src/lib.rs`, `rust/crates/repopact-repository/src/
lib.rs`); the current `repopact-mobile-acquisition` crate (`registry.rs`,
`archive.rs`, `workspace.rs`, `operation.rs`); the mobile Tauri command
surface (`rust/apps/repopact-desktop/src-tauri/src/mobile_acquisition.rs`
and its `permissions/workbench.toml` allowlist); and the workspace's
current dependency graph (`rust/Cargo.toml`, `Cargo.lock`) to confirm no
HTTP client library was already selected.

## Findings that shaped this checkpoint's design

1. **`DesktopService`/`RepositorySession`/`RepositoryTopology` are
   `PathBuf`-based and provider-agnostic already.** Nothing in them
   assumes a filesystem origin's provenance. WI065 proved this by adding
   an entirely new acquisition adapter (SAF/archive) with zero source
   change to any of the three. The same is achievable for a GitHub-sourced
   snapshot: once bytes exist under `workspaces/<id>/repository/`, this
   layer is identical to desktop. This confirms item 5/6's non-negotiable
   invariant is achievable without new abstractions in that layer.

2. **`repopact-mobile-acquisition::archive::import_archive` is already the
   correct materializer** for a GitHub archive/ZIP snapshot -- it takes any
   `Read + Seek` byte source and a staging root, and enforces zip-slip,
   symlink, entry-count/byte/depth bounds independent of where the bytes
   came from. WI067 must not build a second extractor (item 34); it needs
   only to produce a byte source and, per item 35, verify/strip a
   provider-generated wrapper directory before that byte source reaches
   the importer's existing entry-by-entry path validation. Checkpoint A
   proves the handoff shape with a non-GitHub fixture
   (`repopact-remote-provider/tests/provider_neutrality.rs`); wiring a real
   GitHub-downloaded ZIP through it is later-checkpoint work (GH-007
   remains pending).

3. **`AcquisitionKind::RemoteGit` (WI065/Decision 0056) is reserved for
   WI068's real `.git` clone/fetch/pull/push, not for this item's
   immutable ZIP snapshot.** Reusing it for WI067 would blur the two
   acquisition semantics decision 0056 was explicit about keeping apart.
   Checkpoint A adds a distinct `AcquisitionKind::RemoteSnapshot` (item
   33) rather than repurposing `RemoteGit`.

4. **No HTTP client crate exists anywhere in the workspace today**
   (`Cargo.lock`/`Cargo.toml` grep, confirmed empty for `reqwest`/`ureq`/
   `hyper`). Selecting one is a real, consequential decision (TLS
   verification defaults, redirect policy, timeout API) that this
   checkpoint does not need to make yet, because Checkpoint A's scope is
   the device-flow *protocol* state machine, not a live network call.
   Introducing a transport dependency now, before a real download exists
   to exercise it, would be scope creep. Instead this checkpoint defines
   an internal `HttpTransport` seam (`repopact-provider-github::transport`)
   the device-flow logic is tested against via a scripted double; a later
   checkpoint selects and wires the real client behind that same seam
   without touching the device-flow logic.

5. **The mobile Tauri command surface (`workbench.toml`) is a narrow,
   explicit allowlist**, exactly matching Decision 0041's typed-command
   boundary WI065 preserved. WI067's eventual commands (item 45) must be
   added to it the same way WI065's export commands were (and were once
   forgotten, per WI065 Checkpoint D's real defect #2) -- this is recorded
   here as a concrete trap for the checkpoint that adds them, not
   re-litigated now since no command is wired yet.

## Conclusion

The provider-neutral architecture in Decision 0061 (below) and the
concrete crate split in this checkpoint (`repopact-remote-provider`,
`repopact-provider-github`) are grounded in what the live repository
already does, not assumed from the operator's brief. No change was made to
`DesktopService`, `RepositorySession`, `RepositoryTopology`'s core
repository model, the mutation pipeline, or ROG authority.
