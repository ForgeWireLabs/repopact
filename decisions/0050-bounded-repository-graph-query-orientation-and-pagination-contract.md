---
id: 0050
title: Bounded repository graph query, orientation, and pagination contract
status: accepted
date: 2026-09-14
supersedes: []
---

# 0050: Bounded repository graph query, orientation, and pagination contract

## Context

WI063's durable/incremental/overlay graph lifecycles (Decisions 0044,
0046, 0047) and the operational role model (Decisions 0048, 0049) give
RepoPact a real, materialized, deterministic repository graph. Nothing
yet lets a caller ask a bounded question of it: "what does this depend
on," "what tests exist for this," "where should I start reading." The
only pre-existing surface, the engine's generic `graph` operation,
returns the entire in-memory `RepositoryGraph` with no bounds, no
freshness/coverage envelope, and no incremental/overlay awareness --
an unbounded escape hatch that predates ROG-023 entirely and has no
callers in this codebase today.

This decision defines a typed, bounded, read-only query and
orientation surface (ROG-023/024/025/026) sitting on top of the
already-materialized graph -- never a second graph builder, never new
authority.

## Decision

### 1. Query authority

The query kernel (`repopact_graph::query`) is read-only. It never
mutates `RepositoryGraph`, never writes `rog/`, never invokes Git, does
not open source files, and does not call any LLM/embedding/network
service. It answers questions about facts a builder already put in the
graph. Structural impact is not behavioral proof (ROG-025/037/040): a
query result cannot grant ownership, approval, lifecycle authority,
criterion waiver, frozen-surface acknowledgement, or evidence truth --
those remain governed by their existing source records and mutation/
admission paths (WI054, WI050). This is enforced by construction: the
query kernel's public API has no mutation methods and takes `&
RepositoryGraph`, never `&mut`.

### 2. Target resolution

Resolution is typed, never a fuzzy string parser. `NodeSelector` is a
closed enum: `NodeId`, `RepositoryPath`, `WorkItemId`, `Package`,
`Module`, `Symbol { path, language, symbol_kind, container }`.
Resolution returns exactly one of `Exact`, `Ambiguous(candidates)`, or
`NotFound` -- never a first-match guess. A `RepositoryPath` selector
must be a normalized, repository-relative path; an absolute host path
or a path containing a `..` component is rejected as malformed input,
not silently normalized. No fuzzy/semantic-similarity resolution
exists or is planned here.

### 3. Deterministic traversal

Every traversal (`neighbors`, `path`, `dependencies`, `dependents`,
`context`, `impact`, `orient`) is deterministic: iteration order is
always by a stable key (node/edge id, then kind, then role) via
`BTreeMap`/sorted-vector indexes built fresh per query call, never a
`HashMap`/process-order-dependent structure. The same query against
the same graph fingerprint always serializes to byte-identical JSON.

### 4. Bounds

One reusable `QueryBounds` type (`max_nodes`, `max_edges`, `max_depth`,
`layers`, `relation_kinds`, `max_output_bytes`,
`estimated_token_budget`, `page_size`, `cursor`, `compact`) is shared
by every operation, with centralized conservative defaults. No
operation walks the graph without a bound; every one of `max_nodes`/
`max_edges`/`max_depth`/`max_output_bytes`/`page_size` is enforced as a
hard limit, proven by dedicated tests that construct a graph large
enough to trigger each one.

### 5. Pagination and cursors

Truncation is explicit (`truncated: bool` plus `next_cursor`) and
resumable, never a silent drop. A cursor is an opaque,
length-bounded, base64url-encoded canonical JSON payload binding
`query_contract_version`, the graph fingerprint/effective generation,
the operation name, a hash of the normalized request (selector +
filters), and a continuation position. A cursor presented against a
different fingerprint, operation, or request identity is rejected with
a clear, typed error -- it never silently continues against different
graph state. No Rust memory layout is ever serialized into a cursor.

### 6. Output-budget semantics

`max_output_bytes` is a hard, authoritative, serialized-UTF-8 byte
budget. `estimated_token_budget` is an explicitly named, documented,
deterministic estimate (byte-count-derived, not a real provider
tokenizer) -- callers that need it are told it is an estimate, and the
byte budget always wins if the two disagree. No external tokenizer or
provider dependency is added for this.

### 7. Freshness policy

- **Fresh durable graph** -> queries proceed normally.
- **Stale durable graph** -> by default, a structured stale result
  (typed `freshness: stale` plus a warning), not a silent
  business-as-usual answer; a caller may pass `allow_stale: true` to
  knowingly query it anyway, but the response still always carries
  `freshness: stale` and the warning -- no presentation layer may drop
  it.
- **Unsupported/corrupt durable graph** -> fail closed: a typed error,
  never an attempted best-effort read of a graph shape this build
  cannot trust.
- **Absent durable graph** -> a typed `graph-absent` result with build
  guidance, not an exception disguised as data.

### 8. Durable graph vs. working-overlay querying

The same query kernel operates over either an already-loaded
`RepositoryGraph` (durable) or `SessionGraphState::effective_graph()`
(working overlay) -- never a second, overlay-specific query
implementation. A desktop/session caller queries the overlay's
effective graph; the response's `basis` field (reusing
`overlay::GraphBasis`) discloses `working_overlay` whenever dirty
working-tree state contributed, and `coverage` discloses `partial`
when applicable, per the existing three-axis disclosure model
(Decision 0047 section 5) -- not a fourth flattened status.

### 9. Fact vs. navigation-hint distinction

A query result's `facts` (nodes/edges the graph actually contains,
each carrying its own kind/layer/role/source/derivation) are never
mixed with `navigation_hints` (an ordered, deterministic,
reason-coded, non-authoritative "inspect this first" list `graph.orient`
produces). A hint is never presented as a fact; no hint is ever scored
by a model.

### 10. Coverage disclosure, including the fixture gap

Every query response that could be affected by a known coverage
limitation discloses it as a structured warning, not merely an empty
result: unsupported languages, parser-partial coverage, generated
content skipped, calls/references unsupported at the current
extraction tier, and -- named explicitly because ROG-019 remains
pending exactly on this point -- fixture topology. RepoPact's
`fixtures/`-named directories remain excluded from the entire graph by
the pre-existing `IGNORED_PARTS` policy (Decision 0044), proven by
ROG-021; no included RepoPact source or config metadata currently
names an excluded-fixture-boundary fact, so `graph.tests` (and
`graph.orient`'s test section) emit an explicit
`fixture topology unavailable: fixture directories are excluded by
repository projection policy` warning whenever the queried target's
neighborhood could plausibly include fixtures, rather than silently
implying zero fixtures exist. This decision does not touch
`IGNORED_PARTS`, does not index fixture contents, and does not
fabricate a fixture node or role to manufacture ROG-019 satisfaction.

### 11. Protocol versioning

Every query response carries an explicit `query_contract_version`
(starting at `1`), which is independent of `graph_schema_version`
(currently 3, Decision 0048). A query-contract change (a new field, a
new bound, a new operation) does not require a durable-schema bump;
this checkpoint requires none.

### 12. Legacy `graph` operation

The existing generic `graph` engine operation (`repopact-core::
RepoPactCore::graph_snapshot`, an unbounded fresh `RepositoryGraph::
build` on every call, no freshness/coverage envelope) has no caller in
this codebase as of this checkpoint -- not the Python CLI, not the
desktop API. It is classified as a **legacy pre-ROG compatibility
surface** (option A): left exactly as it is, explicitly excluded from
the new typed query contract, not extended with bounds, and not
wired into any new client. A future ROG-027 Workbench decision may
retire it once (and if) a real caller is found; until then it is
neither deprecated with ceremony nor promoted -- it is simply not part
of this contract. This preserves any undiscovered external caller
without accidentally growing it into a second, competing query
surface.

## Alternatives considered

- **A single flattened freshness/basis/coverage enum for query
  responses.** Rejected: Decision 0047 section 5 already established
  that basis, durable freshness, and coverage are orthogonal facts;
  flattening them for queries would regress that precedent and produce
  a combinatorial enum.
- **A persisted query-time graph index (SQLite or an embedded
  key-value store).** Rejected by this checkpoint's own directive and
  by Decision 0044's non-authoritative-derived-projection precedent: a
  query index is disposable, in-memory, and reconstructed per call: it
  never becomes a second durable artifact alongside `rog/`.
  Rebuilding a `BTreeMap`-based index per query is cheap at this
  repository's scale (thousands, not millions, of nodes) and keeps the
  query kernel stateless between calls.
- **Extending the legacy `graph` operation with bounds instead of
  building a new contract (option B).** Rejected: its zero-caller
  status and complete absence of a freshness/coverage envelope make it
  a poor foundation to retrofit; a clean, versioned, bounded contract
  is simpler than bolting bounds onto an operation nothing currently
  depends on.
- **A generic inbound-edge traversal for package reverse dependencies
  instead of a normalized dependents view.** Not chosen as the sole
  mechanism: `graph.dependents` normalizes both the persisted
  governance `ReverseDependency` edge (work items, Decision 0048/0049)
  and the query-derived inverse of `DependsOn` (packages) into one
  shape, disclosing `provenance: persisted | query_derived_inverse` per
  fact, so a caller never needs to know which representation was
  stored.

## Consequences

- ROG-023/024/025/026 are addressed by one canonical Rust query kernel
  (`repopact_graph::query`), reused identically by the engine, the
  desktop API's session-overlay queries, and (through the engine) the
  Python CLI -- no traversal logic is duplicated in any adapter layer.
- `query_contract_version` evolves independently of
  `graph_schema_version`; this checkpoint introduces the former at `1`
  without touching the latter.
- ROG-019 remains pending: this decision's coverage-disclosure
  requirement (section 10) is the mechanism by which the query surface
  honestly names the one remaining gap, but does not close it.
- The legacy `graph` operation is frozen in place, not extended and
  not wired into any new consumer, pending a future ROG-027 decision.
