# Work Item 058 — User-Centric Tabbed Workbench Information Architecture

**Status:** Active

**Owner:** tooling

**Affected scopes:** ui, desktop, tooling, work, docs, evidence

**Depends on:** WI055, WI057

## Purpose

Reduce vertical scanning and long scroll walls in RepoPact Workbench by introducing a consistent secondary-tab information architecture inside every sidebar section.

This is a user-interface organization change, not a governance-semantic or authority change.

The motivating operator preference is explicit: Work should not present one long lifecycle-mixed list. The primary Work tabs are:

1. **Proposed**
2. **Active**
3. **Deferred**
4. **Complete**

The same low-scroll pattern should extend across the other sidebar pages using categories that make sense to a user of that page rather than mechanically repeating Work lifecycle labels.

## Core UX rule

A sidebar page answers one user question at a time.

Secondary tabs divide the page into a small number of meaningful views. Large collections use bounded paging rather than requiring the user to scroll through the entire corpus.

Tabs are presentation projections of the current immutable desktop snapshot. Switching a secondary tab or page must not cause a repository recrawl, Git subprocess, watcher refresh, or independent native snapshot.

## Work

Use exactly these primary tabs:

- **Proposed** — `status == proposed`.
- **Active** — operationally current work. Show `blocked` items in a clearly labelled **Blocked** group at the top, followed by ordinary `active` items. Do not relabel blocked records or hide their real lifecycle state.
- **Deferred** — `status == deferred`.
- **Complete** — `status == completed`.

Each tab shows a count. Search applies to the selected tab by default. Selected record detail remains in the existing right-hand detail pane.

A completed corpus can be large, so the list must use bounded local pagination with a compact range indicator and Previous/Next controls instead of rendering every row into one vertical page.

## Dashboard

Use secondary views oriented around what the operator needs to know:

- **Overview** — key counts and navigation cards.
- **Attention** — validation problems and currently blocked work at a glance, with links to the relevant section.
- **Session** — repository identity, generation, watcher state, and native-session information.

Do not turn the dashboard into another giant all-data page.

## Decisions

Use the decision lifecycle in user-facing form:

- **Current** — accepted decisions.
- **Proposed** — proposed decisions.
- **Deferred** — deferred decisions.
- **History** — rejected, superseded, and deprecated decisions.

Counts must come from one snapshot-backed summary projection. Do not fetch every decision detail merely to classify it.

## Evidence

Evidence is task-oriented rather than lifecycle-oriented:

- **Recent** — newest evidence first, bounded to a practical recent window/page.
- **Passed** — successful runs.
- **Attention** — failed, partial, or blocked runs.
- **All** — complete evidence index with local pagination.

Summary rows should surface useful metadata such as result, timestamp/date, and associated work item when available.

## Graph

Group the relationship table by purpose:

- **Dependencies** — work/dependency flow relationships.
- **Evidence** — support/evidence relationships.
- **Governance** — ownership, scope, constraints, applicability, supersession, and related governance edges.
- **All** — full edge set.

Keep the existing accessible table representation. This work item does not require a graphical node canvas.

## Validation

Use severity-focused tabs with counts:

- **Errors**
- **Warnings**
- **Info**
- **All**

Default to the highest-severity non-empty category. If there are no diagnostics, present the existing all-clear state without empty tab clutter.

## Analysis

Map directly to the existing explainable classifications:

- **Constraints**
- **Suggestions**
- **Facts**

Counts should be visible. Preserve remediation text and provenance/basis data already available to the desktop surface.

## Settings

Organize settings/information into:

- **Appearance** — theme and presentation preferences.
- **Session** — current repository/session/generation state.
- **Boundaries** — native authority and security boundary information.

It is acceptable to retain a top-bar theme shortcut, but Settings must become the canonical place to understand/change presentation preferences.

## Secondary-tab component

Build a reusable accessible secondary-tab primitive rather than implementing eight unrelated button strips.

Requirements:

- selected state is visually obvious;
- counts/badges are supported;
- keyboard navigation supports Left/Right plus Home/End where practical;
- `role=tablist`, `role=tab`, `aria-selected`, and associated tab panels are correct;
- tab strips wrap or adapt on narrower desktop windows rather than causing a page-wide horizontal scroll;
- per-page active sub-tab is remembered while navigating the sidebar during a repository session;
- pagination resets or clamps safely when filters/search/snapshot data change.

## Bounded collection behavior

Default list/table page size should be small enough that a normal desktop Workbench window does not become a long page. Use a single shared default unless a page has a demonstrated reason to differ.

Pagination is client-side over already loaded snapshot projections for this milestone. It must not create a new native read on each page change.

Provide:

- `Previous` / `Next`;
- `1–N of M` range text;
- disabled boundary controls;
- deterministic ordering;
- sensible reset/clamping after repository refresh or tab change.

## Snapshot/process boundary

WI057 is binding.

Secondary tabs, search, paging, sorting, and grouping should operate against data already projected from the current cached native generation. If richer summary metadata is needed for Decisions/Evidence, add typed Rust desktop summary DTOs populated from the existing snapshot/index.

Do **not** solve classification by opening every record or performing per-tab/per-page native queries.

Changing secondary tabs must add zero Git subprocesses and zero new repository snapshots for an unchanged generation.

## Explicitly out of scope

- changing RepoPact work lifecycle semantics;
- adding a fifth Work tab for `blocked` against the operator-requested four-tab layout;
- hiding blocked work;
- changing decision status semantics;
- changing evidence result semantics;
- graphical graph visualization;
- virtualized remote/server pagination;
- generic frontend filesystem/process authority;
- WI056 canonical-engine/Python cutover implementation;
- WI050 security-authority changes.

## Sequencing

WI058 may execute independently of WI056 because its implementation surface is the desktop view/DTO layer. If the same coding agent is used serially, prefer completing this short UI milestone before returning to the larger WI056 cutover.

Any overlap with WI056 must preserve WI057's cached-generation/process guarantees and generated-type source of truth.

## Closeout standard

Close only when the real Workbench demonstrates the requested Work lifecycle tabs, all sidebar sections use a coherent secondary-tab/low-scroll organization, large collections are bounded by local pagination, keyboard/accessibility tests pass, tab/page changes do not trigger new Git/snapshot work, existing guarded mutation/detail workflows remain intact, and a native Windows operator pass confirms the result is materially easier to navigate.