# WI055 Sol Architecture Review — Tauri 2 Desktop Governance Workbench

**Review date:** 2026-09-09  
**Reviewed base:** `b5c3263862d72cfce9ad25ca00b7fdd0ce98a709` plus Decision 0041 activation preparation  
**Coding agent:** Codex  
**Purpose:** verify the live Rust core is ready for a Tauri consumer and define the desktop boundary before implementation.

## Executive conclusion

WI055 is ready to activate.

WI054 delivered the missing governance engine beneath the proposed desktop: a persistent repository session/snapshot, serializable graph and analysis results, typed mutation requests/plans/results, content-addressed stale-plan protection, rollback, dashboard regeneration, and post-write validation. The desktop no longer needs to invent repository or mutation semantics.

The primary remaining architectural work is therefore **adapter and presentation architecture**, not another governance port.

The correct desktop shape is:

```text
                  canonical RepoPact records
                           |
                           v
                reusable Rust governance core
      +--------------------+--------------------+
      |                    |                    |
 repository/session   graph/analysis       mutation plan/apply
      |                    |                    |
      +--------------------+--------------------+
                           |
                           v
                 repopact-desktop-api
        DTO mapping + desktop session state
        plan-handle registry + watcher facade
                           |
                           v
                    Tauri 2 commands
                           |
                           v
                React + TypeScript webview
                 presentation/form state only
```

The most important rule is that **the frontend must not become the holder of executable mutation authority**. It may display a mutation preview, but the original `MutationPlan` remains in Rust memory and apply is by opaque handle.

## WI054 readiness verification

The durable WI054 evidence records:

- Python baseline/final: editable install green, 205 tests passed, 2 skipped;
- Rust format/check/workspace tests green;
- 20/20 legacy conformance;
- 8/8 WI050 admission corpus;
- Python/Rust create + dashboard parity;
- exact canonical edit/transition fixtures accepted by both validators;
- stale-plan, drift, rollback, failure-injection and no-write planning tests;
- linked-worktree/reference/ignored-directory parity retained;
- no Python source changes;
- no WI050 port.

This is sufficient to let WI055 consume the proven Rust surfaces without reopening their semantics.

## Current Rust surface

### `repopact-core`

The live facade owns a `RepositorySession` and exposes:

```text
identity()
snapshot()
validate()
graph()
analyze(query)
plan_mutation(request)
apply_mutation(plan)
apply_mutation_with_options(plan, options)
```

This is intentionally non-Tauri and should remain so.

### `repopact-repository`

The snapshot/index already includes:

- work items;
- evidence;
- decisions;
- policies;
- contracts;
- invariants;
- frozen surface;
- owner scopes/roles;
- audit registry/findings;
- dashboard;
- relevant source-path/read-set inputs.

The internal snapshot/index types are useful engine structures, but they are **not** the right frontend wire contract because they contain paths, parse-result internals, and data organized for semantic processing rather than UI stability.

### `repopact-graph`

`RepositoryGraph`, nodes, edges, node/edge kinds, and source references are serde-serializable. This makes the graph easy to adapt for frontend rendering without rewriting graph semantics.

### `repopact-analysis`

`AnalysisQuery`, `AnalysisReport`, findings, classifications, kinds, source basis and related records are serde-serializable. This is already a good authority boundary for deterministic UI analysis.

### `repopact-mutation`

Mutation request/plan/result types are serde-serializable, but the desktop **should not expose the raw executable `MutationPlan` as a round-trip authority object**.

The plan contains:

- repository identity;
- read set;
- plan token;
- mutation request;
- concrete file operations;
- generated impacts;
- graph impacts;
- diagnostics;
- preview.

That is ideal internal state. It is too authoritative to deserialize from an untrusted webview and execute directly after round-trip modification.

## Durable desktop decision

Decision 0041 records the desktop-specific authority model:

- Rust owns the selected repository session;
- repository selection uses a Rust-side native directory picker;
- frontend receives stable DTOs, not filesystem traversal authority;
- mutation plans remain native in-memory state;
- frontend apply is by opaque plan handle;
- watcher is Rust-owned;
- custom Tauri commands/capabilities are enumerated narrowly;
- local Tauri security is not WI050 enforcement.

No amendment to Decisions 0040 or WI052 is required.

## Frontend stack decision

Use:

```text
Tauri 2
Vite
React
TypeScript
```

Rationale:

- Tauri officially supports React/TypeScript/Vite;
- React is appropriate for the planned dense, stateful workbench views;
- Vite gives a simple local packaged frontend with no SSR/server dependency;
- TypeScript is useful at the adapter boundary, provided types are generated/checked from Rust instead of redefining domain truth.

Use a normal local frontend build. Do not introduce Next.js/SSR, remote web origins, or an application server for WI055.

A package manager choice is not governance architecture. Use one deterministic lockfile and document it. Avoid adding multiple JS package managers.

## Target crate/application boundary

Recommended layout:

```text
rust/
  crates/
    repopact-desktop-api/
      src/
        lib.rs
        dto.rs
        state.rs
        plans.rs
        watcher.rs
        services.rs

  apps/
    repopact-desktop/
      package.json
      vite.config.ts
      tsconfig*.json
      src/
        app/
        components/
        features/
        lib/
        generated/
      src-tauri/
        Cargo.toml
        build.rs
        tauri.conf.json
        capabilities/
        permissions/
        src/
          lib.rs
          main.rs
          commands.rs
```

`repopact-desktop-api` should depend on the reusable RepoPact crates but not Tauri/React. It makes view mapping/session/plan behavior testable independently of a real webview.

`src-tauri` should be a thin transport/lifecycle adapter over that desktop service layer.

## Desktop state model

Use one managed native state object conceptually like:

```text
DesktopState
  active_repository: Option<DesktopRepositoryState>

DesktopRepositoryState
  session_id
  root display identity
  RepoPactCore / RepositorySession owner
  generation
  latest view/snapshot token
  plan_registry
  watcher controller
```

The exact synchronization primitive can be chosen by implementation, but commands must not race repository switching with plan/apply.

### Session identity

Every repository open/switch creates a new opaque session id/generation.

Commands that depend on a currently open repository should include/validate the session id. This prevents a delayed frontend response from repository A from being applied after the user has switched to repository B.

### Repository switch

Switch is a state transition:

```text
stop old watcher
invalidate old plans
open selected root
build core session/snapshot
derive view models
start watcher
increment/new session generation
emit/return new repository view
```

Never silently carry plan handles across repositories.

## Repository selection

Use Tauri's native dialog support from Rust.

Do not expose generic JavaScript filesystem access just to pick a directory. The selected path should be consumed immediately by the native repository-open service.

The returned frontend value should be a repository view/session id, not a grant to recursively read that directory.

If implementation wants to show the selected path, return a display path string from the Rust view model. That does not imply filesystem authority.

## DTO contract

Create a presentation DTO layer rather than serializing engine internals wholesale.

### Repository view

Candidate:

```text
DesktopRepositoryView
  session_id
  generation
  repository_identity
  display_root
  health
  work_counts
  validation_summary
  active_items
  changed_state_token
```

### Work summary/detail

Summary should contain fields needed for list/filter/search without loading full raw records.

Detail may include:

- canonical typed work item;
- criteria/evidence summaries;
- dependency/reverse-dependency data;
- core-provided graph/source relationships;
- relevant analysis findings;
- validation diagnostics;
- indexed raw representation.

Do not synthesize "related decisions" from text search unless it is clearly labeled search context rather than a canonical relation. Current graph authority should remain the source of relationship truth.

### Decision/evidence

These are read-only WI055 views.

Decision view can expose front matter/body/supersession and canonical graph relations.

Evidence view can expose its JSON record, commands/results/artifacts/provenance/timestamp and graph links to work/criteria.

### Raw record

Raw access must accept a core `RecordRef` or stable indexed record key. The backend resolves that indexed record. Do not accept arbitrary frontend file paths.

## Rust -> TypeScript type contract

The frontend needs a checked contract for command DTOs.

Preferred requirement:

- Rust serde DTOs are source;
- TypeScript declarations are generated or checked mechanically;
- generation/check is reproducible in tests/build;
- generated file is marked generated;
- frontend code does not hand-maintain alternate lifecycle/diagnostic/mutation enums.

`ts-rs`, a small repository-owned generator, JSON Schema-based generation, or equivalent is acceptable if it keeps dependencies reasonable. Avoid adopting a heavy RPC framework simply to save a few interfaces.

## Command surface

Commands should be narrow enough to audit.

### Repository/session

```text
select_repository() -> DesktopRepositoryView | Cancelled
repository_overview(session_id)
refresh_repository(session_id)
```

No `open_any_path(path)` as a generic authority command is needed for the primary desktop workflow.

### Work

```text
list_work_items(session_id, filter)
get_work_item(session_id, id)
```

Filters are presentation queries, not authority rules.

### Decisions/evidence

```text
list_decisions(session_id)
get_decision(session_id, id)
list_evidence(session_id)
get_evidence(session_id, id)
```

### Validation/analysis/graph

```text
validate_repository(session_id)
analyze_work_item(session_id, query)
relationship_graph(session_id, optional focus/filter)
```

### Mutation planning

Expose operation-specific commands instead of a generic mutation JSON envelope if that produces a cleaner frontend contract:

```text
plan_work_item_create(session_id, form)
plan_work_item_edit(session_id, id, form)
plan_work_item_transition(session_id, id, status)
```

The Rust adapter fills native date/preflight fields required by the core.

### Plan apply

```text
apply_mutation_plan(session_id, plan_handle)
discard_mutation_plan(session_id, plan_handle)
```

Do not accept `MutationPlan`, `PathOperation`, arbitrary changed content, or file operations from TypeScript.

## Plan registry

Conceptually:

```text
StoredPlan
  handle
  session_id
  core_plan
  presentation_view
```

Properties:

- in-memory only;
- bounded per session to prevent unbounded accumulation;
- cleared on repository switch;
- handle invalidated after successful apply;
- stale-plan failure does not auto-replan;
- caller cannot replace the underlying plan;
- not a security secret: correctness comes from backend lookup/session binding/read-set verification, not handle secrecy.

`MutationPlanView` should contain everything needed to make an informed user decision without exposing a writable executable plan object.

## Creation-flow semantic correction

The old proposal includes an "Evidence plan" step. The current canonical work-item schema has no standalone `evidence_plan` field.

Therefore:

- keep the wizard step as planning assistance if useful;
- map durable output only to existing acceptance-criterion/evidence semantics;
- do not add a schema field during WI055;
- do not persist temporary wizard reasoning unless a governed existing record supports it.

This is a presentation feature, not a schema migration.

## Lifecycle UX correction

The formal model permits any lifecycle state to move to any other state, subject to post-state validity.

The UI may warn about unusual transitions, but it must not invent a stricter state machine unless a future governance decision changes RepoPact semantics.

Completion failure is surfaced through the core plan/post-validation diagnostics.

Reopening completed work is valid and must preserve the full work directory/history/evidence.

## Watcher architecture

Implement Rust-native watching below Tauri.

A suitable internal flow:

```text
OS watcher events
    |
    v
normalize/coalesce/debounce
    |
    v
classify relevant paths
    |
    v
refresh RepositorySession/snapshot
    |
    +-- validate
    +-- derive view generation
    +-- compare snapshot/view token
    |
    v
RepositoryChangedEvent
    |
    v
Tauri emit-to main window
```

Do not use the JavaScript fs watcher.

A Rust `notify`-family dependency is appropriate if needed. Keep watcher dependencies below the webview.

### Full rebuild vs incremental

Correctness first.

A full session/snapshot rebuild after a debounced relevant event is acceptable for WI055 if measured performance is practical. Preserve path classification and a future incremental boundary, but do not introduce fragile partial index mutation just to satisfy the word "incremental."

Record observed refresh latency in evidence.

### Self-originated mutations

After successful apply:

1. derive/return the authoritative new repository generation immediately;
2. watcher events from those writes may arrive afterward;
3. compare the resulting snapshot/view token;
4. suppress duplicate state emission if the repository already converged to that generation.

Avoid time-window-only suppression as the sole correctness mechanism.

### External changes

External editor/Git operations that alter governed state should result in refresh/validation. If the repo becomes invalid, the UI should show the diagnostics; it must not silently repair or rewrite external changes.

## Tauri command and capability security

Current Tauri 2 behavior requires care around custom command ACL configuration.

WI055 should:

- explicitly enumerate application commands in `AppManifest`/permissions;
- create one capability for the local main window containing only required commands/core event listen/unlisten/window basics;
- avoid `core:default` if it grants unrelated APIs not needed by the app;
- avoid frontend event emit/emit-to unless proven necessary;
- avoid fs/shell/process capabilities;
- keep `withGlobalTauri: false`;
- set local-only frontend assets and CSP;
- use Rust-side dialog functionality;
- avoid remote capabilities/origins.

The exact minimal core window/event permissions can be refined while testing because Tauri requires some framework plumbing. Evidence should list the final capability identifiers rather than merely claiming they are narrow.

## CSP and remote content

The workbench has no reason to load remote application code.

Use local Vite assets. Do not disable CSP globally to make development easier. External links, if added later, should not turn the workbench window into a remote browsing surface.

## UI information architecture

### Shell

Recommended desktop shell:

```text
+---------------------------------------------------------------+
| RepoPact | repo name/path | health | refresh/session status   |
+----------------+----------------------------------------------+
| Dashboard      |                                              |
| Work           |                main content                  |
| Decisions      |                                              |
| Evidence       |                                              |
| Graph          |                                              |
| Validation     |                                              |
| Analysis       |                                              |
| Settings       |                                              |
+----------------+----------------------------------------------+
```

Use a single main window for WI055 unless a second window has a clear product need. Fewer webviews simplify capability reasoning.

### Dashboard

Show:

- validity/diagnostic summary;
- work counts;
- active/blocked work;
- evidence/decision counts;
- repository identity/worktree indication;
- live/stale/refresh state.

Do not duplicate canonical dashboard rendering by parsing `dashboard.md` for every data point when the core can provide structured values.

### Work list

Provide search/filter/sort. Prefer client-side presentation filtering over repeated filesystem operations.

### Work detail

Tabs/sections:

```text
Overview
Criteria
Dependencies
Scope / analysis
Evidence
Relationships
Validation
Raw
```

"History" may show durable context available from current records/relationships. Do not promise a full Git history engine unless one is deliberately implemented. A future read-only Git-history view can be added separately.

### Creation/editing

Use a staged form with validation feedback from the core analysis/planner.

Do not attempt to reproduce all core rules synchronously in TypeScript for instant feedback. Lightweight form checks such as required text may exist, but governance diagnostics come from Rust.

### Graph

A visual graph library may handle layout/interactions, but graph nodes/edges come directly from the core DTO.

Also provide a keyboard-accessible relationship list/table so the feature is not dependent on spatial visualization.

## Application preferences

Settings may include non-governed UI preferences such as:

- theme;
- density;
- reduced motion;
- default work filter.

These are application preferences and may use normal app-local/browser persistence. They are not canonical RepoPact records and must not be confused with governance settings.

Do not add provider configuration in WI055 unless needed for a purely optional placeholder; no provider SDK is required.

## Frontend architecture

A practical feature organization:

```text
src/
  app/
    App.tsx
    routing/navigation
    repository context
  lib/
    api.ts
    events.ts
    generated/types.ts
  features/
    dashboard/
    work/
    decisions/
    evidence/
    graph/
    validation/
    analysis/
    settings/
  components/
    primitives/
    layout/
    diagnostics/
    mutation-plan/
```

Avoid a giant global store unless the UI genuinely requires it. The authoritative repository generation is backend state; React state should cache/render DTOs and invalidate them on typed native events.

## Frontend dependencies

Keep the dependency set small.

Reasonable dependencies may include:

- React/React DOM;
- Tauri JS API core/event bindings;
- a focused icon package;
- testing libraries;
- optionally a graph layout/rendering library.

Do not introduce a general filesystem SDK, stateful backend framework, or provider SDK.

A CSS utility/framework is optional. The architectural requirement is accessible, maintainable presentation, not a specific styling technology.

## Accessibility

Required behavior:

- correct heading/nav landmarks;
- keyboard-operable navigation and dialogs;
- visible focus;
- labels/descriptions for forms;
- errors connected to fields;
- status text/icons in addition to color;
- accessible graph alternative;
- reduced-motion support;
- sensible contrast in both themes;
- focus restoration after modal/plan apply flows.

Automated accessibility checks should supplement, not replace, keyboard smoke testing.

## Testing architecture

### Reusable desktop API tests

Test without launching Tauri where possible:

- DTO derivation from a real fixture repo;
- command/service result mapping;
- session generation and repo replacement;
- stale session rejection;
- plan-handle ownership and invalidation;
- mutated frontend view cannot alter backend `MutationPlan`;
- indexed raw-record access path restriction;
- watcher classification/coalescing;
- duplicate post-apply refresh suppression.

### Tauri bridge tests

Keep command functions thin enough that most logic is testable below Tauri macros.

Test permission/capability configuration statically:

- expected command allowlist;
- no fs/shell permission;
- no remote capability;
- `withGlobalTauri` false;
- local frontend configuration.

### Frontend tests

Use TypeScript compiler plus a focused component test stack.

High-value tests:

- repo-open empty/error/success states;
- work filters/detail rendering;
- create/edit form progression;
- plan blocking diagnostics;
- stale-plan UI;
- apply success/failure;
- watcher refresh banner/state;
- keyboard navigation;
- graph accessible alternative.

### End-to-end smoke

On the available Windows host, exercise the built desktop against a scratch adopted RepoPact repository. Do not mutate the development repository as an uncontrolled UI fixture.

## Packaging boundary

WI055 is a development/product foundation, not a full release-signing program.

Expected:

- frontend production build;
- Tauri dev launch on available Windows host;
- local Windows bundle/build if toolchain supports it;
- exact artifact/bundle status documented;
- Linux/macOS constraints documented honestly;
- no claim of signing/notarization unless executed.

Do not modify frozen CI/release workflows without separate operator approval.

## Expected implementation sequence

1. Establish Python + Rust baseline from WI054 closeout.
2. Add `repopact-desktop-api` and DTO/type-generation boundary.
3. Add desktop session manager and plan registry with tests.
4. Add watcher abstraction and tests.
5. Scaffold Tauri 2 + Vite + React + TypeScript app.
6. Configure manifest/custom command permissions/capability before adding many commands.
7. Implement repository selection and typed shell/overview.
8. Implement work list/detail/validation views.
9. Implement decisions/evidence views.
10. Implement analysis/graph views.
11. Implement create plan/review/apply.
12. Implement edit and transition plan/review/apply.
13. Wire native watcher events and repository switching.
14. Complete accessibility/responsive/theme work.
15. Run frontend/Rust/Tauri/security tests.
16. Run local desktop mutation/live-refresh smoke on a scratch repo.
17. Produce available local bundle and record cross-platform status.
18. Record closeout evidence and only then complete WI055.

## Stop/escalation conditions

Codex must stop and report before proceeding if implementation appears to require:

- canonical schema changes;
- changing RepoPact lifecycle semantics;
- changing Decision 0040 stale-plan semantics;
- exposing arbitrary frontend filesystem/shell writes;
- executing a frontend-round-tripped raw `MutationPlan`;
- adding decision/evidence mutation;
- porting WI050 authority;
- remote hosted app content;
- a persistent transaction/plan database;
- frozen CI/release changes;
- production signing/notarization credentials.

## Closeout expectation

WI055 is complete when the desktop proves it is a **secure client of RepoPact**, not when a window merely renders.

A valid closeout must demonstrate:

- actual repository open/switch;
- browse/read surfaces;
- graph/analysis/validation;
- create/edit/transition plan + preview + apply;
- stale-plan and stale-session behavior;
- live external refresh;
- constrained command/capability surface;
- generated/checked frontend DTO contract;
- accessibility and production frontend build;
- host desktop smoke/bundle status;
- preservation of all WI053/WI054 conformance guarantees.
