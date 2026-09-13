---
id: 0046
title: Incremental ROG convergence and contribution-reuse contract
status: accepted
date: 2026-09-13
supersedes: []
---

# 0046: Incremental ROG convergence and contribution-reuse contract

## Context

Decision 0044 established the durable Repository Orientation Graph (ROG)
projection and Decision 0045 extended it with deterministic Tree-sitter
semantic extraction (schema v2). Both checkpoints deliberately built only
a full-rebuild engine: `repopact graph build` always reprocesses every
projected file from scratch. WI063 ROG-012 requires proving that an
*incremental* update -- one that reuses unchanged work rather than
reparsing everything -- produces output canonically identical to a clean
full rebuild:

```text
canonical(G_incremental(s0 -> s1)) == canonical(G_full(s1))
```

This decision binds the architecture that makes that equivalence provable
rather than incidental, and the exact rules for when reuse is trusted and
when it is not. Decision 0044 remains the durable-format authority; this
decision extends it with an incremental-execution contract and one
additive schema-v2 field. It does not rewrite or weaken anything Decision
0044 or 0045 already bound.

## Decision

### 1. The full build remains the correctness oracle

`RepositoryGraph::build_with_fingerprint` (the full build) is authoritative.
Incremental execution is an optimization layered on top of it, never a
second, independently-trusted code path. Every equivalence test in this
checkpoint proves an incremental result against a clean full build of the
same final state, never against another incremental result.

### 2. Only semantic extraction is contribution-incremental

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
deterministic from repository state; rebuilding them globally on every
`graph.update` call is simpler and safer than fine-grained incremental
tracking of either layer, and this checkpoint does not attempt it. Only
semantic extraction -- the genuinely expensive, per-file parsing step --
is contribution-incremental.

### 3. Correctness authorities and non-authorities

Reuse decisions rest **only** on:

- canonical repository-relative path, and
- the `SourceProjection` content digest for that path (already the
  Decision 0044 fingerprint input), plus
- the semantic-compatibility identity (section 5 below).

The following are never correctness authorities, regardless of how
tempting they are as optimizations:

- **Git diff** -- a `git diff`/rename-detection result is not consulted
  anywhere in delta computation (section 4).
- **mtime** -- filesystem modification times are never read for
  correctness; only content digests are.
- **Watcher events** -- no filesystem-event stream exists in this
  checkpoint, and a future one (ROG-013) will never be treated as
  sufficient by itself to invalidate or trust a contribution -- content
  digest remains authoritative underneath any watcher signal.
- **Parser caches** -- no persistent parse-tree cache exists in this
  checkpoint (see section 8); a future one may accelerate reparsing but
  can never itself decide whether reuse is safe.

### 4. Delta classification is add/modify/delete only

A file's classification between a durable baseline and the current
`SourceProjection` is exactly one of:

```text
unchanged  -- present in both, identical content digest
added      -- present now, absent from the baseline
modified   -- present in both, different content digest
deleted    -- present in the baseline, absent now
```

**Rename/move is not a fifth class.** It is a delete at the old path plus
an add at the new path. No persistent object identity is fabricated
across the two; a symbol that moves to a different file gets a new
identity there (consistent with Decision 0045 section 5's stable-identity
rule, which is keyed by file path). A future rename-continuity relation
may link the two explicitly, but nothing here pretends they are the same
node.

### 5. Semantic compatibility identity

Content-unchanged bytes cannot be safely reused if the extraction
behavior that produced the prior contribution has since changed. A
durable graph now records:

```text
SemanticCompatibility {
    graph_schema_major:      u32,
    pipeline_version:        String,
    adapter_versions:        BTreeMap<String, String>,
    resource_policy_version: String,
}
```

`pipeline_version` and `resource_policy_version` are explicit version
constants (`repopact_graph::incremental::CURRENT_SEMANTIC_PIPELINE_VERSION`,
`CURRENT_RESOURCE_POLICY_VERSION`) bumped by hand whenever a change to the
orchestrator's own logic or `ResourcePolicy`'s constants could change what
the same bytes produce; `adapter_versions` reuses the identity each
adapter already declares (Decision 0045). If the baseline's recorded
identity is missing or does not equal the current one exactly, **reuse is
prohibited** and a conservative full semantic rebuild runs instead. This
identity is never inferred from Cargo build timestamps, Git commit age, or
any filesystem metadata -- only from these four explicit fields.

### 6. Additive schema-v2 field, not a schema-major bump

Two additive fields make delta computation and compatibility checking
possible without breaking Decision 0044/0045's compatibility contract:

- `FileCoverageEntry.source_digest: Option<String>` -- the projection
  digest a given coverage entry was computed against.
- `Manifest.semantic_compatibility: Option<SemanticCompatibility>`
  (section 5).

Both are `#[serde(default, skip_serializing_if = "Option::is_none")]`.
A pre-existing schema-v2 reader that does not know either field still
deserializes any durable graph produced by this checkpoint cleanly (it
simply does not see the new keys); a schema-v2 graph written by the
semantic-adapter checkpoint (before this decision) still deserializes
cleanly under this implementation, with both fields `None`. Neither
field's absence is treated as an error -- it is treated as "this
baseline predates incremental support" and handled by the fallback rule
in section 7. **No schema-major bump accompanies this decision.**

### 7. Required update behavior by baseline state

`repopact graph update` (engine operation `graph.update`) classifies the
existing baseline before doing anything else:

| Baseline state | Behavior | `mode` | `fallback_reason` |
|---|---|---|---|
| No `rog/manifest.json` | Full build | `full_fallback` | `graph_absent` |
| Schema v1 | Full schema-v2 rebuild | `full_fallback` | `schema_v1_upgrade` |
| Schema v2, `semantic_compatibility` absent | Full rebuild | `full_fallback` | `incremental_metadata_absent` |
| Schema v2, compatibility mismatch | Full rebuild | `full_fallback` | `semantic_compatibility_mismatch` |
| Structurally corrupt (any supported schema) | Full rebuild | `full_fallback` | `corrupt_baseline_full_rebuild` |
| Unsupported schema major | Fail closed, nothing written | error, no `UpdateResult` | n/a |
| Valid, fingerprint unchanged | True no-op, `rog/` not rewritten | `no_op` | none |
| Valid, fingerprint changed | Incremental update | `incremental` | none |

A full rebuild is **never** hidden behind the `incremental` mode label,
regardless of why it happened. A corrupt baseline is never treated as a
source of reusable contributions -- when corruption is detected, this
implementation chooses an explicit full rebuild over the baseline (a
narrower, self-healing alternative to a hard failure) rather than
attempting to salvage any part of the corrupt shards; an unsupported
schema major, by contrast, fails closed and writes nothing, because this
implementation does not know what shape that data actually has.

### 8. Durable replacement safety

`durable::write`'s build-then-swap sequence is strengthened: instead of
deleting any pre-existing `rog/` before installing the new one, the old
directory is renamed aside to a backup path first, the new one is
installed by renaming staging into the final path, and only then is the
backup removed. If installing the new graph fails, the backup is renamed
back into place. The required property is:

```text
new graph installed
OR
old graph recoverable
```

never a state where a failed replacement leaves neither. This is a narrow
two-step rename-with-rollback, not a transaction database or journal --
consistent with the discipline WI054's GAM-018 already established.

### 9. Coverage is recomputed, not incrementally bookkept

`SemanticCoverage`'s aggregate counters (`files_considered`,
`files_complete`, ...) are recomputed deterministically from the final
per-file inventory after every build or update, by one shared function
(`semantic::aggregate_coverage`), rather than incremented/decremented as
files are added, changed, or removed. A full build and an incremental
update both end with some final `Vec<FileCoverageEntry>`, assembled
differently, but both derive their aggregates from it by the same
computation -- so an equality proof between the two paths rests on that
shared function, not on bookkeeping happening to stay in sync.

### 10. Contribution ownership uses existing source attribution

Every semantic node and edge already carries `source.path` equal to the
repository-relative path of the file that produced it (every adapter
attributes its own output this way; verified across all three adapters).
Because the semantic relations this checkpoint emits (`defines`,
`imports`) are file-local by construction, this existing field is
sufficient to attribute every reusable node/edge to exactly one owning
file. No new contribution-owner field is introduced. A future semantic
tier that emits genuinely cross-file relations (resolved imports, calls,
`implements`/`extends`/`uses_type`) will need to revisit this -- multi-file
ownership or cross-file invalidation is explicitly out of scope here.

### 11. Out of scope for this checkpoint

- Incremental governance or physical topology tracking (section 2).
- Tree-sitter incremental syntax trees / `Tree::edit` / keystroke-level
  reparsing. This checkpoint proves *durable graph contribution reuse*,
  not persistent-parser-tree caching; that is deferred to a future local
  overlay/cache phase.
- The watcher/working-tree overlay (ROG-013) and `Freshness::WorkingOverlay`.
  `graph.update` operates on a `RepositorySnapshot`, exactly like
  `graph.build`; it is not wired into any filesystem watcher, and does not
  write a durable graph on every keystroke.
- Cross-file semantic invalidation, resolved call/reference graphs, and
  any relation beyond `defines`/`imports`.

## Alternatives considered

- **Git diff/rename detection as the delta source.** Rejected: Git state
  is not always available (a non-Git-tracked file, a fresh checkout mid
  clone, a rebase in progress) and this project's own governance already
  treats Git as convenience, not authority, everywhere else in the ROG
  design (Decision 0044 section 8 chose content-hash fingerprinting over
  Git blob identity for the same reason).
- **mtime-based staleness.** Rejected outright per WI057/Decision 0040's
  existing precedent: mtimes are not correctness preconditions anywhere
  in RepoPact, and are trivially wrong across clones, checkouts, and CI
  restores.
- **A persistent Tree-sitter parse-tree cache keyed by file.** Deferred,
  not rejected -- a real future optimization once contribution-level
  reuse is proven safe, but adding it now would conflate two different
  claims (graph contribution correctness vs. parser-tree cache
  correctness) in one checkpoint.
- **Failing closed on a corrupt baseline instead of rebuilding.** Also a
  defensible choice; this implementation chose an explicit full rebuild
  because it is self-healing and a corrupt `rog/` is, by Decision 0044
  section 9, never load-bearing for repository validity -- rebuilding it
  from source is always safe. Either choice satisfies "never reuse
  corrupt data as a contribution."

## Consequences

- `repopact graph update` / engine operation `graph.update` exist as a
  new, additive surface; `graph.build` (always full) is unchanged and
  remains the correctness oracle callers can always fall back to.
- Two additive, optional fields extend schema v2 without a major-version
  bump: `FileCoverageEntry.source_digest` and
  `Manifest.semantic_compatibility`.
- `durable::write` now performs a rename-based backup-and-rollback swap
  instead of delete-then-rename, closing a narrow but real
  data-loss window on install failure.
- A future incremental phase (governance/physical incrementality, a
  persistent parse-tree cache, or the watcher/working-tree overlay) has an
  explicit, tested foundation to build on rather than needing to
  reconstruct these rules from scratch.
