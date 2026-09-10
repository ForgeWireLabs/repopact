# Work Item 054 — Rust Graph, Analysis and Transactional Mutation Core

**Status:** Active

**Owner:** work-coordinator

**Affected scopes:** work, tooling, governance, docs, evidence

**Depends on:** WI053 (completed with evidence `20260909-053-rust-validator-conformance`)

## Intent

Extend the conformant WI053 Rust read/validation foundation with a reusable canonical relationship graph, deterministic repository analysis, and the first typed native mutation plan/apply boundary for explicitly proven work-item surfaces.

WI054 is not a general rewrite and does not make Rust globally canonical. It moves authority only for the graph/analysis/mutation surfaces that are proven by this work item's executable parity, stale-plan, recovery, and post-validation gates.

## Binding architecture

Read before implementation:

- [`architecture-review.md`](architecture-review.md) — live-source architecture review at activation;
- [Decision 0040](../../../decisions/0040-content-addressed-native-mutation-plans.md) — content-addressed read-set and plan/apply contract;
- WI052 architecture inventory;
- WI053 completed work and conformance evidence;
- the current SPEC lifecycle and invariant semantics.

The architectural flow is:

```text
RepositorySession
      |
      v
RepositorySnapshot
      |-------------------------------+
      |               |               |
      v               v               v
   validate         graph          analyze
                                      |
                                      v
MutationRequest -> MutationPlan -> explicit apply -> MutationResult
                       |
                       +-- repository identity
                       +-- exact content-addressed read set
                       +-- diagnostics / graph impacts
                       +-- durable + generated write set
                       +-- canonical preview
```

A mutation plan is a proposed transition over a specific repository snapshot. It is not a generic patch and is not authorization by itself.

## Scope

### Repository session/read model

Evolve the current one-shot Rust façade into a reusable repository session with immutable snapshots and record indexing. Graph, analysis, validation, and planning for one operation must consume consistent snapshot semantics rather than independently walking the filesystem.

### Relationship graph

Add a typed graph for canonical relationships including work dependencies, acceptance criteria, evidence, scopes, decisions, contracts, invariants, frozen-surface constraints, and audit findings where those relationships are structurally derivable.

Graph edges must preserve source path/record context. Arbitrary Markdown mentions are not authoritative edges.

### Deterministic analysis

Add explainable analyzers for work creation/editing decisions: next ID, scope overlap/ownership, dependency/reverse-dependency and cycle effects, evidence gaps, provenance concerns, unresolved findings, contract applicability, frozen-surface intersection, and structurally related work where derivable.

Analysis output distinguishes facts/constraints from suggestions and identifies the repository basis for each finding.

### Mutation planning

Introduce typed request/plan/result APIs. Initial mutation authority is deliberately work-item focused:

- create work item;
- typed edit of allowed work-item fields;
- lifecycle transition/move;
- dependency edits;
- acceptance-criterion edits/state/evidence linkage;
- generated dashboard update where the supported operation changes dashboard inputs.

Do not expose arbitrary JSON Patch or arbitrary filesystem writes.

### Stale-plan protection

Plans carry repository identity and exact content-addressed read facts, including expected-absent target paths. Apply rechecks those facts and rejects stale plans. Git HEAD and mtimes are not correctness tokens.

### Lifecycle fidelity

The formal model permits any lifecycle state to move to any other state, including reopening completed work. WI054 must not invent a narrower transition matrix.

A transition moves the entire work-item directory, updates the JSON status, preserves history/evidence/narrative content, regenerates owned projections, and succeeds only if the supported post-state validates.

### Recoverable apply

Supported writes are staged as a complete write set with preimages sufficient for rollback. Failure before or during apply, generated-artifact failure, detected concurrent drift, or post-validation failure may not be reported as success.

A new persistent repository-local transaction database/journal is not authorized by this work item without a separate durable decision.

## Parity model

Where Python already has an equivalent write/generation surface, use executable Python/Rust before-after parity. This includes work-item creation and dashboard generation.

Where WI054 adds a typed Rust mutation with no Python command equivalent, use canonical before/after fixtures plus both the Python reference validator and Rust validator on the resulting state. Do not create duplicate Python mutation code solely to manufacture a second implementation.

## Frozen surface and WI050

Frozen intersections must be visible during planning. WI054 does not synthesize human approval or port WI050 enforcement. Native apply therefore fails closed for protected changes unless an already-governed approval mechanism explicitly covers the operation.

WI050 admission, guard, IPC, platform backends, protected services, cryptographic approvals, and enforcement remain outside WI054.

## Explicitly deferred

- Tauri/UI implementation;
- decision mutation;
- evidence mutation;
- full SPEC-generation migration unless an actually supported WI054 mutation requires it;
- provider-backed AI analysis;
- PyO3/native wheels;
- Python CLI/canonical-core cutover;
- persistent RepoPact runtime database/journal architecture;
- WI050 migration.

## Acceptance and closeout

The machine-readable acceptance criteria in `work-item.json` are binding. Closeout evidence must include the exact graph queries/analyzers/mutations supported, read-set/stale-plan tests, Python-equivalent parity results, canonical before/after mutation fixtures, recovery/failure-injection results, dashboard projection parity, Rust/Python validator results, legacy conformance and WI050 corpus results, linked-worktree/reference checks, frozen-surface result, and the remaining unsupported mutation/cutover surfaces.

Rust remains an alternate implementation outside the surfaces explicitly proven by WI054.

## Closeout evidence (2026-09-10)

WI054 is complete only after the executable conformance and parity gates, not
merely because the workspace compiles. The durable evidence record is
`20260910-054-rust-graph-mutation-core`.

- Delivered `repopact-types`, `repopact-schema`, `repopact-repository`,
  `repopact-validation`, `repopact-graph`, `repopact-analysis`,
  `repopact-mutation`, `repopact-core`, and a minimal non-Tauri CLI.
- Graph nodes cover repository/work/criterion/evidence/scope/role/
  decision/policy/contract/invariant/frozen/finding records. Queries are
  deterministic for dependencies, reverse dependencies, criteria, evidence,
  and source-backed outgoing edges. Analysis emits source-backed facts,
  constraints, and suggestions for IDs, scopes, dependencies/cycles, evidence,
  findings, contracts, frozen paths, provenance, and explicit related work.
- The typed mutation boundary is `CreateWorkItem`, `EditWorkItem`, and
  `TransitionWorkItem`. Plans carry repository identity, sorted
  content-addressed read facts (including expected absence), a stable token,
  complete durable/generated operations, graph impacts, diagnostics, and a
  canonical preview. Apply rechecks facts, preserves ephemeral preimages,
  regenerates the dashboard, post-validates, and rolls back on failure.
- Python/Rust create and dashboard trees matched after line-ending
  normalization. Canonical Rust edit and transition fixtures matched exactly;
  both Python and Rust validators accepted the resulting states. Completed to
  active reopening moved the whole directory and preserved extra history.
- The full Python suite passed 205 tests with 2 existing skips. The full Rust
  workspace passed. The existing Python-driven Rust alternate implementation
  runner passed legacy conformance 20/20 and the WI050 admission corpus 8/8.
  Focused linked-worktree, ignored-directory, and record-relative-reference
  tests passed. No `.github/workflows/**` or other frozen surface changed.
- No Python implementation changes were needed. Tauri, PyO3/cutover, generic
  decision/evidence mutation, general SPEC mutation, persistent transaction
  state, provider authority, and WI050 migration remain explicitly unsupported.
  Decision 0040 and WI052 sequencing require no amendment.
