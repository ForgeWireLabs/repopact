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
