# WI058 Sol Architecture Review — User-Centric Tabbed Workbench IA

Date: 2026-09-10
Reviewer: Sol High
Implementation agent: Codex

## Finding

The current desktop UI has a consistent structural issue: collection-heavy sidebar pages render one unbounded vertical collection. `WorkPage` maps the full filtered work-item set into one `record-list`; the generic Decisions/Evidence page does the same; Graph renders one edge table; Validation one diagnostic list; Analysis one finding list.

The correct fix is not eight unrelated filters. Introduce one reusable secondary-navigation pattern and page-specific grouping semantics.

## Architectural constraints

1. **Presentation only unless typed summary metadata is required.** Do not change canonical repository/lifecycle semantics.
2. **WI057 remains binding.** Tab changes, paging, sorting, and search must operate over the current cached native generation and add zero Git subprocesses/new snapshots.
3. **No N+1 detail reads.** Decisions/Evidence may need richer summary DTOs; derive them once from the cached snapshot/index rather than calling `get_record` for every row.
4. **Do not over-generalize DTOs.** Prefer explicit `DecisionSummaryView` / `EvidenceSummaryView` or another typed equivalent over an unstructured metadata bag if backend changes are needed.
5. **Preserve detail panes and mutation flows.** Secondary navigation should reduce scanning, not replace guarded Rust-owned planning/apply behavior.
6. **Pagination is local.** No remote/server paging or new process boundary for WI058.

## Shared UI primitive

Create a reusable `SectionTabs`/equivalent component with:

- tab id/label/count;
- controlled selected id;
- semantic tablist/tab/panel wiring;
- keyboard Left/Right/Home/End behavior;
- focus-visible styling;
- adaptive wrapping at narrower widths;
- no horizontal page scroll.

Create a reusable local collection-pager helper/component with a single default page size (recommended 10–12 rows initially) and range display.

Tab/pager state belongs in the frontend presentation layer. Remember each sidebar page's selected sub-tab for the current repository session. Reset safely on repository switch; preserve/clamp on refresh when possible.

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

### Dashboard

`Overview | Attention | Session`

Overview stays compact. Attention surfaces validation errors/warnings and blocked work as navigation affordances, not duplicated full lists. Session contains identity/generation/watcher/native boundary information.

### Decisions

`Current | Proposed | Deferred | History`

Map accepted -> Current; proposed -> Proposed; deferred -> Deferred; rejected/superseded/deprecated -> History.

The current `RecordSummaryView` lacks decision status/title/date. Do not fetch every detail. Add a snapshot-derived typed summary if necessary. Reuse an existing canonical/frontmatter parser if one exists; do not create a contradictory decision-status parser solely in React.

### Evidence

`Recent | Passed | Attention | All`

Recent should default to newest-first and remain bounded. Passed -> result passed. Attention -> failed/partial/blocked. All -> paged full corpus.

The current summary lacks timestamp/result/work-item. Add typed snapshot-derived summary metadata rather than N detail calls.

### Graph

`Dependencies | Evidence | Governance | All`

Recommended mapping:

- Dependencies: `depends_on`, `reverse_dependency`
- Evidence: `supported_by`, `supports_work_item`
- Governance: remaining governance/ownership/scope/constraint/supersession edges
- All: all edges

Keep the accessible table. Pagination applies after edge grouping.

### Validation

`Errors | Warnings | Info | All`

If diagnostics exist, choose the highest-severity non-empty tab on first entry/new generation unless the user has explicitly selected another tab for the current generation. If no diagnostics exist, keep the compact all-clear card.

### Analysis

`Constraints | Suggestions | Facts`

Mapping is exact from `FindingClassification`: constraint, suggestion, fact. Preserve remediation text and related/basis information already exposed.

### Settings

`Appearance | Session | Boundaries`

Move or duplicate the theme selector into Appearance. Session describes repository identity/generation/watcher. Boundaries explains local/native authority and plan/session behavior. Top-bar theme shortcut may remain.

## Ordering

Use deterministic user-facing ordering rather than filesystem accident:

- Work: blocked first in Active; otherwise prefer most recently updated first if `updated` is available without extra reads, with stable ID fallback.
- Decisions: newest decision/date first where typed metadata exists.
- Evidence: timestamp descending.
- Validation/Analysis: preserve stable canonical order within classification unless another order is explicitly justified.
- Graph: stable relationship/source order.

## Testing requirements

Frontend tests must prove:

- exact Work tab labels/order and status mapping;
- blocked appears only under Active and retains `blocked` label;
- pagination boundaries and range text;
- sub-tab counts;
- search + pagination interaction;
- keyboard tab navigation and ARIA semantics;
- per-page tab-state memory;
- no extra desktop API calls when switching sub-tabs/pages;
- Decisions/Evidence classification without per-record detail calls;
- Validation and Analysis mappings;
- narrow-window/tab-wrap behavior at least structurally.

Rust/Desktop API tests, if DTOs change, must prove summary metadata comes from the supplied cached snapshot and does not introduce fresh snapshot/Git work.

Re-run WI057 process-count/cache regressions and the WI055 frontend/security/type/build gates.

## Native acceptance

A Windows Workbench pass should verify:

- Work opens on a useful default tab (Active is preferred when non-empty);
- Proposed/Active/Deferred/Complete are immediately understandable;
- Complete does not produce a giant page;
- all sidebar pages use their secondary organization coherently;
- detail selection and guarded create/edit/transition still work;
- no terminal flash/process storm regression;
- normal use requires materially less scrolling.

## Sequencing recommendation

WI058 is small and largely isolated from WI056. If one Codex session is acting serially, complete WI058 first, close it, then resume WI056 from the resulting `main`. If two independent worktrees/sessions are used, they may proceed concurrently, but Codex must merge normally and reconcile any generated DTO/type conflicts rather than overwrite either stream.