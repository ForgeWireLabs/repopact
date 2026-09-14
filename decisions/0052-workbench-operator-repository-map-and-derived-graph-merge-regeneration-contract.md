---
id: 0052
title: Workbench operator repository map and derived-graph merge/regeneration contract
status: accepted
date: 2026-09-14
supersedes: []
---

# 0052: Workbench operator repository map and derived-graph merge/regeneration contract

## Context

Decision 0050's bounded query kernel and Decision 0051's capability/
adoption/backfill/clean-clone contract are now mature enough to
integrate with two remaining WI063 surfaces: a Workbench operator-
oriented repository map (ROG-027/028) and a defined branch/merge
contract for derived graph files (ROG-029). Both need one binding
architectural rule stated up front, because the two most likely
failure modes are the same shape: something outside the canonical
Rust kernel starts inventing graph semantics of its own.

```text
ROG data/query kernel
        |
        v
typed desktop query boundary
        |
        v
operator repository map

NOT

raw graph dump
        |
frontend invents traversal/search/impact semantics
```

The frontend must consume the canonical bounded query system. It must
not become a second graph engine. Likewise, a Git merge must not
become a place where RepoPact invents graph semantics from conflict-
marker source, or infers a governance choice (enabled vs. disabled)
on the operator's behalf.

## Decision

### 1. Workbench operator map: typed queries are authoritative

- Every fact the operator map shows came back from a typed
  `QueryEnvelope<...>` produced by `repopact_graph::query::
  GraphQueryEngine` -- reached through exactly one Tauri command,
  `graph_query(GraphQueryRequest) -> QueryEnvelope<...>` (JSON), which
  the desktop already had as `DesktopSession::graph_query` but had not
  yet exposed to the frontend. No second traversal/search/impact
  implementation exists in TypeScript.
- The operator map is bounded: every request goes through the same
  `QueryBounds`/cursor contract Decision 0050 established. Truncation
  is always visibly disclosed, never silently hidden.
- Fact vs. navigation hint stays explicit: `NavigationHint`s are never
  rendered as if they were graph facts.
- Freshness/coverage/capability state is always visible, and the eight
  distinguishable states named by Decision 0051/ROG-039 are never
  collapsed into fewer categories or represented by color alone.
- Source navigation stays repository-relative: no absolute filesystem
  path is invented, and no arbitrary shell/file-open capability is
  added merely to satisfy "click through to source."
- Rebuild is an explicit, confirmed, durable mutation, reached through
  one narrow Rust/Tauri path (`graph_build`) -- never a shell-out to
  the CLI from JavaScript. Verify is read-only by construction
  (`graph_verify`/`graph_status`) and cannot build, update, enable,
  disable, or repair as a side effect.
- A force-directed visual canvas is not required. The accepted
  interaction model is operator-centered: search/resolve -> select ->
  inspect identity/source -> drill bounded neighbors -> filter
  relationships -> inspect impact/tests/governance -> navigate to
  source.

### 2. `graph.search`: a new, additive, bounded query operation

`graph.resolve` requires an exact typed selector and is not suited to
an operator free-text search box, where several plausible matches are
legitimate. `graph.search` is added to `repopact_graph::query` as a
new operation: bounded, deterministic, in-memory over the existing
query index -- no repository scan, no source read, no Git, no fuzzy/
embedding/LLM similarity search. Ranking is a documented, deterministic
tier: exact match, then exact match after case/whitespace
normalization, then prefix match, then substring match, each over a
fixed field set (stable ID, repository-relative path, label, node
role), with stable-ID tie-breaking. Results are navigation candidates,
never a new graph fact. This is additive: query contract version 1 is
unchanged, since no existing operation's wire semantics changed.

### 3. Git/branch behavior: source is authoritative, `rog/**` is derived

- `source`/configuration content is authoritative for merge purposes;
  ordinary Git conflict resolution governs it exactly as it always
  has. RepoPact never runs graph semantics against conflict-marker
  source and calls the result authoritative.
- `rog/**` is derived: any Git conflict confined to `rog/**` is
  repaired by deterministic regeneration from the already-merged
  authoritative source, never by a human hand-merging JSONL graph
  semantics, and never by a generic `merge=union` driver (which would
  silently produce an invalid, duplicate-laden shard).
- `governance/rog-capability.json` is authoritative *configuration*,
  not derived content, despite living outside `rog/`. An
  enabled-vs-disabled conflict in this file is a real policy choice
  and is never auto-resolved by RepoPact in either direction ("enabled
  beats disabled" and its converse are both explicitly rejected).
- Unresolved source/configuration conflicts always block graph
  regeneration. RepoPact never decides which branch's source is
  correct, and never repairs `rog/**` while an authoritative conflict
  remains outstanding.

### 4. `repopact graph reconcile-merge`: the explicit derived-graph repair command

A new, explicit, bounded operator command (never run automatically
during `git merge`):

1. Query unresolved Git paths once (one bounded Git invocation via the
   shared `GitRunner`, no per-file subprocess fan-out).
2. Classify them: authoritative (source/config, including
   `governance/rog-capability.json`) vs. derived (`rog/**`).
3. If any authoritative path is unresolved -- including a capability
   conflict -- refuse and list the unresolved authoritative paths.
   Fail closed; do not guess.
4. If every unresolved path is confined to `rog/**`, rebuild the graph
   from the merged authoritative source (`repopact_graph::
   build_and_write`, the same canonical builder every other lifecycle
   entry point uses) and verify it.
5. Only after verification succeeds, stage the regenerated `rog/**`
   result with one scoped Git operation. A failed rebuild/verify
   leaves the merge unresolved and reports the graph failure -- it
   never marks a derived conflict resolved before verification proves
   the regenerated graph good.

If the resolved capability state is `explicit_disabled`, the canonical
derived state is "no `rog/`"; reconciliation may stage the removal of
conflicting `rog/**` paths accordingly, but must never re-enable the
graph as a side effect. A legacy-enabled repository that reaches
reconciliation is migrated to Decision 0051's explicit-enabled
declaration by the same `build_and_write` capability-persist-after-
proof-good path every other build already uses -- no separate hidden
migration exists; this is disclosed and tested explicitly.

No developer-configured local merge driver
(`git config merge.repopact-rog.driver ...`) is required for
correctness. The acceptance path works from a normal clone using only
RepoPact's explicit command.

### 5. Sharding churn stays minimized, proven, not assumed

The existing stable hash-to-shard design (Decision 0044 section 6) is
retained unchanged. This checkpoint adds executable evidence rather
than asserting minimality from shard count alone: a real graph is
built, one isolated source contribution is mutated, the graph is
rebuilt, and the exact changed-shard set is measured and compared
against the full shard set, proving the untouched shards remain
byte-identical.

## Consequences

- The Workbench gains a real operator repository map without any
  duplicated graph engine in the frontend; every future consumer
  (agent tooling, a future richer UI) can rely on the same typed
  boundary.
- `graph.search` becomes the second bounded search-shaped primitive in
  the query kernel (after `graph.resolve`'s exact-selector form),
  giving future clients (CLI, agents) a documented, deterministic
  search contract instead of each inventing their own.
- Branch/merge workflows never require a human to understand or edit
  the durable graph's on-disk shard format; `rog/**` behaves, from an
  operator's perspective, like a derived build artifact that Git
  happens to track.
- `governance/rog-capability.json`'s authoritative-configuration
  status is now explicit in both code (never classified as derived by
  `reconcile-merge`) and in this record, closing the ambiguity a
  capability-record merge conflict would otherwise create.

## Alternatives considered

- **A force-directed graph canvas as the primary ROG-027 deliverable.**
  Rejected for this checkpoint: it does not change the underlying
  typed-query-boundary requirement, is explicitly named optional by
  the AC text, and would consume the checkpoint's time on rendering
  rather than on the bounded query/merge contract this decision
  actually needs to bind.
- **A `merge=union` Git attribute for `rog/**`.** Rejected: union merge
  concatenates conflicting JSONL lines without deduplication or
  re-validation, producing a structurally invalid durable graph
  (duplicate node IDs, no fingerprint match) that would pass Git's own
  merge step while failing RepoPact's structural validator -- exactly
  the "hand-merged graph semantics" this decision exists to prevent.
- **Automatically running `reconcile-merge` inside a `git merge`/
  post-merge hook.** Rejected: correctness must not depend on every
  developer's local hook configuration, and an automatic hook cannot
  safely refuse-and-explain an authoritative conflict the way an
  explicit operator command can.
- **Letting `graph build` overwrite an existing `governance/rog-
  capability.json` merge conflict.** Rejected: this is exactly the
  "enabled beats disabled" inference this decision explicitly
  forbids; the capability conflict must be resolved by the operator
  through normal Git conflict resolution first.
