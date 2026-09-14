# The Repository Orientation Graph (ROG)

This is the canonical human + agent guide to RepoPact's durable Repository
Orientation Graph (WI063). It covers what ROG is, what it is not, how to
enable/use/troubleshoot it, and how it relates to RepoPact's governance
authority. See Decisions 0044-0053 for the binding architectural record;
this document explains the same system in operational terms.

## What ROG is

ROG is a **deterministic, derived, bounded orientation index** over a
repository's own committed source and governance records. It answers
questions like "what depends on this work item," "what tests exist for this
package," or "what governance applies here" without a repository-wide text
search, by maintaining a durable graph of nodes (files, symbols, work items,
decisions, packages, ...) and typed relations between them.

It is built deterministically from repository content: the same source
tree always produces the same graph (same node/edge counts, same content
hashes, same fingerprint), on Windows, Linux, or macOS.

## What ROG is not

- **Not a second source of truth.** Every governance fact ROG can surface
  (work-item status, ownership, frozen surfaces, evidence, decisions)
  already lives in RepoPact's canonical records (`work/`, `governance/`,
  `decisions/`, `evidence/`). ROG never invents or overrides those facts —
  see [Authority boundary](#authority-boundary-what-rog-cannot-do) below.
- **Not an LLM, embedding index, or cloud service.** The deterministic core
  requires no model, no vector database, no external provider, and no
  network access (ROG-036). It is a local, Git-tracked artifact plus a
  disposable in-memory query index rebuilt on demand.
- **Not a persistent local cache/database.** There is no SQLite file, no
  local acceleration index outside `rog/` itself. The durable graph
  committed to `rog/` *is* the persistent representation; queries build a
  throwaway in-memory index from it on each call.
- **Not required.** A repository with ROG disabled, or never enabled, is a
  completely valid RepoPact repository (Decision 0051).

## Fact classes

Every ROG-surfaced value belongs to exactly one of these classes, and every
query result discloses which:

| Class | Source of truth | Example |
|---|---|---|
| **Authoritative source/governance record** | `work/*.json`, `governance/*.json`, `decisions/*.md`, `evidence/*.json` | work-item status, ownership, frozen surfaces |
| **Deterministic derived graph fact** | Computed from the above by a fixed, versioned algorithm | "file X defines symbol Y", "work item A depends on B" |
| **Navigation hint** | Ranked, reason-coded suggestion computed at query time | `graph.orient`'s `navigation_hints` list |
| **Unsupported/partial coverage disclosure** | An honest gap, not a fact | "fixture topology unavailable", "partial semantic coverage" |
| **Future heuristic/inferred fact** (not implemented today) | Would carry `derivation: inferred` or `heuristic` | reserved for future assisted enrichment; never authoritative (ROG-036) |

A derived graph fact is never presented as if it were an authoritative
record, and a navigation hint is never presented as if it were a fact.

## Enable / disable / adopt / backfill

```bash
# New repository, adopting RepoPact governance and opting into the graph
# in one pass:
repopact adopt <path> --graph

# Already-governed repository, opting the graph in later (backfill):
repopact graph build --root <path>

# Explicitly turn the graph off (removes rog/, persists capability=disabled):
repopact graph disable --root <path>
```

The graph capability is tracked in `governance/rog-capability.json`
(`{"capabilities": {"rog": "enabled"|"disabled"}}`), a small, schema-
validated, committed record — never a graph node itself. Five states are
possible (Decision 0051 / ROG-039):

| State | Declaration | `rog/` exists | Meaning |
|---|---|---|---|
| `legacy_absent` | none | no | valid, disabled (an ordinary pre-ROG repo) |
| `legacy_enabled` | none | yes | valid, binding (pre-0051 graph, still honored) |
| `explicit_disabled` | disabled | no | valid, disabled |
| `explicit_enabled` | enabled | yes | valid, binding |
| `enabled_missing` | enabled | **no** | **hard failure** — never silently "absent" |

## Build / rebuild / status / verify

```bash
repopact graph build --root <path>    # build (first time) or rebuild
repopact graph status --root <path>   # read-only: capability + freshness
repopact graph verify --root <path>   # read-only: structural verification
```

`status`/`verify` never write anything — pressing "Verify" in the Workbench
cannot enable, disable, build, or repair the graph as a side effect. `build`
is the only write path, and it persists the `enabled` capability **only
after** the build has already succeeded and been verified (Decision 0051)
— a failed build never leaves capability falsely claiming success.

## Freshness and coverage

Every status/query result discloses, as orthogonal facts:

- **capability state** (see table above);
- **durable freshness**: `fresh` / `stale` / `unsupported` / `corrupt` /
  `absent` — whether the committed graph still matches current source;
- **basis**: `durable` (the committed graph) or `working_overlay`
  (uncommitted working-tree edits are reflected, in an open session);
- **coverage**: `complete` or `partial` — whether every supported file was
  fully processed.

A client must never flatten these into "current, complete." A partial or
stale answer is disclosed, not hidden.

## Unsupported languages

A file in a language ROG's semantic adapters do not parse (Rust, Python,
JavaScript, and TypeScript are currently supported) is still listed as a
physical file fact, but contributes no semantic (symbol/import) facts. This
is a distinct, disclosed coverage state (`unsupported_language`), never
silently treated as "no content" or folded into a parse failure.

## Fixture-topology boundary (privacy/exclusion)

A `fixtures`-named directory anywhere in the repository is a recognized
**excluded boundary** (Decision 0053 section 1 / ROG-019): the graph knows
*that* it exists (one `Directory` node, `node_role = "test_fixture"`), and
that fact participates in the projection fingerprint (creating/removing/
renaming the boundary changes the fingerprint) — but its **contents are
never read, enumerated, or hashed**. Editing a file inside a fixture
boundary produces zero graph churn. This composes with, and does not
weaken, the pre-existing rule that `fixtures/` content is fully excluded
from source projection (ROG-021).

## Graph search, query, and orientation

The canonical bounded query surface (Decision 0050, extended by Decision
0052 with `graph.search`):

```bash
repopact graph search --root <path> "063"          # bounded, deterministic text search
repopact graph resolve --root <path> --work-item 063
repopact graph orient --root <path> --work-item 063 # identity+deps+tests+governance+hints
repopact graph dependencies --root <path> --work-item 063
repopact graph dependents --root <path> --work-item 063
repopact graph impact --root <path> --work-item 063  # structural_only, never behavioral proof
repopact graph tests --root <path> --work-item 063
repopact graph governance --root <path> --work-item 063
```

Every query is read-only, bounded (`max_nodes`/`max_edges`/`max_depth`/
`page_size`, opaque cursor pagination), and never reads source text or
invokes Git beyond the graph's own load. `graph.search` ranks matches
deterministically (exact, exact-normalized, prefix, substring over stable
ID/path/label/role) — never a fuzzy or embedding-based similarity score.

## Workbench operator map

The Workbench's Graph tab offers an "Operator map" view (toggle next to
the pre-existing relationship table) driven entirely by the same typed
query boundary: search → select (by stable node ID) → identity/context →
bounded neighbor drill-in (with layer/direction filters) → impact/tests/
governance → source navigation. Truncation is always visible with a
load-more control. Verify and Build/Rebuild are exposed as explicit
controls — Verify is read-only; Rebuild requires an explicit confirmation
step before the durable write runs.

## Source navigation

Clicking through from a graph fact routes work-item/decision/evidence
records into RepoPact's existing detail views. Every other kind of fact
(files, symbols, ...) shows its repository-relative path and a
"copy path" action — ROG does not grant any new filesystem-open or
shell-execution capability.

## Merge reconciliation

```bash
repopact graph reconcile-merge --root <path>
```

An explicit, operator-run command (never invoked automatically by
`git merge`) that repairs a merge leaving `rog/**` conflicted. Source and
`governance/rog-capability.json` are authoritative — a conflict in either
blocks repair entirely and is never guessed at or auto-resolved. Only when
every unresolved path is confined to `rog/**` does it regenerate the graph
from the already-merged source, verify it, and stage the result — never a
hand-merge of graph JSONL.

## Cache behavior

There is no persistent local acceleration cache. The durable graph in
`rog/` (committed to Git) is the only persistent representation; every
query rebuilds a small in-memory index from it on demand. A clean `git
clone` of a graph-enabled repository can `validate`/`status`/`verify`/
query immediately — no rebuild, no re-parsing of source.

## Troubleshooting

| Symptom | Meaning | Action |
|---|---|---|
| `enabled_missing` / "capability declares rog=enabled but no durable graph exists" | Capability says enabled but `rog/` is missing (deleted, `.gitignore`'d, or a corrupted clone) | `repopact graph build` to repair, or `repopact graph disable` if intentional |
| `stale` | Source changed since the last build | `repopact graph build` (or `graph update` for incremental) |
| `corrupt` | Structural/hash validation failed | `repopact graph build` to regenerate; never hand-edit `rog/` |
| `unsupported schema` | The durable graph's schema major is newer/older than this build understands | Upgrade RepoPact, or rebuild with a compatible version |
| partial parser coverage | Some supported files failed to parse or were skipped by size policy | Check the disclosed `coverage: partial` warning; not itself an error |
| "would be ignored" enablement refusal | `.gitignore` excludes `rog/` or the capability record | Fix `.gitignore` yourself — RepoPact never rewrites it |
| `reconcile-merge` refuses with `AuthoritativeConflict` | An ordinary source/config file is still conflicted | Resolve it through normal Git conflict resolution first |
| `reconcile-merge` refuses with `CapabilityConflict` | `governance/rog-capability.json` itself is conflicted (an enable-vs-disable policy choice) | Resolve the conflict manually — RepoPact never infers which side wins |
| Fresh clone reports `stale` unexpectedly on Windows | Git line-ending normalization altered committed bytes | Should not occur — `.gitattributes` `-text` protection is written automatically on build; if it recurs, treat it as a defect, not a workaround target |
| A linked `git worktree` reports odd node counts | (Historical, fixed) a worktree's own `.git` pointer file leaking into the projection | Update to a version with the fix; not user-actionable |

## Authority boundary: what ROG cannot do

Graph queries and the Workbench operator map are **informational**. They
never:

- grant ownership or approval;
- waive an acceptance criterion;
- acknowledge a frozen surface;
- change a work item's lifecycle status;
- authorize or apply a mutation.

Those authorities always derive from their existing canonical records
(`governance/owners.json`, `work/*/work-item.json`, `governance/frozen-
surface.json`, `evidence/runs/*.json`) and RepoPact's existing mutation-
plan/admission machinery (WI054, WI050) — never from a graph shard. This
is proven by executable tests (see `rust/crates/repopact-graph/src/
authority_boundary_tests.rs` and `repopact-mutation`'s adversarial tests):
a durable graph shard tampered to contain a fabricated
"APPROVED"/"owner"-labeled node changes nothing about mutation planning,
diagnostics, or canonical work-item state.

## Agent workflow examples

A typical bounded orientation sequence for an agent starting cold on a
work item:

```bash
repopact graph status --root .              # capability/freshness first
repopact graph orient --root . --work-item 063
repopact graph dependencies --root . --work-item 063
repopact graph tests --root . --work-item 063
repopact graph impact --root . --work-item 063
repopact graph governance --root . --work-item 063
```

Or, to find a target by name first:

```bash
repopact graph search --root . "orientation graph"
repopact graph orient --root . --id <resolved-id>
```

**How to react to each disclosed state:**

- **`ambiguous`** — do not guess. Present (or programmatically inspect)
  the candidate list and disambiguate with a more specific selector
  (repository path, symbol container, etc.).
- **`truncated: true`** — the result is a bounded page, not the whole
  answer. Use the returned `next_cursor` to continue, or narrow the
  request (layer/relation filters) rather than assuming completeness.
- **`coverage: partial`** — some supported files were not fully processed.
  Trust what is returned as far as it goes; do not treat absence of a
  fact here as proof the fact does not exist.
- **`durable_freshness: stale`** — the committed graph predates current
  source. Either rebuild (`graph build`/`graph update`) or explicitly pass
  `allow_stale` if a stale-but-disclosed answer is acceptable for the task.
- **`corrupt`** — do not trust any query result; rebuild.
- **`enabled_missing`** — do not treat this as "graph absent, repository
  fine." It is a hard failure; rebuild or explicitly disable.
- **`disabled`** / **`legacy_absent`** — no graph is expected. Fall back to
  RepoPact's ordinary governance-record reading (work items, decisions,
  evidence) — this is a fully valid, ordinary state, not an error.

**Do not** fall back to a broad repository-wide grep merely because the
graph exists — use the bounded query surface first where it applies.
**Do** still verify source directly whenever a task's correctness actually
requires reading real file content (the graph never substitutes for that;
it only helps you find where to look).

## Reference: capability/schema versions

- Graph schema major: `repopact_graph::durable::CURRENT_GRAPH_SCHEMA_VERSION`
  (3 as of this document; versions 1-3 remain permanently readable).
- Query contract version: `repopact_graph::query::QUERY_CONTRACT_VERSION`
  (1 as of this document).
- Capability declaration schema:
  [`repopact/schemas/rog-capability.schema.json`](../repopact/schemas/rog-capability.schema.json).

See also: [Decisions 0044-0053](../decisions/), and WI063's
`implementation-progress.md` for the full checkpoint-by-checkpoint
implementation and evidence history.
