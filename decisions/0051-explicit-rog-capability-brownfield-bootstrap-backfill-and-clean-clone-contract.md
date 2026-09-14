---
id: 0051
title: Explicit ROG capability, brownfield bootstrap, backfill, and clean-clone contract
status: accepted
date: 2026-09-14
supersedes: []
---

# 0051: Explicit ROG capability, brownfield bootstrap, backfill, and clean-clone contract

## Context

WI063's durable/incremental/overlay/query stack (Decisions 0044-0050)
is now mature enough to integrate with RepoPact's repository lifecycle:
brownfield adoption (ROG-014), backfill for already-governed
repositories (ROG-015), and the clean-clone success path (ROG-016).
None of that can be built safely on Decision 0044's original capability
signal -- `rog/manifest.json` exists -- because that signal cannot
distinguish "the graph was deliberately never enabled" from "the graph
was enabled, and its committed durable artifact then disappeared"
(cloned into a `.gitignore` that swallows it, deleted by hand, lost in
a partial checkout). ROG-039 requires the second case to be a hard,
visible failure, never a silent degrade to "absent, therefore valid."

## Decision

### 1. Capability state is a fact orthogonal to freshness/coverage/corruption

A repository's ROG capability is one of five states, computed from two
independent inputs -- whether a capability declaration exists and what
it says, and whether `rog/` physically exists -- never conflated with
`Freshness`/`GraphCoverageState`/`DurableFreshness`, which continue to
describe the *quality* of a graph that capability says should exist:

```text
LegacyAbsent      no capability record, no rog/            -- valid, disabled
LegacyEnabled     no capability record, rog/ exists         -- valid, binding
                  (every graph from Decisions 0044-0050 stays exactly this)
ExplicitDisabled  capability = disabled                     -- valid, disabled
ExplicitEnabled   capability = enabled, rog/ exists          -- valid, binding
EnabledMissing    capability = enabled, rog/ absent          -- HARD FAILURE
```

`EnabledMissing` is never reported as `Absent`. It is a distinct,
named, binding validation failure (Decision 0050's fail-closed
precedent extended to capability). A repository already carrying a
`rog/` graph from any prior WI063 checkpoint, with no capability
record, is `LegacyEnabled` -- fully valid and binding, requiring no
migration to keep working exactly as it does today.

### 2. The capability declaration: `governance/rog-capability.json`

A narrow, versioned, schema-validated, committed JSON record --
following the exact convention already established by
`governance/adopters.json`/`governance/verification.json` (a top-level
`$schema` pointer, `repopact/schemas/rog-capability.schema.json`,
`additionalProperties: false`) -- not a general settings system:

```json
{
  "$schema": "../schemas/rog-capability.schema.json",
  "version": 1,
  "capabilities": { "rog": "enabled" }
}
```

Absence of this file is not an error at any schema version: it is the
`Legacy*` branch of the state table above, permanently valid unless a
later governed decision changes that rule (per ROG-039's own text).
This record is read directly by `repopact-graph`, never indexed as a
graph node and never given a new `RecordKind` variant -- doing so would
recreate the exact closed-enum-in-a-durable-graph hazard Decisions
0048/0049 exist to avoid, for a fact that has no reason to appear in
the graph at all.

### 3. Enable ordering: never write "enabled" before the graph is proven good

`durable::write` (the single choke point every successful full build,
full-rebuild fallback, and incremental update already funnels through)
persists `capabilities.rog = "enabled"` **only after** the atomic
build-then-swap has already succeeded. A build/update that fails never
touches the capability record; an already-`ExplicitEnabled` repository
whose rebuild fails keeps its prior graph and its prior capability
value untouched (the existing swap-with-rollback guarantee already
covers the graph half of this; capability persistence simply never
runs on the failure path). If capability persistence itself somehow
fails after a successful graph install, the repository lands in
`LegacyEnabled` -- still valid and binding, not a hard failure -- since
`rog/` demonstrably exists.

### 4. Explicit disable

`repopact graph disable` (`graph.disable`) persists
`capabilities.rog = "disabled"` and removes the derived `rog/`
directory -- nothing else. It never touches source files, governance
records, work items, decisions, or evidence. It is idempotent (running
it twice, or on an already-disabled repository, succeeds with no
further change) and order-safe: `rog/` is removed before the
capability record is written, so a failure partway through never
claims `disabled` while a stale graph still sits on disk.

### 5. Adoption stays opt-in and reuses the canonical engine

`repopact adopt` remains graph-off by default -- brownfield adoption
does not silently hand every adopter a persistent graph artifact. An
explicit `--graph` flag runs: discovery -> governance adoption ->
governance validation -> `graph.build` -> `graph.verify` -> a bounded
`graph.orient`-derived orientation summary -> final validation.
Graph truth is produced exclusively by the existing canonical Rust
engine operations; adoption code coordinates, it does not parse source
or build a second graph implementation. A `--graph` bootstrap failure
never fabricates governance facts and never falsely marks the
capability enabled; it is reported as a distinct failure from
governance-adoption success, and the CLI exits non-zero when `--graph`
was explicitly requested and bootstrap failed. `--dry-run --graph`
performs no durable write of any kind -- not the capability record, not
`rog/`, not governance -- and reports only the planned steps.

### 6. Backfill is explicit, not automatic

For an already-governed repository with no graph, the supported
migration path is exactly `repopact graph build` (then `status`/
`verify`/`orient` as normal use) -- no `doctor` repair step is part of
the supported path. `repopact doctor` may *report* capability drift
(`enabled` but `rog/` missing; an invalid/unparseable capability
declaration; a stale/corrupt enabled graph) and recommend
`repopact graph build`, but it never silently enables ROG for a
repository that never asked for it. This mirrors the exact WI050
precedent already established for admission: the diagnostic runs for
every repository so `doctor`/`status` can disclose an explicit
`not-required`/`legacy-absent` baseline, while an absent capability
contributes no warning to legacy adopters who never opted in.

### 7. The ignored-artifact guard

Before persisting `capabilities.rog = "enabled"`, and before reporting
adoption's graph-bootstrap step successful, a bounded, single batched
`git check-ignore` invocation (the exact precedent already established
by `adopt_repo.gitignored_records`, ported to Rust: one process
invocation over every candidate path via `--stdin`, never per-shard,
never rewriting `.gitignore`) checks whether Git would ignore the
capability declaration or the durable graph directory. If so, the
enable operation fails honestly rather than persisting a capability
that clean-clone can never actually carry.

### 8. Read-only operations never mutate capability state

`graph status`, `graph verify`, and every `graph.*` query operation
(`resolve`/`context`/`neighbors`/`path`/`dependencies`/`dependents`/
`tests`/`governance`/`impact`/`orient`) are read-only with respect to
capability, exactly as they already are with respect to the graph
itself (Decision 0050 section 1). A query failure never rewrites the
capability record.

## Alternatives considered

- **Continuing to use `rog/manifest.json` existence as the sole
  capability signal.** Rejected: this is the exact ambiguity ROG-039
  requires resolved -- it cannot express "enabled but missing" as
  anything other than indistinguishable from "never enabled."
- **A general repository settings/config system.** Rejected as
  overbuilt for one boolean-shaped fact; `governance/rog-capability.json`
  follows the narrowest existing convention (a single-purpose,
  schema-validated JSON record) rather than introducing a new class of
  infrastructure.
- **A new `RecordKind`/graph-node representation for capability.**
  Rejected: capability is read directly by the graph subsystem's own
  status/build/query code, never needs to be a graph fact, and giving
  it a new closed-enum variant would recreate the durable-schema hazard
  Decisions 0048/0049 were written to avoid.
- **`doctor` auto-enabling ROG when it detects the repository could
  support it.** Rejected per this checkpoint's explicit instruction and
  the WI050 admission precedent: opt-in capabilities are never silently
  granted by a repair tool.

## Consequences

- Every `rog/` graph produced under Decisions 0044-0050, with no
  capability record, remains `LegacyEnabled` -- fully valid and
  binding -- with no required migration.
- `EnabledMissing` becomes a real, testable, hard-failure state,
  closing ROG-039's exact gap.
- Brownfield adoption (ROG-014), backfill (ROG-015), and clean-clone
  (ROG-016) all build on this same five-state model rather than each
  inventing its own capability signal.
- ROG-019 is untouched: this decision does not change fixture
  exclusion or fabricate a fixture fact.
