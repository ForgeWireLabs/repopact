---
id: 0047
title: Session-scoped working-tree ROG overlay and dirty-state contract
status: accepted
date: 2026-09-13
supersedes: []
---

# 0047: Session-scoped working-tree ROG overlay and dirty-state contract

## Context

Decision 0044 established the durable Repository Orientation Graph (ROG)
projection; Decision 0045 added deterministic semantic extraction;
Decision 0046 proved that an incremental durable update
(`repopact graph update`) converges canonically to a clean full rebuild
while remaining explicit and operator-driven. None of the three touch
interactive session behavior: the existing desktop/session flow
(`repopact-desktop-api`) called `core.graph_snapshot(snapshot)` -- a full
governance+physical+semantic rebuild from scratch -- on every
watcher-debounced filesystem event, every explicit session refresh, and
every mutation apply. Once semantic adapters existed (Decision 0045),
this implied a full source-projection content-hash walk and full
semantic reparse on ordinary interactive file edits, exactly the cost
Decision 0046's own performance evidence showed dominates wall time on a
real repository. WI063 ROG-013 requires eliminating that cost without
introducing a second graph-correctness implementation, a hidden durable
write on every keystroke, or a repository-local database.

This decision binds the session-scoped working-tree overlay that
replaces the naive "watcher event -> full rebuild" behavior, and is
equally explicit that "watcher event -> `graph.update` -> rewrite `rog/`
on every event" would be the wrong fix: it would still perform a full
projection walk per event and would additionally start silently
rewriting the committed durable baseline as a side effect of ordinary
editing, which nothing in Decision 0044-0046 authorizes.

## Decision

### 1. Durable baseline

`rog/` remains the durable projection exactly as Decision 0044 bound it.
Watcher activity, session opening, mutation application, and ordinary
overlay reconciliation never write to it. Only an explicit
already-authorized durable graph operation -- `repopact graph build` or
`repopact graph update` -- writes `rog/`.

### 2. Working overlay

A working overlay is:

```text
in-memory
session-scoped
reconstructable
non-authoritative
non-durable
```

It is held by `repopact_graph::overlay::SessionGraphState`, a value owned
by the embedding session (the desktop/Tauri process's `ActiveSession`, or
any future embedder). It disappears when the session closes or the
process ends; it is never serialized into `rog/`, never written to a
repository-local database, and never persisted anywhere the durable graph
or governance records are. Opening a new session, or switching
repositories within one, discards any existing overlay completely and
constructs a fresh one from durable state plus current source (section 7
below) -- an overlay is reconstructable, not something that must survive
being lost.

### 3. Authority

The overlay may describe current working-tree facts (which supported
files exist, their content digests, and the semantic symbols/relations a
deterministic parse of their current content yields). It does not
authorize mutations, does not override source or governance authority,
and does not grant any client a basis for claiming a file's committed or
governed state differs from what RepoPact's existing mutation/validation
machinery already says. It is read-only orientation data layered on top
of the same facts Decision 0040/0044 already recognize as authoritative.

### 4. Relationship to Decision 0046

The overlay reuses Decision 0046's contribution machinery directly:
`semantic::build_file_contribution` is the same per-file primitive both
the durable `graph.update` path and the overlay call to regenerate a
file's contribution; `incremental::plan_reconciliation` (extracted from
the incremental-equivalence checkpoint's `incremental_update` for this
purpose) is the same delta-reconciliation algorithm both a durable update
and an overlay's session-open/explicit-refresh path use to reconcile a
full projection against a known prior inventory. Neither path duplicates
graph-correctness logic; the durable path additionally calls
`durable::write`, while the overlay path keeps every result in memory.

### 5. Orthogonal disclosure model

The pre-existing `status::Freshness` enum (used by the durable CLI,
`repopact graph status/build/verify`) is left untouched: it still reports
`fresh`/`stale`/`partial`/`unsupported`/`corrupt`/`absent` about the
durable graph relative to current source, exactly as Decision 0044/0046
defined. Forcing a session-facing client to choose between disclosing
"working-tree data contributed" and "semantic coverage is partial" would
be a false choice -- a working overlay can itself contain partial
semantic coverage (one file failed to parse) while every other file
reconciled cleanly. Three orthogonal types capture this instead, in a new
`repopact_graph::overlay` module (deliberately not a change to
`status::Freshness`):

```text
GraphBasis:        durable | working_overlay
GraphCoverageState: complete | partial
DurableFreshness:  absent | fresh | stale | unsupported | corrupt
```

`EffectiveGraphStatus` combines them plus baseline/effective fingerprints,
a changed-path count, and an overlay generation counter, so a client can
truthfully represent, simultaneously:

```text
basis = working_overlay
coverage = partial
durable_baseline = stale
```

without any of the three implying or contradicting the others.

### 6. Canonical engine lives in graph/core, not desktop

`SessionGraphState` and the `overlay` module live in `repopact-graph`,
the same crate that owns `durable`, `semantic`, and `incremental` --
never only inside `repopact-desktop-api`. The overlay engine owns durable
baseline loading, the in-memory source inventory, cached semantic
contributions, the effective graph, coverage, changed paths, reuse/
reparse counts, baseline/effective fingerprints, and dirty/overlay state.
Desktop/Tauri code orchestrates this engine (calls `open`/`reconcile`/
`refresh`/`status`) but must not derive graph correctness itself, and
frontend code must not derive it either -- it only renders the typed
disclosure `EffectiveGraphStatus` already computed by the Rust core.

### 7. Session open prefers loading over reparsing

On session open, if the durable graph is present, structurally valid, and
its recorded source-projection fingerprint already matches a freshly
computed current fingerprint, the effective graph is loaded verbatim from
durable shards (`durable::load_graph`) -- no source file is read, no
adapter runs. If the durable graph is stale relative to current source,
the overlay reconciles once, in memory, using the shared machinery from
section 4; this is a one-time, session-open cost, not a per-event cost.
If no durable graph exists (or it is corrupt/schema-unsupported), the
overlay performs one full in-memory build. In every case, `rog/` itself
is never written by this process.

### 8. Watcher events are hints, not infallible authority

A watcher-reported changed path is never trusted at face value. Before
any contribution is regenerated or dropped, the overlay re-stats the
path directly (`Repository::path_state`, or a cheap `symlink_metadata`
classification first) to determine its actual current state. The
following conditions are conservative-reconciliation triggers rather than
targeted per-path patches, because a per-path patch cannot be trusted to
be correct under them:

- a changed path currently resolves to a directory or a symlink
  (ambiguous -- could be a whole-subtree rename, a mass creation, or a
  removal);
- a single reported burst exceeds a centralized, documented threshold
  (`overlay::LARGE_BURST_FALLBACK_THRESHOLD`), suggesting a branch
  switch, a mass regeneration, or watcher coalescing rather than an
  ordinary edit.

In these cases the overlay performs a full in-memory reconcile
(`SessionGraphState::refresh`) rather than guessing, and reports that it
did so (`ReconcileOutcome.fell_back_to_full_reconcile`). It never writes
durable state as this fallback, and it never silently reports the
overlay as current when watcher certainty was lost -- the alternative
this decision explicitly rejects is treating an ambiguous or oversized
watcher report as though it were a precise, trustworthy diff.

### 9. Explicit refresh is correctness recovery

`refresh_repository()`'s effective-graph refresh is `SessionGraphState::
refresh`: a full source-projection walk diffed against the overlay's own
current in-memory inventory (not necessarily the durable baseline, which
this call does not re-read). After it completes, the effective session
graph truthfully represents the current filesystem state, even though the
durable baseline may remain stale -- that distinction (`durable_freshness`
vs. `basis`) stays visible in `EffectiveGraphStatus` rather than being
collapsed.

### 10. Self-applied mutations update immediately

A session already knows the exact `changed_paths` a successful RepoPact
mutation produced. The overlay is reconciled against those paths
immediately after `apply_mutation_plan` succeeds, without waiting for the
OS watcher to observe the same edit. When the watcher later reports the
identical self-applied paths, the pre-existing `pending_self_paths`/
`ChangeOrigin::SelfApply` de-duplication (unchanged by this decision)
still applies, and the overlay's own no-op detection (an unchanged digest
produces no reparse and does not bump `overlay_generation`) additionally
guarantees the semantic work is not performed twice.

### 11. Durable CLI vs. session-state distinction

`repopact graph status`/`verify` describe the durable graph relative to
current source and may legitimately report `stale` while a session's
`EffectiveGraphStatus.basis` simultaneously reports `working_overlay` --
this is not a contradiction, it is two different, both-true statements
about two different things (the committed durable artifact vs. an
in-memory session view). The CLI is not changed into a hidden persistent
session manager by this decision; it remains a one-shot process reading/
writing `rog/` exactly as before.

## Alternatives considered

- **Watcher event -> `graph.update` -> rewrite `rog/` on every event.**
  Rejected explicitly (see Context): still pays the full projection-walk
  cost per event and additionally starts silently rewriting the committed
  durable baseline as a side effect of ordinary editing, which no prior
  decision authorizes and which would make `rog/`'s git history noisy
  with every keystroke's worth of change.
- **A repository-local acceleration database (SQLite or similar) for
  session state.** Rejected, consistent with WI054's GAM-018 precedent
  and Decision 0044/0046: `SessionGraphState` is a plain in-process Rust
  value, not a file on disk.
- **Collapsing `GraphBasis`/`GraphCoverageState`/`DurableFreshness` into
  one flattened enum.** Rejected per section 5 -- a working overlay with
  partial coverage over a stale durable baseline is a real, simultaneous
  state that a single linear enum cannot express without forcing clients
  to pick one dimension and lose the others.
- **Persistent Tree-sitter syntax trees / `Tree::edit` for keystroke-level
  reparsing.** Deferred, not rejected: this checkpoint proves durable
  graph *contribution* reuse across a session, not parser-tree-level
  incremental reparsing. A future local-cache/acceleration phase may add
  this on top of the same contribution primitive.

## Consequences

- `repopact-graph::overlay` is a new, canonical module; `repopact-desktop-
  api` orchestrates it instead of calling `graph_snapshot` (a full
  rebuild) on every event.
- `EffectiveGraphStatus` becomes the typed surface a session-aware client
  reads to avoid silently presenting stale or partial graph output as a
  current, complete repository map (ROG-010).
- The durable graph format itself is untouched by this decision -- no
  schema-major change, no new manifest fields. Everything here is
  in-memory session state layered on top of the existing schema-v2
  durable representation.
- A future ROG-013-adjacent phase (a persistent parse-tree cache, or a
  richer working-tree query surface) has an explicit, tested foundation
  (the shared contribution/reconciliation primitives, the typed status
  model) to build on rather than needing to invent these rules from
  scratch.
