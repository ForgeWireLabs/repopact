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

