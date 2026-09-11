# Work Item 058 — User-Centric Tabbed Workbench Information Architecture

**Status:** Completed

**Owner:** tooling

**Affected scopes:** ui, desktop, mobile, tooling, work, docs, evidence

**Depends on:** WI055, WI057

## Purpose

Reduce vertical scanning and long scroll walls in RepoPact Workbench by introducing a consistent secondary-tab information architecture inside every primary application section.

RepoPact Workbench is a cross-platform Tauri 2 application targeting **Windows, Linux, macOS, Android, and iOS**. WI058 is therefore an adaptive cross-platform information-architecture milestone, not a Windows-desktop polish pass.

This is a user-interface organization change, not a governance-semantic or authority change.

The motivating operator preference is explicit: Work should not present one long lifecycle-mixed list. The primary Work tabs are:

1. **Proposed**
2. **Active**
3. **Deferred**
4. **Complete**

The same low-scroll pattern should extend across the other primary application pages using categories that make sense to a user of that page rather than mechanically repeating Work lifecycle labels.

## Core UX rule

A primary page answers one user question at a time.

Secondary tabs divide the page into a small number of meaningful views. Large collections use bounded paging rather than requiring the user to scroll through the entire corpus.

Tabs are presentation projections of the current immutable native snapshot/session state. Switching a secondary tab or page must not cause a repository recrawl, Git subprocess, watcher refresh, or independent native snapshot.

The same semantic page/tab model must survive all supported form factors. A wide desktop window may use a persistent sidebar and side-by-side list/detail panes. A narrow desktop window, tablet, or phone must adapt navigation and detail presentation rather than squeeze the desktop composition into a smaller viewport.

## Cross-platform adaptive shell

The information architecture is shared across Windows, Linux, macOS, Android, and iOS.

### Wide desktop/tablet landscape

- persistent or comfortably visible primary section navigation is acceptable;
- list/detail split panes may be used where width allows;
- secondary tabs remain close to the page heading;
- collection paging keeps the main viewport compact.

### Narrow desktop/tablet portrait/phone

- the persistent sidebar may collapse into an app-bar navigation drawer/sheet or equivalent compact primary navigation;
- do not force eight primary sections into an unusable bottom tab bar;
- list/detail layouts become drill-in navigation: list first, selected detail as the focused view, with an obvious Back action and native/system back behavior where available;
- secondary tabs wrap or compact into a touch-friendly multi-row control rather than causing page-wide horizontal scrolling;
- primary actions remain reachable without hover;
- touch targets, focus states, safe-area insets, virtual-keyboard behavior, and portrait/landscape rotation must be considered;
- do not rely on right-click, hover-only disclosure, desktop keyboard shortcuts, or fixed pointer precision for essential behavior.

The navigation model may adapt by breakpoint/platform, but page names, tab meanings, counts, selected state, and governance semantics must remain consistent.

## Work

Use exactly these primary tabs:

- **Proposed** — `status == proposed`.
- **Active** — operationally current work. Show `blocked` items in a clearly labelled **Blocked** group at the top, followed by ordinary `active` items. Do not relabel blocked records or hide their real lifecycle state.
- **Deferred** — `status == deferred`.
- **Complete** — `status == completed`.

Each tab shows a count. Search applies to the selected tab by default.

On wide layouts, selected record detail may remain in the right-hand detail pane. On narrow/mobile layouts, selecting a record opens the detail as the focused page/panel and Back returns to the same Work tab, query, and page position.

A completed corpus can be large, so the list must use bounded local pagination with a compact range indicator and Previous/Next controls instead of rendering every row into one vertical page. The page size may adapt between wide and compact layouts, but must remain deterministic for a given layout and never trigger backend paging or new native reads.

## Dashboard

Use secondary views oriented around what the operator needs to know:

- **Overview** — key counts and navigation cards.
- **Attention** — validation problems and currently blocked work at a glance, with links to the relevant section.
- **Session** — repository identity, generation, watcher state, and native-session information.

Do not turn the dashboard into another giant all-data page.

On mobile, cards stack in a compact order without requiring long ornamental spacing before actionable information.

## Decisions

Use the decision lifecycle in user-facing form:

- **Current** — accepted decisions.
- **Proposed** — proposed decisions.
- **Deferred** — deferred decisions.
- **History** — rejected, superseded, and deprecated decisions.

Counts must come from one snapshot-backed summary projection. Do not fetch every decision detail merely to classify it.

Decision details use side-by-side detail on wide layouts and drill-in detail on compact/mobile layouts.

## Evidence

Evidence is task-oriented rather than lifecycle-oriented:

- **Recent** — newest evidence first, bounded to a practical recent window/page.
- **Passed** — successful runs.
- **Attention** — failed, partial, or blocked runs.
- **All** — complete evidence index with local pagination.

Summary rows should surface useful metadata such as result, timestamp/date, and associated work item when available.

Evidence detail follows the same wide split / compact drill-in pattern as Decisions.

## Graph

Group relationships by purpose:

- **Dependencies** — work/dependency flow relationships.
- **Evidence** — support/evidence relationships.
- **Governance** — ownership, scope, constraints, applicability, supersession, and related governance edges.
- **All** — full edge set.

Keep the existing accessible table representation on wide layouts. On compact/mobile layouts, the same rows may render as stacked relationship cards or another accessible narrow representation rather than forcing a wide table to create horizontal page scrolling.

This work item does not require a graphical node canvas.

## Validation

Use severity-focused tabs with counts:

- **Errors**
- **Warnings**
- **Info**
- **All**

Default to the highest-severity non-empty category. If there are no diagnostics, present the existing all-clear state without empty tab clutter.

Diagnostic rows/cards must remain readable on a phone without horizontal page scrolling.

## Analysis

Map directly to the existing explainable classifications:

- **Constraints**
- **Suggestions**
- **Facts**

Counts should be visible. Preserve remediation text and provenance/basis data already available to the application surface.

## Settings

Organize settings/information into:

- **Appearance** — theme and presentation preferences.
- **Session** — current repository/session/generation state.
- **Boundaries** — native authority and security boundary information.

It is acceptable to retain a top-bar theme shortcut on wide layouts, but Settings must become the canonical place to understand/change presentation preferences. Compact/mobile layouts should not spend persistent header width on redundant controls that fit better inside Settings.

## Secondary-tab component

Build a reusable accessible secondary-tab primitive rather than implementing eight unrelated button strips.

Requirements:

- selected state is visually obvious;
- counts/badges are supported;
- keyboard navigation supports Left/Right plus Home/End where a hardware keyboard is present;
- pointer and touch activation are first-class;
- `role=tablist`, `role=tab`, `aria-selected`, and associated tab panels are correct;
- tab strips wrap, compact, or otherwise adapt on narrow windows/phones rather than causing page-wide horizontal scrolling;
- essential tabs must not require hover to discover;
- per-page active sub-tab is remembered while navigating primary sections during a repository session;
- pagination resets or clamps safely when filters/search/snapshot data change.

## Bounded collection behavior

Default list/table page size should be small enough that a normal desktop Workbench window does not become a long page, and compact enough that a phone does not require excessive vertical scanning.

Use shared wide/compact defaults unless a page has a demonstrated reason to differ. A reasonable implementation may start around 10–12 rows on wide layouts and 6–8 on compact layouts, but viewport testing should determine the exact constants.

Pagination is client-side over already loaded snapshot projections for this milestone. It must not create a new native read on each page change.

Provide:

- `Previous` / `Next`;
- `1–N of M` range text;
- disabled boundary controls;
- deterministic ordering;
- sensible reset/clamping after repository refresh or tab change;
- touch-friendly controls on mobile;
- preservation of the selected tab/query/page when returning from a compact-layout detail view.

## Snapshot/process boundary

WI057 is binding.

Secondary tabs, search, paging, sorting, grouping, primary-navigation adaptation, and wide/compact layout changes should operate against data already projected from the current cached native generation. If richer summary metadata is needed for Decisions/Evidence, add typed Rust summary DTOs populated from the existing snapshot/index.

Do **not** solve classification by opening every record or performing per-tab/per-page native queries.

Changing secondary tabs, pages, responsive breakpoints, or compact-detail navigation must add zero Git subprocesses and zero new repository snapshots for an unchanged generation.

## Cross-platform runtime boundary

Do not assume that desktop filesystem/process affordances exist on Android/iOS merely because the React view is shared.

WI058 owns presentation and adaptive navigation. It must preserve the native capability/security boundary selected for each platform. Platform-specific repository selection, filesystem sandboxing, lifecycle/background behavior, and packaging remain governed by their owning architecture/work items unless a concrete UI blocker is discovered.

The frontend must not introduce new generic Tauri filesystem, shell, or process permissions to make mobile layouts convenient.

## Explicitly out of scope

- changing RepoPact work lifecycle semantics;
- adding a fifth Work tab for `blocked` against the operator-requested four-tab layout;
- hiding blocked work;
- changing decision status semantics;
- changing evidence result semantics;
- graphical graph visualization;
- virtualized remote/server pagination;
- generic frontend filesystem/process authority;
- platform-specific repository storage/security redesign;
- WI056 canonical-engine/Python cutover implementation;
- WI050 security-authority changes.

## Sequencing

WI058 may execute independently of WI056 because its implementation surface is the shared application view/DTO layer. If the same coding agent is used serially, prefer completing this UI milestone before returning to the larger WI056 cutover.

Any overlap with WI056 must preserve WI057's cached-generation/process guarantees and generated-type source of truth.

## Closeout standard

Close only when:

- the requested Work lifecycle tabs and page-specific secondary navigation are implemented;
- large collections are bounded by local pagination;
- wide and compact/mobile layouts express the same information architecture without page-wide horizontal scrolling;
- compact list/detail flows preserve user context and system/native Back behavior where applicable;
- keyboard, pointer, touch, and accessibility tests pass at the layers available to the implementation;
- tab/page/layout changes do not trigger new Git/snapshot work;
- existing guarded mutation/detail workflows remain intact;
- Windows runtime behavior is verified on the available host;
- Linux/macOS/Android/iOS build/runtime status is reported truthfully according to available toolchains/devices/simulators, with no unexecuted platform falsely claimed as validated.
