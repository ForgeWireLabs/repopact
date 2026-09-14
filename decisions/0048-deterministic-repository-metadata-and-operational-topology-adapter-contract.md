---
id: 0048
title: Deterministic repository metadata and operational topology adapter contract
status: accepted
date: 2026-09-14
supersedes: []
---

# 0048: Deterministic repository metadata and operational topology adapter contract

## Context

Decisions 0044-0047 established the durable ROG projection, deterministic
Tree-sitter source-language extraction, incremental contribution reuse,
and the session working-tree overlay. WI063 ROG-018/019/021/022 require
the graph to also understand structured project metadata (JSON, TOML,
YAML, Markdown) and operational topology (Cargo/Python/JS-TS package and
build facts, CI workflow references, runtime entrypoints) -- otherwise
the graph cannot orient a real repository's dependency, build, or CI
structure, which the later bounded query/orientation layer (ROG-023-026)
needs to consume.

Before adding any new vocabulary, this decision audits whether it can be
added under schema v2 without breaking the compatibility contract
Decision 0044 established ("same major version evolution must remain
readable by an implementation supporting that major").

### Schema compatibility audit

`GraphNodeKind`, `GraphEdgeKind`, and `SymbolKind` are closed serde enums
with `#[serde(rename_all = "snake_case")]` and no `#[serde(other)]`
catch-all variant. A focused test
(`durable::tests::a_reader_cannot_deserialize_an_unrecognized_node_kind_string`,
proven before schema v2 existed, and its edge-kind counterpart) already
established that an implementation lacking a given variant hard-fails
deserializing a shard line naming it, rather than degrading gracefully.
This checkpoint adds a further test,
`a_nested_closed_enum_field_carries_the_identical_hazard_as_a_top_level_kind`,
proving this is not limited to the top-level `kind` discriminator: a
closed enum nested one level deeper inside `GraphNode` (the exact shape
`SymbolKind` already has, and any new `ManifestKind` would have) carries
the identical hazard. Decision 0045 section 3's framing -- that growing
`SymbolKind` "does not require another schema-major bump" -- is true only
in the narrow sense that a *pre-Symbol* v1 reader never had the field at
all; it does not mean a v2-era reader compiled before a new `SymbolKind`
variant existed can read a shard naming that variant. Every closed-enum
vocabulary addition, wherever it is nested, requires the same
compatibility treatment as a top-level `GraphNodeKind`/`GraphEdgeKind`
addition.

**Conclusion: schema v3 is genuinely necessary for this checkpoint**,
because it adds real new closed-enum vocabulary (a `ManifestKind` field
and new `SourceLanguage` variants for JSON/TOML/YAML/Markdown, both
serialized into the durable schema). This is not a decision to silently
extend v2; `CURRENT_GRAPH_SCHEMA_VERSION` moves to 3 and
`SUPPORTED_GRAPH_SCHEMA_VERSIONS` becomes `[1, 2, 3]` as part of this
decision, following the same treatment Decision 0045 gave the v1-to-v2
transition: v1 and v2 remain permanently valid and readable; v3 is
additive vocabulary, not a rewrite; migration from v1/v2 to v3 is full
deterministic rebuild only, never in-place.

Two kinds of change remain schema-major-safe and do **not** trigger this
audit: adding new optional struct **fields** with
`#[serde(default, skip_serializing_if = "Option::is_none")]` (the
precedent Decision 0046 already established for `source_digest` and
`semantic_compatibility`), and extending open, non-enum values (plain
strings, maps). This decision does not change that.

## Decision

### 1. Metadata adapter boundary

A new, distinct trait -- conceptually `MetadataAdapter` (identity,
accepts, extract) -- parallels `SemanticAdapter` (Decision 0045 section
6) for structured, non-source-language files. It is parser-neutral: no
`serde_json`/`toml`/YAML-parser AST type crosses into a canonical graph
DTO. Both trait families are dispatched from the same per-file
classification point (`semantic::build_file_contribution`), so a
metadata file and a source-language file are both, from the orchestrator's
perspective, "one file, one contribution" -- there is no second,
parallel full/incremental/overlay pipeline for metadata.

### 2. Deterministic output, repository-relative identity, source provenance

Every metadata-derived node/edge carries the same provenance discipline
as semantic nodes: a repository-relative `source.path`, a `GraphLayer`
(new: none required beyond the existing set -- metadata facts are tagged
`GraphLayer::Package`/`Build`/`Test`/`Runtime` as appropriate, reusing
Decision 0044's layer vocabulary), and a `DerivationClass` of `Manifest`
(a structured file was deterministically parsed) rather than `Parser`
(reserved for source-language AST extraction) -- so a client can always
distinguish "this fact came from parsing declared metadata" from "this
fact came from parsing source code."

### 3. Malformed-file isolation and explicit coverage

A malformed JSON/TOML/YAML/Markdown file produces a `Partial` or
`Failed` coverage entry for that file alone (the same `FileCoverage`
enum semantic adapters already use) and never aborts the build or
destroys unrelated contributions -- identical to Decision 0045's
malformed-source-isolation guarantee.

### 4. No network, no execution, no borrowed authority

Metadata extraction never resolves a package registry (crates.io, PyPI,
npm), never calls a GitHub API, never invokes `pip`/`npm`/`cargo`
subprocesses for correctness (an optional, offline, bounded `cargo
metadata` invocation may be used as a cross-check but is never the
authority a fact depends on -- source manifests must remain sufficient
on their own), never executes a project's own scripts/workflows, and
never imports a Python module to inspect it. A metadata fact is derived
only from the bytes of the manifest file itself.

### 5. No governance/mutation authority

A metadata-derived graph node or edge is orientation data. It never
grants mutation authority, never overrides governance/source authority,
and is never treated as equivalent to a RepoPact governance record --
consistent with Decision 0047 section 3's identical constraint on the
working overlay.

### 6. Coverage taxonomy extension

`SemanticCoverage`'s per-file model gains new `SourceLanguage` variants
(`Json`, `Toml`, `Yaml`, `Markdown`) so metadata files are classified
distinctly from `UnsupportedLanguage`, and two new `SkipReason` variants
(`MinifiedContent`, `GeneratedContent`) so ROG-022's minified/generated
policy is reported as an explicit, named skip -- never silently folded
into `UnsupportedLanguage` or `OversizedFile`.

### 7. Dependency direction

Package-level dependencies reuse the existing `GraphEdgeKind::DependsOn`
(already used for governance work-item dependencies) tagged with
`GraphLayer::Package` and `DerivationClass::Manifest`, rather than a new
edge kind -- one directional canonical relation; reverse dependencies
are obtained by inbound traversal, not a persisted inverse edge, unless
a concrete future need proves otherwise (WI063 step 22).

## Alternatives considered

- **Silently add new variants under v2.** Rejected: the audit above
  proves this reproduces the exact hazard Decision 0045 fixed.
- **A fully open, string-typed fact-kind field instead of a closed
  `ManifestKind` enum**, to avoid ever needing another major bump for
  metadata vocabulary growth. Deferred, not rejected: it would trade
  compile-time exhaustiveness checking for permanent extensibility. This
  checkpoint keeps `ManifestKind` closed, consistent with the existing
  `SymbolKind` precedent, and explicitly flags (see Consequences) that a
  future decision should consider an open vocabulary if metadata
  categories keep growing at a pace that makes repeated major bumps
  costly.
- **Force JSON/TOML/YAML through the Tree-sitter substrate** (there are
  Tree-sitter grammars for JSON/TOML/YAML). Rejected: these are
  structured/declarative formats, not programming languages with
  symbol/scope semantics; a dedicated deterministic parser
  (`serde_json`, a TOML crate, a minimal line-oriented YAML reader) is a
  better fit and keeps the adapter boundary parser-neutral as Decision
  0045 already requires.

## Consequences

- `rog/` durable graphs written by this checkpoint declare
  `graph_schema_version: 3`. v1 and v2 graphs remain permanently valid
  and readable; upgrading either to v3 is full deterministic rebuild
  only.
- A `ManifestKind` enum and `GraphNodeKind::Manifest` variant carry the
  new metadata vocabulary, following the same "generic node + typed
  sub-kind" pattern Decision 0045 established for `Symbol`/`SymbolKind`.
- Every metadata-derived fact is provenance-tagged distinctly
  (`DerivationClass::Manifest`) from parser-derived source facts
  (`DerivationClass::Parser`).
- A future decision should reconsider whether `ManifestKind` (and
  `SymbolKind`) should become open, string-typed vocabularies if
  metadata/symbol category growth continues to require repeated
  schema-major bumps -- this decision does not resolve that tension, it
  only discloses it honestly.
