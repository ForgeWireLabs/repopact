# Codex Execution Directive — WI055

**Coding agent:** Codex  
**Authoritative work item:** WI055  
**Architecture umbrella:** WI052  
**Prerequisites:** WI053 and WI054 completed  
**Required starting main:** WI055 activation commit containing Decision 0041  
**Delivery mode:** direct to `main`, preserve concurrent work, no force-push/reset/history rewriting.

## Mission

Implement RepoPact's first Tauri 2 desktop governance workbench as a thin, secure client of the Rust engine proven by WI053/WI054.

The success condition is not "Tauri launches." The success condition is that a user can open a RepoPact repository, browse and understand canonical state, run validation/analysis/graph views, perform WI054-supported work-item create/edit/transition through backend-held mutation plans with explicit preview/apply, receive live repository updates, and do so without granting the webview a second governance or filesystem authority.

## Required reading before edits

Read all applicable contracts, then at minimum:

1. root `AGENTS.md` and `governance/owners.json`;
2. WI052 active architecture inventory;
3. completed WI053 and its evidence;
4. completed WI054 README, architecture review, work item and evidence;
5. Decision 0040;
6. Decision 0041;
7. WI055 `README.md`, `architecture-review.md`, and `work-item.json`;
8. live Rust crates `repopact-core`, `repopact-repository`, `repopact-graph`, `repopact-analysis`, `repopact-mutation`, `repopact-types`;
9. current Tauri 2 capability/permission/command documentation as needed while implementing.

## Start-state rules

- Fetch latest `origin/main` and confirm the WI055 activation commit is present.
- Work from a clean tree.
- Inspect concurrent changes before each push; preserve them.
- No force-push, hard reset of shared history, or history rewriting.
- Run the existing Python + Rust + legacy conformance baseline before desktop changes.
- Keep Cargo target/build output outside the repository where practical.
- Do not modify frozen `.github/workflows/**`, canonical schemas, invariants, or charter without a separate approved reason.
- Do not change Python source unless a real independently proven defect requires it; report before doing so if the desktop appears to require Python changes.

## Baseline to preserve

The WI054 closeout baseline includes:

- Python: 205 passed, 2 skipped;
- Rust workspace format/check/tests green;
- 20/20 legacy conformance;
- 8/8 WI050 admission corpus;
- create/dashboard parity;
- canonical edit/transition fixtures;
- linked worktree/reference parity;
- frozen surface clean;
- no Python source changes.

Desktop work must not regress that baseline.

## Phase 1 — desktop adapter crate before Tauri UI

Create a reusable Rust adapter crate, preferably `repopact-desktop-api` or a clearly equivalent boundary.

It should depend on the proven RepoPact core crates and must **not** depend on React/TypeScript. Tauri dependency should be avoided here if practical so service logic is testable without a webview/runtime.

Own here:

- frontend-safe DTOs;
- desktop repository/session state;
- view derivation;
- plan-handle registry;
- command/service error model;
- watcher abstraction/lifecycle where practical.

Do not expose internal filesystem crawling or duplicate semantic rules.

## Phase 2 — typed DTO contract and TypeScript generation/check

Define explicit frontend DTOs rather than serializing `RepositorySnapshot`/`RecordIndex` wholesale.

Minimum families:

- repository/health/session view;
- work summary/detail/filter;
- decision summary/detail;
- evidence summary/detail;
- validation diagnostics;
- analysis report view;
- graph view;
- mutation-plan presentation view;
- mutation apply result view;
- repository-change event.

Rust serde DTOs are the source. Generate or mechanically check the TypeScript declarations from Rust. `ts-rs`, a small owned generator, schema generation, or equivalent is acceptable.

Do not hand-copy lifecycle, diagnostic, graph, or mutation semantics into TypeScript.

Add an executable test/check that fails when generated frontend bindings drift from Rust.

## Phase 3 — desktop session manager

Implement native state equivalent to:

```text
DesktopState
  active_repository

DesktopRepositoryState
  session_id
  generation
  RepoPactCore/session owner
  plan registry
  watcher controller
  latest view token
```

Requirements:

- only one active repository is required for WI055;
- selecting another repository tears down the prior watcher;
- prior plan handles become invalid;
- stale session ids are rejected;
- concurrent commands cannot apply a plan to the wrong repository;
- no persistent transaction/plan database.

Use synchronization appropriate to Tauri command concurrency. Keep lock scope small around IO-heavy operations where possible.

## Phase 4 — authoritative plan registry

This is mandatory before the UI can apply mutations.

A stored entry contains:

```text
StoredPlan
  opaque handle
  desktop session id
  original repopact_mutation::MutationPlan
  MutationPlanView
```

Rules:

- frontend never supplies executable `file_operations` for apply;
- frontend never round-trips a raw `MutationPlan` as authority;
- plan handle lookup must match current session;
- plan registry is bounded;
- switch repository clears it;
- successful apply invalidates the handle;
- terminal invalid-plan/session errors invalidate where appropriate;
- stale WI054 read-set failure is surfaced as stale and requires explicit re-plan;
- no automatic silent replan.

Test that modifying the frontend-visible plan DTO cannot alter what the backend applies.

## Phase 5 — Rust-native watcher

Implement watcher logic below the webview.

Use a Rust watcher (`notify` family or equivalent) rather than the JavaScript filesystem plugin.

Required:

- recursive observation of the selected repository;
- ignore build/cache noise consistent with repository semantics;
- normalize paths;
- debounce/coalesce bursts;
- classify whether refresh is relevant;
- rebuild/refresh the core snapshot safely;
- validate/derive a new view generation;
- stop old watcher on repo switch;
- stop on app shutdown;
- avoid duplicate post-mutation refresh when watcher events resolve to the already returned snapshot token;
- surface invalid externally edited states rather than repairing them.

A bounded full core snapshot rebuild after relevant debounced events is acceptable for WI055. Do not over-engineer fragile partial index mutation.

Test external event coalescing and self-originated mutation convergence without requiring a real Tauri window where possible.

## Phase 6 — scaffold the desktop app

Create:

```text
rust/apps/repopact-desktop/
```

Use:

- Tauri 2;
- Vite;
- React;
- TypeScript;
- one deterministic JS package manager/lockfile.

The Tauri Rust crate may live in the conventional `src-tauri` directory and should be added to the Rust workspace if that is clean for builds/tests.

Frontend application content must be local. No SSR/server framework or remote hosted UI.

Keep `withGlobalTauri` disabled.

## Phase 7 — Tauri permissions/capabilities before broad feature wiring

Configure the security boundary early, not as cleanup.

Use Tauri 2 AppManifest/custom permissions so application commands are explicitly enumerated. Configure one main-window capability with only the needed commands and minimal framework/event permissions.

Do **not** expose to the frontend:

- `tauri-plugin-fs`;
- shell/process execution;
- unrestricted path functions as an authority mechanism;
- generic dialog JS authority as the primary repository selector;
- frontend event emit/emit-to unless an actual requirement proves it necessary;
- remote origins/capabilities.

Use Rust-side `tauri-plugin-dialog` (or current official equivalent) from a dedicated repository-selection command.

Set a reasonable CSP. Do not globally disable CSP for convenience.

Add static tests/inspection proving the final capability does not contain filesystem/shell/remote authority.

## Phase 8 — repository selection/open/switch

Implement a dedicated command:

```text
select_repository()
```

It opens a native folder picker in Rust, then immediately attempts to open/derive the repository through `repopact-desktop-api`.

Return a typed repository view/session id or a typed cancelled/error outcome.

Do not expose a generic `read_path`/`browse_path` API.

Repository switch must invalidate old plans and old session ids and replace the watcher cleanly.

## Phase 9 — repository shell and dashboard

Build the main UI shell:

- repository identity/path display;
- health state;
- watcher/refresh indicator;
- navigation: Dashboard, Work, Decisions, Evidence, Graph, Validation, Analysis, Settings.

Dashboard should use structured DTO data, not re-parse `audits/reports/dashboard.md` in TypeScript.

Initial empty/no-repository/error states must be intentional.

## Phase 10 — work browse/detail

Implement:

- text search;
- lifecycle filter;
- scope filter;
- dependency filter where useful;
- active/blocked emphasis;
- list summary;
- detail view;
- criteria/evidence relationships;
- dependency/reverse-dependency relationships;
- source-backed analysis/frozen context;
- validation diagnostics;
- read-only raw canonical representation for indexed records.

`get_indexed_record_raw` must resolve only core-indexed record ids/refs. Never accept arbitrary frontend filesystem path read.

Do not invent canonical "related decision" edges from Markdown text search. Only show canonical graph edges as relationships. Non-authoritative search suggestions must be labeled as such if added.

## Phase 11 — decisions and evidence

Decision browsing is read-only in WI055:

- id/title/status/date/front matter;
- body/rationale text;
- supersession relationships;
- graph/source references actually known by the core.

Evidence browsing is read-only:

- id/work item/result/provenance/timestamp;
- command/check summaries;
- artifacts;
- criterion/work relationships;
- validation context.

Do not add decision/evidence mutation to satisfy UI convenience.

## Phase 12 — validation, analysis, graph

Expose validation and deterministic analysis through narrow commands.

Graph view uses the core graph DTO. A frontend graph-layout library may be used only for presentation.

Provide a non-visual relationship table/list that is keyboard and screen-reader usable.

Every analysis finding should display classification (`fact`, `constraint`, `suggestion`) and source basis when available.

## Phase 13 — guided work-item creation

Create a multi-step form using existing schema/core surfaces.

Stages should cover:

1. title/intent;
2. repository analysis;
3. owner/affected scopes;
4. dependencies;
5. acceptance criteria;
6. evidence expectations/planning assistance;
7. governance/frozen review;
8. plan preview;
9. explicit apply.

Do **not** create a new `evidence_plan` record field. It is UI assistance only unless mapped to existing acceptance/evidence semantics.

The backend adapter supplies the canonical current date/preflight inputs required by WI054 create planning. The browser should not provide an editable durable creation date as authority.

Before apply, the UI must display `MutationPlanView` including blocking diagnostics, file/record changes, generated dashboard impact, graph impacts, preview/diff, and plan handle/session state.

Apply by plan handle only.

## Phase 14 — typed edit

Expose only WI054-supported edit fields:

- title;
- owner scope;
- affected scopes;
- dependencies;
- provenance where supported;
- acceptance criteria.

Backend supplies `updated` date.

Plan, preview, explicit apply. No raw JSON editor write mode.

## Phase 15 — lifecycle transition

Expose a transition flow using the WI054 typed transition request.

Do not implement an invented frontend transition matrix. RepoPact currently permits any state -> any state, with post-state validation and preserved history.

Warnings are okay; semantic rejection comes from the core planner/validator.

Reopen completed work must preserve the complete directory/history/evidence through the core.

## Phase 16 — watcher events into React

Native watcher refresh should emit a typed event to the main webview.

Include enough data to safely invalidate frontend cached DTOs, e.g.:

- session id;
- generation;
- view/snapshot token;
- origin classification if known;
- changed path summaries/capped count;
- validity/diagnostic summary or signal to refetch.

Prefer emit-to main window. Frontend needs listen/unlisten; it should not require event emit authority for this workflow.

React should refetch/replace typed views on a newer generation rather than merging raw filesystem events.

## Phase 17 — UI quality and accessibility

Make the app usable, not merely functional.

Requirements:

- dense but readable desktop layout;
- responsive behavior for practical narrow windows;
- dark/light themes;
- keyboard navigation;
- visible focus;
- non-color-only status;
- accessible form labels/errors;
- focus management for dialogs/plan review;
- reduced-motion handling;
- accessible graph relationship alternative;
- clear loading/empty/error/stale/refresh states.

Do not let visual polish bypass core diagnostics or hide blocked plans.

## Phase 18 — testing

### Preserve repository baseline

Re-run:

- editable Python install;
- Python repository validation;
- Python tests;
- Rust fmt/check/workspace tests;
- existing legacy conformance runner against Rust validator;
- WI050 admission corpus;
- frozen-surface check.

### Desktop Rust tests

Must cover:

- DTO mappings;
- session replacement;
- stale session rejection;
- plan-handle registry/bounds;
- handle invalidation;
- frontend cannot substitute executable operations;
- watcher classification/debounce;
- duplicate post-mutation refresh suppression;
- indexed raw access restriction;
- typed command/service errors.

### Frontend tests

Run at minimum:

- TypeScript checking;
- generated-binding drift check;
- production Vite build;
- component/unit tests for major workflows;
- accessibility-focused tests for navigation/forms/plan review.

### Security configuration tests

Prove/inspect:

- custom commands enumerated;
- no frontend fs permission/plugin authority;
- no shell/process permission;
- no remote capability;
- `withGlobalTauri` false;
- local frontend assets;
- CSP present;
- event capability no broader than needed.

## Phase 19 — local end-to-end smoke

Use a scratch adopted RepoPact repository, not uncontrolled direct mutation of the development repo.

Demonstrate on the available Windows host:

1. launch desktop;
2. choose/open repo;
3. browse dashboard/work/decisions/evidence;
4. validation/analysis/graph render;
5. create work item via plan + apply;
6. edit via plan + apply;
7. lifecycle transition via plan + apply;
8. external file change triggers refresh;
9. create a plan, externally change a consumed input, then demonstrate stale-plan rejection;
10. create a plan, switch repository, then demonstrate old plan/session rejection;
11. final scratch repo validates with both Python and Rust where supported.

Capture exact observed results in evidence.

## Phase 20 — build/package status

Run production frontend and Tauri build commands available on the host.

If Windows bundle creation succeeds, record exact artifact types/paths and whether they are signed or unsigned.

Do not claim Linux/macOS executable validation from Windows. Record:

- source/build portability checks actually run;
- known platform prerequisites;
- Linux/macOS status as unexecuted if not executed;
- signing/notarization as deferred unless actually performed.

No frozen CI/release changes in this work item without separate operator approval.

## Commit/push discipline

- Prefer coherent commits by architecture phase, not every file.
- Keep `main` clean between completed pushes where practical.
- Preserve concurrent work.
- No force push/reset/history rewrite.
- Regenerate dashboard when lifecycle/evidence records require it.
- Do not move WI055 to completed until the closeout evidence is durable and every satisfied acceptance criterion points to it.

## Explicit prohibitions

Do not:

- implement RepoPact validation rules in TypeScript;
- expose frontend arbitrary filesystem reads/writes;
- expose frontend shell/process;
- execute a frontend-supplied raw `MutationPlan`;
- auto-replan stale mutations silently;
- invent a new lifecycle transition matrix;
- invent an `evidence_plan` schema field;
- add decision/evidence mutation;
- add provider/LLM authority;
- add a persistent transaction/plan database;
- port WI050 admission/guard/enforcement/IPC/platform/protected-service semantics;
- weaken Decision 0040/0041;
- modify frozen CI/release files without approved governance.

## Stop and report if

Stop before implementation proceeds into any of these:

- canonical schema change is required;
- current core lacks a necessary semantic operation and you are tempted to recreate it in TypeScript;
- Tauri capability constraints appear to require broad fs/shell authority;
- production signing credentials are needed;
- a persistent native database/journal appears necessary;
- Linux/macOS packaging requires changes to frozen CI;
- WI050 operator authority appears necessary for normal desktop apply.

## Required completion report

Return:

- final `main` SHA;
- commits created;
- exact Tauri/Rust/frontend versions chosen;
- new crate/app layout;
- command list;
- final capability/permission identifiers;
- whether fs/shell/remote frontend authority exists (expected: no);
- DTO/type-generation mechanism and drift-test result;
- session/plan-handle design implemented;
- watcher design and refresh behavior;
- work/decision/evidence/graph/analysis/validation UI delivered;
- create/edit/transition UX delivered;
- Python test result;
- Rust test result;
- legacy conformance result;
- WI050 corpus result;
- frontend typecheck/test/build result;
- Tauri dev/build/bundle result;
- local E2E scratch-repo result;
- stale-plan and stale-session results;
- frozen-surface result;
- Windows/Linux/macOS exact tested/unverified status;
- unsigned/signed packaging status;
- known deferred surfaces;
- whether WI055 is genuinely ready for closeout;
- any finding requiring WI052/WI056 amendment.

Do not report WI055 complete solely because the UI launches or compiles.
