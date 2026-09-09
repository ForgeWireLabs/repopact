# Work Item 054 — Rust Graph, Analysis and Transactional Mutation Core

**Status:** Proposed

**Owner:** work-coordinator

**Affected scopes:** work, tooling, governance, docs, evidence

**Depends on:** WI053

## Intent

After WI053 proves Rust repository/validator conformance, extend the reusable Rust core with a first-class governance relationship graph, deterministic repository analysis, and a typed transactional mutation boundary suitable for desktop, CLI, and future integration clients.

WI054 is the point at which Rust may begin to plan and apply governed changes. That authority must be earned through executable parity and recovery guarantees rather than introduced through ad hoc file writes.

## Preconditions

Implementation must not begin until WI053 has durable evidence for the Rust read/validation foundation needed by the mutation engine.

Python remains the reference for any mutation/generation behavior not yet proven through WI054 parity fixtures.

## Relationship graph

Expose RepoPact's durable relationships explicitly without inventing facts absent from canonical records.

Minimum graph concepts should include:

- work item -> dependency -> work item;
- work item -> contains -> acceptance criterion;
- acceptance criterion/work item -> supported by -> evidence;
- work item -> affects/owned by -> scope;
- work/decision/evidence -> constrained by -> applicable contract/invariant/frozen surface where derivable;
- decision -> referenced by -> governed records;
- reverse dependencies;
- provenance/review relationships used by existing semantics.

Graph queries must identify their source records and remain deterministic.

## Deterministic analysis

Build explainable repository analyzers for creation/editing workflows, including where derivable:

- next available work-item ID;
- overlapping active/proposed work;
- scope ownership/overlap;
- direct path/frozen-surface intersection;
- dependency candidates and reverse dependencies;
- proposed dependency on non-actionable lifecycle states;
- related durable decisions;
- unresolved audit findings;
- applicable contracts;
- evidence gaps;
- provenance/review issues;
- structurally related governed work.

Analysis output must cite the repository facts that produced it. Opaque scoring alone is insufficient.

## Mutation architecture

All native governed writes should move through a typed plan/apply boundary.

Conceptual flow:

```text
MutationRequest
      |
      v
plan
      +-- resolve current repository identity/version
      +-- validate requested transition/authority
      +-- compute durable record changes
      +-- compute relationship and generated-artifact impact
      +-- produce diagnostics/warnings
      +-- render canonical preview/diff
      v
MutationPlan
      |
      | explicit apply against expected repository state
      v
recoverable/atomic write
      +-- canonical serialization
      +-- derived projection regeneration
      +-- post-write validation
      v
MutationResult
```

A plan must be invalidated when the repository state it was calculated against has materially changed.

## Initial mutation coverage

At minimum design/prove operations required by the future desktop work-item flow:

- propose/create work item;
- edit allowed work-item fields;
- lifecycle transition/move where current governance permits it;
- dependency changes;
- acceptance-criterion edits/state transitions under existing rules;
- related generated dashboard/spec projection updates where applicable.

Decision/evidence mutations should be added only with their existing durable-history/immutability semantics preserved. Completed evidence must not become a generic editable record.

## Atomicity and recovery

No API may report success after partially updating a governed transition.

Implementation must define and test:

- precondition/state checks;
- write ordering/staging;
- failure before commit;
- failure during multi-file apply;
- cleanup/recovery behavior;
- post-write validation failure behavior;
- generated-artifact consistency;
- stale mutation plan rejection;
- concurrent external edit detection where practical.

Git availability may be used as context/evidence, but correctness must not depend on silently rewriting Git history.

## Parity expansion

The legacy validator conformance corpus is not enough for mutation authority. WI054 must add cross-implementation before/after fixtures for the mutation/generation semantics it claims.

Each fixture should be able to compare canonical repository state rather than merely matching a success message.

## AI boundary

Deterministic analysis is in scope. Provider-backed assistance is not authority.

WI054 may define a provider-neutral proposal interface for future assisted analysis, but no hosted/local LLM is required and no provider result may activate work, waive criteria, approve frozen changes, fabricate authorization, or bypass the mutation validator.

## Explicitly out of scope

- Tauri UI implementation;
- Python canonical-core cutover;
- provider-specific AI integration as a core dependency;
- WI050 admission/guard/enforcement migration;
- relaxing completed evidence/history durability;
- direct caller writes that bypass mutation planning/application.

## Closeout standard

WI054 closes when the Rust core can explain governed relationships, perform deterministic planning analysis, preview supported mutations, apply them atomically/recoverably against expected repository state, regenerate the projections it owns, and prove those supported transformations with executable parity fixtures and post-write validation.
