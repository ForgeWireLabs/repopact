# Codex Directive — WI058 User-Centric Tabbed Workbench Information Architecture

Implementation agent: **Codex**

## Start state

Fetch `origin/main`, verify a clean worktree, and begin from the commit that activates WI058. Preserve concurrent WI056 work if another session has advanced `main`; merge normally and do not reset/force-push shared history.

Read first:

- root and relevant nested `AGENTS.md` files;
- `work/active/058-user-centric-tabbed-workbench-information-architecture/README.md`;
- `work/active/058-user-centric-tabbed-workbench-information-architecture/architecture-review.md`;
- completed WI055 and WI057 closeout/evidence;
- Decision 0041;
- active WI056 only to avoid conflicting authority/cutover assumptions;
- current `rust/apps/repopact-desktop/src/App.tsx`, `styles.css`, generated types, frontend tests, desktop API DTOs, and type-generation mechanism.

## Scope

Implement the secondary-tab/low-scroll Workbench IA only.

Do not implement WI056 canonical-engine/Python cutover work in this session.

Do not alter governance lifecycle/status semantics, decision status semantics, evidence result semantics, WI050 authority, frozen workflows, or generic filesystem/process permissions.

## Phase 1 — baseline

Before edits run the focused desktop baseline:

- generated type check;
- TypeScript typecheck;
- frontend tests;
- production frontend build;
- relevant `repopact-desktop-api` tests;
- WI057 process/cache focused tests;
- governance validation/frozen check as appropriate.

Record exact counts.

## Phase 2 — presentation primitives

Add a reusable accessible secondary-tab component (name flexible) rather than page-specific ad hoc strips.

Requirements:

- count badge support;
- controlled active id;
- `role=tablist`, `role=tab`, `aria-selected`, panel association;
- keyboard Left/Right/Home/End;
- visible focus state;
- adaptive wrapping/no page-wide horizontal overflow;
- no data fetch/process behavior in the component.

Add a reusable local pager with one shared default page size, preferably 10–12 rows unless viewport evidence justifies another number.

Pager requirements:

- Previous/Next;
- disabled first/last boundaries;
- `start–end of total` text;
- deterministic clamping when filters/data change;
- page reset on category/search changes where that is least surprising;
- no backend/native call on page change.

## Phase 3 — Work first

Implement Work exactly as the operator requested:

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

Default to Active if it is non-empty, otherwise first non-empty lifecycle tab.

Search is scoped to the selected tab. The detail pane remains intact. Create/edit/transition behavior remains behind existing Rust-owned plans.

Use local pagination so Complete cannot become a long scroll wall.

After transition/apply/refresh, if the selected item moves lifecycle category, reconcile sub-tab/selection deterministically.

## Phase 4 — typed summaries for Decisions/Evidence

The current generic `RecordSummaryView` is insufficient for useful category tabs.

Do not solve this by fetching every record detail.

Prefer explicit Rust DTOs such as:

- `DecisionSummaryView`: reference/readable/title/status/date/supersedes as available;
- `EvidenceSummaryView`: reference/readable/timestamp/work_item/result/provenance as available.

Names may differ, but keep them typed and generated into TypeScript.

Populate from the **existing cached `RepositorySnapshot`/index**. Reuse existing canonical parsers/helpers where available. Do not introduce frontend parsing that can disagree with validation semantics.

Prove summary generation adds no fresh snapshot and no additional Git calls for an unchanged generation.

## Phase 5 — page-specific secondary tabs

### Dashboard

`Overview | Attention | Session`

Keep each concise. Attention should link users to Validation/Work rather than duplicate full datasets.

### Decisions

`Current | Proposed | Deferred | History`

Mapping:

- Current = accepted
- Proposed = proposed
- Deferred = deferred
- History = rejected + superseded + deprecated

Show counts and newest-first ordering where decision date is available.

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

Retain the accessible table and page it locally.

### Validation

`Errors | Warnings | Info | All`

Default to highest-severity non-empty view on a new generation. If no diagnostics exist, keep the compact all-clear card.

### Analysis

`Constraints | Suggestions | Facts`

Map exactly from current `FindingClassification` values. Keep remediation and explanatory content.

### Settings

`Appearance | Session | Boundaries`

Put theme/presentation controls under Appearance. Keep session identity/generation/watcher under Session. Keep authority/security explanations under Boundaries. A top-bar theme shortcut may remain.

## Phase 6 — state behavior

Remember the selected secondary tab for each sidebar page during the current repository session.

On repository switch, reset to useful defaults rather than leaking old repo UI state.

On ordinary snapshot refresh, preserve selected sub-tabs when still valid.

Do not introduce localStorage/persistent cross-repository UI state unless explicitly justified and recorded; in-memory session state is sufficient for WI058.

## Phase 7 — scrolling/viewport behavior

The purpose is less scrolling, not tabs for their own sake.

Audit the actual layout at normal Windows desktop sizes.

The top-level page heading and secondary tabs should remain quickly visible. Collections should be bounded. Detail content may scroll when intrinsically long, but navigation should not require scrolling through unrelated rows first.

Do not create nested tiny scroll boxes everywhere. Prefer tabs + pages over multiple independent vertical scroll regions.

## Phase 8 — tests

Add frontend tests for at least:

1. exact Work tab order/labels;
2. Work lifecycle mapping;
3. blocked nested under Active and still labelled blocked;
4. count badges;
5. Complete pagination;
6. Previous/Next boundaries and range text;
7. search within selected Work tab;
8. tab/page changes cause no new desktop API invocation;
9. keyboard/ARIA tab behavior;
10. Decisions mapping;
11. Evidence mapping and newest-first Recent;
12. Graph grouping;
13. Validation severity grouping/default;
14. Analysis classification grouping;
15. per-sidebar-page tab memory;
16. repository-switch reset behavior.

If Rust DTOs change, add Desktop API tests proving metadata is projected from the cached snapshot and does not trigger N+1 reads/fresh snapshots.

Re-run WI057 Git-count/cache tests to prove the UI refactor does not regress process amplification.

## Phase 9 — native Windows verification

Build the Workbench and manually/native-operator verify:

- select RepoPact;
- Work shows `Proposed | Active | Deferred | Complete` immediately;
- Active visibly surfaces blocked + in-progress groups;
- switch through all four Work tabs;
- Complete is paged and does not create a giant page;
- exercise search and paging;
- visit every sidebar section and confirm its secondary tabs are coherent;
- inspect at least one Work detail and one Decision/Evidence detail;
- confirm create/edit/transition plan UI still opens correctly without applying unnecessary repository mutations;
- resize the desktop window narrower and confirm secondary tabs remain usable;
- confirm no terminal flashes/process storm and normal responsiveness.

If GUI automation is unavailable, use the same operator-assisted/manual evidence standard accepted in WI057. Do not make false automation claims.

## Phase 10 — full closeout

Run:

- frontend generated types/typecheck/tests/build;
- Rust desktop API/workspace relevant checks;
- Python regression suite if repository policy requires full closeout;
- 20/20 conformance;
- WI050 8/8;
- linked-worktree/reference parity;
- WI057 process/cache regression;
- RepoPact validation/dashboard regeneration;
- frozen-surface check.

Record evidence and complete WI058 only after the native UI behavior is verified.

## No-go conditions

Stop for architecture review rather than improvising if implementation would require:

- a lifecycle/status schema change;
- a new persistent UI-state database;
- per-record detail fetching for list classification;
- server/native pagination merely for UI convenience;
- new Tauri fs/shell/process permissions;
- bypassing cached snapshot APIs;
- changing WI056 engine architecture;
- changing WI050 security semantics;
- changing frozen workflows.

## Completion report

Return:

- final `main` SHA and commits;
- exact secondary tabs for each sidebar page;
- Work blocked handling;
- pager size/behavior;
- DTO/type changes;
- proof Decisions/Evidence do not N+1 fetch;
- frontend accessibility/keyboard behavior;
- API/process-count proof for tab/page changes;
- native Windows UX result;
- all test counts;
- frozen-surface status;
- any residual UX issues;
- whether WI056 can continue without reconciliation.