---
id: 0045
title: Deterministic semantic extraction substrate and ROG schema v2
status: accepted
date: 2026-09-13
supersedes: []
---

# 0045: Deterministic semantic extraction substrate and ROG schema v2

## Context

Decision 0044 established the durable Repository Orientation Graph (ROG)
projection: authority, durable path, schema/version contract, canonical
serialization, sharding, source projection, fingerprint, and freshness
model, all proven at schema version 1 for physical topology
(`Repository`/`Directory`/`File`/`Workspace`/`ConfigurationFile`/
`NestedRepository` nodes; `Contains`/`BelongsToWorkspace`/`ConfiguredBy`/
`Intersects` edges).

WI063's next phase extracts deterministic code-semantic facts (module,
function, type, import declarations and similar) from source files.
Before doing so, two questions need durable answers: which parser
substrate to use, and how to add new node/edge kinds without breaking
Decision 0044's same-major compatibility promise.

### The schema-compatibility hazard

`GraphNodeKind` and `GraphEdgeKind` are closed Rust enums with
`#[serde(rename_all = "snake_case")]` and no catch-all/unknown variant.
`serde_json` deserializes a closed enum by exact string match; an
unrecognized variant string is a hard deserialization error. A focused
test added before this decision
(`repopact_graph::durable::tests::a_v1_only_reader_cannot_deserialize_an_unrecognized_node_kind_string`
and its edge-kind counterpart) confirms this directly: a JSONL line naming
a `"symbol"` node kind or a `"defines"` edge kind fails to parse against
the schema-v1 enum set. Adding semantic variants directly to these enums
while leaving `graph_schema_version` at `1` would therefore make an older
v1-only reader fail on a shard the manifest still claims is version 1 --
exactly the "unknown major version interpreted optimistically" failure
mode Decision 0044 section 4 already forbids, just arriving one version
earlier than expected.

## Decision

### 1. Parser substrate: Tree-sitter

Tree-sitter is selected over language-specific parser stacks (Rust `syn`,
a Python-specific parser, SWC/Oxc-style JS/TS parsers), evaluated on:

- **one common adapter architecture**: a single `tree_sitter::Parser` +
  per-language grammar crate gives one `Node`/`Tree`/cursor API across
  Rust, Python, JavaScript, TypeScript, and TSX. Language-specific stacks
  would require four structurally different parser APIs and four
  different AST shapes normalized by hand -- a materially larger
  adapter-architecture cost for this checkpoint's language count, and it
  would work against ROG-002's "one canonical graph/query authority"
  principle by fragmenting the extraction layer itself.
- **determinism**: both classes are deterministic for well-formed input.
  Tree-sitter's error-recovery grammar additionally produces deterministic
  partial output (explicit `ERROR`/`is_missing` nodes) for malformed
  input, rather than aborting -- directly usable for ROG-022's "a parser
  failure should degrade coverage explicitly" requirement.
- **incremental capability**: Tree-sitter has first-class incremental
  re-parse (`Tree::edit` + old-tree reuse). Most language-specific parsers
  do not. Not exercised this phase (ROG-012/013 are explicitly deferred),
  but it keeps the door open for the phase that does.
- **malformed-input behavior**: proven directly (see spike below) via
  `parse_with_options` + a progress callback that aborts a parse; the
  parser returns `None` rather than blocking or crashing.
- **cancellation/resource bounding**: Tree-sitter 0.27 removed the
  deprecated `set_timeout_micros` timeout API entirely (confirmed by
  direct inspection of the 0.27.0 source: no `timeout_micros` symbol
  exists anywhere in the crate). The current mechanism is
  `Parser::parse_with_options(&mut read_callback, old_tree,
  Some(ParseOptions { progress_callback: Some(&mut |state: &ParseState|
  ControlFlow<()> ) }))`; returning `ControlFlow::Break(())` aborts the
  parse. `Parser::reset()` must be called before reusing a parser instance
  after a cancelled parse. Both behaviors were spiked and proven (see
  below) rather than assumed from documentation.
- **Windows/Linux portability**: grammar crates are portable C sources
  compiled via the existing `cc` build-dependency pattern already used
  elsewhere in the workspace; no system Tree-sitter install or external
  library is required. Proven by a real compile spike on this Windows
  machine; the existing Rust workspace already builds identically on
  Linux (WSL2 Debian 13), and Tree-sitter's C sources carry no
  platform-specific code paths relevant to this project.
- **packaging cost**: no `libclang`/system dependency; grammar crates
  compile to modest static object code linked into the engine binary.
  Measured release-binary size delta recorded in
  `implementation-progress.md` (this decision does not duplicate that
  measurement).
- **license**: Tree-sitter core and all four grammar crates evaluated
  (`tree-sitter-rust`, `tree-sitter-python`, `tree-sitter-javascript`,
  `tree-sitter-typescript`) are MIT-licensed.
- **maintenance activity**: all five crates have current crates.io
  releases as of this session (`tree-sitter` 0.27.0, `tree-sitter-rust`
  0.24.2, `tree-sitter-python` 0.25.0, `tree-sitter-javascript` 0.25.0,
  `tree-sitter-typescript` 0.23.2, confirmed via `cargo info`/`cargo add
  --dry-run` against the live crates.io index during this session, not
  from memory).

**MSRV finding:** `tree-sitter` 0.27.0 declares `rust-version: 1.90`
(confirmed via `cargo info tree-sitter`). RepoPact has **no documented
MSRV contract** anywhere in the repository (no `rust-version` field in any
workspace `Cargo.toml`, no MSRV statement in `docs/` or `README.md`,
confirmed by direct search). The installed toolchain on this development
machine is rustc/cargo 1.96.0, well above 1.90, so this pin does not
require raising any *documented* public floor -- there is none to raise.
This decision establishes 1.90 as the RepoPact Rust workspace's de facto
minimum from this point forward (transitively required by the graph
crate) and records it here since no other document currently does.

**Spike evidence:** a scratch crate outside the workspace (not committed)
added `tree-sitter = "0.27"`, `tree-sitter-rust = "0.24"`,
`tree-sitter-python = "0.25"`, `tree-sitter-javascript = "0.25"`, and
`tree-sitter-typescript = "0.23"` together, compiled cleanly, and at
runtime: parsed real Rust, Python, JavaScript, TypeScript, and TSX source
into the expected distinct root node kinds (`source_file`, `module`,
`program`, `program`, `program`); triggered `parse_with_options` with a
progress callback that returns `ControlFlow::Break(())` on the first
check against a large (200,000-line) synthetic Rust source, confirming
the parse returns `None` (cancelled) rather than completing; called
`Parser::reset()` and successfully reused the same parser instance for a
subsequent unrelated parse. Exact pinned versions are recorded in
`rust/Cargo.lock` once the real adapter crate depends on them.

### 2. Graph schema v2 for semantic content

`graph_schema_version` becomes `2` for any durable graph containing
semantic (`GraphLayer::Semantic`) content. Schema v1 (physical-only,
proven at Decision 0044) remains a fully valid, permanently readable
format -- v2 is additive, not a replacement:

- The reading implementation supports both major version `1` and major
  version `2`. An unknown major version (anything other than 1 or 2)
  still fails/reports unsupported, per Decision 0044 section 4, unchanged.
- `repopact graph build` always writes the newest schema version the
  running binary's node/edge vocabulary requires. Once semantic support
  lands, that is always version 2 (a v1-only build with zero semantic
  content is not specially preserved as v1 output -- the binary that can
  produce v2 always does, since v2's node/edge vocabulary is a superset of
  v1's and a v2 reader understands both).
- There is no in-place schema migration. A v1 durable graph becomes a v2
  durable graph only via a full, deterministic `repopact graph build`
  rebuild -- the same backfill mechanism Decision 0044 section 14 already
  established for enabling the graph at all. This preserves the existing
  "no opaque migration" principle rather than inventing a second one for
  schema version specifically.
- Fixture coverage (added alongside this decision, before language
  adapters) proves: an existing v1 durable graph remains readable and
  passes structural validation unchanged; rebuilding that same repository
  produces a valid, deterministic v2 graph; a v1-schema-version manifest
  is never emitted once the binary has semantic node/edge kinds in its
  vocabulary.

### 3. New vocabulary is a symbol model, not per-language node kinds

`GraphNodeKind` gains one new variant, `Symbol`, carrying a separate typed
`SymbolKind` field (module/namespace, function, method, struct/class/type,
enum, trait/interface/protocol, implementation block, type alias,
constant/static, macro, test) rather than per-language variants such as
`RustStruct`/`PythonClass`/`TsInterface`. This is deliberate: a new
ordinary symbol category in an existing supported language (for example,
Rust `union`, or a new TypeScript declaration form) becomes a new
`SymbolKind` value, not a new schema-major-version-bumping
`GraphNodeKind` variant, provided the receiving field itself is
represented in a schema-evolution-safe way (see below) -- keeping schema
v2 stable as language coverage within it grows.

`GraphEdgeKind` gains the semantic relation vocabulary needed by this
checkpoint and predeclared for the remainder of WI063: `Defines`,
`Imports`, `Exports`, `Implements`, `Extends`, `References`, `Calls`,
`UsesType`. Only `Defines` and `Imports` are actually emitted this
checkpoint (see `implementation-progress.md` for the exact per-language
matrix); the remainder are declared now so a later phase does not need
another schema-major bump merely to emit a relation this decision already
anticipated. Predeclaring this vocabulary is not permission to claim any
of these relations exist before an adapter actually emits them.

**On "does adding `SymbolKind` values require a v3":** `SymbolKind` values
are transmitted as plain strings inside the existing `#[serde(default)]`-
safe structures, and a reader that does not recognize a particular
`SymbolKind` string can still validate structural integrity (manifest,
shard hashes, duplicate IDs, dangling edges) without needing to interpret
the symbol category semantically -- unlike `GraphNodeKind`/`GraphEdgeKind`,
whose exact identity governs graph traversal and edge validity. If
practical experience during Phase 3+ shows `SymbolKind`'s closed-enum
representation causes the same hazard, that decision will be revisited
explicitly rather than assumed away here.

### 4. Graph-local source location, not a change to shared `SourceRef`

`SourceRef` (`= RecordRef { kind, id, path }`) is used throughout
governance and remains unchanged -- no line/byte/column fields are added
to it. A new, optional, graph-local `GraphSourceLocation` type (byte-range
start/end plus zero-indexed row/column start/end, UTF-8 byte offsets, LF
line-ending convention matching the rest of the durable format) is added
to `GraphNode`/`GraphEdge` as `#[serde(skip_serializing_if =
"Option::is_none")] location: Option<GraphSourceLocation>`. Existing v1
nodes/edges (and any v2 node/edge without a meaningful span, such as
governance nodes) deserialize cleanly with `location: None`. A source
span is explanatory metadata only -- it is never part of a symbol's
primary identity (see below) and is never used as a correctness
precondition anywhere in validation.

### 5. Stable symbol identity

A symbol's graph ID is a deterministic function of:

```text
language + normalized repository-relative file path
         + semantic container / qualified path
         + symbol kind
         + declared name
```

Line number is never a component. Legitimate duplicate declarations
(overloads, `impl` blocks for the same type, re-exported names) are
disambiguated, when the above key alone is insufficient, by a normalized
signature/header digest (a deterministic hash of the declaration's
syntactic header, not its full body) appended to the key -- never by
source line number or traversal ordinal. Anonymous constructs with no
stable semantic identity (anonymous closures, unnamed impl targets, etc.)
are omitted from the graph rather than assigned an unstable synthetic ID.

### 6. Parser-neutral adapter boundary

A `SemanticAdapter` trait is added to `repopact-graph`, accepting a
bounded, already-approved source input (repository-relative path,
language identity, byte content, a resource policy, and a cancellation
budget) and returning normalized graph contributions (nodes/edges in the
existing `GraphLayer::Semantic`/`DerivationClass::Parser` vocabulary) plus
a coverage/diagnostic report. The trait signature exposes no
Tree-sitter-specific type (no `tree_sitter::Node`, `Tree`, `Query`,
`TreeCursor`, or grammar ID) across its boundary -- a future non-
Tree-sitter adapter (or a higher-fidelity language-specific tool adapter)
implements the same trait without the canonical graph DTOs changing.

### 7. Adapters do not perform their own I/O or repository discovery

The graph orchestrator (source projection) decides which files are
eligible and reads their bytes through the existing repository
containment path (`Repository::relative_path`/`resolve_within_root`
family). Adapters receive already-loaded content; they do not walk
directories, invoke Git, follow symlinks, or reopen arbitrary paths
themselves. This preserves the WI057 bounded-Git-invocation guarantee
unconditionally -- semantic extraction adds zero Git invocations per file
or per symbol.

## Alternatives considered

- **Per-language node-kind enums** (`RustStruct`, `PythonClass`,
  `TsInterface`, ...): rejected. Every new language, and every new symbol
  category within an existing language, would require a new closed-enum
  variant and (per the schema-hazard analysis above) plausibly another
  schema-major bump. The `Symbol` + `SymbolKind` shape absorbs ordinary
  language growth without touching the closed `GraphNodeKind` enum.
- **Leaving semantic content at schema version 1**: rejected outright,
  confirmed unsafe by the compatibility test added before this decision.
- **In-place schema migration (rewriting a v1 manifest/shards to v2
  without a full rebuild)**: rejected. It would require understanding and
  faithfully preserving physical-layer content while injecting semantic
  content computed some other way, recreating exactly the "opaque
  migration" risk Decision 0044 already avoided for the durable format's
  existence at all. A full deterministic rebuild is simpler, already
  proven, and already the established backfill mechanism.
- **Retrofitting line/byte fields onto the shared `SourceRef`/`RecordRef`
  type**: rejected. That type is used throughout governance where line/
  byte semantics are meaningless, and overloading it would either force
  every governance call site to populate meaningless span fields or make
  the type's meaning context-dependent. A separate, optional, graph-local
  type keeps the governance type's contract exactly as WI054 defined it.
- **Language-specific parser stacks (`syn`, a Python-only parser, SWC/
  Oxc)**: not rejected on principle -- explicitly evaluated and found to
  cost more architecturally for this checkpoint's language count and
  malformed-input/cancellation requirements than Tree-sitter's one common
  adapter shape. A future higher-fidelity adapter for a specific language
  remains possible under the same `SemanticAdapter` trait if evidence
  later justifies it.

## Consequences

WI063 gains a genuinely extensible, schema-safe path from physical
topology to code semantics without breaking the compatibility promise
Decision 0044 made explicit. The `Symbol`/`SymbolKind` shape trades a
small amount of type-level specificity (a `SymbolKind` typo is a runtime
string mismatch, not a compile error at every call site) for materially
better schema-evolution headroom. The cost is a new native-code dependency
(Tree-sitter core plus four grammar crates) with a measured binary-size
and compile-time impact, recorded in `implementation-progress.md`, and a
now-explicit (if previously undocumented) 1.90 Rust-toolchain floor for
this workspace.
