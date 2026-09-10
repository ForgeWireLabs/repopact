---
id: 0041
title: Tauri desktop uses Rust-owned repository sessions and opaque mutation-plan handles
status: accepted
date: 2026-09-09
supersedes: []
---

# 0041: Tauri desktop uses Rust-owned repository sessions and opaque mutation-plan handles

## Context

WI053 and WI054 established a reusable Rust core that owns repository discovery, immutable snapshots, validation, graph construction, deterministic analysis, typed mutation planning, content-addressed stale-plan protection, recoverable apply, dashboard regeneration, and post-write validation. WI055 is the first graphical client of those semantics.

A Tauri webview is a presentation environment, not a trusted RepoPact authority kernel. If the frontend recursively reads repository files, reconstructs domain semantics, accepts arbitrary filesystem paths, or receives a complete mutation plan and later sends a potentially modified plan back for execution, the desktop would recreate exactly the second-authority problem the Rust migration was designed to eliminate.

The desktop also needs live refresh and repository switching. Those operations require state that is meaningful to the native process: which repository is currently open, which immutable snapshot generation the UI represents, which watcher belongs to that repository, and which mutation plans were produced from that session.

## Decision

1. The Tauri 2 application is a thin adapter over the reusable Rust core. Repository discovery, validation, graph, analysis, and mutation semantics remain in the reusable Rust crates. TypeScript/React owns presentation, interaction state, and form state only.
2. The native process owns the selected `RepositorySession`. Opening or switching repositories creates a new desktop-session generation, tears down the prior watcher, invalidates prior mutation-plan handles, and returns a typed frontend view model. The frontend does not own a recursive repository crawler.
3. Repository selection is performed by a dedicated Rust-side native directory-picker command. WI055 does not grant the webview generic filesystem read/write, shell execution, or arbitrary path traversal merely to choose or operate on a repository.
4. Tauri commands are narrow and operation-oriented. They accept domain intent such as analysis queries or typed work-item forms, not arbitrary filesystem operations or raw JSON Patch.
5. A planned mutation remains authoritative only in Rust memory. The frontend receives a serializable `MutationPlanView` containing an opaque plan handle/token, diagnostics, changed-record/file summaries, graph/generated impacts, and canonical preview. It does not receive authority to alter the underlying `MutationPlan` and resubmit modified file operations.
6. `apply_mutation` accepts the opaque plan handle/token for the current desktop repository session. The backend retrieves the original Rust `MutationPlan`, verifies the session binding and WI054 content-addressed read set, applies it through the core, and invalidates the plan after success or terminal failure. A plan from a prior repository/session is rejected.
7. Frontend-facing DTOs are a separate adapter contract from internal repository structures. TypeScript API types must be mechanically generated or mechanically checked against the Rust serde DTO contract; RepoPact domain rules are not duplicated in hand-maintained TypeScript models.
8. Filesystem watching runs in Rust below the webview. Relevant path events are normalized, coalesced/debounced, classified against RepoPact semantics, and result in a new repository view generation or a typed change event. A bounded full snapshot rebuild is an acceptable correctness fallback; the frontend does not implement its own watcher.
9. Self-originated mutation events are correlated/coalesced with the apply result so the UI converges to the post-apply snapshot without feedback loops. External editor/Git changes remain distinguishable as externally originated refreshes where practical.
10. Tauri events are native-to-frontend state notifications. The normal desktop capability grants listening/unlistening required for those events but does not need frontend event emission as an authority mechanism.
11. The application uses Tauri 2 with a local Vite + React + TypeScript frontend. It does not load remote application content. `withGlobalTauri` remains disabled. Any plugin or capability must be justified by a WI055 operation rather than enabled as a broad default.
12. Custom application commands are explicitly enumerated in the Tauri application manifest/permission model and capability configuration. Generic filesystem and shell plugins are not exposed to the frontend. The native dialog plugin may be used from Rust for repository selection without granting its JavaScript API as a general frontend capability.
13. The desktop capability boundary is local application containment only. It does not claim to provide WI050 admission, operator approval, guard, cryptographic authorization, IPC enforcement, or protected-service guarantees.
14. WI055 does not make Rust globally canonical. It consumes the Rust surfaces proven by WI053/WI054 while WI056 remains responsible for Python compatibility and any later canonical-core cutover.

## Alternatives considered

- Let React read files through `tauri-plugin-fs`: rejected because it would duplicate repository interpretation and widen the webview filesystem authority.
- Let JavaScript use the generic dialog API and then pass arbitrary selected paths to backend commands: rejected for the primary workflow because the native process can perform repository selection and validation directly with a narrower authority boundary.
- Serialize the complete `MutationPlan` to JavaScript and accept it back at apply time: rejected because the webview could modify file operations or other plan details before execution.
- Re-plan automatically when a plan is stale: rejected because Decision 0040 deliberately requires explicit re-planning against changed repository facts.
- Persist mutation plans or a transaction database in application/repository state: rejected for WI055; plans are ephemeral native process state unless separately governed later.
- Implement repository watching in React: rejected because path classification and RepoPact refresh semantics belong below the presentation boundary.
- Use a remote/SSR web application as the desktop frontend: rejected because WI055 is a local governance workbench and does not require remote content or server authority.

## Consequences

The desktop can be feature-rich without becoming a competing RepoPact implementation. Repository switching, live refresh, mutation preview, and apply all have explicit native ownership. The frontend remains replaceable because it consumes a typed DTO/command contract rather than internal filesystem structures.

The cost is additional adapter code: Rust view models, command permissions, plan-handle storage, watcher lifecycle, and generated/checked TypeScript bindings. That cost is intentional. It makes the security and authority boundary reviewable and allows future desktop, IDE, or other clients to reuse the same core without granting a webview direct governed-file authority.
