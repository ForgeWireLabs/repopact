# Work Item 055 — Tauri 2 Desktop Governance Workbench

**Status:** Completed

**Owner:** work-coordinator

**Affected scopes:** work, tooling, docs, evidence

**Depends on:** WI053, WI054

## Intent

Build RepoPact's Tauri 2 desktop governance workbench as a secure, local, user-facing adapter over the Rust engine proven by WI053 and WI054.

The desktop is not a second RepoPact implementation and is not a generic Markdown/filesystem editor. TypeScript/React owns presentation and interaction state. Rust continues to own repository discovery, snapshots, validation, graph construction, analysis, mutation planning, stale-plan checks, recoverable apply, and generated projections.

WI055 is the first product layer allowed to expose the proven Rust mutation surfaces to a human user. It does not expand those surfaces merely because a UI would benefit from extra write authority.

## Activation basis

WI054 completed with durable evidence `20260910-054-rust-graph-mutation-core` proving:

- reusable `RepositorySession` / immutable snapshot / indexed record model;
- graph and deterministic analysis APIs;
- typed create/edit/transition mutation requests;
- content-addressed plan/read-set staleness checks;
- recoverable apply and rollback;
- Python/Rust create + dashboard parity;
- canonical edit/transition fixtures accepted by both validators;
- no Python source changes and no WI050 migration.

The Sol architecture review for this activation is recorded in `architecture-review.md`. Decision 0041 records the durable desktop authority boundary.

## Product surface

The initial workbench includes:

- repository chooser/switcher;
- repository dashboard/health;
- work browser and work detail;
- guided work-item creation;
- supported work-item edit and lifecycle transition flows;
- mutation review/preview before apply;
- decisions browser/detail;
- evidence browser/detail;
- relationship graph and source-backed analysis;
- validation diagnostics;
- live repository refresh;
- application settings for presentation-only preferences.

## Desktop authority boundary

The native process owns the selected repository session. The webview receives typed view models and sends typed user intent.

The frontend does **not** receive:

- generic filesystem read/write;
- arbitrary repository paths to crawl;
- unrestricted shell execution;
- arbitrary JSON Patch;
- a mutable authoritative `MutationPlan` to edit and resubmit;
- operator approval/WI050 authority.

A planned mutation is held in Rust memory. The frontend receives a `MutationPlanView` with an opaque plan handle, diagnostics, file/record/generated impacts, graph effects, and canonical preview. Apply accepts the opaque handle, retrieves the original Rust plan, checks its session binding/read set, and calls the WI054 apply path.

## Target application layout

The implementation should keep the Tauri adapter distinct from reusable RepoPact semantics. A target layout is:

```text
rust/
  crates/
    repopact-types/
    repopact-repository/
    repopact-validation/
    repopact-graph/
    repopact-analysis/
    repopact-mutation/
    repopact-core/
    repopact-desktop-api/      # UI-safe DTO/service adapter, no web framework

  apps/
    repopact-cli/
    repopact-desktop/
      package.json
      src/                     # React/TypeScript presentation
      src-tauri/               # Tauri command/event adapter
```

Exact naming may change if implementation evidence demonstrates a cleaner boundary, but Tauri and frontend dependencies must not leak into the reusable governance crates.

## Frontend stack

WI055 uses:

- Tauri 2;
- Vite;
- React;
- TypeScript;
- local packaged frontend assets only.

RepoPact domain DTOs used by TypeScript must be mechanically generated or mechanically checked from the Rust serde-facing desktop contract. Hand-maintained TypeScript copies of lifecycle/diagnostic/mutation domain semantics are not acceptable.

The UI may maintain ordinary presentation state such as selected tab, filters, unsaved form fields, theme, and expansion state.

## Repository selection and session lifecycle

Repository selection is a dedicated Rust-side native directory-picker operation.

Opening/switching repositories:

1. obtains a user-selected directory in Rust;
2. constructs/validates the RepoPact core session;
3. creates a new opaque desktop session generation/id;
4. clears mutation-plan handles from the previous session;
5. stops the previous watcher;
6. starts the new repository watcher;
7. returns the initial typed desktop snapshot/view.

Commands operating on repository state must bind to the current session generation. Stale commands/plan handles from a previous repository are rejected.

## Desktop DTO/view layer

Internal `RepositorySnapshot` and `RecordIndex` are not themselves the frontend API.

The desktop adapter should provide stable, serializable view types such as:

- `DesktopRepositoryView`;
- `RepositoryHealthView`;
- `WorkItemSummaryView`;
- `WorkItemDetailView`;
- `DecisionSummaryView` / `DecisionDetailView`;
- `EvidenceSummaryView` / `EvidenceDetailView`;
- `DiagnosticView`;
- `AnalysisView`;
- `GraphView`;
- `MutationPlanView`;
- `MutationApplyView`;
- `RepositoryChangedEvent`.

View models may denormalize data for presentation, but may not create new governance truth.

Raw representation is available only for records already present in the core index and is read-only. There is no arbitrary `read_file(path)` desktop command.

## Narrow command surface

The command layer should be operation-oriented. Candidate commands include:

```text
select_repository
repository_overview
list_work_items
get_work_item
list_decisions
get_decision
list_evidence
get_evidence
validate_repository
analyze_work_item
relationship_graph
plan_work_item_create
plan_work_item_edit
plan_work_item_transition
apply_mutation_plan
discard_mutation_plan
refresh_repository
get_indexed_record_raw
```

Exact names may be refined, but commands must remain repository/session scoped and typed.

## Work UX

### Browse/detail

Support:

- lifecycle filtering;
- owner/affected scope filtering;
- dependency filtering;
- search;
- active/blocked emphasis;
- acceptance-criterion state;
- evidence linkage;
- relationship navigation;
- frozen/analysis context where core data exists;
- validation context;
- read-only canonical representation.

Do not infer relationships from arbitrary Markdown mentions.

### Guided creation

The creation flow should cover:

1. intent/title;
2. deterministic repository analysis;
3. owner/affected scopes;
4. dependencies;
5. acceptance criteria;
6. evidence expectations/planning assistance;
7. governance/frozen review;
8. core mutation preview;
9. explicit apply.

The current work-item schema does not contain a standalone `evidence_plan` field. The UI may help a user think through evidence, but it must not invent an on-disk field or schema extension merely to mirror a wizard step.

The native adapter supplies canonical dates/preflight values required by the existing Rust mutation API rather than trusting editable browser clock/form fields as durable record authority.

### Edit and transition

Only WI054-supported typed edit fields and lifecycle transitions are writable.

A lifecycle transition uses the core whole-directory move semantics. The frontend does not implement a transition matrix that contradicts RepoPact's existing any-state-to-any-state formal model.

## Plan review

Before apply, display at least:

- mutation type and target;
- blocking/non-blocking diagnostics;
- changed records/files;
- generated dashboard impact;
- relationship impacts;
- canonical preview/diff;
- current/stale plan state;
- frozen-surface constraints.

The UI applies only an opaque backend-held plan handle. If stale, the user must re-plan explicitly.

## Decision and evidence UX

Decisions and evidence are read/browse surfaces in WI055.

Decision views expose canonical front matter, status, supersession, body/rationale, and only source-backed relationships supplied by the core.

Evidence views expose associated work, result, commands/checks, artifacts, provenance/timestamp data, criterion relationships, and validation context. Completed evidence is never presented as a generic overwrite editor.

## Graph and analysis UX

Graph data comes from `repopact-graph` through the desktop DTO layer. Visualization may use a frontend layout library, but that library has no semantic authority and persists no graph truth.

Every relationship/analysis finding should support navigation to its core-supplied source reference when possible. Provide an accessible non-canvas/tabular relationship representation in addition to any visual graph so graph meaning is not available only through pointer interaction or color.

## Filesystem watching and refresh

Watching runs in Rust, not through a JavaScript filesystem watcher.

Required behavior:

- recursively observe the selected repository through a Rust watcher;
- normalize and coalesce bursty events;
- classify relevant paths and ignore irrelevant build/cache noise;
- refresh the core session/snapshot;
- re-run relevant validation/view derivation;
- emit a typed event containing a new view generation/snapshot token;
- distinguish external vs self-originated refresh where practical;
- suppress duplicate UI refresh when watcher events resolve to the same post-mutation state;
- stop cleanly when switching repository or closing the app.

A bounded full-snapshot rebuild is an acceptable correctness fallback when a safe incremental update is not yet available. Frontend-owned recursive watching is not.

## Tauri security configuration

The app must use a deliberately narrow Tauri 2 permission/capability model.

Requirements:

- enumerate custom commands in the Tauri application manifest/permission model;
- grant only commands needed by the main local window;
- keep `withGlobalTauri` disabled;
- load local app content only;
- do not expose `tauri-plugin-fs` or shell execution to the webview;
- use the dialog plugin from Rust for repository selection rather than granting its generic JS API as the primary path;
- permit frontend event listening only to the extent required for native repository-change notifications;
- configure a reasonable CSP rather than disabling it for development convenience;
- do not label this boundary as WI050 enforcement.

## Accessibility and interaction quality

The app should be keyboard-usable and information-dense without becoming visually noisy.

At minimum:

- semantic navigation and controls;
- visible focus states;
- keyboard access to primary workflows;
- status not encoded by color alone;
- accessible labels and error messages;
- responsive sizing down to a practical narrow desktop window;
- dark/light themes;
- reduced-motion respect for nonessential animation;
- graph/table alternatives;
- loading, empty, stale-plan, validation-error, and repository-change states.

## Testing

WI055 requires proof below the level of screenshots.

### Rust/desktop adapter

Test:

- DTO mapping;
- session replacement and stale-session rejection;
- plan-handle storage/retrieval/invalidation;
- apply-by-handle cannot substitute caller-provided operations;
- watcher debounce/coalescing/classification;
- mutation-origin refresh convergence;
- indexed raw-record access rejects arbitrary paths;
- command/service error mapping.

### Frontend

At minimum run:

- TypeScript type check;
- production Vite build;
- component/unit tests for major navigation/workflows;
- accessibility-focused tests for key forms/plan review/navigation;
- tests proving frontend code does not implement RepoPact validation/mutation semantics.

### End-to-end/local application

Exercise on the available host:

- open a real adopted RepoPact checkout;
- browse work/decisions/evidence;
- run validation/analysis;
- create a work item through plan/apply;
- edit it;
- transition it;
- observe external file refresh;
- demonstrate stale-plan rejection;
- demonstrate repository switch invalidates prior plan handles.

## Cross-platform and packaging

Target Windows, Linux, and macOS desktop.

WI055 must distinguish:

- behavior actually executed on the development host;
- compilation/static checks available for other platforms;
- assumptions/documented prerequisites not yet executed;
- unsigned local bundles from production signing/notarization.

A local Windows bundle should be produced when the environment supports it. Linux/macOS signing/notarization do not need to be fabricated from a Windows host. Production distribution/signing may be follow-on work if not available during WI055.

Frozen `.github/workflows/**` remains outside WI055 absent separate operator approval.

## Explicitly out of scope

- Python compatibility/canonical-core cutover;
- PyO3/native-wheel packaging;
- generic decision/evidence mutation;
- arbitrary JSON Patch or filesystem writes;
- general SPEC mutation authority;
- provider-specific AI as authority;
- persistent mutation-plan/transaction database;
- WI050 admission/guard/enforcement/IPC/platform/protected-service migration;
- remote hosted application content;
- claiming production signing/notarization that was not actually performed.

## Closeout standard

WI055 closes when the Tauri desktop can open/switch an adopted repository, present health/work/decisions/evidence/validation/graph/analysis through typed Rust-backed views, perform WI054-supported work-item creation/edit/transition through opaque plan handles and explicit preview/apply, react to repository changes through the Rust watcher/event layer, and demonstrate a constrained Tauri security surface with no duplicate frontend governance authority.

## Closeout evidence

WI055 is satisfied by `20260910-055-tauri-desktop-governance-workbench`. The evidence records the exact Tauri/Rust/frontend versions, command and capability allowlists, Rust-owned TypeScript generation, bounded in-memory session/plan registry, Rust-native watcher, UI surfaces, frontend and Rust tests, Python baseline/conformance, Windows launch/build/bundle results, and the scratch-repository smoke.

The Windows host launch and native folder picker were exercised against an adopted scratch repository. The reusable Rust desktop boundary smoke additionally proved typed create/edit/transition plan-preview-apply, external refresh, stale-plan rejection after input drift, and stale-session rejection after repository switch. The configured desktop automation helper was unavailable on this host, so the visual automation limitation is recorded explicitly in the evidence rather than presented as a screenshot-only claim.

No Python source, canonical schema, invariant, charter, `.github/workflows/**`, or WI050 authority surface changed. PyO3/Python cutover, generic decision/evidence mutation, persistent transaction state, and cross-platform runtime/package verification remain outside WI055.
