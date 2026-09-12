# Work Item 063 - Durable Repository Orientation Graph and Incremental Semantic Index

**Status:** Proposed

**Owner:** work-coordinator

**Affected scopes:** work, tooling, governance, docs, evidence

**Depends on:** WI054, WI056, WI058

## Intent

Build a durable, repository-native **Repository Orientation Graph (ROG)** that lets a fresh human or agent understand where things are, how they relate, what depends on them, where their tests live, and what governance applies without beginning every session with broad filesystem walks and repeated `grep`/`rg` discovery.

The ROG is a derived index of the repository. It is not a new source of truth. Source code and repository metadata remain authoritative for software structure. RepoPact records remain authoritative for governance. The ROG materializes relationships that can be deterministically derived from those sources and carries enough provenance and freshness information for a client to explain where each relationship came from.

The intended end state is that a clean clone can answer questions such as:

```text
What is this file, crate, package, module, type, or symbol?
What contains it?
What does it depend on?
What depends on it?
Which tests cover or target it?
Which build, packaging, CI, or runtime surfaces reference it?
Which RepoPact scope, contract, invariant, frozen surface, work item, decision, or finding applies?
What is the likely change radius if I modify it?
What should I read first before working here?
```

without requiring the receiving worker to rediscover the repository from scratch.

## Problem statement

RepoPact now carries durable intent, authority, lifecycle, evidence, provenance, decisions, and validation state across sessions. That solves a large part of governance continuity, but a new worker can still spend substantial effort reconstructing the *shape of the codebase*.

Typical orientation today still looks like:

```text
find / glob
rg symbol
open manifest
rg imports
rg callers
find tests
open nested AGENTS.md
search decisions
search work items
repeat until the worker believes it understands enough
```

That process is expensive in wall-clock time, tool calls, file reads, tokens, and attention. It is also repeated by every new agent, every context reset, and often every machine change.

The repository already contains most of the information needed to build a better orientation substrate. RepoPact already has a canonical Rust repository snapshot, governance relationship graph, deterministic analysis layer, Workbench, and a clean-clone governance-continuity research program. WI063 extends those foundations into a durable orientation index instead of creating a separate search service or model-specific memory system.

## Relationship to WI054

WI054 delivered `repopact-graph` for canonical governance relationships. Its graph currently models nodes such as repositories, work items, acceptance criteria, evidence runs, scopes, roles, decisions, policies, contracts, invariants, frozen surfaces, and findings. It deliberately refuses to treat arbitrary Markdown mentions as authoritative edges.

WI063 does **not** replace that graph and must not create a competing graph authority.

The intended architecture is one canonical graph/query domain with distinguishable layers:

```text
Repository Orientation Graph

  physical topology
       +
  code semantics
       +
  build/test/operational topology
       +
  existing RepoPact governance relationships
       |
       v
  bounded orientation / impact / context queries
```

Governance edges remain derived from governed RepoPact records. Code and topology edges are derived from source structure, parser output, manifests, build metadata, and other deterministic repository facts. The graph may connect the two layers, for example by showing that a source path falls under a scope or nested contract, but it may not infer new authorization from code structure.

## Architectural thesis

### 1. Derived, never sovereign

The ROG is a materialized projection. It can accelerate orientation, impact analysis, and context selection, but it cannot overrule source code, manifests, Git state, RepoPact records, validation, or operator authority.

If graph state conflicts with the authoritative source, the graph is stale or wrong and must be rebuilt or repaired.

A graph edge is never sufficient by itself to:

- activate or complete work;
- waive acceptance criteria;
- approve a frozen-surface mutation;
- change ownership;
- create a binding invariant;
- grant tool or execution authority;
- prove that an implementation is correct.

### 2. Durable baseline, dynamic working overlay

The graph must be both durable and dynamic.

A committed repository should carry a deterministic graph baseline tied to a canonical source fingerprint. A local working session may layer uncommitted changes over that baseline using the existing repository watcher/session model.

Conceptually:

```text
committed source projection
        |
        v
 durable graph baseline  Gc
        |
        +---- working-tree delta ----> local graph overlay Dw
                                      |
                                      v
                              effective graph Gw
```

The durable representation travels with the repository. The working overlay may be ephemeral because uncommitted filesystem state is itself local. Once changes become durable, canonical graph regeneration updates the repository representation.

### 3. Deterministic core first

The authoritative graph core must be generated from deterministic repository facts. Language parsers, package manifests, build metadata, Git metadata, RepoPact records, and explicit configuration are acceptable sources.

An LLM may later propose or explain relationships, but model output is never silently promoted to a concrete graph edge. Any heuristic or inferred relation must remain explicitly typed as such and cannot become an enforcement fact merely because a model asserted it.

### 4. Provenance and explanation on every relationship

A useful graph is not just `A -> B`. Clients need to know *why* the edge exists.

Nodes and edges should carry enough metadata to identify at least:

- stable graph identity;
- node/edge kind;
- repository-relative source path;
- source span or symbol location where available;
- derivation mechanism;
- provenance or certainty class;
- graph schema/generator version;
- content or source fingerprint needed for freshness;
- optional coverage or confidence metadata for non-concrete relationships.

Graph queries must be able to return source-backed explanations rather than opaque relevance scores alone.

### 5. No committed binary database as the sole durable form

A local database may be the right acceleration structure for graph queries, but it should not be the only durable representation committed to Git.

The durable representation should be deterministic, portable, versionable, rebuildable, and reviewable. Candidate formats include canonically sorted JSON/JSONL shards plus a manifest and content hashes.

A SQLite, sled, RocksDB, or other local index may be generated under `.git/`, an OS cache directory, or another explicitly non-authoritative location for fast lookup. It must be disposable and reconstructable from the repository representation.

If implementation evidence shows that a different durable format is materially superior, that change requires a recorded architecture decision and must preserve portability, deterministic rebuild, corruption detection, and clean-clone usability.

## Target graph model

### Layer A: physical repository topology

Minimum node classes should include, where applicable:

- repository;
- directory;
- file;
- workspace;
- package/crate/module;
- configuration file;
- generated artifact classification;
- vendored/external subtree classification;
- submodule or nested repository boundary.

Useful edges include:

```text
contains
belongs_to_workspace
belongs_to_package
configured_by
generated_from
```

The physical layer should give an agent an immediate map without requiring a full text search.

### Layer B: code semantics

Language adapters may add:

- module/namespace;
- function/method;
- type/class/struct/enum;
- trait/interface/protocol;
- implementation block;
- exported/public symbol;
- test symbol;
- executable or runtime entry point.

Useful relationships include:

```text
defines
imports
exports
implements
references
calls
extends
uses_type
```

Not every language can provide every relationship with equal certainty. The graph must distinguish exact parser/build-system facts from lower-certainty semantic relationships. It is better to expose a coverage gap than to invent an edge.

### Layer C: build, test, packaging, and operational topology

The graph should ingest deterministic project metadata where available:

- Cargo workspace/package/dependency metadata;
- `pyproject.toml` and Python package/test structure;
- `package.json`, workspaces, tsconfig, and frontend entry points;
- CI workflows;
- build scripts;
- installer/package surfaces;
- test targets and fixtures;
- generated-code boundaries;
- runtime/application entry points;
- other ecosystem manifests through adapters.

Useful edges include:

```text
depends_on
builds
packages
tests
configured_by
invokes
produces
consumes
```

### Layer D: RepoPact governance overlay

Reuse the WI054 graph rather than reimplementing it.

The orientation graph should be able to answer governance questions for code/topology nodes, including:

- applicable owner/scope;
- nested contract applicability;
- frozen-surface intersection;
- binding invariants that structurally apply;
- active/proposed work affecting a path or scope;
- relevant decisions when an explicit relationship exists;
- findings concerning a path/scope;
- evidence supporting related completed work where the relation is explicit.

Useful cross-layer edges include:

```text
owned_by
governed_by
constrained_by
affected_by
concerns
covered_by_contract
intersects_frozen_surface
```

A cross-layer edge must preserve the source rule that caused it. For example, a path-to-scope edge should point back to the owner-map path/glob that matched it.

## Stable identities

Graph node identity must be deterministic and independent of local absolute paths.

For filesystem nodes, repository-relative canonical paths are appropriate identity inputs. For symbol nodes, use a deterministic language-qualified symbol identity such as repository-relative path plus namespace/symbol path and symbol kind, not line number alone.

Line/column spans are useful source locations but are too unstable to be primary identity.

Rename continuity may be added later as an inferred relationship, but WI063 must not pretend that rename detection creates permanent object identity when the source system does not guarantee one.

## Durable representation

A durable graph snapshot should include a small canonical manifest and partitioned node/edge data.

A representative shape is:

```text
<governed graph directory>/
    manifest.json
    nodes/
        <stable shard>.jsonl
    edges/
        <stable shard>.jsonl
```

The exact repository path and sharding rule must be recorded in a durable decision before implementation becomes canonical.

The manifest should identify at least:

```text
graph_schema_version
generator_version
RepoPact/version compatibility
source_projection_fingerprint
generated_at or generation identity when deterministic policy permits
node_count
edge_count
coverage by adapter/language
shard hashes
excluded-path policy
freshness/status information
```

Timestamps must not make otherwise identical graph rebuilds byte-different unless they are intentionally excluded from fixpoint comparison or represented in a separate non-canonical record.

## Source fingerprint and freshness

The graph cannot fingerprint the entire Git tree including itself because that creates a circular dependency. Define a canonical **source projection** that excludes the graph's own derived files and other explicitly excluded generated/cache locations.

The source fingerprint should be computed over stable repository facts such as sorted repository-relative path plus Git blob/content identity for included inputs.

When Git object identities are available, freshness verification should use them rather than rereading every source file merely to prove that the graph still corresponds to the same committed source projection.

The graph should expose explicit freshness states such as:

```text
fresh
working_overlay
partial
stale
unsupported
corrupt
```

Clients must not silently serve a stale graph as if it were current.

## Full-build determinism and incremental equivalence

The core correctness invariant is:

```text
G_incremental(s0 -> s1) == G_full(s1)
```

for every supported change class after canonical normalization.

This must be tested, not assumed.

The same source projection and graph schema version should produce byte-equivalent canonical durable graph output across repeated builds and supported platforms, subject only to explicitly documented platform-normalization rules.

Incremental update may use Git diffs, changed blob identities, and the existing repository watcher to invalidate affected graph neighborhoods rather than reparsing the entire repository.

The implementation must prove that incremental update converges to the same result as a clean full rebuild.

## Working-tree dynamics

For an open `RepositorySession`, the existing watcher can identify changed files. The ROG should update or invalidate only the affected local graph regions where possible.

Working-tree graph state is not automatically durable. It is an overlay over the last verified baseline.

Queries must disclose when their answer includes dirty working-tree state.

If a parser fails on an in-progress edit, clients should receive a partial/stale diagnostic for that region rather than an invented clean graph.

## Adoption and bootstrap

`repopact adopt` is the natural bootstrap point because it already discovers repository structure and imports governance signals.

The intended sequence is:

```text
repository discovery
    -> governance adoption
    -> orientation source discovery
    -> graph build
    -> graph validation
    -> durable graph materialization
    -> orientation summary
    -> final RepoPact validation
```

Graph construction must not fabricate governance facts to make adoption succeed.

For unsupported languages or bounded parser failures, adoption may remain valid with explicit graph coverage gaps if the selected capability contract allows partial ROG coverage. A partially covered graph must say that it is partial.

The implementation must define how users opt out when repository size, confidentiality policy, generated content, or unsupported tooling makes durable indexing undesirable. Opt-out must be explicit and must not cause RepoPact to claim graph-backed orientation is available.

## Existing adopter backfill

Repos already governed by RepoPact need a deterministic migration path. Candidates include:

```text
repopact graph build
repopact graph status
repopact doctor --fix   # only if graph backfill belongs in doctor semantics
```

The chosen behavior must be recorded. `doctor` must not silently create a large new persistent artifact unless that is an explicit versioned migration contract.

## Clean-clone behavior

A principal success case is:

1. clone a graph-enabled repository;
2. verify graph schema/version and source fingerprint;
3. load the durable graph representation or rebuild a local acceleration cache;
4. answer orientation queries without reparsing or grepping the entire repository first.

The receiving client may read source files that are directly relevant to the task. The goal is not zero source reads. The goal is to replace blind broad discovery with targeted, graph-guided reads.

## Language adapter architecture

The graph core should be language-neutral. Language-specific extraction belongs behind adapter interfaces.

A likely stack is:

```text
filesystem/Git topology
       +
manifest/build adapters
       +
Tree-sitter or equivalent incremental parsers
       +
optional higher-fidelity language adapters where justified
       |
       v
normalized ROG nodes and edges
```

Tree-sitter is a strong candidate because it supports many languages and incremental parsing, but the work item does not mandate it if executable evidence supports a better bounded architecture.

Initial implementation should cover the languages needed to orient RepoPact itself, including at least Rust, Python, and TypeScript/JavaScript source structure plus the repository's relevant JSON/TOML/YAML/Markdown and manifest metadata. Support must be stated per adapter and relation. "Language supported" may not imply a complete call graph if only definitions/imports are available.

## Coverage reporting

The graph must report what it knows and what it does not.

Examples:

```text
Rust: packages, modules, definitions, imports, Cargo deps, selected references
Python: packages, modules, definitions, imports, pyproject metadata
TypeScript: modules, exports/imports, definitions, package/tsconfig metadata
C++: physical/build topology only (example if semantic adapter is absent)
```

Coverage is a first-class output. Unknown or unsupported regions are not silently omitted from a supposedly complete graph.

## Exclusions and repository boundaries

The builder must respect repository boundaries and explicit policy.

At minimum:

- do not follow symlinks outside the repository by default;
- do not traverse `.git` internals as source;
- honor RepoPact ignored-directory rules and graph-specific exclusions;
- classify submodules/nested repositories instead of blindly absorbing them;
- bound vendored/generated/dependency trees and make their inclusion policy explicit;
- avoid build outputs, package caches, virtual environments, `node_modules`, target directories, and equivalent high-churn/generated trees by default unless explicitly configured;
- do not treat ignored secret files as graph input merely because they exist locally.

## Privacy and data minimization

The durable graph should primarily store topology and semantic metadata, not duplicate source bodies.

Do not persist source file contents, secret values, arbitrary string literals, full comments, or model-generated summaries in the core durable graph unless a later explicit feature and privacy decision authorizes them.

Paths and symbol names can themselves be sensitive, but a graph committed to a private repository has the same repository visibility boundary. Export, telemetry, and benchmark artifacts must still avoid leaking private graph content.

No cloud indexing service, embedding provider, or external model call is required for the deterministic ROG core.

## Parser safety and resource limits

Repository content is untrusted input. Language adapters must have bounded behavior for pathological files.

Define and test limits for:

- maximum file size or parse policy;
- binary detection;
- generated/minified files;
- parse timeout or cancellation;
- recursion/depth limits where needed;
- malformed syntax;
- path normalization;
- symbolic-link and nested-repository boundaries;
- memory use on large repositories.

A parser failure should degrade coverage explicitly. It must not crash adoption, corrupt the durable graph, or invent edges.

## Local acceleration index

A local SQLite or equivalent index is allowed and likely useful for fast neighborhood/path/reverse-edge queries.

It is explicitly **non-authoritative**:

```text
durable graph files -> local query index
```

not:

```text
local query index -> truth
```

Deleting the local cache must never lose durable repository state. A fresh client must be able to rebuild it from the committed graph representation.

## Agent query surface

The primary product value is an agent API, not a decorative graph visualization.

The canonical engine should expose bounded, typed operations equivalent to:

```text
graph.status()
graph.resolve(target)
graph.context(target)
graph.neighbors(node, depth, edge_kinds)
graph.path(from, to)
graph.dependents(target)
graph.dependencies(target)
graph.impact(target)
graph.tests(target)
graph.governance(target)
graph.orientation(scope_or_target)
```

Queries should return stable identifiers, relation kinds, source references, freshness, and derivation/provenance metadata.

### Orientation query

`orientation` should produce a bounded starting map, not the entire graph.

For a target work item, path, package, or symbol, a useful response can include:

```text
identity and kind
source path / package / workspace
parents and key children
direct dependencies and dependents
relevant tests
build/runtime/package surfaces
applicable RepoPact scope and contracts
frozen/invariant intersections
related active work where structurally known
important decisions/findings where explicitly linked
coverage/freshness warnings
recommended source files to read first
```

Recommendations must distinguish deterministic facts from heuristics.

### Impact query

Impact analysis should be conservative and explainable. It may report direct/reverse graph neighborhoods and affected build/test/governance surfaces. It must not claim that the graph proves complete semantic impact in languages or relationships where coverage is incomplete.

### Test query

`tests(target)` should use explicit test metadata and derivable relationships. Filename naming conventions may be used as lower-certainty heuristics only when labeled accordingly.

## Bounded context delivery

Graph queries exist partly to reduce context waste. They should support limits such as:

- maximum nodes/edges;
- maximum depth;
- relation allow/deny lists;
- output byte/token budget;
- summary/detail mode;
- source-reference-only mode.

Truncation must be explicit and resumable. A client should be able to ask for another page/neighborhood rather than receive a silent incomplete answer.

## CLI and language-neutral protocol

The public surface should provide practical commands such as:

```text
repopact graph build
repopact graph status
repopact graph verify
repopact graph context <target>
repopact graph impact <target>
repopact graph tests <target>
repopact graph governance <target>
repopact graph orient <target>
```

Exact syntax may differ after live CLI review.

The versioned Rust engine protocol should expose the same semantic operations so agents, Python compatibility tooling, Workbench, and future clients do not scrape human CLI output.

No client may gain arbitrary filesystem-write authority merely because it can query the graph.

## Workbench integration

RepoPact Workbench already has graph-oriented views backed by WI054. WI063 should expand those views to the orientation model without requiring a giant force-directed spiderweb.

The Workbench should prioritize useful operator questions:

- What am I looking at?
- What depends on this?
- What will this touch?
- Where are the tests?
- What governance applies?
- Is the graph current?
- Which regions are unsupported or partial?

Useful UI capabilities include:

- layer and relationship filters;
- searchable node resolution;
- list/table/tree views;
- bounded neighborhood drill-in;
- source-location navigation;
- freshness/coverage badges;
- impact and test panels;
- governance overlay;
- rebuild/verify controls where authorized.

A graphical node canvas is optional. It is not an acceptance requirement unless later evidence shows it materially improves orientation.

The compact/mobile Workbench must not depend on hover or right-click for essential graph operations.

## Cross-platform behavior

The durable representation and Rust query semantics should be platform-independent. Path normalization must avoid Windows/Linux/macOS disagreement over separators, case handling, and absolute paths.

Android/iOS may have repository-access constraints distinct from desktop. WI063 must compose with WI061's eventual repository-acquisition decision rather than assume a system Git binary or unrestricted path access on mobile.

Unsupported client/platform combinations should fail or report capability gaps explicitly rather than inventing a different graph model.

## Branch, merge, and conflict behavior

The graph is derived, so source merge conflict resolution remains authoritative.

The work item must define what happens when two branches modify source and graph shards concurrently. Preferred behavior is deterministic regeneration, not hand-merging semantic graph truth.

Graph sharding should reduce unrelated merge churn. Content-addressed or stable partitioning should be evaluated.

If graph files conflict after a source merge, tooling should be able to regenerate canonical output from the merged source projection.

## Schema and compatibility

The durable graph needs an explicit schema version independent enough to support migration without pretending every RepoPact governance-schema change is a graph-format change.

Unknown graph major versions should not be interpreted optimistically.

The capability contract must define whether a RepoPact repository without a graph remains conformant. The default recommendation for WI063 is:

- existing and intentionally graph-disabled repositories remain valid RepoPact repositories;
- if a repository declares the ROG capability enabled, the durable graph manifest, freshness rules, and validation requirements become binding;
- new `adopt` behavior may enable the capability by default only after the migration/version contract is explicitly recorded.

This prevents WI063 from retroactively invalidating every existing adopter merely because a new optional orientation capability exists.

## Graph validation and conformance

Validation must cover more than JSON shape.

At minimum test:

- manifest and shard schema validity;
- shard hashes;
- duplicate node IDs;
- edges to missing nodes;
- canonical sort/serialization;
- source fingerprint match;
- graph self-exclusion from fingerprint;
- deterministic full rebuild;
- incremental/full equivalence;
- clean-clone load;
- corruption detection;
- stale graph detection;
- partial coverage reporting;
- unsupported language behavior;
- path normalization across supported OS families;
- symlink/nested repository boundaries;
- graph-disabled repository compatibility.

Where appropriate, add conformance fixtures so alternate implementations can reproduce observable ROG behavior without copying internal code.

## Performance and scale evidence

Do not choose performance claims from intuition.

Closeout must report, on documented fixtures and hardware:

- initial full graph build time;
- incremental update time by change class;
- peak memory;
- durable graph size;
- local cache size;
- node/edge counts;
- clean-clone verification/load time;
- query latency for representative orientation/impact/reverse-edge queries;
- parser coverage;
- graph size relative to source repository size;
- rebuild cost after branch/merge change.

Use at least RepoPact itself plus synthetic or real larger fixtures. Any size or performance guard adopted as a product limit must be justified by measurements and recorded rather than invented after implementation.

## Research and S8

The governance-continuity research pass intentionally registered orientation-cost metrics *before* WI063 implementation. S8 currently measures:

- orientation wall-clock time;
- input/output tokens;
- tool calls;
- file reads;
- repository-wide text-search/grep operations;
- bytes read before an accepted orientation answer;
- human interventions;
- governance-field recovery and violation recovery.

S8 currently has baseline conditions B0 and R0:

```text
B0 = reasonable convention baseline
R0 = RepoPact without the future orientation graph
```

Before any graph-enabled result is collected, add a dated benchmark-protocol amendment defining an **R1 graph-enabled condition** without rewriting B0 or R0.

The core comparison is:

```text
B0  ordinary repository conventions
R0  RepoPact governance, no ROG
R1  RepoPact governance + durable ROG
```

R1 is successful only if it preserves or improves correctness while reducing blind orientation work. Fewer grep calls with worse understanding is not a win.

The graph implementation must not tune its evaluation metric after seeing results. The existing registered metrics remain primary.

Do not update the archival paper to claim graph benefits before implementation evidence and R1 results exist. It may describe WI063 as prospective future work until then.

## Security boundary

The ROG is a read/orientation capability unless a separate governed mutation path is invoked.

Graph queries must not bypass:

- WI054 typed mutation plan/apply rules;
- frozen-surface approval;
- WI050 admission/enforcement semantics;
- Tauri capability/IPC boundaries;
- repository path/symlink restrictions;
- existing evidence/provenance rules.

Graph-derived impact information may *inform* a mutation plan. It does not authorize the plan.

A malicious repository may attempt parser bombs, path traversal, graph-file forgery, extreme fan-out, or adversarial metadata. Those classes need explicit tests.

## Rollout plan

### Phase 0: durable architecture decision

- confirm repository path, graph schema, sharding, fingerprint, enablement, and cache boundaries;
- record the decision before a persistent graph format becomes canonical;
- refresh this architecture review if the Rust core changed materially since WI063 creation.

### Phase 1: deterministic topology core

- repository/file/directory/workspace/package/config nodes;
- generic containment and package/build topology;
- manifest, hashing, serialization, verification;
- integration with existing `RepositorySession` snapshot semantics.

### Phase 2: semantic language adapters

- normalized adapter API;
- initial Rust, Python, TypeScript/JavaScript coverage;
- relevant manifest/config extraction;
- explicit coverage reporting;
- parser failure isolation.

### Phase 3: governance overlay

- bridge WI054 governance nodes/edges to repository topology;
- path-to-scope/contract/frozen/invariant applicability where deterministically derivable;
- no duplicate governance authority.

### Phase 4: incremental update and local query acceleration

- Git/blob-driven invalidation;
- watcher-driven dirty overlay;
- local disposable query index;
- prove incremental/full equivalence.

### Phase 5: agent and CLI query APIs

- status, resolve, context, orientation, dependencies, dependents, impact, tests, governance;
- versioned engine protocol;
- bounded output and pagination.

### Phase 6: Workbench and adoption lifecycle

- operator orientation views;
- freshness/coverage controls;
- `adopt` bootstrap and existing-adopter backfill;
- cross-platform validation.

### Phase 7: research evaluation and closeout

- pre-register S8 R1 before graph runs;
- collect orientation-cost and correctness evidence;
- performance/scale runs;
- documentation and final decision reconciliation.

## Explicit non-goals

WI063 is **not** authorization to build:

- a model-generated knowledge graph as the canonical index;
- a cloud-hosted code index required for normal operation;
- an embeddings/vector database as a prerequisite;
- a replacement for source code, Git, language compilers, or IDE language servers;
- a perfect whole-program call graph for every language;
- a committed opaque binary database as the only durable form;
- a new mutation authority or generic patch API;
- a graph that silently grants ownership, approval, or completion authority;
- unrestricted indexing of ignored, secret, vendored, generated, or external files;
- a graph visualization project whose primary success criterion is visual appearance;
- WI050 admission/security migration;
- changes to WI061 mobile repository acquisition before that architecture is decided.

## Activation rule

This item is intentionally created as `proposed`. RepoPact public-release hardening remains the immediate priority.

Before WI063 becomes `active`:

1. identify the coding agent;
2. use Sol High to refresh the live architecture review if `main` has materially changed;
3. reconcile any release-time changes to `repopact-repository`, `repopact-graph`, `repopact-analysis`, the engine protocol, or Workbench;
4. record the durable graph-format/enablement decision;
5. confirm that the work will not destabilize the imminent public release path.

Implementation should then proceed from one consolidated architecture plan rather than a sequence of grep-driven symptom patches.

## Closeout evidence

WI063 may close only with durable evidence covering at least:

- exact graph schema/version and persistent paths;
- node and edge taxonomy;
- graph/source-of-truth boundary;
- deterministic full-build fixtures;
- incremental/full equivalence fixtures;
- source-fingerprint and stale/corrupt detection;
- graph-disabled compatibility;
- language and relationship coverage matrix;
- parser failure/resource-boundary tests;
- symlink, ignored path, generated/vendor, nested repo, and submodule tests;
- clean-clone orientation without full-repository reparsing as the first action;
- CLI and engine protocol examples;
- Workbench orientation/freshness/coverage behavior;
- Windows/Linux/macOS normalization tests where available, plus mobile capability handling consistent with WI061;
- local acceleration-cache deletion/rebuild proof;
- branch/merge/regeneration behavior;
- performance and storage measurements;
- security/adversarial graph tests;
- existing RepoPact conformance and regression suites;
- no regression to WI054 mutation authority or WI050 enforcement;
- S8 R1 amendment committed before R1 runs;
- R1 results or an explicit, evidence-backed reason why comparative graph evaluation remains deferred;
- documentation of unsupported languages and known coverage gaps.

The success condition is not "RepoPact can draw a graph." The success condition is that a fresh human or agent can use a durable, current, explainable repository map to orient and reason about change with materially less blind discovery, while all source and governance authority remains where RepoPact already says it belongs.
