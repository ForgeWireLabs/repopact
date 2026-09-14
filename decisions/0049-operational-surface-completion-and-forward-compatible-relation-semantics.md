---
id: 0049
title: Operational surface completion and forward-compatible relation semantics
status: accepted
date: 2026-09-14
supersedes: []
---

# 0049: Operational surface completion and forward-compatible relation semantics

## Context

Decision 0048 closed ROG-018/021/022 and established the metadata
adapter contract, but left ROG-004 and ROG-019 with named gaps: no
test/runtime relation semantics, no npm workspace resolution, no
deterministic test-target/fixture/generated-boundary/installer-surface/
runtime-entrypoint topology. ROG-024 (a future bounded query surface)
cannot honestly answer "what test/build/runtime surfaces relate to this
package/module?" without the graph actually carrying those facts first.

### The compatibility trap this decision must not fall into

Decision 0048's own audit proved that `GraphNodeKind`, `GraphEdgeKind`,
and nested closed enums (`SymbolKind`, `ManifestKind`) are all subject to
the identical hazard: an implementation compiled before a given variant
existed hard-fails deserializing a shard line naming it, even under the
same schema major. Naively adding `Tests`, `Runs`, `GeneratedBy`,
`EntryPointFor`, `InstallerFor` to `GraphEdgeKind` (or equivalent new
`ManifestKind` variants for test/runtime/generated surfaces) while still
writing schema v3 would recreate that exact hazard one release after
Decision 0048 fixed it. This decision does not do that.

## Decision

### 1. An additive, open, string-backed role model -- not new closed enums

`GraphNodeKind`, `GraphEdgeKind`, and `GraphLayer` remain the coarse,
always-true classification and are **not** extended with new variants
for test/runtime/generated/installer surfaces. Instead, two new,
additive, optional fields carry finer, forward-compatible specificity:

```rust
/// A bounded, validated, open vocabulary token -- not a closed enum.
/// Deserializes as an ordinary string; an unrecognized value is not a
/// deserialization error, only (optionally) an application-level
/// "unknown role" the reader may choose to ignore. New role values
/// never require a schema-major bump.
pub struct GraphNodeRole(String);
pub struct GraphRelationRole(String);
```

Added to `GraphNode`/`GraphEdge` as
`#[serde(default, skip_serializing_if = "Option::is_none")] node_role:
Option<GraphNodeRole>` / `relation_role: Option<GraphRelationRole>` --
the identical additive-optional-field pattern Decision 0046 already
established for `source_digest`/`semantic_compatibility`, which does
**not** require a schema-major bump (only new *enum variants* do; new
*struct fields* that default to absent do not, because an old reader
that does not know the field simply never sees it referenced and a new
reader encountering its absence treats it as `None`).

Validation is a bounded, permissive **syntax** check (non-empty, at most
64 bytes, lowercase ASCII letters/digits/underscores, starting with a
letter) -- never a check against a permanently closed list of known
values. Constructing a `GraphNodeRole`/`GraphRelationRole` with invalid
syntax fails at construction time (a programmer error caught in this
codebase's own tests), not at deserialization time for an unrecognized
-but-well-formed value written by a newer producer.

### 2. Old-v3 readability is proven before this design is adopted

A compatibility test constructs a "shadow" representation of the exact
pre-0049 `GraphNode`/`GraphEdge` JSON shape (the fields that existed the
moment schema v3 was released under Decision 0048, with no knowledge of
`node_role`/`relation_role`) and proves it deserializes a new-v3 JSONL
line containing role metadata successfully, silently ignoring the role
fields, and recovering a coarse `kind`/`layer` whose meaning remains
fully true on its own. This is the load-bearing proof for this decision:
if it had failed, this design would be invalid and a v4 decision would
be required instead -- it did not fail (see
`durable::tests::an_old_v3_reader_shape_still_deserializes_a_node_edge_carrying_new_role_metadata`).

### 3. The coarse relation must remain independently true

Every operational fact this checkpoint emits is chosen so that the
coarse `GraphEdgeKind`/`GraphLayer` pair, read alone with the role
ignored, is still a true statement -- never merely a plausible-sounding
one. Concretely:

| Fact | Coarse relation (true without the role) | Role (adds precision) |
|---|---|---|
| A test target exercises the package/crate under test | `DependsOn`, `layer=Test` | `role=tests` |
| A manifest names a runtime entry file/command | `Contains` or `DependsOn`, `layer=Runtime` | `role=runtime_entrypoint` (node) |
| A generated file has a known generator source | `DependsOn`, `layer=Build` | `role=generated_by` |
| An npm workspace member belongs to its root package | `BelongsToWorkspace`, `layer=Package` | `role=workspace_member` |
| An installer/packaging manifest names a bundle target | `Contains`, `layer=Package` | `role=installer_surface` (node) |

If a fact cannot be phrased so the coarse relation stays true without
the role, it is not encoded this way -- it is either left unemitted (as
an unsupported-at-this-tier relation, per ROG-004's own qualifier) or
routed through an existing, already-true relation instead.

### 4. Stable initial role vocabulary

**Node roles:** `test_target`, `generated_surface`, `installer_surface`,
`runtime_entrypoint`. (`test_fixture`/`package_surface` were considered
and deliberately not added this checkpoint -- see Alternatives.)

**Relation roles:** `tests`, `generated_by`, `entry_point_for`,
`installer_for`, `workspace_member`.

Every role's direction, coarse `GraphEdgeKind`, `GraphLayer`,
`DerivationClass`, expected node kinds, and source/provenance are
documented in `implementation-progress.md`'s role table. No speculative
role is added without a concrete RepoPact-relevant fact or a realistic
synthetic-fixture proof driving it.

### 5. Dependency/reverse-dependency model stays singular

Package-level dependencies continue to use `GraphEdgeKind::DependsOn`
only (Decision 0048 section 7). This decision does not introduce a
`ReverseDependency` edge for packages: reverse lookups remain inbound
traversal over the same `DependsOn` edges, exactly as chosen for
governance. Every adapter that emits a dependency fact follows this one
model -- none emits a duplicate inverse edge.

### 6. Contribution pipeline and compatibility identity

Every operational fact is produced by the same shared contribution
architecture (`semantic::build_file_contribution`, the metadata adapters,
and orchestrator-level cross-file resolution passes that already have
projection access) -- consumed identically by the full build,
`graph.update`, and `SessionGraphState`. No desktop-only topology, no
second operational graph builder. `CURRENT_SEMANTIC_PIPELINE_VERSION`
bumps because this checkpoint's orchestrator passes change what facts
the same bytes produce; an old contribution generated without them is
not silently treated as complete.

## Alternatives considered

- **New closed `GraphEdgeKind`/`ManifestKind` variants for test/runtime/
  generated/installer surfaces.** Rejected: recreates the exact
  old-reader-hard-fails hazard Decision 0048 just fixed, one release
  later.
- **A schema v4 bump instead of an open role field.** Rejected as
  unnecessary once the old-v3-readability proof (section 2) succeeded --
  a v4 bump is reserved for when this proof fails, which it did not.
- **`test_fixture` and `package_surface` node roles.** Deferred, not
  implemented: RepoPact's own `fixtures`-named directories are already
  excluded from the entire graph by the pre-existing, cross-cutting
  `IGNORED_PARTS` list (Decision 0044), and no other source-backed
  fixture convention (e.g. a `conftest.py`) exists in this repository to
  ground a real instance. Adding a speculative role with no real or
  realistically-fixtured justification was avoided per this checkpoint's
  own "do not add speculative roles" instruction; `package_surface` is
  redundant with the already-existing `ManifestKind` document
  classification and adds no new information.
- **A generic `Fact` node role library covering every conceivable future
  operational category up front.** Rejected: the initial vocabulary is
  deliberately the smallest set RepoPact's actual facts (or a realistic
  synthetic fixture, where RepoPact has no self-instance) justify.

## Consequences

- `rog/` graphs written by this checkpoint remain schema v3 -- no major
  bump. Old v3 readers built before this decision continue to read the
  durable graph correctly, simply without the finer role information.
- `GraphNodeRole`/`GraphRelationRole` are the first genuinely open,
  string-backed vocabulary in the durable schema; `SymbolKind`/
  `ManifestKind` remain closed as before. A future decision may
  reconsider converting those to the same open pattern if their growth
  rate continues to force schema-major bumps (Decision 0048's own
  disclosed tension).
- ROG-004's build/package/test/runtime relation gap and ROG-019's
  test-target/fixture/generated-boundary/installer-surface/runtime-
  entrypoint gaps are addressed to the extent RepoPact's own facts (or a
  documented realistic fixture) support them; remaining gaps are named
  explicitly in `implementation-progress.md`, not silently closed.
