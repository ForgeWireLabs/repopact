# Codex Directive — WI058 User-Centric Tabbed Workbench Information Architecture

Implementation agent: **Codex**

## Start state

Fetch `origin/main`, verify a clean worktree, and begin from the latest commit containing the WI058 cross-platform amendment. Preserve concurrent WI056 work if another session has advanced `main`; merge normally and do not reset/force-push shared history.

Read first:

- root and relevant nested `AGENTS.md` files;
- `work/active/058-user-centric-tabbed-workbench-information-architecture/README.md`;
- `work/active/058-user-centric-tabbed-workbench-information-architecture/architecture-review.md`;
- completed WI055 and WI057 closeout/evidence;
- Decision 0041;
- active WI056 only to avoid conflicting authority/cutover assumptions;
- current `rust/apps/repopact-desktop/src/App.tsx`, `styles.css`, generated types, frontend tests, desktop API DTOs, Tauri configuration, and type-generation mechanism.

## Scope

Implement the secondary-tab/low-scroll Workbench IA as a **shared cross-platform Tauri 2 application UI** targeting Windows, Linux, macOS, Android, and iOS.

Do not treat this as Windows-desktop-only styling.

Do not implement WI056 canonical-engine/Python cutover work in this session.

Do not alter governance lifecycle/status semantics, decision status semantics, evidence result semantics, WI050 authority, frozen workflows, or generic filesystem/process permissions.

## Phase 1 — baseline

Before edits run the focused application baseline:

- generated type check;
- TypeScript typecheck;
- frontend tests;
- production frontend build;
- relevant `repopact-desktop-api` tests;
- WI057 process/cache focused tests;
- Tauri configuration/build checks available on the current host;
- governance validation/frozen check as appropriate.

Record exact counts and available platform toolchains.

## Phase 2 — adaptive application shell

Refactor the primary navigation composition so the same information architecture works at wide and compact widths.

Wide mode:

- persistent left primary navigation is acceptable;
- existing list/detail split panes may remain where useful.

Compact mode for narrow desktop, tablet portrait, Android, and iOS:

- collapse the persistent primary sidebar into an app-bar navigation drawer/sheet or equivalent compact primary navigation;
- do not squeeze all eight top-level sections into a bottom tab bar;
- preserve the same page names and hierarchy;
- avoid page-wide horizontal scrolling;
- do not rely on hover/right-click/pointer precision for essential actions.

Choose breakpoints by available layout width/capability rather than user-agent branching where practical.

## Phase 3 — presentation primitives

Add a reusable accessible secondary-tab component rather than page-specific ad hoc strips.

Requirements:

- count badge support;
- controlled active id;
- `role=tablist`, `role=tab`, `aria-selected`, panel association;
- hardware-keyboard Left/Right/Home/End;
- pointer and touch activation;
- visible focus state;
- sufficiently large touch targets;
- adaptive wrapping/compaction with no page-wide horizontal overflow;
- no hover-only essential state;
- no data fetch/process behavior in the component.

Add a reusable local pager with shared wide/compact page-size profiles. Start around 10–12 rows wide and 6–8 compact unless viewport evidence justifies different deterministic constants.

Pager requirements:

- Previous/Next;
- disabled first/last boundaries;
- `start–end of total` text;
- deterministic clamping when filters/data/layout change;
- page reset on category/search changes where least surprising;
- touch-friendly mobile controls;
- no backend/native call on page change.

## Phase 4 — Work first

Implement Work exactly as requested:

`Proposed | Active | Deferred | Complete`

Do not add a fifth top-level Blocked tab.

Inside Active:

1. blocked records first under a visible `Blocked` group/heading;
2. active records second under an `In progress`/equivalent group;
3. each blocked record still displays `blocked` as its actual state.

Counts:

- Proposed = proposed count;
- Active = active + blocked count;
- Deferred = deferred count;
- Complete = completed count.

Default to Active if non-empty, otherwise first non-empty lifecycle tab.

Search is scoped to the selected tab.

Wide mode may keep list + detail pane.

Compact/mobile mode must use list -> focused detail drill-in. Back must restore the same Work tab, query, pager state, and meaningful scroll/list context. Support Android system Back where platform integration permits, and provide an explicit Back affordance suitable for iOS and compact desktop/tablet use.

Create/edit/transition remain behind existing Rust-owned plans. Mobile/compact mutation forms and plan review must remain usable with a virtual keyboard; use a sheet/full-screen presentation instead of squeezing a desktop modal if needed.

Use local pagination so Complete cannot become a long scroll wall.

After transition/apply/refresh, if the selected item moves lifecycle category, reconcile sub-tab/selection deterministically.

## Phase 5 — typed summaries for Decisions/Evidence

The current generic `RecordSummaryView` is insufficient for useful category tabs.

Do not solve this by fetching every record detail.

Prefer explicit Rust DTOs such as:

- `DecisionSummaryView`: reference/readable/title/status/date/supersedes as available;
- `EvidenceSummaryView`: reference/readable/timestamp/work_item/result/provenance as available.

Names may differ, but keep them typed and generated into TypeScript.

Populate from the **existing cached `RepositorySnapshot`/index**. Reuse existing canonical parsers/helpers. Do not introduce frontend parsing that can disagree with validation semantics.

Prove summary generation adds no fresh snapshot and no additional Git calls for an unchanged generation.

Wide mode may use list/detail. Compact/mobile uses list -> detail drill-in with Back preserving tab/pager state.

## Phase 6 — page-specific secondary tabs

### Dashboard

`Overview | Attention | Session`

Keep each concise. Attention links to Validation/Work rather than duplicating full datasets. Compact layouts prioritize actionable information and avoid ornamental vertical whitespace.

### Decisions

`Current | Proposed | Deferred | History`

- Current = accepted
- Proposed = proposed
- Deferred = deferred
- History = rejected + superseded + deprecated

Show counts and newest-first ordering where date is available.

### Evidence

`Recent | Passed | Attention | All`

- Recent = newest-first bounded recent view;
- Passed = `passed`;
- Attention = `failed`, `partial`, `blocked`;
- All = full paged index.

Show result, timestamp/date, and associated work item compactly when available.

### Graph

`Dependencies | Evidence | Governance | All`

- Dependencies = `depends_on`, `reverse_dependency`;
- Evidence = `supported_by`, `supports_work_item`;
- Governance = all remaining governance/ownership/scope/constraint/supersession relationship kinds;
- All = all edges.

Wide mode retains the accessible table. Compact/mobile mode must provide an accessible stacked relationship representation or equivalent that does not force horizontal page scrolling.

### Validation

`Errors | Warnings | Info | All`

Default to highest-severity non-empty view on a new generation. If no diagnostics exist, keep the compact all-clear card.

### Analysis

`Constraints | Suggestions | Facts`

Map exactly from current `FindingClassification` values. Keep remediation and explanatory content.

### Settings

`Appearance | Session | Boundaries`

Put theme/presentation controls under Appearance. Keep session identity/generation/watcher under Session. Keep authority/security explanations under Boundaries. A wide top-bar theme shortcut may remain; compact/mobile headers need not spend permanent space on it.

## Phase 7 — state and native back behavior

Remember selected secondary tab for each primary page during the current repository session.

On repository switch, reset to useful defaults rather than leaking old-repo UI state.

On ordinary snapshot refresh, preserve selected sub-tabs when still valid.

Compact drill-in detail must be presentation state over the same loaded generation. Opening/closing a detail surface must not trigger a fresh repository snapshot merely because the composition changed.

Support native/system Back semantics where available:

- Android Back first unwinds compact detail/drawer state before leaving the app where supported;
- iOS and other compact platforms have an explicit visible Back affordance with the same logical stack;
- browser-like accidental history/navigation is not required if Tauri provides a better local state stack.

Do not introduce persistent cross-repository UI state unless explicitly justified. In-memory session state is sufficient.

## Phase 8 — mobile ergonomics

Audit compact/mobile behavior for:

- touch-first operation without hover;
- safe-area insets;
- portrait and landscape orientation;
- virtual keyboard obscuring create/edit/search/plan controls;
- app bar/drawer usability;
- tab wrapping/compaction;
- pagination reachability;
- long record/detail scrolling without unrelated list scrolling;
- dialog/sheet/full-screen adaptation;
- no nested tiny scroll boxes.

Do not add generic Tauri fs/shell/process authority to solve mobile layout problems.

Platform-specific repository-selection/filesystem/background-lifecycle limitations are not silently redesigned under WI058. Record a concrete blocker against the owning architecture/work item if one appears.

## Phase 9 — tests

Add frontend tests for at least:

1. exact Work tab order/labels;
2. Work lifecycle mapping;
3. blocked nested under Active and still labelled blocked;
4. count badges;
5. Complete pagination;
6. Previous/Next boundaries and range text;
7. search within selected Work tab;
8. tab/page/breakpoint presentation changes cause no new native API invocation;
9. keyboard/ARIA tab behavior;
10. touch/click activation does not rely on hover;
11. Decisions mapping;
12. Evidence mapping and newest-first Recent;
13. Graph grouping and compact no-horizontal-scroll representation;
14. Validation severity grouping/default;
15. Analysis classification grouping;
16. per-primary-page tab memory;
17. repository-switch reset behavior;
18. compact list -> detail -> Back restores list state;
19. wide/compact primary-navigation composition;
20. orientation/layout state preservation where testable in the frontend.

If Rust DTOs change, add Desktop API tests proving metadata is projected from the cached snapshot and does not trigger N+1 reads/fresh snapshots.

Re-run WI057 Git-count/cache tests.

## Phase 10 — platform verification matrix

### Windows host — required runtime acceptance here

Build and run the Workbench. Verify both wide and compact-width modes:

- select RepoPact;
- Work shows `Proposed | Active | Deferred | Complete` immediately;
- Active visibly surfaces blocked + in-progress groups;
- switch through all four Work tabs;
- Complete is paged and compact;
- exercise search and paging;
- visit every primary section and confirm secondary tabs;
- inspect Work and Decision/Evidence detail in wide mode;
- resize to compact width and verify primary nav collapse and list -> detail -> Back flows;
- inspect Graph compact representation;
- open create/edit/transition/plan UI without applying unnecessary repository mutations;
- confirm no terminal flashes/process storm and normal responsiveness.

If GUI automation is unavailable, use the operator-assisted/manual evidence standard accepted in WI057.

### Linux and macOS

If corresponding hosts/toolchains are available, build/run and exercise the same shared IA. If unavailable, record deterministic build instructions and status as **unexecuted**, not passed.

### Android and iOS

If Android SDK/emulator/device and Apple/Xcode simulator/device infrastructure are available, build/run and verify:

- touch-first primary navigation;
- secondary tabs and pagination;
- compact list/detail drill-in;
- Android system Back behavior;
- iOS explicit/native-feeling Back behavior;
- portrait/landscape rotation;
- safe-area handling;
- virtual-keyboard usability;
- platform-native repository-selection/capability behavior already exposed by the existing architecture.

If unavailable, prove responsive/touch semantics and configuration/build integrity to the extent possible and record runtime status as **unexecuted**. Never claim unexecuted platforms as validated.

## Phase 11 — full closeout

Run:

- frontend generated types/typecheck/tests/build;
- Rust desktop API/workspace relevant checks;
- Python regression suite if repository policy requires full closeout;
- 20/20 conformance;
- WI050 8/8;
- linked-worktree/reference parity;
- WI057 process/cache regression;
- RepoPact validation/dashboard regeneration;
- frozen-surface check;
- Tauri cross-platform build/configuration checks available on the host.

Record exact platform validation status separately for Windows, Linux, macOS, Android, and iOS.

Complete WI058 only after the shared UI implementation is genuinely adaptive and the available-host native UI behavior is verified.

## No-go conditions

Stop for architecture review rather than improvising if implementation would require:

- a lifecycle/status schema change;
- a new persistent UI-state database;
- per-record detail fetching for list classification;
- server/native pagination merely for UI convenience;
- new generic Tauri fs/shell/process permissions;
- bypassing cached snapshot APIs;
- a desktop-only implementation that cannot express the same IA on mobile;
- a mobile-only fork of the semantic page model;
- changing WI056 engine architecture;
- changing WI050 security semantics;
- changing frozen workflows.

## Completion report

Return:

- final `main` SHA and commits;
- exact secondary tabs for each primary page;
- Work blocked handling;
- wide/compact primary-navigation behavior;
- wide vs compact list/detail behavior;
- pager sizes/behavior;
- DTO/type changes;
- proof Decisions/Evidence do not N+1 fetch;
- keyboard/pointer/touch/accessibility behavior;
- API/process-count proof for tab/page/layout changes;
- Windows native UX result;
- Linux/macOS/Android/iOS build/runtime status individually;
- all test counts;
- frozen-surface status;
- residual platform/UX issues;
- whether WI056 can continue without reconciliation.
