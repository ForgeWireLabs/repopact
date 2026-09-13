# WI063 Architecture Review - Durable Repository Orientation Graph

**Review date:** 2026-09-12  
**Reviewed base:** `98c2aa8f88a70c470e93310d8d5d682e347d2b48`  
**Architecture reviewer:** GPT-5.6 Sol, High reasoning  
**Coding agent:** not yet assigned  
**Status:** pre-implementation architecture review for proposed WI063

---

## 2026-09-13 activation refresh

**Refreshed baseline:** `8642b91c09fa0bfbbf72375f23bafee6fdf8cf45`  
**Coding agent:** Claude Code  
**Architecture reviewer:** GPT-5.6 Sol, High reasoning (operator-relayed refresh)  
**Status:** activation gate satisfied; WI063 moved `proposed` -> `active`

This section supplements the original 2026-09-12 review above. The original
review's analysis, recommendations, and risk table are not rewritten; they
remain accurate as architecture guidance. This section records what changed
between the reviewed base and activation, and what does not need to change.

### Activation gate

The original review's Activation recommendation held WI063 `proposed` until
"the release-parity UI work and public-release hardening reach the agreed
release gate." That gate is now crossed:

- WI055 (Tauri Workbench foundation) — completed.
- WI058 (tabbed/responsive IA) — completed.
- WI060 (Android Workbench runtime bring-up) — completed.
- WI062 (cross-platform installation/launcher validation) — completed.
- WI046 (local-first verification/release architecture) — completed, with a
  genuine passing Linux CI proof and a real, reproducible host release
  build (evidence: `evidence/runs/20260913-046-final-closeout.json`).
- WI064 (research metadata consistency repair) — completed.

No release-destabilizing work remains queued ahead of WI063 per the
`work/active` and `work/proposed` lifecycle state at the refreshed baseline.

### What changed in the Rust core since the reviewed base (`98c2aa8`)

The reviewed base already reflected `repopact-graph`/`repopact-repository`/
the engine protocol accurately for WI063's purposes. Between `98c2aa8` and
`8642b91`, the commits that touched adjacent architecture were:

- **WI046** (`9d4c508`, `017f73f`, `bbcb023`, and the WI050/WI057 portability
  fixes `74b00e0`/`37dad3e`/`267b80e`): added `CoverageSummary.host_ready`
  to `repopact/verification.py`, a `repopact_release.aggregate_release_readiness`
  operation, and a new `repopact verify status` local-only status view. None
  of this touches `repopact-graph`, `repopact-repository`'s session/snapshot
  model, or the engine protocol's operation dispatch shape. The engine
  protocol's `EngineRequest`/`EngineResponse` envelope and one-shot-per-process
  dispatch model (Decision 0042) are unchanged and remain the correct seam
  for new `graph.*` operations.
- A real, previously-latent **POSIX process-containment gap** was found and
  fixed in `rust/crates/repopact-repository/src/git.rs` (commit `37dad3e`):
  `contain_child()` on non-Windows was a no-op, so `NativeGitRunner`'s
  timeout path could leave an orphaned grandchild process running past its
  configured timeout under process models where a shell does not exec
  directly into its target command. This is now fixed with POSIX
  process-group containment (`process_group(0)` + `killpg` via
  `libc::kill(-pgid, SIGKILL)`). This is directly relevant to WI063: any new
  Git invocation added for graph-orientation purposes inherits this fixed,
  bounded-timeout behavior automatically through `NativeGitRunner`, and the
  WI057 git-invocation-count regression test
  (`snapshot_git_invocation_count_is_bounded_independent_of_work_items`,
  bound `<= 4` invocations per `RepositorySession::snapshot()`) must
  continue to pass unmodified.
- A **cross-platform release-build reproducibility gap** was found and fixed
  in `repopact/release_build.py` (commit `267b80e`): the release build's
  anti-nondeterminism `RUSTFLAGS` (`-C debuginfo=0`, path remapping) were
  previously Windows-only. This is orthogonal to WI063's own durable-graph
  determinism requirement (ROG-011) but is worth noting as a precedent: the
  same class of "ephemeral build/export path leaking into a supposedly
  deterministic output" defect is exactly what WI063's canonical
  serialization rule (no host absolute paths, no nondeterministic
  timestamps) must guard against from the start, not discover after the
  fact.
- **WI054's `repopact-graph` crate** (`rust/crates/repopact-graph/src/lib.rs`,
  511 lines, single file) is unchanged in shape from what the original
  review describes: a flat `GraphNodeKind`/`GraphEdgeKind` enum pair, no
  "layer" concept, `RepositoryGraph::build(&RepositorySnapshot)` walking
  `RecordIndex` sections into nodes/edges, deterministic `sort()`/`dedup()`
  on edges. The original review's recommendation to extend this crate with
  a "second, derived repository-orientation domain" rather than replace it
  is directly actionable as written.
- **No `.repopact` transaction database, daemon, or hidden journal** has
  been introduced anywhere between the reviewed base and the refreshed
  baseline (confirmed via WI054's closeout evidence, which explicitly
  records "no persistent repository-local transaction DB / hidden journal /
  daemon state permitted" as GAM-018, unchanged and unaffected by any
  subsequent work item).
- **`repopact-mutation`'s plan/apply boundary** (Decision 0040) is
  unchanged. It has no atomic-write (tempfile+rename) primitive of its own;
  durability instead comes from content-addressed stale-plan detection,
  preimage capture before any write, and a mandatory post-write
  `repopact_validation::validate()` gate. WI063's foundation phase
  implements its own self-contained atomic durable-graph write (build to a
  fresh temporary directory, verify, then swap into place) rather than
  forcing graph regeneration through `MutationRequest`/`MutationPlan`: a
  full-graph rebuild is a "recompute a deterministic projection from
  filtered source, write derived output" operation, not a targeted
  governance-record edit with a small content-addressed read set, and the
  fit with WI054's typed mutation surface (scoped to work-item
  create/edit/lifecycle/dependency/AC operations, GAM-008/009) is poor. This
  foundation phase's atomic-write behavior is documented in Decision 0044
  rather than silently diverging from WI054's safety properties.

### Stale proposal assumptions to correct

- The proposal's rollout-plan Phase 0 line "refresh this architecture
  review if the Rust core changed materially since WI063 creation" is
  satisfied by this section; the Rust core did not change materially for
  WI063's purposes (see above), so implementation proceeds from the
  original review's recommendations without further architectural
  reconsideration.
### Consolidated implementation sequence for this session

One plan across the layers named in the activation brief, with explicit
phase boundaries. This session (the foundation checkpoint) implements
phases 1-4, 9 (protocol/CLI only, no semantic operations), and 13-partial
(structural validation only). It does not implement phases 5-8, 10-12, or
14.

1. **Durable graph contract** — Decision 0044 (this session).
2. **Graph core** — `GraphLayer`/`DerivationClass` typed vocabulary, new
   physical `GraphNodeKind`/`GraphEdgeKind` variants added to the existing
   `repopact-graph` crate (this session).
3. **Repository/session integration** — physical graph built from
   `RepositorySession`/`RepositorySnapshot`/`Repository::all_files()`; no
   second crawler (this session).
4. **Durable serialization** — manifest + stable-sharded JSONL writer/reader
   with atomic swap-in (this session).
5. **Source projection/fingerprint** — `SourceProjection` module, SHA-256
   content-digest based, self-exclusion, additive to shared
   `IGNORED_PARTS` (this session).
6. **Freshness** — typed `fresh/partial/stale/unsupported/corrupt/absent`
   status derived by recomputing the projection fingerprint against the
   manifest (this session; `working_overlay` is declared in the enum but
   never emitted this session — no watcher/dirty-tree integration yet).
7. **Incremental updates** — NOT this session. Full-build determinism is
   the oracle this phase establishes; incremental update requires that
   oracle to already be stable (per the proposal's own rollout order).
8. **Semantic adapters** — NOT this session. No Tree-sitter, no
   symbol/call graph. Only physical topology (directory/file/workspace/
   config/nested-repository) this session.
9. **Build/test/operational adapters** — NOT this session beyond the
   minimal, already-deterministic manifest-presence classification
   (`Workspace`/`ConfigurationFile` nodes for a directory containing
   `Cargo.toml`/`pyproject.toml`/`package.json`); no Cargo-metadata/
   pyproject/package.json *parsing* or dependency-edge extraction.
10. **Engine protocol/CLI** — `graph.status`/`graph.build`/`graph.verify`
    engine operations and `repopact graph status|build|verify` CLI (this
    session). No `context`/`impact`/`tests`/`governance`/`orient` query
    operations yet beyond a bounded `graph.status` summary.
11. **Adoption/backfill** — NOT this session. `repopact adopt` is
    unmodified. Backfill is the explicit, already-implemented
    `repopact graph build` command; no `adopt`/`doctor` integration.
12. **Workbench** — NOT this session. No new Tauri command, no new frontend
    view. The existing `relationship_graph`/`GraphView` surface is
    unmodified and continues to serve the flat governance graph exactly as
    before (physical-layer nodes/edges are additive to the same
    `RepositoryGraph`, so `GraphView` will begin including them once the
    physical builder runs, but no UI is added to filter/present them
    differently this session).
13. **Validation/conformance** — structural durable-graph validation
    (manifest/shard integrity, duplicate IDs, dangling edges, canonical
    ordering, path safety, fingerprint match) integrated into the Rust
    validation path this session. Incremental/full equivalence,
    clean-clone timing, symlink/adversarial fuzzing, and cross-language
    coverage-matrix validation are NOT this session.
14. **Benchmarking/research** — NOT this session. No S8 R1 amendment, no
    R1 run, no performance/scale evidence beyond what falls out of the
    correctness tests above.

### Fingerprint primitive at the refreshed baseline

The original review's observation that no git blob-hash/content-identity
API exists is still true at the refreshed baseline (confirmed by direct code
  inspection: `git.rs`'s `NativeGitRunner`/`GitRunner` trait exposes only a
  generic bounded `run(root, args, label)`, and the only git queries
  actually issued by `RepositoryTopology::build` remain
  `rev-parse --git-common-dir`, `worktree list --porcelain`,
  `ls-files --cached`, and one bounded `log` call for evidence-run
  commits). Decision 0044 therefore fixes the source-projection fingerprint
  on the existing SHA-256 content-digest primitive
  (`Repository::path_state`) rather than git blob identity, to avoid adding
  per-file git invocations that would violate the WI057 bounded-invocation
  guarantee. This is recorded as a decision, not an oversight; a future
  bounded git blob-identity API (e.g. one `git ls-tree -r` call folded into
  `RepositoryTopology::build`) remains open for a later phase if profiling
  justifies it.

## Executive conclusion

RepoPact already has the right foundation for a durable repository-orientation system, but the existing WI054 graph is intentionally limited to RepoPact governance relationships. WI063 should extend the canonical Rust graph/query stack with a second, derived repository-orientation domain rather than create a new service, external database, or model-specific memory layer.

The recommended architecture is:

```text
                           authoritative sources

          source / manifests / Git      RepoPact governance records
                    |                              |
                    +---------------+--------------+
                                    |
                                    v
                         RepositorySession snapshot
                                    |
              +---------------------+---------------------+
              |                                           |
              v                                           v
   topology / semantic adapters                existing WI054 governance graph
              |                                           |
              +---------------------+---------------------+
                                    |
                                    v
                         canonical Repository Graph
                                    |
                  +-----------------+------------------+
                  |                                    |
                  v                                    v
        durable derived baseline              working-tree overlay
                  |                                    |
                  +-----------------+------------------+
                                    |
                                    v
                         bounded query interface
                   /          |          |           \
                  v           v          v            v
                agent        CLI       Workbench    automation
```

The durable graph is a materialized projection, not a source of truth. The core correctness rule should be that a clean full rebuild and a supported incremental update converge to the same canonical graph for the same repository state.

## Current architecture facts

### 1. WI054 already owns the canonical governance relationship graph

`rust/crates/repopact-graph/src/lib.rs` currently defines a deterministic `RepositoryGraph` over the `RepositorySnapshot` and models governance-oriented node types including:

- repository;
- work item;
- acceptance criterion;
- evidence run;
- scope;
- role;
- decision;
- policy;
- contract;
- invariant;
- frozen surface;
- audit finding.

Its current edge types include relationships such as dependency/reverse dependency, containment, evidence support, ownership, affected scopes, supersession, concern, constraints, intersections, applicability, and role allowance.

This graph is source-backed and explicitly avoids promoting arbitrary text mentions into authoritative edges. That principle should be preserved.

### 2. WI054 already established the correct read model

WI054 introduced `RepositorySession` plus immutable snapshot/index semantics so validation, graph, analysis, and mutation planning do not each crawl the filesystem independently.

WI063 should consume that same snapshot model. It should not create a second repository crawler that races the canonical session or disagrees about ignored paths, linked worktrees, record-relative references, or repository identity.

### 3. WI056 made the Rust core canonical for proven semantic surfaces

Decision 0042 and WI056 establish one semantic engine rather than permanent Rust/Python dual authority. Python is a compatibility client for migrated surfaces. The Workbench calls the Rust crates directly.

The ROG therefore belongs in the canonical Rust engine and language-neutral protocol. Reimplementing repository orientation in Python or TypeScript would recreate the semantic split WI056 just removed.

### 4. The Workbench is already a real graph consumer

The Tauri 2 Workbench is a thin user-facing client over the Rust repository model and graph/analysis surfaces. WI058 also established that the UI must remain usable on wide and compact form factors and cannot depend on hover or right-click for essential behavior.

The new graph should therefore expose useful typed query projections rather than make the UI calculate semantic relationships itself.

### 5. There is currently no authorized persistent repository-local runtime database

WI054 explicitly rejected introducing a persistent `.repopact` transaction database, daemon, or hidden journal as an implementation detail without a separate durable architecture decision.

WI063 is the appropriate separate architecture work item to decide a *derived graph representation*, but the distinction matters. A durable graph index is not a transaction journal. Its representation must remain reconstructable from authoritative repository state.

### 6. Research metrics are now registered before graph implementation

H15/S8 governance-continuity work has already registered orientation-cost measures including tool calls, file reads, bytes read, and repository-wide grep/search operations. The benchmark protocol explicitly preserves B0 and R0 and permits a future graph-enabled condition only through a new dated amendment before graph runs.

That gives WI063 an unusually clean evaluation opportunity. The graph can be tested against metrics chosen before implementation.

## Problem decomposition

The orientation problem has four distinct parts and they should not be collapsed into one undifferentiated graph.

### A. Physical topology

The system needs a cheap, deterministic map of repository structure: files, directories, packages, workspaces, modules, configs, generated boundaries, nested repositories, and key entry points.

This layer should be available even when no rich language parser exists.

### B. Code semantics

Language adapters should add definitions, imports, exports, type/implementation relations, selected references or calls, and test symbols where those relations can be extracted with known quality.

The system should never claim whole-program semantic completeness merely because syntax parsing succeeded.

### C. Build/test/operational topology

Package manifests, Cargo metadata, pyproject metadata, package.json/tsconfig, CI workflows, build scripts, installers, runtime entry points, and test structure often answer orientation questions better than raw source parsing alone.

This should be a first-class layer, not an afterthought.

### D. Governance overlay

The existing WI054 graph should connect to source/topology nodes so an agent can ask what scope, contract, invariant, frozen rule, work item, decision, evidence, or finding applies to a code region.

This is where RepoPact has a unique advantage over ordinary code indexes.

## Recommended state model

Use three distinct states:

```text
Gc = durable committed graph baseline
Dw = local dirty working-tree overlay
Q  = optional disposable query acceleration index
```

The effective working graph is conceptually:

```text
Gw = apply(Gc, Dw)
```

`Q` may index `Gc` or `Gw` for performance, but it is never authority.

This separation avoids two bad designs:

1. committing a binary database that rewrites constantly and merges poorly;
2. keeping the useful graph only in a local cache, which would destroy clean-clone continuity.

## Durable representation recommendation

The exact path and schema should be fixed in a recorded decision, but the preferred class of format is:

```text
manifest.json
nodes/<stable-shard>.jsonl
edges/<stable-shard>.jsonl
```

with canonical sorting and stable partitioning.

Reasons:

- text-friendly and inspectable;
- deterministic rebuild is testable;
- content hashes can detect corruption;
- shards reduce merge/diff churn;
- local SQLite can be rebuilt from it;
- alternate implementations can consume the format;
- Git can replicate it with the repository.

A single huge JSON file would be simple but would create unnecessary rewrite and merge churn on large repositories. A committed SQLite file would query well but would be poor as the only reviewable, mergeable, deterministic source representation.

## Source fingerprint recommendation

Do not bind the graph to the raw Git tree hash because the graph files themselves would contribute to that tree and create a circular dependency.

Define a canonical source projection `P(repo)` that excludes:

- durable ROG output itself;
- local caches;
- `.git` internals;
- explicitly ignored/generated/vendor areas according to policy;
- any other derived artifact intentionally excluded from graph input.

Then compute a deterministic fingerprint over sorted repository-relative input identities, preferably Git blob IDs where available and content hashes otherwise.

This allows a clean clone to verify that the committed graph corresponds to the committed source projection without rereading all source bytes.

## Freshness model recommendation

Graph queries should return a freshness state with their data.

A useful initial state machine is:

```text
fresh
working_overlay
partial
stale
unsupported
corrupt
```

`partial` and `stale` are deliberately different. Partial means the graph truthfully lacks coverage for some regions or relation classes. Stale means previously covered source changed without a corresponding graph update.

A client must not convert either state into an implicit clean success.

## Incremental update invariant

The most important implementation invariant is:

```text
canonicalize(incremental_update(G0, delta))
    ==
canonicalize(full_build(source_after_delta))
```

This should be tested across representative change classes.

Git diff/blob identity can drive committed-state invalidation. The existing Workbench watcher can drive dirty working-tree invalidation. Parser-specific dependency information can narrow the recomputation neighborhood further.

Do not optimize incremental behavior until the full-build canonical form is stable enough to serve as the oracle.

## Parser recommendation

Tree-sitter is the best default candidate for broad syntax coverage and incremental parsing, but it should be treated as an implementation choice to validate, not a dogma.

Use the cheapest deterministic source that can accurately establish a relationship:

- filesystem/Git for topology;
- Cargo metadata for crate/workspace/dependency facts;
- pyproject/package metadata for Python packaging;
- package.json/tsconfig/workspaces for JS/TS package structure;
- CI/build manifests for operational topology;
- Tree-sitter or language-specific parsers for symbols/imports and selected references;
- specialized language tooling only when the fidelity gain justifies packaging and runtime cost.

The normalized graph schema should hide adapter-specific implementation details from clients.

## Provenance recommendation

Do not use a single vague `confidence` field for everything.

Prefer an explicit derivation/provenance class with optional confidence only for relations that are genuinely heuristic.

Examples:

```text
canonical_record
filesystem
manifest
parser
build_metadata
heuristic
inferred
```

Where RepoPact's existing provenance vocabulary is semantically appropriate, reuse it instead of inventing a conflicting certainty system. The final decision should specify the mapping.

Every edge should be able to point back to the source fact that produced it.

## Agent API recommendation

The graph should be designed around orientation questions, not visualization.

Minimum semantic operations:

```text
status
resolve
context/orient
neighbors
path
dependencies
dependents
impact
tests
governance
```

A bounded `orient(target)` response is especially important. It should return a small high-value neighborhood and source-backed reading list rather than dumping thousands of graph edges into the context window.

The output should support depth, edge filters, pagination/cursors, and output size/token limits.

## Workbench recommendation

Reuse the existing Graph section and typed Rust DTO pattern.

Do not make a force-directed canvas a blocker. For actual repository work, filtered tables, trees, lists, relationship groups, impact panels, test panels, governance overlays, and source navigation are likely more useful and accessible.

The UI must surface graph freshness and coverage prominently enough that the operator knows when an answer is partial or stale.

## Adoption recommendation

Build the initial graph during `repopact adopt` after governance records have been reconstructed, because that is the moment RepoPact is already paying repository-discovery cost and can immediately connect topology to governance.

However, graph capability should remain versioned and explicitly disableable. A repository with unsupported languages or policy constraints should not become invalid merely because it cannot or does not want to persist the optional graph capability.

Existing adopters should receive an explicit backfill operation rather than a hidden large write during an unrelated `doctor` command unless a later decision deliberately makes graph creation part of doctor migration.

## Privacy and security recommendation

The durable graph should store metadata, not source duplication.

Default exclusions should cover build outputs, package caches, virtual environments, node_modules/target-like trees, ignored secret files, binaries, and external symlink targets.

Parser and graph builders must treat the repository as hostile input. Tests should include malformed syntax, giant files, symlink escape attempts, malicious graph files, extreme graph fan-out, and corrupted shards.

No external service is required for core graph generation.

## Merge and version-control recommendation

A graph conflict is a derived-artifact conflict. Source merge resolution wins.

Provide deterministic regeneration so developers do not need to manually reason about JSONL edge conflicts. Stable sharding should keep unrelated changes from touching the same files whenever practical.

If the durable graph is stale after a merge, validation/status should say so directly and provide the canonical repair command.

## Backward compatibility recommendation

Do not make the existence of ROG mandatory for every RepoPact 3.x repository.

Use an explicit capability contract:

```text
ROG disabled/absent -> repository can remain valid
ROG declared enabled -> graph manifest/freshness/validation is binding
```

A later major/versioned decision may change defaults for newly adopted repositories after migration behavior is proven.

## Evaluation recommendation

Preserve the already-registered S8 B0 and R0 conditions. Before graph-enabled runs, add R1 by dated amendment.

The most informative R1 outputs are likely:

- governance recovery precision/recall;
- seeded violation recall;
- false-clean rate;
- orientation time;
- tool calls;
- file reads;
- broad grep/search operations;
- bytes read;
- input tokens;
- human intervention.

The graph should count as useful only if it reduces orientation cost without lowering correctness.

## Primary risks

| Risk | Failure mode | Required mitigation |
| --- | --- | --- |
| stale graph | agent trusts obsolete structure | source fingerprint, explicit freshness, fail-visible queries |
| graph bloat | large diffs/storage cost | exclusions, stable sharding, coverage tiers, benchmarks |
| false semantic precision | syntax edge presented as whole-program truth | per-relation coverage and provenance |
| duplicate authority | graph conflicts with RepoPact records | governance overlay reuses WI054 sources, graph cannot grant authority |
| merge churn | derived shards conflict constantly | deterministic rebuild, stable partitioning |
| parser attack | malformed repo consumes memory/time | bounded adapters, cancellation, failure isolation |
| privacy leak | graph captures secrets or source bodies | metadata-only core, ignore/exclusion policy |
| client divergence | UI/CLI/agent compute different relationships | one canonical Rust graph/query engine |
| hidden local dependency | graph only works on original machine | durable portable representation and clean-clone tests |
| premature benchmark tuning | metrics chosen after implementation | S8 metrics already registered, R1 amendment before runs |

## What must not happen during implementation

- Do not add a second Python or TypeScript semantic graph engine.
- Do not make an LLM-generated summary the durable graph.
- Do not commit a local cache database and call it the canonical format without a new decision.
- Do not let parser output grant governance authority.
- Do not hide unsupported regions.
- Do not make graph freshness a UI-only concern.
- Do not require broad repository grep as the graph query implementation.
- Do not destabilize RepoPact's imminent public-release path merely to land this feature early.

## Activation recommendation

Keep WI063 proposed until the release-parity UI work and public-release hardening reach the agreed release gate.

When activated, refresh this review against the then-current Rust core, name the coding agent, and produce one implementation plan across the graph crate, repository/session model, protocol, adoption, Workbench, validation, docs, and research boundary before coding begins.

## Final architecture judgment

The concept is a strong fit for RepoPact because it extends the same portability idea from governance state to repository orientation while keeping the repository itself as the rendezvous point.

The architecture is viable if three boundaries remain strict:

1. the graph is derived and freshness-aware, never sovereign;
2. one canonical Rust graph/query engine serves all clients;
3. durable graph usefulness is measured against pre-registered clean-clone orientation metrics rather than asserted from intuition.

Under those constraints, WI063 can turn RepoPact from a system that tells a worker *what is governed* into one that can also tell the worker *where to look and what the surrounding change surface is*, without making every new agent rediscover the repository by brute force.
