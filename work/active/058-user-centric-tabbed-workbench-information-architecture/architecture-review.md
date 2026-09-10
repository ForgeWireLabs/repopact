# WI058 Sol Architecture Review — User-Centric Tabbed Workbench IA

Date: 2026-09-10
Reviewer: Sol High
Implementation agent: Codex

## Finding

The current UI has a consistent structural issue: collection-heavy primary pages render one unbounded vertical collection. `WorkPage` maps the full filtered work-item set into one `record-list`; the generic Decisions/Evidence page does the same; Graph renders one edge table; Validation one diagnostic list; Analysis one finding list.

The correct fix is not eight unrelated filters. Introduce one reusable secondary-navigation pattern and page-specific grouping semantics.

This architecture is **cross-platform from the start**. RepoPact Workbench targets Windows, Linux, macOS, Android, and iOS through Tauri 2. The React information architecture must therefore be independent of a permanent desktop sidebar, mouse hover, right-click behavior, or a two-column detail pane.

## Architectural constraints

1. **Presentation only unless typed summary metadata is required.** Do not change canonical repository/lifecycle semantics.
2. **WI057 remains binding.** Tab changes, paging, sorting, search, breakpoint changes, and compact-detail navigation must operate over the current cached native generation and add zero Git subprocesses/new snapshots.
3. **No N+1 detail reads.** Decisions/Evidence may need richer summary DTOs; derive them once from the cached snapshot/index rather than calling `get_record` for every row.
4. **Do not over-generalize DTOs.** Prefer explicit `DecisionSummaryView` / `EvidenceSummaryView` or another typed equivalent over an unstructured metadata bag if backend changes are needed.
5. **Preserve detail panes and mutation flows.** Secondary navigation should reduce scanning, not replace guarded Rust-owned planning/apply behavior.
6. **Pagination is local.** No remote/server paging or new process boundary for WI058.
7. **One semantic IA, multiple compositions.** Wide and compact layouts may compose the same pages differently, but tab meanings, counts, selected records, lifecycle labels, and actions remain consistent.
8. **No desktop-only interaction assumptions.** Essential functions must work with keyboard, pointer, touch, and native/system Back behavior where applicable.
9. **No permission expansion for responsiveness.** Do not add generic Tauri filesystem/shell/process capabilities merely to support mobile layouts.

## Adaptive application shell

### Wide mode

Use for normal desktop windows and sufficiently wide tablet landscape layouts.

- Persistent left-side primary navigation is appropriate.
- List/detail split layouts are appropriate.
- Page heading and secondary tabs should remain visible without scrolling through records.
- Tables can remain tables when the viewport supports them.

### Compact mode

Use for narrow desktop windows, tablet portrait, Android phones, and iPhones.

- Collapse the persistent primary sidebar into a navigation drawer/sheet triggered from the app bar or an equivalently clear compact primary-navigation control.
- Do **not** put all eight primary sections into a cramped bottom tab bar.
- Page names and hierarchy remain the same as wide mode.
- List/detail sections become drill-in navigation. Selecting Work/Decision/Evidence opens a focused detail surface; Back restores the prior page, selected secondary tab, search query, and pager state.
- On Android, system Back should follow the app navigation stack before attempting to leave the application where Tauri/platform integration permits.
- On iOS, an explicit Back affordance and native-feeling back-stack behavior are required; do not depend solely on Android-style hardware/system Back.
- Secondary tabs with 3–4 choices should wrap/compact into touch-friendly rows rather than force page-wide horizontal scrolling.
- Long tables such as Graph may become accessible stacked relationship cards in compact mode while preserving the same data and grouping.
- Dialogs that are comfortable on desktop may become sheets/full-screen panels on mobile if needed for usable forms and plan review.

Breakpoint selection should be driven primarily by available layout width/capability, not user-agent string branching.

## Shared UI primitive

Create a reusable `SectionTabs`/equivalent component with:

- tab id/label/count;
- controlled selected id;
- semantic tablist/tab/panel wiring;
- keyboard Left/Right/Home/End behavior;
- pointer and touch activation;
- visible focus treatment;
- adaptive wrapping/compaction;
- no page-wide horizontal scroll;
- sufficiently large touch targets;
- no hover-only essential state.

Create a reusable local collection-pager helper/component with shared wide and compact defaults. Recommended starting point: about 10–12 rows wide and 6–8 compact, subject to real viewport testing.

Tab/pager state belongs in the frontend presentation layer. Remember each primary page's selected sub-tab for the current repository session. Reset safely on repository switch; preserve/clamp on refresh when possible.

The compact detail navigation stack must preserve list context in memory rather than rerunning native reads merely because the user opened and closed a detail view.

## Page taxonomy

### Work

Required top tabs, in this exact order:

`Proposed | Active | Deferred | Complete`

Mapping:

- Proposed -> proposed
- Active -> blocked group first + active group second
- Deferred -> deferred
- Complete -> completed

The Active tab badge represents active + blocked. Within it, show a compact blocked count and make blocked rows visually distinguishable without using alarmist styling.

Search is scoped to the selected tab. Paging happens after grouping/filtering. When an already-selected work item no longer belongs to the current sub-tab after transition/refresh, select the appropriate destination tab or clear selection deterministically.

Wide: list + detail pane.
Compact: list -> detail drill-in, with Back preserving tab/search/page state.

### Dashboard

`Overview | Attention | Session`

Overview stays compact. Attention surfaces validation errors/warnings and blocked work as navigation affordances, not duplicated full lists. Session contains identity/generation/watcher/native boundary information.

Compact Dashboard should prioritize actionable content over decorative vertical whitespace.

### Decisions

`Current | Proposed | Deferred | History`

Map accepted -> Current; proposed -> Proposed; deferred -> Deferred; rejected/superseded/deprecated -> History.

The current `RecordSummaryView` lacks decision status/title/date. Do not fetch every detail. Add a snapshot-derived typed summary if necessary. Reuse an existing canonical/frontmatter parser if one exists; do not create a contradictory decision-status parser solely in React.

Wide uses list/detail; compact uses drill-in detail.

### Evidence

`Recent | Passed | Attention | All`

Recent should default to newest-first and remain bounded. Passed -> result passed. Attention -> failed/partial/blocked. All -> paged full corpus.

The current summary lacks timestamp/result/work-item. Add typed snapshot-derived summary metadata rather than N detail calls.

Wide uses list/detail; compact uses drill-in detail.

### Graph

`Dependencies | Evidence | Governance | All`

Recommended mapping:

- Dependencies: `depends_on`, `reverse_dependency`
- Evidence: `supported_by`, `supports_work_item`
- Governance: remaining governance/ownership/scope/constraint/supersession edges
- All: all edges

Wide mode retains the accessible table. Compact mode may render the same edge rows as stacked cards/definition rows so relationship data remains readable without horizontal page scroll. Pagination applies after grouping.

### Validation

`Errors | Warnings | Info | All`

If diagnostics exist, choose the highest-severity non-empty tab on first entry/new generation unless the user has explicitly selected another tab for the current generation. If no diagnostics exist, keep the compact all-clear card.

### Analysis

`Constraints | Suggestions | Facts`

Mapping is exact from `FindingClassification`: constraint, suggestion, fact. Preserve remediation text and related/basis information already exposed.

### Settings

`Appearance | Session | Boundaries`

Move or duplicate the theme selector into Appearance. Session describes repository identity/generation/watcher. Boundaries explains local/native authority and plan/session behavior.

A wide top-bar theme shortcut may remain. Compact/mobile header space should be conserved, so presentation controls may live only in Settings there.

## Ordering

Use deterministic user-facing ordering rather than filesystem accident:

- Work: blocked first in Active; otherwise prefer most recently updated first if `updated` is available without extra reads, with stable ID fallback.
- Decisions: newest decision/date first where typed metadata exists.
- Evidence: timestamp descending.
- Validation/Analysis: preserve stable canonical order within classification unless another order is explicitly justified.
- Graph: stable relationship/source order.

## Mobile interaction details

- Essential controls must not depend on hover.
- Secondary tabs and pager buttons must remain operable by touch without precision targeting.
- Respect device safe-area insets so app bars, bottom actions, dialogs/sheets, and pagination controls are not obscured by notches/home indicators/system UI.
- Forms and mutation-plan review must remain usable when a virtual keyboard is open.
- Portrait and landscape orientation changes must preserve current repository/session/page state.
- Do not create multiple nested scrolling regions just to fit desktop composition onto phones.
- A long intrinsically textual record/detail may scroll; navigation chrome and unrelated record lists should not have to be traversed first.

## Testing requirements

Frontend tests must prove:

- exact Work tab labels/order and status mapping;
- blocked appears only under Active and retains `blocked` label;
- pagination boundaries and range text;
- sub-tab counts;
- search + pagination interaction;
- keyboard tab navigation and ARIA semantics;
- touch/click activation paths do not rely on hover;
- per-page tab-state memory;
- compact list -> detail -> Back restores list context;
- responsive shell switches away from persistent sidebar/list-detail composition at compact width;
- no extra desktop/native API calls when switching sub-tabs/pages or opening/closing already-loaded compact presentation state;
- Decisions/Evidence classification without per-record detail calls;
- Validation and Analysis mappings;
- Graph has a no-horizontal-page-scroll compact representation;
- repository switch resets session-scoped presentation state.

Rust/Desktop API tests, if DTOs change, must prove summary metadata comes from the supplied cached snapshot and does not introduce fresh snapshot/Git work.

Re-run WI057 process-count/cache regressions and the WI055 frontend/security/type/build gates.

## Cross-platform verification matrix

### Required on available Windows host

Perform full native Workbench acceptance including wide and compact-width resizing, navigation, tabs, paging, detail drill-in behavior where compact mode can be exercised, mutation-plan opening, repository switching, and no process-storm regression.

### Linux/macOS

Where the corresponding Tauri toolchains/hosts are available, build and run the same frontend/native shell and record results. If unavailable, record deterministic build commands and status as **unexecuted**, not passed.

### Android/iOS

Where Android SDK/emulator/device and Apple/Xcode simulator/device infrastructure are available, build/run and verify touch, compact navigation, Back behavior, orientation, safe areas, virtual keyboard/form usability, and repository-selection/capability behavior exposed by the existing platform architecture.

If those toolchains/devices are unavailable in the current environment, frontend responsive/touch semantics and compile/configuration integrity may be proven here, but runtime validation must be reported as unexecuted rather than silently omitted or claimed.

A platform-specific native capability blocker discovered during WI058 should be recorded against the appropriate owning work item instead of weakening the shared UI architecture.

## Native acceptance

On the available Windows host verify:

- Work opens on a useful default tab (Active is preferred when non-empty);
- Proposed/Active/Deferred/Complete are immediately understandable;
- Complete does not produce a giant page;
- all primary pages use their secondary organization coherently;
- detail selection and guarded create/edit/transition still work;
- wide and compact widths both work;
- primary sidebar collapses appropriately in compact mode;
- compact Work/Decision/Evidence detail navigation has a usable Back path;
- Graph does not create page-wide horizontal scrolling in compact mode;
- no terminal flash/process storm regression;
- normal use requires materially less scrolling.

## Sequencing recommendation

WI058 is small and largely isolated from WI056. If one Codex session is acting serially, complete WI058 first, close it, then resume WI056 from the resulting `main`. If two independent worktrees/sessions are used, they may proceed concurrently, but Codex must merge normally and reconcile any generated DTO/type conflicts rather than overwrite either stream.
