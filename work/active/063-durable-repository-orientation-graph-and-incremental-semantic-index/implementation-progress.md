# WI063 Implementation Progress — Foundation Checkpoint (2026-09-13)

**Coding agent:** Claude Code
**Session type:** foundation checkpoint, not WI063 closeout
**Starting SHA:** `8642b91c09fa0bfbbf72375f23bafee6fdf8cf45`
**This checkpoint's final SHA:** see `git log` HEAD at time of the final commit in this session

## What this session proves

A deterministic Repository Orientation Graph foundation exists, extends
WI054's `repopact-graph` crate rather than competing with it, is durable,
structurally validated by the canonical Rust validator, cross-platform
byte-identical, and exposed through the canonical engine protocol and
public CLI. Nothing beyond the foundation phase was built.

### Decision

**Decision 0044** — "Durable Repository Orientation Graph projection and
freshness contract" (`decisions/0044-durable-repository-orientation-graph-projection-and-freshness-contract.md`).
Binds authority, durable path, schema/version, canonical serialization,
sharding, source projection, fingerprint, capability contract, freshness
states, cache boundary (defined, not built), privacy, and security.

### Durable format

- Path: `rog/manifest.json`, `rog/nodes/shard-NNNN.jsonl`, `rog/edges/shard-NNNN.jsonl`.
- Schema version: `1` (u32), unknown-major fails closed (tested).
- Sharding: SHA-256(key) mod 16, deterministic, independent of process/
  platform hash randomization (tested).
- Serialization: UTF-8, LF, forward-slash repository-relative paths,
  `BTreeMap`-only keyed collections (no `HashMap`), canonical per-shard
  sort order, no timestamps in the canonical surface.
- Write: build-to-temp-directory then atomic rename-swap; a previous valid
  graph is only replaced after the new one is fully written and hashed.

### Source projection and fingerprint

`repopact_graph::projection::SourceProjection`: walks
`Repository::files_under_with_topology` (reusing an already-open
snapshot's topology, never a fresh `all_files()` call — see the git-
invocation-bound fix below), excludes `rog/` (self-exclusion, tested),
adds `target` to the shared `repopact_repository::IGNORED_PARTS` list.
Fingerprint: `sha256(sorted("{path}\0{content_sha256}"))`, hex-encoded —
content hash, not Git blob identity (no bounded blob-identity API exists
yet; documented as a deferred future optimization in Decision 0044).

### Graph core

`GraphLayer` (governance/physical/semantic/build/test/runtime/package) and
`DerivationClass` (canonical_record/filesystem/manifest/parser/
build_metadata/heuristic/inferred) added to `GraphNode`/`GraphEdge` with
`#[serde(default)]`. Only `Governance`+`Physical` and
`CanonicalRecord`+`Filesystem`+`Manifest` are populated by any builder this
session — `heuristic`/`inferred` are never emitted.

**Node classes implemented:** `Directory`, `File`, `Workspace` (a directory
directly containing `Cargo.toml`/`pyproject.toml`/`package.json`),
`ConfigurationFile` (the manifest file itself), `NestedRepository` (a
directory directly containing its own `.git`, detected via one bounded
existence check per implied directory — never descended into). All
existing governance node classes are unchanged.

**Edge classes implemented:** physical `Contains` (directory/file
containment, derived from file-path ancestors, no second filesystem walk),
`BelongsToWorkspace` (file → nearest ancestor Workspace), `ConfiguredBy`
(directory → its manifest file), and the pre-existing `Intersects` kind
reused for a real cross-layer link: file → `FrozenSurface` node when the
file's repository-relative path matches a frozen-surface glob (a minimal
glob matcher covering exact paths and `<prefix>/**`, matching
`repopact/admission.py`'s existing frozen-surface semantics). Import/
export/call/dependency edges are explicitly **not implemented** — those
require semantic language adapters, which are out of scope this session.

### Repository/session integration

No second crawler. Physical topology is built entirely from
`RepositorySnapshot`'s already-open `RepositoryTopology` and one
`files_under_with_topology` walk. A genuine regression was caught and
fixed during this session: the first draft called
`Repository::all_files()`, which recomputes `RepositoryTopology` (fresh
Git invocations) on every call. This broke two pre-existing WI057
bounded-git-invocation tests in `repopact-desktop-api` before being
caught by the full workspace test suite. Fixed by threading the
already-open snapshot's topology through (`RepositoryGraph::build_with_fingerprint`),
restoring both tests to green and adding an equivalent new test in
`repopact-graph` itself (`randomized_git_invocation_count_is_still_bounded_after_graph_build`,
asserting `<= 4` git invocations, matching the existing WI057 bound).

### Validation

`repopact_graph::validate::validate_structure` checks: manifest presence/
parseability, schema major version, shard-hash match, duplicate node IDs,
dangling edge endpoints, canonical (sorted) ordering, and path safety (no
absolute paths, no `..` escapes) recorded in node/edge `source` fields.
Integrated into the canonical Rust validation path via
`repopact_validation::graph` (not a separate Python-only validator) —
`Validator::validate_graph()` is called from the same `validate()` pipeline
as every other check. A repository with no `rog/manifest.json` reports zero
graph diagnostics (Decision 0044's capability contract).

### Freshness

`repopact_graph::status::Freshness`: `Absent`, `Fresh`, `WorkingOverlay`,
`Partial`, `Stale`, `Unsupported`, `Corrupt`. `Absent`/`Fresh`/`Stale`/
`Unsupported`/`Corrupt` are all genuinely emitted and tested.
`WorkingOverlay` and `Partial` are declared in the type but **never
emitted by any builder this session** — they are reserved for the future
watcher/dirty-tree overlay (ROG-013) and future semantic-adapter coverage
gaps (ROG-020), neither of which exists yet. This is stated explicitly
rather than silently claimed.

### Engine protocol and CLI

Three new operations: `graph.status`, `graph.build`, `graph.verify`,
registered in `repopact-protocol`'s `capabilities()`. `repopact graph
status|build|verify [--json]`, wired through both `repopact/cli.py`'s
REMAINDER+delegated-main() pattern (matching `verify`/`release`) and the
Rust `repopact.rs` launcher shim's direct module-dispatch table. No client
receives raw internal graph structs as an accidental wire format; results
are the typed `GraphStatus`/`Manifest` DTOs. Query operations
(`resolve`/`context`/`neighbors`/`path`/`dependents`/`dependencies`/
`impact`/`tests`/`governance`/`orient`) are **not implemented this
session** — `graph.status` returning the manifest's coverage summary is
the only "useful query" proven this phase, deliberately bounded per the
foundation-phase scope (ROG-024/025/026's query-surface requirements are
explicitly deferred).

### Deterministic rebuild — cross-platform proof

A minimal, byte-identical (confirmed by SHA-256 of every source file)
5-file fixture was created independently on native Windows
(`C:\Projects\repopact` checkout) and on a Linux-native WSL2 Debian 13
checkout (`~/repopact-linux`, never `/mnt/c/...`). `graph.build` was run
against the same fixture on both platforms via the raw engine stdio
protocol. Result: **byte-for-byte identical** source-projection
fingerprint, node/edge counts, and every individual node/edge shard
SHA-256 hash:

```text
fingerprint:  0d95a16cffaaf1ffce81189ca8e6ab90b81fe03daea6114ec2a920f6eba9b5e5
nodes: 8   edges: 13
node_shards (8):  0000/0001/0008/0009/0011/0012/0013/0015 — identical hashes
edge_shards (9):  0001/0002/0003/0005/0008/0009/0011/0013/0015 — identical hashes
```

No macOS execution occurred or is claimed.

### Process/Git scaling

`randomized_git_invocation_count_is_still_bounded_after_graph_build`
(repopact-graph) asserts `<= 4` Git invocations for a full graph build,
matching the pre-existing WI057 bound
(`snapshot_git_invocation_count_is_bounded_independent_of_work_items`,
repopact-repository) and the two `repopact-desktop-api` tests
(`desktop_reads_reuse_one_snapshot_generation_without_git_fanout`,
`watcher_burst_has_one_bounded_refresh_and_ignores_build_churn`), both of
which briefly regressed during development (see above) and are now green.
No filesystem-pass count was separately measured beyond what the tests
above already exercise; no per-work-item or per-node Git call was added.

### Test counts

- `repopact-graph`: 14 tests (new this session).
- `repopact-validation`: 49 tests (46 pre-existing + 3 new graph-integration tests).
- Full `cargo test --workspace`: green (all crates, no regressions).
- `tests/test_graph_cli.py`: 3 new Python tests, green.
- Full Python suite (`python -m unittest`/`pytest tests/`): 270 passed, 2
  pre-existing skips (disclosed WI050 Windows gap; unrelated), green.
- Canonical `repopact validate --root .`: passes.
- `repopact.legacy_validate` (Python comparator): passes.
- `tests/test_conformance.py`: 6 passed, 18 subtests passed.

### Not built this session (explicit deferral, not oversight)

- Tree-sitter or any language parser/adapter.
- Symbol/call graph of any kind.
- Incremental update engine (`G_incremental == G_full` equivalence).
- Local acceleration cache (SQLite or otherwise) — boundary defined in
  Decision 0044, not implemented.
- `adopt`/`doctor` integration — `repopact adopt` is completely unmodified.
- Query operations beyond `graph.status`'s coverage summary
  (`resolve`/`context`/`orient`/`impact`/`tests`/`governance`/etc.).
- Workbench UI changes — the existing `relationship_graph`/`GraphView`
  Tauri command and frontend are unmodified. (Physical-layer nodes/edges
  are additive to the same `RepositoryGraph`, so `GraphView`'s payload
  will grow once a repository has a graph built, but no UI was added to
  filter or present the new layer differently.)
- S8 R1 benchmark-protocol amendment or any graph-enabled research run.
- Performance/storage measurements beyond what the test suite incidentally
  exercises (no dedicated benchmarking pass).
- Symlink/adversarial fuzz testing specific to the graph builder (symlink
  exclusion is inherited from the shared repository walker, which has its
  own existing tests; not independently re-tested here).
- Branch/merge/conflict-regeneration workflow testing.

## Acceptance criteria assessed this session

Marked `satisfied` only where the full acceptance-criterion text is
genuinely met by executed, tested evidence (the work-item schema has no
"partial" state, so any AC not fully met stays `pending`):

**Satisfied:** ROG-001, ROG-002, ROG-006, ROG-007, ROG-008, ROG-009, ROG-011.

**Pending, with the subset actually proven noted below** (all other
ROG-* criteria unchanged from `pending`; only these carry a same-session
note since they were plausible candidates per the original brief):

- **ROG-003** (typed multilayer node model): physical directory/file/
  workspace/configuration-file/nested-repository classes are implemented
  and tested with stable, platform-independent IDs. Symbol-level nodes
  (language-qualified symbol identity) are not implemented — deferred to
  the semantic-adapter phase. Not marked satisfied because the AC text
  explicitly requires symbol coverage "at minimum."
- **ROG-004** (edge taxonomy): physical containment/workspace/
  configuration edges and one real cross-layer `intersects_frozen_surface`
  link are implemented. Definitions/imports/exports/calls/references and
  build/package/test/runtime relations are not implemented (no semantic
  or build-metadata adapters exist yet). Not marked satisfied.
- **ROG-005** (explanation metadata on every edge): every edge that exists
  this session does carry relation kind, layer, derivation class, and a
  `SourceRef`. Not marked satisfied at the whole-criterion level because
  the criterion is written against the full eventual edge taxonomy, most
  of which does not exist yet to demonstrate the property on.
- **ROG-010** (freshness states): `fresh`/`stale`/`unsupported`/`corrupt`/
  `absent` are genuinely emitted and tested. `working_overlay`/`partial`
  are declared but never emitted (no watcher integration, no semantic
  coverage gaps possible yet). Not marked satisfied.
- **ROG-023/024/025/026** (query surface, CLI/protocol): `graph.status`/
  `graph.build`/`graph.verify` and their CLI equivalents are implemented,
  tested, and cross-platform proven. The broader typed query API
  (`resolve`/`context`/`neighbors`/`path`/`dependents`/`dependencies`/
  `impact`/`tests`/`governance`/`orient`) is not implemented. Not marked
  satisfied.
- **ROG-030** (schema/version/migration): schema version and unknown-
  major-fails-closed are implemented and tested. There is no migration
  path yet beyond full rebuild (acceptable per Decision 0044, since no
  schema version 2 exists to migrate from/to), and no evidence this
  criterion's full intent (a proven migration story) is met. Not marked
  satisfied.
- **ROG-031** (validation/conformance coverage): manifest/shard schema,
  hashes, duplicate node IDs, dangling edges, canonical sort, source-
  fingerprint match, self-exclusion, deterministic rebuild, corruption,
  staleness, and graph-disabled compatibility are all implemented and
  tested. Incremental/full equivalence, clean-clone timing as a distinct
  scenario, partial-coverage reporting, unsupported-language behavior, and
  dedicated symlink/nested-repo adversarial tests are not covered this
  session (several are not yet applicable — no incremental engine, no
  language adapters). Not marked satisfied.
- **ROG-036/037/039/040** (no-LLM-required core, no bypass of WI050/
  frozen-surface/WI054, capability contract, no fabricated authority): all
  true by construction and consistent with the implementation (no LLM/
  network call anywhere in this session's code; graph build only ever
  writes inside `rog/`; the capability contract precisely matches Decision
  0044 section 9 and is tested). Not marked satisfied at the whole-
  criterion level because these are closeout-level criteria meant to be
  demonstrated by the *complete* WI063 implementation, not a foundation
  slice, and marking them now would overstate what a foundation-phase
  session can actually prove about the finished system.

## Next recommended WI063 phase

**Phase 2: semantic language adapters.** Per the rollout plan and this
session's own consolidated sequence (architecture-review.md), the next
phase should:

1. Evaluate Tree-sitter vs. alternatives against the criteria already
   listed in the architecture review (determinism, incremental parse
   support, Rust packaging, language coverage, malformed-input safety,
   memory behavior, license/maintenance cost, binary footprint) and record
   the decision — before writing a parser adapter.
2. Define the language-adapter trait/interface (bounded source facts in,
   normalized graph contributions + coverage/failure info out) without
   hard-wiring Tree-sitter concepts into the canonical graph DTOs.
3. Implement Rust, then Python, then TypeScript/JavaScript coverage
   incrementally, each with its own coverage-reporting and parser-failure-
   isolation tests, reusing the existing `DerivationClass::Parser` variant.
4. Only after semantic coverage exists does `partial` freshness become
   reachable and worth testing for real.

Do not begin incremental-update engineering before Phase 2's full-build
output is itself proven stable with real semantic content (per the
existing rollout plan's own ordering: full-build determinism is the oracle
incremental update must converge to). Do not begin Workbench UI, `adopt`
integration, or S8 R1 preregistration until the query surface (ROG-023–026)
exists to give the Workbench and research phases something real to
consume.

---

# WI063 Implementation Progress — Semantic-Adapter Checkpoint (2026-09-13)

**Starting SHA:** `c8c56739bfbe355d4a8a382eebe36aef550fa6b4`
**This checkpoint's final SHA:** see `git log` HEAD at the final commit of this session
**Decision:** 0045 (Tree-sitter selection, schema v2)

## What this session proves

Deterministic Rust/Python/JavaScript/TypeScript/JSX/TSX symbol and import
extraction, built on the foundation checkpoint's durable physical graph,
with schema v1 permanently preserved and schema v2 additive.

### Selected parser and exact pinned versions

Tree-sitter 0.27.0, `tree-sitter-rust` 0.24.2, `tree-sitter-python`
0.25.0, `tree-sitter-javascript` 0.25.0, `tree-sitter-typescript` 0.23.2 --
all pinned with exact (`=X.Y.Z`) version requirements in
`rust/Cargo.toml`. See Decision 0045 for the full evaluation against
language-specific parser stacks and the real compile/parse/cancellation
spike that grounded the choice.

### MSRV

`tree-sitter` 0.27.0 declares `rust-version: 1.90`. RepoPact has no
documented MSRV anywhere in the repository (confirmed by direct search
before this decision). The installed toolchain (1.96.0) already exceeds
1.90; this session establishes 1.90 as the workspace's de facto floor
going forward, recorded in Decision 0045 since no prior document did.

### Schema v2

`GraphNodeKind::Symbol` + a separate `SymbolKind` enum (module, function,
method, type, enum, interface, implementation, type_alias, constant,
macro, test). `GraphEdgeKind` gains `Defines`/`Imports`/`Exports`/
`Implements`/`Extends`/`References`/`Calls`/`UsesType` (only `Defines`/
`Imports` are emitted this checkpoint). `GraphSourceLocation` (byte
offsets + row/column) added as optional metadata on both `GraphNode` and
`GraphEdge`, kept fully separate from the shared governance `SourceRef`.
`durable::CURRENT_GRAPH_SCHEMA_VERSION` is now `2` (always written);
`durable::SUPPORTED_GRAPH_SCHEMA_VERSIONS` is `[1, 2]`. Proven: a genuine
v1 manifest (physical-only vocabulary) remains structurally valid and
`Fresh`; the only supported v1-to-v2 path is a full deterministic
rebuild, proven to converge correctly.

### Adapter API

`repopact_graph::semantic::SemanticAdapter` -- `fn extract(&self, input:
&SourceInput) -> AdapterOutput`. No Tree-sitter type crosses this
boundary. The orchestrator (`semantic::extend`) owns file eligibility
(from the already-built source projection) and content loading; adapters
never perform their own I/O, Git calls, or path resolution. Adapter
panics are caught and downgraded to a per-file `Failed` coverage entry
rather than aborting the whole build.

### Language/relation coverage

| Language | Symbols covered | Relations emitted | Known gaps |
| --- | --- | --- | --- |
| Rust | module, function, method (via impl block, tagged Function), struct, enum, trait, type alias, impl block, macro_rules!, `#[test]`/`#[tokio::test]`-style test functions | Defines, Imports | no visibility/exported detection |
| Python | class, function, method, async def, pytest-convention `test_*` | Defines, Imports | no decorator-based test detection beyond naming convention |
| JS/JSX/TS/TSX | function, class, method, TS interface/type-alias/enum | Defines, Imports | no exports detection; test detection is a narrow name-prefix heuristic only |

No call-graph edges are emitted at all (ROG-019's false-precision risk is
avoided by emitting nothing, not by hedged/heuristic edges).

### Stable-ID strategy

`language + repository-relative path + qualified container + symbol-kind
tag + declared name`. Proven unaffected by leading blank lines/comments.
Anonymous constructs are omitted rather than assigned unstable IDs (no
adapter emits one).

### Resource/cancellation policy (ROG-022)

1 MiB max file size, 2s max parse deadline (conservative, unmeasured-in-
production constants, centralized in `ResourcePolicy`). Cancellation uses
`parse_with_options` + a progress callback checking a wall-clock deadline
-- the deprecated `set_timeout_micros` does not exist in tree-sitter
0.27.0 at all (confirmed by source inspection). A 512-level recursion
depth guard was added to each adapter's own AST walker mid-session, after
re-checking this AC's full text against the first implementation (which
had none) -- proven by a test with 2000 levels of pathological nesting.
Binary content (NUL-byte sniff) and oversized files are rejected before
any parser runs. **Disclosed, not implemented:** minified-file and
generated-file-specific policies (both named explicitly in ROG-022's
text), and parser memory-behavior measurement.

### `Freshness::Partial` -- now real

A supported file that cannot be fully processed (parse error,
cancellation, adapter failure) makes repository-wide status `Partial`,
proven by a direct test. An unsupported-language file or a policy
exclusion alone does *not* make the graph `Partial` -- also proven
directly, in the other direction. `working_overlay` remains declared-only
(ROG-013, out of scope this session).

### Real RepoPact self-build (engineering validation, not S8/R1)

`repopact graph build --root .` against RepoPact's own live checkout:
18.2s, 749 files considered, 106 adapted (Rust/Python/JS/TS), **106/106
complete, 0 partial, 0 failed** -- 643 files correctly classified
unsupported-language. `node_count=4481` (governance 773, physical 954,
semantic 2754), `edge_count=6122` (governance 1583, physical 1738,
semantic 2801), schema v2, 16/16 shards, 4.0MB on disk. `graph verify`
reported `Fresh`; canonical `repopact validate` accepted it with zero
diagnostics. The `rog/` directory was deleted afterward, not committed --
this checkpoint's scope is proving the mechanism works, not shipping a
built graph.

### Cross-platform determinism

A 7-file fixture (Rust/Python/JS/TS/TSX plus one malformed-but-recoverable
Rust file and one unsupported README.md), confirmed byte-identical by
SHA-256, built independently on native Windows and Linux-native WSL2
Debian 13: **byte-for-byte identical** fingerprint
(`12e09a01...eb899a2`), node/edge counts (18/17), every one of 21 shard
hashes, and identical semantic coverage breakdown (7 considered, 5
complete, 1 partial, 1 unsupported). No macOS execution.

### Git/process bound

`randomized_git_invocation_count_is_still_bounded_after_graph_build`
(<=4 invocations) passes unchanged after semantic extraction. The
orchestrator is single-threaded and sequential; no per-file thread or
process is spawned.

### Packaging impact

`repopact-engine.exe` release build: 6,955,520 bytes before this
checkpoint vs. 12,073,984 bytes after -- **+5,118,464 bytes (+73.6%)**.
No system Tree-sitter install, `libclang`, or runtime grammar download is
required; the released binary carries full parsing capability locally.

### Test counts

`repopact-graph`: 43 tests (up from 18 at the foundation checkpoint).
Full `cargo test --workspace`: green, including the full Tauri desktop
build. Canonical `repopact validate`, the Python legacy comparator, and
`tests/test_conformance.py` (6 passed, 18 subtests) all pass.

### Acceptance criteria assessed this session

**Satisfied:** ROG-003 (typed multilayer node model now includes real
symbol coverage with proven deterministic, non-line-based IDs), ROG-005
(every edge across both physical and semantic layers carries relation
kind, layer, derivation, source, and location where meaningful), ROG-020
(coverage gaps -- unsupported language, parse failure, policy exclusion,
adapter failure -- are five distinct, tested, first-class states, never
collapsed into one "skipped" bucket).

**Pending, with the proven subset and exact gap disclosed** (see the
evidence record's `ac_notes` for full detail): ROG-004 (edge taxonomy
broader than what's emitted), ROG-010 (`working_overlay` still
unreachable pending ROG-013), ROG-017 (no exported/public detection),
ROG-018 (no exports detection, narrow test-naming heuristic), ROG-022
(no minified/generated-file policy, no memory measurement).

**Not attempted:** ROG-012/013 (incremental/watcher), ROG-014–016 beyond
the foundation checkpoint's own proof, ROG-023–030 beyond
`graph.status/build/verify`, ROG-027/028 (Workbench), ROG-029
(branch/merge), ROG-032 (dedicated benchmark suite), ROG-033/034 (S8 R1),
ROG-035/038 (documentation/closeout).

## Next recommended WI063 phase

Two credible options: (a) close this checkpoint's disclosed gaps
(visibility/exports detection for Rust/JS/TS, minified/generated file
policy) before broadening further -- narrow, well-understood work; or (b)
proceed to ROG-012/013 incremental-update proof against this checkpoint's
now cross-platform-proven full-build oracle, per the architecture
review's own phase ordering. A fresh architecture review should decide
between the two rather than this session assuming either.

---

# WI063 Implementation Progress -- Incremental-Equivalence Checkpoint (2026-09-13)

## Scope

Building directly on the accepted semantic-adapter checkpoint (Decision
0045), this checkpoint implements and proves ROG-012: an incremental
graph.update whose output, after canonical normalization, is exactly
equal to a clean full rebuild of the same final repository state, for
every change class the AC names. Decision 0046 binds the architecture.
Explicitly not attempted: incremental governance/physical topology
tracking, a persistent Tree-sitter syntax-tree cache, the ROG-013
watcher/working-tree overlay, any query surface beyond status/build/
verify/update, and any broadening of semantic scope beyond what the
semantic-adapter checkpoint already covers.

## Architecture

```text
current RepositorySnapshot
       |
       +--> governance rebuild globally
       |
       +--> physical rebuild globally
       |
       +--> semantic contribution delta
                |
                +--> unchanged -> reuse
                +--> modified  -> reparse
                +--> added     -> parse
                +--> deleted   -> remove
       |
       v
candidate graph
       |
       v
canonical durable serialization
```

Governance and physical topology are cheap and already fully
deterministic from repository state, so they are rebuilt globally on
every graph.update call rather than incrementally tracked -- this
checkpoint does not attempt fine-grained incremental governance or
physical topology. Only semantic extraction, the genuinely expensive
per-file parsing step, is contribution-incremental.

The full build remains the correctness oracle. graph.update is an
optimization layered on RepositoryGraph::build_with_fingerprint, never
a second, independently-trusted extraction path. Every equivalence test
proves an incremental result against a clean full build of the same
final state.

## The single contribution-generation primitive

semantic::build_file_contribution(repository, relative_path, digest,
policy) is the one function that turns one file's approved input into a
coverage entry plus nodes/edges. A full build (semantic::extend) calls
it once per projected file, unconditionally. An incremental update
(incremental::update) calls it only for files classified added or
modified; for unchanged files it instead reuses the prior contribution's
nodes/edges, read back from the previous durable graph's shards and
grouped by each item's own existing source.path (already sufficient:
every adapter attributes every node/edge it emits to exactly the file it
parsed, and current semantic relations -- defines, imports -- are
file-local, so no separate contribution-owner field was needed). There
is exactly one semantic extraction implementation; full build and
incremental update differ only in which files invoke it this call.

semantic::aggregate_coverage(per_file) recomputes the aggregate coverage
counters deterministically from whatever final per-file inventory either
path assembles, rather than incrementing/decrementing counters as files
are added, changed, or removed. Both paths call this same function, so
their coverage output is provably equal by construction, not by parallel
bookkeeping staying in sync.

## Correctness authorities (Decision 0046 section 3)

Reuse decisions rest only on: canonical repository-relative path, the
SourceProjection content digest for that path, and an explicit semantic-
compatibility identity (graph_schema_major + pipeline_version +
adapter_versions + resource_policy_version). Git diff, mtime, watcher
events, and parser caches are never consulted for correctness anywhere in
this checkpoint's code.

## Schema-v2 additive fields (no schema-major bump)

- FileCoverageEntry.source_digest: Option<String> -- the projection
  digest a coverage entry was computed against.
- Manifest.semantic_compatibility: Option<SemanticCompatibility> -- the
  pipeline/adapter/resource-policy identity a durable graph was written
  under.

Both are serde-default, skip-if-none. A schema-v2 graph from the
semantic-adapter checkpoint (before this decision) deserializes cleanly
under this implementation with both fields None, and is correctly
treated as "predates incremental support" -- incremental::update falls
back to a full rebuild (fallback_reason "incremental_metadata_absent")
rather than guessing.

## Required update behavior (all six states tested)

| Baseline state | Mode | fallback_reason |
|---|---|---|
| No rog/manifest.json | full_fallback | graph_absent |
| Schema v1 | full_fallback | schema_v1_upgrade |
| Schema v2, no semantic_compatibility | full_fallback | incremental_metadata_absent |
| Schema v2, compatibility mismatch | full_fallback | semantic_compatibility_mismatch |
| Structurally corrupt | full_fallback | corrupt_baseline_full_rebuild |
| Unsupported schema major | error (nothing written) | n/a (fails closed) |
| Valid, fingerprint unchanged | no_op | none |
| Valid, fingerprint changed | incremental | none |

A full rebuild is never hidden behind the incremental label. A corrupt
baseline is never treated as a source of reusable contributions --
Decision 0046 chose an explicit full rebuild over the corrupt baseline
(self-healing, and rog/ is never load-bearing for repository validity
per Decision 0044 section 9) rather than a hard failure; an unsupported
schema major, by contrast, fails closed and writes nothing.

## Durable replacement safety

durable::write's build-then-swap sequence was upgraded from
delete-then-rename to rename-based backup-and-rollback
(durable::swap_with_rollback): any pre-existing rog/ is renamed aside
before the new one is installed, and rolled back into place if
installation fails. A fault-injection test (forcing the second rename to
fail via an injected closure) proves the prior graph's manifest remains
byte-recoverable at the final path afterward -- never a half-written or
missing graph.

## ROG-012 mutation-class equivalence matrix (all proven byte-for-byte)

Every case below uses a shared harness: build s0 in one fixture, mutate
to s1, run graph.update; independently build a second fixture straight
to s1 and take one clean full build; assert every manifest field and
every node/edge shard's bytes match exactly.

- Addition -- Rust/Python/TypeScript files added; classified added,
  reparsed, output matches a clean rebuild.
- Edit -- a declaration added to an existing file; classified modified,
  reparsed, matches.
- Non-semantic edit -- a leading comment/blank line inserted; the file's
  digest changes (so it is reparsed), but every affected symbol's stable
  ID is proven byte-identical before and after.
- Delete -- a file removed; its symbols, edges, and coverage entry are
  proven absent from the resulting durable graph.
- Rename/move -- old path deleted, new path added; proven classified as
  delete+add (files_modified == 0), never a tracked rename, per Decision
  0046 section 4.
- Manifest change -- Cargo.toml/pyproject.toml/package.json
  added/changed; converges through the always-global physical rebuild.
- Relationship change -- an import statement changed; the old import
  fact is proven absent from the resulting graph, the new one present.
- Governance change -- a work-item JSON record edited; proven to
  reparse exactly one file semantically (the JSON itself, cheaply
  classified unsupported-language), not the whole repository.
- Parser-failure introduction -- a valid file replaced with malformed
  syntax; proven that only that file reparses, unrelated contributions
  (a Python file in the same fixture) remain Complete, and freshness
  becomes Partial.
- Parser-failure recovery -- the malformed file repaired; Partial
  clears, freshness returns to Fresh, and the result matches a clean
  full rebuild of the fixed state exactly.
- Policy transitions -- a file toggled source -> binary -> source;
  proven no stale symbol survives the binary phase, and the final state
  matches a clean full rebuild exactly.
- Unsupported file add/delete -- a Markdown file added and another
  removed; proven correctly counted as added/deleted with truthful
  (non-code) coverage classification.
- ROG-only mutation -- a second update() call with zero source changes
  (the only prior change was rog/ itself, from the first build); proven
  no_op, zero reparses.
- Excluded-tree mutation -- files written under target/, node_modules/,
  .venv/; proven to produce zero source-projection delta and a no_op
  update.
- True no-op -- proven that rog/manifest.json's bytes are completely
  unchanged (not merely semantically equivalent) after a no-op update.

## Observable reuse proof

A 100-file Rust fixture, one file changed: semantic_reparsed == 1,
semantic_reused == 99, output byte-identical to a clean full rebuild. A
second 100-file fixture with zero changes: semantic_reparsed == 0,
semantic_reused == 100.

## Compatibility invalidation

Directly tampering with a durable manifest's recorded
semantic_compatibility.pipeline_version (simulating what a real
pipeline-version bump would look like) forces full_fallback with
fallback_reason "semantic_compatibility_mismatch" and semantic_reused ==
0 -- reuse is never attempted against a baseline whose extraction
behavior might have changed.

## Cross-platform proof

A 5-file fixture (Rust module+test, Python class, TypeScript interface),
confirmed byte-identical by SHA-256 of every file, was built to s0 and
mutated to s1 (2 files added, 2 modified including one comment-only edit,
1 file renamed) independently via the raw engine stdio protocol on native
Windows and a freshly re-synced Linux-native WSL2 Debian 13 checkout
(~/repopact-linux, never /mnt/c/...). Result: byte-for-byte identical
graph.update output (mode, fingerprints, every count) and every one of
the final manifest's node/edge shard SHA-256 hashes. No macOS execution
occurred or is claimed.

## WI057 re-check

update_does_not_exceed_the_wi057_bounded_git_invocation_count confirms
graph.update stays within the existing <=4-invocations-per-snapshot
bound; delta computation and contribution reuse read only the durable
graph's own shards (disk I/O, not Git or filesystem-diff calls).

## Performance-correctness fix

While gathering engineering timing evidence, incremental::update was
found to compute the SourceProjection twice per call -- once to compare
fingerprints and classify the delta, again inside the always-global
governance+physical rebuild. On a real RepoPact-scale fixture the
projection walk (content-hashing every projected file) dominates wall
time far more than semantic parsing does, so this roughly doubled every
incremental update's real cost. Fixed by splitting
RepositoryGraph::build_governance_and_physical into a
projection-computing wrapper and a
build_governance_and_physical_with_projection variant that
incremental::update now calls with the projection it already has -- all
69 repopact-graph tests remained green before and after.

## Real RepoPact-scale engineering timing (engineering validation only)

On a disposable ~5,100-file copy of RepoPact's own live checkout: full
build 25.2s (755 files considered, 107 semantically complete,
node_count=4574, edge_count=6237); a subsequent no-op update 24.0s (zero
reparses, byte-identical manifest); a single-file change 21.3s (one
reparse, 754 reused). All three are the same order of magnitude because
the projection's content-hashing walk, not semantic parsing, dominates
wall time on this fixture -- a genuine, disclosed finding, not a result
this checkpoint claims to have optimized to zero. This is engineering
validation only, explicitly not S8/R1 evidence and not claimed toward
ROG-032, which requires a broader, dedicated measurement suite (peak
memory, local-cache size, clean-clone load time, query latency,
branch/merge rebuild cost) not attempted here.

## Test results

69/69 repopact-graph tests (43 pre-existing unchanged + 26 new). Full
cargo test --workspace green. cargo fmt --check/cargo check --workspace
clean. 6/6 Python graph_cli tests (3 new update tests) against the real
built engine binary. Canonical repopact validate clean. Broad Python
regression suite: see the evidence record's closeout.python_regression
for the exact count captured after this document was written.

## AC assessment

Newly satisfied: ROG-012 (every named change class proven byte-for-byte
equivalent to a clean full rebuild, cross-platform), ROG-030 (schema
versioning plus now-proven migration/fallback behavior -- v1-to-v2
upgrade only via full rebuild, old-v2/corrupt/unsupported-schema
fallback behavior all tested).

Still pending: ROG-010 (working_overlay unreachable pending ROG-013),
ROG-013 (not attempted -- no watcher/overlay integration),
ROG-004/017/018/022 (unchanged disclosed gaps from the semantic-adapter
checkpoint; this checkpoint did not broaden semantic scope), ROG-031
(the AC's broad closeout matrix now includes a proven incremental/full
equivalence item, but this checkpoint did not audit every other item in
that matrix as one coordinated pass), ROG-032 (engineering timing
evidence gathered, but not the broader dedicated benchmark suite the AC
requires; not claimed).

## Next recommended WI063 phase

ROG-013 (watcher/working-tree overlay integration) is the natural next
phase now that both the full-build oracle and incremental contribution
reuse are proven cross-platform; alternatively, closing ROG-017/018/022's
disclosed semantic-coverage gaps before adding overlay complexity. A
fresh architecture review should decide between the two.

---

# WI063 Implementation Progress -- Working-Overlay Checkpoint (2026-09-13)

## Scope

Building on the accepted incremental-equivalence checkpoint, this
checkpoint implements ROG-013 (session working-tree overlay), reassesses
ROG-010 against real disclosure behavior, and reassesses ROG-017 against
already-existing semantic-checkpoint evidence. Decision 0047 binds the
architecture. Explicitly not attempted: build/test/runtime operational
adapters, rich JSON/TOML/YAML/Markdown semantic coverage, query/orient/
impact APIs, the Workbench repository-map UI, adoption/backfill
integration, a local acceleration database, a persistent Tree-sitter
syntax-tree cache, or S8 R1.

## The problem this checkpoint eliminates

Before this checkpoint, `repopact-desktop-api` called
`core.graph_snapshot(&snapshot)` -- a full governance+physical+semantic
rebuild from scratch -- on every session open, every explicit refresh,
both branches of `apply_mutation_plan`, and every non-no-op watcher poll.
Two compounding costs made this expensive once semantic adapters existed
(the prior checkpoint): the source-projection content-hash walk (proven
by ROG-012's own performance evidence to dominate wall time), and a
*second*, entirely separate full rebuild inside `overview_from_snapshot`
purely to compute `graph_node_count`/`graph_edge_count`. A further,
previously-undiagnosed gap: `RepositorySnapshot::token()` reflects only
governance-record content, so an ordinary source-file edit (exactly the
case ROG-013 targets) never changed it, and the pre-existing
`poll_repository_events` logic used token equality to skip all graph
work -- meaning plain source edits never updated the cached graph at all,
regardless of how much semantic-extraction work existed under the hood.

## Architecture

```text
                   durable ROG baseline
                          Gd
                          |
                          v
                 session graph state
                          |
        +-----------------+------------------+
        |                                    |
        v                                    v
RepositorySnapshot                    watcher change set
current governance                    changed paths
current topology                           |
        |                                   |
        +----------------+------------------+
                         |
                         v
               in-memory reconciliation
                         |
                         v
                    working overlay
                         Gw
```

No durable `rog/` file is changed by ordinary overlay reconciliation --
only an explicit `repopact graph build`/`graph update` writes it.

## SessionGraphState (repopact-graph::overlay)

The canonical engine lives in `repopact-graph`, not
`repopact-desktop-api` -- desktop/Tauri code orchestrates it; it does not
derive graph correctness itself, and neither does the frontend.

- `open(snapshot)`: one full source-projection walk (a session-open
  cost, not a per-event cost). If the durable graph is present,
  structurally valid, and its fingerprint already matches, the effective
  graph is loaded verbatim (`durable::load_graph`) -- no source file is
  read, no adapter runs. If stale, reconciles once in memory against the
  durable baseline (reusing `incremental::plan_reconciliation`, the same
  algorithm the durable `graph.update` path uses). If absent/corrupt/
  unsupported, performs one full in-memory build.
- `reconcile(snapshot, changed_paths)`: the watcher-driven hot path. For
  each changed path, `symlink_metadata` classifies it first (a directory
  or symlink triggers a conservative fallback to `refresh()`, never a
  guess); a regular file's current digest is compared against the
  in-memory inventory (a genuine no-op is dropped without reparsing);
  only a real add/modify calls `semantic::build_file_contribution` for
  that one file, and every other cached contribution is reused
  untouched. Never performs a full projection walk. A burst above
  `LARGE_BURST_FALLBACK_THRESHOLD` (64, centralized and documented) also
  falls back to `refresh()`.
- `refresh(snapshot)`: the explicit correctness-recovery path
  (`refresh_repository()`'s effective-graph refresh). One full
  projection walk, diffed against the overlay's own current in-memory
  inventory (not the durable baseline, which this call does not
  re-read) -- after it returns, the effective graph truthfully reflects
  current filesystem state even if the durable baseline remains stale.

## Shared machinery, not a second implementation

`incremental::plan_reconciliation` was extracted from
`incremental_update` (a behavior-preserving refactor -- all 69
pre-existing `repopact-graph` tests passed unchanged both before and
after) so the durable `graph.update` path and the overlay's
session-open/stale-reconcile/refresh paths call the identical delta-
reconciliation algorithm; the durable path additionally calls
`durable::write`, the overlay path keeps the result in memory only.
Every contribution regeneration, in both the durable and overlay paths,
funnels through the same `semantic::build_file_contribution` primitive
Decision 0046 established -- there is no second semantic-extraction
implementation anywhere in this checkpoint.

## Orthogonal status model (ROG-010)

`GraphBasis` (durable/working_overlay), `GraphCoverageState` (complete/
partial), and `DurableFreshness` (absent/fresh/stale/unsupported/corrupt)
are three new, independent types combined into `EffectiveGraphStatus`.
The pre-existing `status::Freshness` (the durable CLI's own model for
`repopact graph status/build/verify`) is untouched -- `repopact graph
status` may legitimately report `stale` at the exact moment a session's
`EffectiveGraphStatus.basis` reports `working_overlay`; both are true,
neither contradicts the other (Decision 0047 section 5/step 19). The
effective status can simultaneously express `basis=working_overlay,
coverage=partial, durable_baseline=stale` -- proven directly by a
dedicated frontend test.

## Watcher/session integration

- `open_repository`: `SessionGraphState::open(&snapshot)`.
- `refresh_repository`: `overlay.refresh(&snapshot)`.
- `apply_mutation_plan` success path: `overlay.reconcile(&snapshot,
  &changed_paths)` using the mutation's own exact changed paths --
  self-applied edits update the graph immediately, before any watcher
  poll (WI063 step 15). The stale-read-set failure path calls
  `overlay.refresh` instead, since a failed apply is not a self-apply.
- `poll_repository_events`: `overlay.reconcile(&snapshot, &paths)` now
  runs on every non-empty watcher burst unconditionally, closing the
  governance-token gap described above. `overview_from_snapshot` no
  longer performs its own separate full rebuild -- it now takes the
  overlay's already-computed node/edge counts as parameters, removing
  the second redundant full rebuild every call site had been paying for.
- The pre-existing `pending_self_paths`/`ChangeOrigin::SelfApply`
  de-duplication is preserved unchanged; the overlay's own no-op
  detection (an unchanged digest produces no reparse and does not bump
  `overlay_generation`) additionally guarantees no duplicate semantic
  work when the watcher later reports the same self-applied paths.

## Durable no-write proof

A dedicated `repopact-graph::overlay` test hashes `rog/manifest.json`
and every node/edge shard before and after a 5-round burst of
`reconcile()` calls (an edit repeated five times, a file added, a file
deleted) and asserts byte-identical output. A dedicated
`repopact-desktop-api` test proves ordinary session activity (open,
watcher-driven edits, an explicit refresh) never creates `rog/` at all.
A real ~1,470-file fixture run (see Performance evidence below)
corroborates this on real content: the durable directory was written
exactly once (the initial build) across seven subsequent overlay
operations.

## Test matrix (ROG-013)

14 new `repopact-graph::overlay` tests plus 3 new `repopact-desktop-api`
tests cover: a single semantic edit (one reparse, unrelated contributions
reused, basis becomes `working_overlay`); a multi-file burst (only
touched files reparse); addition/deletion/rename (delete+add, no
fabricated identity); an unsupported-file change (inventory updates
without fabricating semantics); a parser failure (coverage becomes
`partial` without destroying unrelated contributions) and its recovery;
pre-existing dirty state detected at session open with zero watcher
history; a directory-level event and an oversized burst both falling
back to a full reconcile; a duplicate/no-op watcher event (zero reparse,
no generation bump); an explicit refresh reconciling a change the
watcher never reported; a 100-file fixture proving exactly one reparse
for one changed file; the durable-shard-hash-before/after proof above; a
watcher-reported plain source edit updating the graph despite an
unchanged governance snapshot token; a self-applied mutation updating
the graph immediately, with the watcher's later report of the same paths
recognized as `SelfApply` rather than duplicated; and ordinary session
activity never writing `rog/`.

Repository switching (a fresh session discards any prior overlay by
construction -- a new `SessionGraphState::open` call, never reused across
sessions) and session close/reopen (the overlay is a plain Rust value
owned by `ActiveSession`, never serialized) were not given dedicated
tests because both properties follow directly from the type's ownership
structure rather than from any conditional logic that could regress
independently.

## Performance evidence (engineering validation only)

On a disposable ~1,470-file copy of RepoPact's own live checkout, via a
small standalone harness driving `SessionGraphState` directly (the
overlay has no engine-CLI surface -- it is a session/desktop-only
primitive): full durable build 3.28s; session open against a fresh
durable baseline 2.63s (still pays one projection walk to confirm the
fingerprint match, per Decision 0047 section 7); a single-file
watcher-style update 559ms (1 reparsed, 696 reused); a duplicate/no-op
watcher event 294.5 microseconds; a 10-event burst on the same file
541ms total; a 5-file multi-file burst 683ms; an explicit full session
refresh 2.23s. This is engineering validation only, explicitly not
S8/R1 evidence and not claimed toward ROG-032 (which requires a broader,
dedicated benchmark suite -- peak memory, local-cache size, clean-clone
load time, query latency, branch/merge rebuild cost -- not attempted
here).

## WI057 re-check

Both pre-existing `repopact-desktop-api` WI057 tests
(`desktop_reads_reuse_one_snapshot_generation_without_git_fanout`,
`watcher_burst_has_one_bounded_refresh_and_ignores_build_churn`) pass
unchanged. Overlay reconciliation performs zero additional Git
invocations -- delta computation and contribution reuse are pure
in-memory/disk-shard operations.

## Cross-platform proof

The WSL2 Debian 13 checkout at `~/repopact-linux` (never `/mnt/c/...`)
was fast-forwarded to this checkpoint's pushed commits and rebuilt;
`cargo test -p repopact-graph -p repopact-desktop-api` and
`cargo test --workspace` reproduce the exact same pass counts (83/14,
full workspace green) as native Windows. The durable graph format is
unchanged by this checkpoint (no schema-major bump, no new manifest
field), so no new byte-equivalence cross-platform proof was required
beyond ROG-012's own -- this run re-proves the overlay engine's test
suite is platform-independent. No macOS execution occurred or is
claimed.

## AC assessment

**Newly satisfied:** ROG-013 (session-local, non-durable, reconstructable
overlay; watcher events treated as hints and re-verified; explicit
conservative fallback for directory-level/oversized ambiguity;
self-applied mutations reflected immediately; pre-existing dirty state
detected without watcher history; durable `rog/` never rewritten by
ordinary session activity), ROG-010 (all six required states are real,
typed, and disclosed through `GraphView.status`; the desktop Workbench's
Graph page status line renders working-overlay/partial/stale states
distinctly, proven by two dedicated frontend tests).

**Reassessed (not newly implemented this checkpoint):** ROG-017.
Direct inspection of Decision 0045 and the semantic-adapter checkpoint's
own evidence record confirms every clause of ROG-017's exact text
already has genuine evidence (language-neutral adapter interface,
Tree-sitter evaluated against alternatives, determinism/incremental-
capability/portability/parse-safety/language-coverage/packaging-cost/
Rust-core-integration all recorded). Marked satisfied against evidence
ID `20260913-063-semantic-checkpoint`, not this checkpoint's own -- this
working-overlay checkpoint did not touch semantic extraction.

**Still pending:** ROG-004/018/022 (unchanged disclosed gaps -- broader
edge taxonomy; JSON/TOML/YAML/Markdown/manifest semantic coverage
explicitly named in ROG-018's own text; minified/generated file policy),
ROG-031 (the broad conformance matrix now includes a genuinely proven
durable-no-write-during-overlay-use property in addition to ROG-012's
equivalence proof, but this checkpoint did not audit every remaining
item in that matrix as one coordinated pass), ROG-032 (overlay-specific
timing evidence gathered, but not the AC's full dedicated benchmark
suite).

## Next recommended WI063 phase

Two credible options: (a) a persistent Tree-sitter syntax-tree cache /
incremental reparse layer on top of the now-proven contribution-reuse
overlay, for genuinely large repositories where even single-file
reparse cost matters; or (b) closing the disclosed ROG-018/022 semantic-
coverage gaps (JSON/TOML/YAML/Markdown adapters, exports detection,
minified/generated policy) before adding further overlay sophistication.
A fresh architecture review should decide between the two.

---

# WI063 Implementation Progress -- Metadata/Operational-Topology Checkpoint (2026-09-14)

## Scope

Building on the accepted working-overlay checkpoint, this checkpoint
adds deterministic structured-metadata (JSON/TOML/YAML/Markdown) and
operational-topology (Cargo/Python/npm/CI) extraction, emits real
Rust `Exports` edges, closes ROG-021 with direct graph-level evidence,
and closes ROG-022 with a real parser-memory measurement. Decision 0048
binds the architecture, including a schema-compatibility audit that
found schema v3 genuinely necessary. Explicitly not attempted:
ROG-023-026 query/orientation APIs, ROG-027 Workbench repository map,
adoption/backfill integration, S8 R1, a persistent parser-tree cache, or
a SQLite/local graph database.

## Canonical pending-AC matrix (from `work-item.json`, not a summary)

The prior working-overlay checkpoint's final report understated the
remaining acceptance-criteria inventory. The actual canonical state at
this checkpoint's start (`dfd5690`) was:

- **Satisfied (15):** ROG-001, 002, 003, 005, 006, 007, 008, 009, 010,
  011, 012, 013, 017, 020, 030.
- **Pending (25):** ROG-004, 014, 015, 016, 018, 019, 021, 022, 023,
  024, 025, 026, 027, 028, 029, 031, 032, 033, 034, 035, 036, 037, 038,
  039, 040.

This checkpoint's primary candidates were ROG-018/019/021/022, plus
ROG-004 if its complete taxonomy became satisfied. See "AC assessment"
below for the actual outcome and the full remaining inventory.

## Schema-compatibility audit and Decision 0048

Before adding any new closed-enum vocabulary, a focused test
(`durable::tests::a_nested_closed_enum_field_carries_the_identical_hazard_as_a_top_level_kind`)
proved that a nested closed enum field (the exact shape a new
`manifest_kind` field would have) carries the identical old-reader-
hard-fails hazard as a top-level `GraphNodeKind`/`GraphEdgeKind`
addition. Decision 0045 section 3's framing -- that growing `SymbolKind`
"does not require another schema-major bump" -- was only ever true for a
*pre-Symbol* v1 reader, never for a v2-era reader compiled before a
given variant existed. **Conclusion: schema v3 is genuinely necessary**
for this checkpoint's new vocabulary. `CURRENT_GRAPH_SCHEMA_VERSION`
moves to 3, `SUPPORTED_GRAPH_SCHEMA_VERSIONS` becomes `[1, 2, 3]`; v1
and v2 remain permanently readable; migration to v3 is full
deterministic rebuild only. New tests
(`a_genuine_v2_semantic_graph_remains_readable_and_valid`,
`a_v2_graph_rebuilds_deterministically_into_a_valid_v3_graph`) mirror
the existing v1 compatibility proofs one major up.

## Source semantic adapters vs. metadata adapters

Both families are reached from the identical per-file classification
point, `semantic::build_file_contribution` -- there is exactly one
contribution-generation pipeline. `SourceLanguage` gained `Json`/
`Toml`/`Yaml`/`Markdown` variants; `SourceLanguage::is_metadata()`
routes these to a new, distinct `MetadataAdapter` trait
(`repopact_graph::metadata`) rather than the Tree-sitter-backed
`SemanticAdapter` trait used for Rust/Python/JS/TS. Both trait families
return the same `AdapterOutput` shape, so the shared orchestration
(size/binary/minified/generated policy checks, panic isolation, per-file
coverage entries) applies identically regardless of which family
handled a given file.

Four `MetadataAdapter` implementations:

| Adapter | Recognizes | Facts extracted |
|---|---|---|
| TOML | `Cargo.toml`, `pyproject.toml` | package/project identity, workspace members, dependencies (dependencies/dev-dependencies/build-dependencies/workspace.dependencies), optional-dependency groups, script entrypoints, build backend |
| JSON | `package.json`, `tsconfig*.json` | package identity, dependencies/devDependencies/peerDependencies, scripts, tsconfig extends/references/select compilerOptions |
| YAML | `.github/workflows/*.yml`/`.yaml` | workflow name, job ids, `uses`/`run`/`working-directory` per step (`${{ ... }}` recorded symbolically, never evaluated) |
| Markdown | `*.md`/`*.markdown` | heading hierarchy, repository-relative links (external URLs/anchors excluded), code-fence languages, front-matter presence |

An unrecognized file within a recognized metadata *language* (e.g. an
arbitrary `.json` data file) is explicitly skipped
(`SkipReason::UnrecognizedMetadataSchema`) -- distinct from
`UnsupportedLanguage` (the language itself isn't handled at all) and
never fabricated into facts, per Decision 0048's "do not flatten
arbitrary JSON keys" constraint.

## Dependency direction and fact vs. resolved-edge design

Package-level dependencies reuse the existing `GraphEdgeKind::DependsOn`
(already used for governance work-item dependencies) tagged
`GraphLayer::Package`/`DerivationClass::Manifest`, rather than a new
edge kind -- one directional canonical relation; reverse dependencies
are obtained by inbound traversal, not persisted, consistent with the
checkpoint directive's explicit preference. Dependency/reference targets
(a crate name, an npm package, a tsconfig `extends` path, a Cargo
workspace member) are recorded as **local fact nodes**, never as edges
resolved into another file's own manifest node -- resolving whether a
named dependency is even present in this repository (versus a registry
dependency with no local file at all) would require cross-file lookup
no per-file adapter performs, consistent with Decision 0045's "import
fact, not resolved target" precedent for source-language imports.

## Rust Exports emission (ROG-004)

Bare `pub` modules, functions, structs, enums, traits, and type aliases
now emit the already-declared (Decision 0045) but previously-unemitted
`GraphEdgeKind::Exports` edge, detected via Rust's `visibility_modifier`
AST node. Deliberately narrow: `pub(crate)`/`pub(super)`/`pub(in path)`
and private items never emit `Exports`; a `#[test]` function never does
even if marked `pub`, since test functions are not part of a crate's
public API surface. This is a syntactic visibility fact, not a claim of
full re-export (`pub use`) resolution. JS/TS/Python export detection was
not implemented this checkpoint -- a disclosed gap.

## Coverage-disclosure bug found and fixed

While inspecting a real RepoPact self-build's output, `aggregate_coverage`'s
`adapter_versions`/`relations_supported` (the durable manifest's own
coverage record) were found to only ever include the three source-
language adapters, even though 424 metadata files had genuinely been
processed by the four new metadata adapters. Fixed by merging
`metadata::adapter_versions()`/`relations_supported()` into the same map
a client already reads -- distinct from, and in addition to, the
separate `incremental::current_semantic_compatibility()` identity, which
tracks a different concern (safe-to-reuse) from this one (what actually
ran and what it claims to support).

## ROG-022: minified/generated policy and real memory evidence

`metadata::policy` adds two deterministic, centralized classifiers,
applied uniformly ahead of every adapter (source and metadata alike),
each backed by a new `SkipReason` variant:

- **Minified**: a file at or above 4 KiB with either zero newlines or an
  average line length above 500 bytes.
- **Generated**: a case-insensitive match against a small, specific
  marker set ("generated by", "@generated", "do not edit", ...) within
  the first 1 KiB of content only.

8 tests include explicit false-positive guards: ordinary prose
mentioning "generate" in an unrelated sentence does not trigger the
generated classifier; a marker outside the bounded scan window is not
detected (the scan is deliberately bounded, not a full-file search); a
short one-liner is not classified minified merely for being one line.

**Real parser-memory measurement** (previously the one ROG-022 clause
this WI had not addressed): a near-limit fixture (three ~2,000-line/
~100 KB Rust/Python/TypeScript files, plus a pathological single-line
deeply-nested Rust file) was built via the real engine binary on native
Windows, with peak memory measured directly from the OS process handle
(`PeakWorkingSet64`) using file-redirected I/O (a first attempt using
pipe-based stdin/stdout deadlocked and was corrected). Result: **16.27
MB peak working set** for the full build (6,009 nodes, 8,008 edges). The
pathological nested file was caught by the minified-content policy (a
single ~16 KB line, near-zero newline density) before ever reaching the
parser -- a real, *measured* instance of defense-in-depth, not merely a
unit-tested code path in isolation. This is engineering measurement, not
a claimed hard memory cap: the honest resource contract remains bounded
input size + parser timeout/cancellation + this measured (not guessed)
peak-memory data point.

## ROG-021: graph-level boundary evidence

5 new tests build a real graph and inspect its actual nodes/edges,
rather than asserting on `repopact-repository`'s own (separately tested)
walker logic:

- Excluded `target/`/`node_modules/`/`.venv/` trees produce zero graph
  nodes.
- `.git` internals (including a nested repository's own `.git`) are
  never indexed as source.
- A nested repository is classified `GraphNodeKind::NestedRepository`
  while its internals remain bounded.
- A `.env`-shaped file is still truthfully listed as a physical fact (a
  content digest) but its actual secret value never appears as any node
  label -- containment is a boundary decision, not an accidental content
  leak.
- A symlink pointing outside the repository is not traversed; content
  reachable only through it never appears in the graph (symlink creation
  succeeded on this Windows machine during the test run, not merely
  skipped as unsupported).

## Metadata preserves ROG-012 and ROG-013

8 new incremental-equivalence tests (reusing the exact same shared
harness as the source-language mutation matrix) prove byte-for-byte
equivalence between an incremental `graph.update` and a clean full
rebuild for Cargo.toml dependency/workspace-member changes, a
pyproject.toml dependency change, a package.json script change, a
tsconfig.json reference change, a CI workflow job change, and a
Markdown link change -- plus a dedicated test proving a metadata-only
edit reparses exactly the one manifest file, never the unrelated Rust/
Python/TypeScript source files in the same fixture. 1 new overlay test
proves the same properties (immediate in-memory update, unrelated-
contribution reuse, `basis=working_overlay` disclosure, zero durable
writes) for a metadata mutation through `SessionGraphState::reconcile`.

This work incidentally caught and fixed a genuine bug: an "unchanged"
metadata file's previously-durable contribution was silently dropped
during incremental reconciliation, because `read_semantic_contributions_by_file`
filtered reusable prior contributions by `layer == GraphLayer::Semantic`
only -- metadata facts live in `GraphLayer::Package`/`Build` and were
never being reused, while a clean full rebuild always regenerated them
correctly (masking the bug until the equivalence tests explicitly
compared incremental output against a clean rebuild byte-for-byte).
Fixed by filtering for "not Governance/Physical" (the two layers
exclusively produced by the always-global rebuild) instead of a single
named layer, so every current and future contribution layer is
reusable.

## Cross-platform proof

An 11-file mixed-metadata fixture (Rust with a module/test, Python,
TypeScript, `Cargo.toml` with a dependency, `pyproject.toml` with a
script entrypoint, `package.json` with a dependency/script,
`tsconfig.json` with `extends`, a CI workflow YAML, a Markdown document
with headings/links/a fence, an unrecognized generic JSON file, and a
malformed Rust file), confirmed byte-identical by SHA-256 of every
source file, was built independently on native Windows (two consecutive
builds, byte-identical `rog/` directories, proving same-platform
determinism too) and a freshly re-synced Linux-native WSL2 Debian 13
checkout. Result: identical fingerprint, node/edge counts, coverage, and
all 32 manifest+shard SHA-256 hashes. No macOS execution occurred or is
claimed.

## Real RepoPact self-build (engineering validation only)

766 files considered; `node_count=7,979` (governance 784 / physical 971
/ semantic 3,009 / build 17 / package 3,198); `edge_count=9,751`
(governance 1,613 / physical 1,772 / semantic 3,419 / build 15 /
package 2,932); 380 files complete, 0 partial, 0 failed, 175
unsupported-language, 211 policy-skipped (204 unrecognized-metadata-
schema, 5 generated-content, 1 binary-content, 1 minified-content);
~10.4s wall-clock. This is engineering validation only, explicitly not
S8/R1 evidence.

## Test results

126 `repopact-graph` tests total (83 pre-existing unchanged + 43 new).
Full `cargo test --workspace` green. `cargo fmt --check`/`cargo check
--workspace` clean. Canonical `repopact validate` clean. Broad Python
regression suite: see the evidence record's `closeout.python_regression`
for the exact count captured after this document was written.

## AC assessment

**Newly satisfied:** ROG-018 (Rust/Python/TS/JS source structure plus
relevant JSON/TOML/YAML/Markdown and manifest metadata, coverage
reported per language and relation, no call-graph implication),
ROG-021 (every boundary/exclusion clause now has direct graph-level
evidence), ROG-022 (every clause including the previously-missing
parser-memory-behavior evidence, now real and measured).

**Still pending, with the exact gap named:**

- **ROG-004** -- the edge taxonomy for containment/definitions/imports/
  exports/dependencies is real and tested (Exports newly emitted for
  Rust); Implements/Extends/References/Calls/UsesType remain correctly
  declared-but-unemitted (no false resolver). The AC's required taxonomy
  also names test and runtime relations, and neither exists as an edge
  class yet. Left pending on this named gap.
- **ROG-019** -- Cargo/Python/npm/tsconfig/CI facts are all real, but
  the AC's full conjunctive list also names test targets/fixtures,
  generated-boundary path classification (distinct from ROG-022's
  content-level policy), installer/package surfaces, and runtime/
  application entry points, none of which were implemented, nor was the
  npm `package.json` `workspaces` field. Left pending on these named
  gaps.
- **ROG-014-016, 023-029, 033-040** -- not attempted, explicitly out of
  scope for this checkpoint.
- **ROG-031/032** -- gained further genuine evidence but were not
  audited/measured as their AC's full text requires.

## Next recommended WI063 phase

The typed bounded query/orientation surface (ROG-023 through ROG-026)
is the natural next phase now that the graph carries real package/
build/CI metadata alongside source symbols. Alternatively, closing this
checkpoint's own disclosed ROG-004/019 gaps (test/runtime relations,
test-target/fixture/generated-boundary/installer-surface/runtime-
entrypoint nodes) first. A fresh architecture review should decide
between the two.

## 2026-09-14 operational-surface-completion checkpoint

Starting SHA `bae9c66a212e6f9a67338006e968037f278e4acc` (this
checkpoint's own commits through the ROG-012/013 equivalence proofs).
Primary targets: ROG-019, ROG-004, per Decision 0049.

### The compatibility-trap avoidance (Decision 0049)

Rather than adding new closed `GraphEdgeKind`/`ManifestKind` variants
for test/runtime/generated/installer surfaces -- which would recreate
the exact old-reader-hard-fails hazard Decision 0048 just fixed --
this checkpoint adds `GraphNodeRole`/`GraphRelationRole`: open,
syntax-validated (not closed-list-validated), string-backed newtypes,
carried as new additive `#[serde(default, skip_serializing_if =
"Option::is_none")]` optional fields on `GraphNode`/`GraphEdge`. Schema
stays v3 -- no major bump. The required load-bearing proof
(`durable::tests::an_old_v3_reader_shape_still_deserializes_a_node_edge_carrying_new_role_metadata`)
confirms a pre-0049 reader shape still deserializes new-v3 data
carrying role metadata, silently ignoring it, recovering a coarse
`kind`/`layer` that remains independently true -- exactly Decision
0049 section 2's precondition for adopting this design at all.

### Per-file adapter extensions

- Rust/Python/JavaScript adapters tag `SymbolKind::Test` symbols with
  `node_role=test_target`.
- TOML adapter: Cargo `[[bin]]`/`[[test]]` array-of-tables extraction
  (role=runtime_entrypoint/test_target, with `DependsOn+Test`/
  `Contains+Runtime` edges role=tests/entry_point_for); pyproject.toml
  `[tool.maturin]` recognition (role=installer_surface); `[project.
  scripts]` now tagged role=runtime_entrypoint (previously untagged).
- JSON adapter: `tauri.conf.json` recognized, reusing the existing
  `ManifestKind::JsonDocument` (not a new closed variant) with
  role=installer_surface, extracting productName/identifier/
  bundle.targets; `package.json` `workspaces` glob array recorded as
  raw, unresolved local facts (resolution against the repository's
  actual directory listing is deliberately deferred to the
  orchestrator, which alone has full projection access).

### Orchestrator-level cross-file passes (`semantic::operational`)

A new module holding every fact that requires knowing about *other*
files, which no per-file adapter is allowed to do: npm workspace glob
resolution (bounded to an exact-path or single `dir/*` wildcard shape,
matched against `package.json` paths the `SourceProjection` already
lists, never executing npm or touching the network, never descending
into `node_modules`), generated-boundary tagging (reusing the existing
ROG-022 `SkipReason::GeneratedContent` signal rather than re-reading
file content -- composing with, not replacing, the content-skip
policy), a small explicit known-generator-contract table (RepoPact's
own `types.ts`<-`generate-types.rs` and `dashboard.md`<-
`generate_dashboard.py`, emitting an edge only when both real files are
present in the graph), Rust implicit-default-binary detection
(`Cargo.toml` with no `[[bin]]` but a sibling `src/main.rs`), Cargo's
sibling `tests/*.rs` integration-test convention, and frontend/Python
test-file naming conventions.

Wired into both `semantic::extend` (full build) and
`incremental::plan_reconciliation` (the single delta-reconciliation
algorithm shared by `graph.update` and the session working-tree
overlay) -- the same shared contribution pipeline, not a second graph
builder (Decision 0049 section 6). It is *not* re-run by
`overlay::SessionGraphState::reconcile`'s targeted single-file watcher
fast path -- a disclosed, bounded limitation (locked in by
`overlay::tests::targeted_reconcile_does_not_yet_surface_a_new_operational_role_fact`),
not a silent gap: an operational cross-file fact from a single-path
watcher event becomes visible on the next full `refresh`/`open`, not
immediately.

### Compatibility identity bump

Every touched adapter's identity string bumped (rust/python/javascript
0.1.0->0.2.0, toml/json 0.1.0->0.2.0) and
`CURRENT_SEMANTIC_PIPELINE_VERSION` bumped to `semantic-pipeline-2`:
this checkpoint's per-file role tagging and new orchestrator passes
change what the same bytes produce, so a durable baseline written
before this checkpoint correctly falls back to a full rebuild rather
than silently reusing contributions that predate these operational
facts (proven generically by the pre-existing
`compatibility_mismatch_forces_a_full_semantic_rebuild` test, which
exercises the same equality check any pipeline-version bump relies on).

### ROG-012/013 proofs for the new mutation classes

Three new incremental-equivalence tests
(`npm_workspace_glob_addition_is_equivalent_to_full_rebuild`,
`cargo_bin_target_addition_is_equivalent_to_full_rebuild`,
`generated_marker_addition_is_equivalent_to_full_rebuild`) prove a
`graph.update` incremental path converges byte-for-byte with a clean
full rebuild for every new operational mutation class. Two new overlay
tests prove `refresh` surfaces a new orchestrator-derived operational
fact without ever writing `rog/`, and lock in the disclosed boundary
that a targeted `reconcile` does not (yet) re-run the cross-file
passes.

### Cross-platform determinism proof

A 12-file synthetic fixture covering every new operational surface
(npm workspace root + one glob-matched member, a Cargo crate with
explicit `[[bin]]`+`[[test]]`, a pyproject.toml with `[project.
scripts]`+`[tool.maturin]`, a `tauri.conf.json`, a file carrying a
recognized generated-content marker, a frontend `*.test.ts` file, a
Python `test_*.py` file, and a `fixtures/` directory containing a file
-- included specifically to prove that directory stays invisible to
the graph, per the pre-existing `IGNORED_PARTS` exclusion), hash-
verified byte-identical, was built independently via the raw engine
stdio protocol on native Windows and a freshly re-synced Linux-native
WSL2 Debian checkout (`~/repopact-linux`, synced via a git bundle of
this checkpoint's own commits, never `/mnt/c/...`). Result: identical
fingerprint (`def1387e02d5e8e8406d2f34ffff7ef3773a338dc17f4b66a458f29cb930f180`),
identical node_count/edge_count (34/46), and all 31 node+edge shard
SHA-256 hashes byte-identical between platforms (confirmed after
normalizing the CRLF/LF line-ending artifact introduced by shell
redirection on Windows, which is not part of the durable graph
content itself). The `fixtures/` file produced zero graph nodes on
both platforms, confirmed by direct grep of every shard -- the
directory-name exclusion from Decision 0044 is untouched by this
checkpoint. No macOS execution occurred or is claimed.

### Real RepoPact self-build (ROG-019 disclosure)

A real build against RepoPact's own live checkout (~8s wall-clock,
8,230 nodes / 10,071 edges including a temporary scratch fixture
present at build time; removed afterward) produced genuine, real
operational facts, not fixture-only evidence:

- `manifest:rust/apps/repopact-engine/Cargo.toml` ->
  `file:rust/apps/repopact-engine/src/main.rs` (Contains, layer=Runtime,
  role=entry_point_for) -- a real Rust implicit-default-binary
  detection, since `repopact-engine/Cargo.toml` declares no `[[bin]]`.
- `manifest:rust/apps/repopact-desktop/src-tauri/Cargo.toml` ->
  `file:rust/apps/repopact-desktop/src-tauri/src/main.rs` (same shape).
- `manifest:rust/apps/repopact-cli/Cargo.toml` ->
  `manifest-fact:...:bin_target:repopact-cli` (Contains, Runtime,
  entry_point_for) -- the *explicit* `[[bin]]` path, proving the
  implicit-binary pass correctly does not double-tag a crate that
  already declares one.
- `file:audits/reports/dashboard.md` ->
  `file:repopact/generate_dashboard.py` and
  `file:rust/apps/repopact-desktop/src/generated/types.ts` ->
  `file:rust/crates/repopact-desktop-api/src/bin/generate-types.rs`
  (DependsOn, Build, generated_by) -- both entries in the known-
  generator-contract table fired against real files.
- `manifest:rust/crates/repopact-mutation/Cargo.toml` ->
  `file:rust/crates/repopact-mutation/tests/verification_post_validation.rs`
  and `manifest:rust/crates/repopact-desktop-api/Cargo.toml` ->
  `file:rust/crates/repopact-desktop-api/tests/verification_validation_parity.rs`
  (DependsOn, Test, tests) -- Cargo's own sibling-`tests/`-directory
  convention, against the two real integration-test directories this
  repository actually has.

**Honest disclosure, not silently omitted:** RepoPact's own single
`package.json` (`rust/apps/repopact-desktop/package.json`) does not
declare `workspaces`, so npm workspace-member resolution has no real
self-repo instance to prove against this checkpoint -- it is proven
only via the synthetic fixture and unit tests above, per this
checkpoint's own instruction to disclose wherever RepoPact has no
self-instance rather than fabricate one. Similarly, no `test_fixture`
node role exists: RepoPact's own `fixtures/`-named directories
(`conformance/fixtures`) are architecturally invisible to the entire
graph via the pre-existing `IGNORED_PARTS` exclusion (Decision 0044),
and no other on-repo fixture convention (e.g. a `conftest.py`) was
found to ground a real instance -- Decision 0049's alternatives
section records this as a deliberate deferral, not an oversight.

### Performance evidence (engineering validation only, not ROG-032 closeout)

- Full build of RepoPact's own repository: 7.9-8.2s wall-clock (two
  runs), 8,230 nodes / 10,071 edges (including the temporary scratch
  fixture noted above).
- No-op `graph.update` immediately after a full build: 4.1s wall-clock
  (dominated by the bounded WI057 git invocation and re-walking the
  source projection to compute the fingerprint match, not by semantic
  reparsing -- zero files reparsed).
- Single-file operational-metadata edit (`package.json` `scripts`
  addition) via `graph.update`: 7.7s wall-clock, 1 file modified, 1
  file reparsed, 774 files reused unparsed -- consistent with the
  existing ROG-012 contribution-reuse behavior, now proven to hold for
  this checkpoint's new metadata shape too.
- Real-repo operational role/edge density: 586 node roles
  (test_target=565 dominated by real pytest/Vitest/Cargo test symbols
  and files; generated_surface=6; installer_surface=10;
  runtime_entrypoint=5) and 12 relation-role edges (tests=4,
  workspace_member=1 [from the temporary scratch fixture --
  RepoPact itself has none], entry_point_for=5, generated_by=2) across
  the real repository plus the temporary fixture.

This is engineering measurement on one developer machine, not a
claimed benchmark contract and not ROG-032 closeout -- ROG-032's full
dedicated benchmark suite (local-cache size, clean-clone load time,
query latency, branch/merge rebuild cost) remains unattempted, exactly
as the prior checkpoint disclosed.

### ROG-004 relation matrix

| Relation category | Status | Evidence |
|---|---|---|
| Physical containment | Emitted | `GraphEdgeKind::Contains`, layer=Physical (`physical.rs`) |
| Definitions | Emitted | `GraphEdgeKind::Defines` (all four source adapters) |
| Imports | Emitted | `GraphEdgeKind::Imports` (all four source adapters, fact not resolved target) |
| Exports | Emitted | `GraphEdgeKind::Exports` (Rust bare-pub items, prior checkpoint) |
| Dependencies | Emitted | `GraphEdgeKind::DependsOn`, layer=Package (TOML/JSON adapters) |
| Reverse dependencies | Emitted (governance) / Query-derived (package) | `GraphEdgeKind::ReverseDependency` emitted for governance work-item deps (`lib.rs`); package-level reverse lookups are inbound traversal over the same `DependsOn` edges by deliberate design (Decision 0048 section 7, Decision 0049 section 5) -- never a duplicate inverse edge |
| References (exact) | Unsupported at current tier | `GraphEdgeKind::References` predeclared (Decision 0045), never emitted; disclosed via `relations_supported` coverage metadata, never presented as a fact |
| Calls (where supported) | Unsupported at current tier | `GraphEdgeKind::Calls` predeclared, never emitted; same disclosure |
| Implementation relations (where supported) | Unsupported at current tier | `Implements`/`Extends`/`UsesType` predeclared, never emitted; same disclosure |
| Build relations | Emitted | `GraphEdgeKind::DependsOn`, layer=Build, role=generated_by (this checkpoint, `semantic::operational::emit_known_generator_contracts`) |
| Package relations | Emitted | `Contains`/`DependsOn`/`BelongsToWorkspace`, layer=Package (TOML/JSON adapters, npm workspace resolution) |
| Test relations | Emitted | `GraphEdgeKind::DependsOn`, layer=Test, role=tests (this checkpoint: Cargo `[[test]]`, sibling `tests/*.rs` convention) |
| Runtime relations | Emitted | `GraphEdgeKind::Contains`, layer=Runtime, role=entry_point_for (this checkpoint: Cargo `[[bin]]`, implicit binary, `pyproject.toml` scripts) |
| Governance applicability | Emitted | `SupportedBy`/`SupportsWorkItem`/`OwnedBy`/`Affects`/`Supersedes`/`Concerns`/`ConstrainedBy`/`Intersects`/`AppliesTo`/`Allows` (pre-existing WI054 governance graph) |

Every emitted relation's coarse `GraphEdgeKind`/`GraphLayer` remains
independently true with its `relation_role` ignored (Decision 0049
section 3) -- none of the newly-emitted build/package/test/runtime
relations are presented with more certainty than the coarse edge
alone actually carries.

### Regression matrix

`cargo fmt --check` clean. `cargo check --workspace` / `cargo build
--workspace` clean. `cargo test -p repopact-graph`: 147/147 (133 at
this checkpoint's start + 2 role-tagging tests + 9 operational-pass
tests + 1 compatibility proof + 3 ROG-012 equivalence tests -- overlay
ROG-013 tests counted separately below). `cargo test -p
repopact-desktop-api`: 14/14 unaffected (no DTO/protocol surface
change this checkpoint beyond the additive `GraphNode`/`GraphEdge`
fields already used internally). Overlay module: 17/17 including the
two new ROG-013 tests. `cargo test --workspace`: green, zero failures.
`tests/test_graph_cli.py`/`tests/test_engine_client.py`: 12/12 (via
`REPOPACT_ENGINE` pointed at the freshly built debug binary). Broad
Python regression suite: 273 passed, 2 skipped, 18 subtests passed, 13
failed, 897.19s. All 13 failures are pre-existing and unrelated to
this checkpoint: 12 trace to one root cause
(`research.freshness-coverage-incomplete` for two unregistered
research documents, `research/arxiv-submission-prep.md`/
`research/paper-reconciliation-2026-09-13.md`, added by commits
`f6de78a`/`f1b7800` which land between the prior working-overlay
checkpoint and this one's actual starting commit `542cd5c` -- verified
by reproducing the identical failures with every commit from this
checkpoint stashed out) and 1 is the identical pre-existing
`cryptography`-module gap already disclosed in both prior WI063
checkpoints' evidence records. Neither touches WI050; neither was
worked around. `repopact validate --root .` independently reproduces
both root causes plus a third pre-existing, unrelated issue
(`owners.unowned-tracked-path` for `CITATION.cff`) -- the dashboard
staleness this checkpoint's own `work-item.json`/evidence changes
triggered was regenerated (`repopact dashboard --root .`) and is the
only validation issue this checkpoint caused or fixed.

### AC assessment

**Newly satisfied:** ROG-004 (the edge taxonomy's full conjunctive
list -- physical containment, definitions/imports/exports,
dependencies/reverse-dependencies, references/calls/implementation
relations *where supported* (none, disclosed), build/package/test/
runtime relations, governance applicability -- is now genuinely
complete per the matrix above; the test/runtime gap that blocked this
AC at the prior checkpoint is closed).

**Still pending, with the exact gap named:**

- **ROG-019** -- Cargo workspace/dependency, Python package/project,
  npm package/workspace, tsconfig, CI workflow, test-target, generated-
  boundary, installer/package-surface, and runtime/application-
  entrypoint facts are now all real and tested. The one remaining named
  clause is **fixtures**: RepoPact's own `fixtures/`-named directories
  are architecturally invisible to the entire graph via the
  pre-existing `IGNORED_PARTS` exclusion (Decision 0044), and no other
  on-repo fixture convention exists to ground a real instance --
  Decision 0049 deliberately deferred a `test_fixture` role rather than
  fabricate one. Left pending on this one named, exact gap; not marked
  satisfied on a partial conjunctive list.
- **ROG-014-016, 023-029, 033-040** -- not attempted, explicitly out of
  scope for this checkpoint.
- **ROG-031/032** -- gained further genuine evidence (this checkpoint's
  self-build stats and timing) but were not audited/measured as their
  AC's full text requires.

### Next recommended WI063 phase

The typed bounded query/orientation surface (ROG-023 through ROG-026)
is the natural next phase now that the graph carries real test/build/
runtime operational facts alongside package/CI/source metadata.
Closing ROG-019's one remaining named gap (a `fixtures/`-directory
convention, which would require either relaxing the `IGNORED_PARTS`
exclusion for that one name or introducing a distinct, narrower
fixture-visibility mechanism) is a real architectural decision, not a
small addition, and should be a deliberate choice by a future
directive rather than an incidental side effect of the query-API
phase. This checkpoint stops here per its own explicit instruction --
ROG-023-026 work does not begin.

## 2026-09-14 bounded-query-and-orientation checkpoint

Starting SHA `2157494aa28527b753bb46a3d1b5a87c905f20b4` (the accepted
operational-surface-completion checkpoint). Primary targets: ROG-023,
ROG-024, ROG-025, ROG-026, per Decision 0050.

### Query kernel architecture

One canonical module, `repopact_graph::query`, implements every
operation. It is a pure function of an already-materialized
`RepositoryGraph` (durable or `SessionGraphState::effective_graph()`)
plus a disposable, in-memory, `BTreeMap`-based index
(`GraphQueryIndex`) rebuilt fresh per call -- non-durable, non-
authoritative, never `rog/`-adjacent. No traversal logic is duplicated
in the engine binary, the CLI, or the desktop API; all three are thin
adapters that resolve typed params, call into the kernel, and forward
its typed result.

```text
                    RepositoryGraph
                         +
               graph state / coverage
                         |
                         v
                 pure query kernel
                         |
        +----------------+----------------+
        |                |                |
        v                v                v
  durable engine    session overlay    desktop API
      queries            queries            queries
        |                |                |
        +----------------+----------------+
                         |
                         v
               typed bounded results
```

### Target resolution

`NodeSelector`: `NodeId`, `RepositoryPath` (validated -- an absolute
host path or a `..` component is rejected as malformed, never
normalized), `WorkItemId`, `Package` (matched against a manifest's own
ecosystem-prefixed identity label, e.g. `cargo:name`/`npm:name`/
`python-project:name`, never a substring match), `Module` (a `Symbol`
node with `symbol_kind=Module`), and `Symbol` (name plus optional
path/symbol_kind/container disambiguation -- container is a best-
effort substring match against the symbol's own stable ID, disclosed
as such, not a claim of exact structural parsing). The AC's "type"
selector clause is satisfied via `Symbol{symbol_kind: Some(Type)}`,
not a seventh selector variant. Resolution always returns exactly one
of `Exact`/`Ambiguous(candidates)`/`NotFound` -- proven against a real,
previously-unknown case: RepoPact's own `repopact-desktop` name is
genuinely ambiguous between a real npm `package.json` and a real Cargo
`src-tauri` crate that happen to share it.

### Bounds, pagination, and cursors

One `QueryBounds` type (`max_nodes`, `max_edges`, `max_depth`, layer/
relation-kind filters, `max_output_bytes`, `estimated_token_budget`,
`page_size`, `cursor`, `compact`, `allow_stale`) is shared by every
operation. Every hard bound is proven to trigger (a dedicated test per
bound) rather than silently drop content -- an operation exceeding a
bound emits an explicit warning naming which bound fired. Pagination
uses an opaque, length-bounded, base64url-encoded cursor (implemented
without adding a new dependency) binding `query_contract_version` +
graph fingerprint + operation name + a SHA-256 hash of the normalized
request + a continuation position; a cursor minted under a different
fingerprint, operation, target, or filter set is rejected with a
specific typed reason, never silently continued against different
graph state. A dedicated test proves paging through a bounded result
set reconstructs the same content as the unpaginated call, with no
duplicates or gaps.

### Freshness and coverage policy

`open_durable_graph` is the single durable-graph query entry point:
absent returns a typed `graph_absent` result with build guidance;
unsupported/corrupt fail closed; stale is refused by default with a
typed result callers can branch on, openable only via an explicit
`allow_stale`, which still always discloses `durable_freshness=stale`
-- no presentation layer may drop that warning (proven by a dedicated
test that mutates source after a build). Every query envelope carries
the same three-axis `EffectiveGraphStatus` (`basis`/`durable_freshness`/
`coverage`) Decision 0047 already established for the desktop
`GraphView` -- not a fourth, flattened status. A session query against
`SessionGraphState::effective_graph()` discloses `basis=working_overlay`
whenever dirty working-tree state contributed (proven by a desktop-api
test resolving a file added after the session opened, before any
durable graph existed at all).

### Fact vs. navigation-hint separation and structural impact

`graph.orient`'s `navigation_hints` are a separately-typed, reason-
coded (`target_source`/`owning_manifest`/`direct_dependency`/
`direct_dependent`/`relevant_test`/`runtime_entrypoint`/
`applicable_governance`), deterministically rank-ordered list -- never
mixed with the `facts` a resolved target's containment/dependency/
test/governance sections carry, and never model-scored. `graph.impact`
explicitly labels its result `impact_semantics: "structural_only"`:
reverse dependencies, known test targets, and build/package/runtime
surfaces are structural graph facts, never a claim of proven runtime
behavior or compile/test outcome.

### ROG-019 fixture-coverage disclosure (unchanged, still pending)

`graph.tests` and `graph.impact` always emit the fixture-topology
coverage warning this checkpoint's own directive requires; `graph.
orient` includes it whenever the resolved target's neighborhood could
plausibly include fixtures. No fixture node or role was fabricated to
manufacture ROG-019 satisfaction -- it remains pending on exactly the
one gap named in the prior checkpoint.

### Legacy `graph` operation

Audited: `RepoPactCore::graph_snapshot` performs an unbounded, fresh
`RepositoryGraph::build` on every call, with no freshness/coverage
envelope, and has zero callers anywhere in this codebase (not the
Python CLI, not the desktop API). Classified per Decision 0050 section
12 as a frozen, pre-ROG compatibility surface (option A): left exactly
as-is, explicitly excluded from the new typed query contract.

### Engine protocol, CLI, and desktop plumbing

`graph.resolve`/`context`/`neighbors`/`path`/`dependencies`/
`dependents`/`tests`/`governance`/`impact`/`orient` are registered in
engine dispatch (`query_ops.rs`) and `capabilities()`. `repopact graph
resolve|context|neighbors|path|dependencies|dependents|impact|tests|
governance|orient` use explicit, mutually-exclusive typed target flags
(`--id`/`--path`/`--work-item`/`--package`/`--module`/`--symbol[+
--symbol-path]`) that map directly into the engine's `NodeSelector` --
never a bare positional string the CLI has to guess the meaning of;
commands other than resolve/orient first call `graph.resolve` client-
side and print an honest ambiguous/not-found result rather than
guessing. `DesktopSession::graph_query(GraphQueryRequest)` runs the
identical kernel against the session overlay; `GraphQueryRequest`
mirrors the engine's own per-operation params so the CLI, engine, and
desktop boundary all agree on one wire shape. Generated TypeScript
bindings for the full query surface were added and verified (`tsc
--noEmit`, full vitest suite) -- the pre-existing `GraphNode`/
`GraphEdge` TS staleness from before this checkpoint was disclosed,
not fixed (out of scope).

### Authority and safety proofs

`GraphQueryEngine` borrows `&RepositoryGraph`, never `&mut` -- no
method it exposes can mutate the graph by construction; a dedicated
test runs `orient`/`dependencies` and asserts the graph is unchanged
(`PartialEq`) before and after. `DesktopSession::graph_query` requires
an open session (no ambient graph access). Path selectors reject
absolute host paths and `..` escapes. A `CountingGitRunner`-backed
test proves 8 different query operations issue zero additional Git
invocations beyond the graph's own construction (WI057 preserved). A
dedicated test deletes every source file after `graph.build` and
confirms `resolve`/`orient` still succeed purely from the loaded
graph -- the query kernel needs nothing beyond what is already in
memory.

### Cross-platform determinism and real RepoPact evidence

A 7-file fixture (a 3-item governance dependency chain plus a Cargo
crate with explicit `[[bin]]`/`[[test]]`), hash-verified byte-
identical, produced byte-identical `graph.build`/`resolve`/
`dependencies`/`path`/`orient` output on native Windows and a freshly
re-synced Linux-native WSL2 Debian checkout. Against RepoPact's own
live repository (8,491 nodes): `graph.orient(work_item_id=063)`
returned 3 real dependencies (054/056/058) and 16 rank-ordered
navigation hints; `graph.orient` against `rust/crates/repopact-graph/
src/lib.rs` and the `repopact-graph` package both resolved exactly
(the package result carrying 11 real package_surfaces);
`graph.resolve(symbol=SessionGraphState)` resolved uniquely; and
`graph.resolve(package=repopact-desktop)` genuinely disclosed
`Ambiguous` between the repository's real npm `package.json` and its
real Cargo `src-tauri` crate, both named `repopact-desktop` -- a real
naming collision this checkpoint discovered and correctly disclosed
rather than silently resolving.

### Performance evidence (engineering validation only, not ROG-032 closeout)

On a synthetic fixture: cold graph-open (durable load + the pre-
existing freshness projection walk) ~55ms, query-index build ~0.4ms,
warm per-operation queries (resolve/dependencies/dependents/tests/
governance) all under 150us, `orient` (composing several sub-queries)
~340us. A real-repo round trip through the engine binary is ~3.1s,
attributable almost entirely to the pre-existing freshness-check
projection walk over ~800 files, not this checkpoint's query kernel --
cold and warm timings are reported separately, never combined into one
misleading number.

### Test results

199 `repopact-graph` tests (147 pre-existing + 52 new). `repopact-
desktop-api`: 17 (14 pre-existing + 3 new). Full `cargo test
--workspace` green. `cargo fmt --check`/`cargo check --workspace`
clean. `tsc --noEmit` and the full frontend vitest suite (16/16) pass;
`npm run types:check` confirms generated bindings are fresh.
`tests/test_graph_cli.py`/`tests/test_engine_client.py`: 20/20 (8 new
query-command tests). Canonical `repopact validate` reports only the
two pre-existing, unrelated diagnostics already disclosed in the prior
checkpoint (unregistered research-claim documents, `CITATION.cff`
ownership) plus a dashboard staleness this checkpoint's own changes
caused and then fixed by regenerating it. Broad Python regression
suite: see the evidence record's `closeout.python_regression`.

### AC assessment

**Newly satisfied:** ROG-023 (typed query coverage for status/
resolution/context/neighbors/path/dependencies/reverse-dependencies/
impact/tests/governance, every result carrying stable IDs/relation
kinds/source references/freshness/provenance), ROG-024 (`graph.orient`
handles work item/path/package/module/symbol directly and type via
`Symbol{symbol_kind: Type}`, with facts and navigation hints kept
structurally separate), ROG-025 (explicit, tested bounds/pagination/
cursors/output budget/compact mode with no silent dropping), ROG-026
(practical public CLI plus versioned engine-protocol operations,
consumed as typed structured results by Python/desktop -- never
presentation-string scraping; rich Workbench UI remains ROG-027).

**Still pending, with the exact gap named:**

- **ROG-019** -- unchanged: the one remaining named clause (test-
  fixture topology) stays intentionally unclosed, per this
  checkpoint's own directive not to touch `IGNORED_PARTS` or fabricate
  a fixture fact. The query API discloses this limitation wherever
  relevant (`graph.tests`/`graph.impact`/`graph.orient`).
- **ROG-014-016, 027-029, 033-040** -- not attempted, explicitly out
  of scope for this checkpoint.
- **ROG-031/032** -- gained further genuine evidence (this
  checkpoint's cross-platform proof and query-latency measurements)
  but were not audited/benchmarked as their AC's full text requires.
- **ROG-037/040** -- this checkpoint's read-only-by-construction proof
  and the desktop session-authority test are evidence toward these
  criteria, not a claim of their full closeout, which spans mutation/
  admission/Tauri-capability surfaces this checkpoint did not touch.

### Next recommended WI063 phase

Either ROG-014-016 (adoption/backfill/clean-clone) or ROG-027-029
(the Workbench repository-map UI plus branch/merge workflow) is the
natural next phase now that a real, bounded, typed query surface
exists for a future UI or adoption tooling to consume. This checkpoint
stops here per its own explicit instruction -- neither begins, nor
does S8 R1, a persistent parser cache, or a SQLite/local graph
database.

