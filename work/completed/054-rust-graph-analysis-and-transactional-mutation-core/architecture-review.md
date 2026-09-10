# WI054 Architecture Review — Graph, Analysis and Transactional Mutation Core

**Review date:** 2026-09-09  
**Reviewed `main`:** `9facd78be758dd839f26c787e524af117d7a72a9`  
**Coding agent:** Codex  
**Architecture review:** Sol High

## Executive conclusion

WI053 is a valid prerequisite and is complete enough to activate WI054. Its Rust surface is intentionally read-only: `RepoPactCore` owns a `Repository`, exposes identity, and consumes the core for validation. The repository crate already has deterministic work/evidence discovery, path normalization, linked-worktree identity, contract discovery, and record-relative reference resolution; validation has proven 20/20 legacy conformance through the existing Python alternate-implementation runner.

WI054 should extend that foundation rather than create a parallel mutation subsystem beside it.

The architectural center of WI054 is a persistent repository session with immutable snapshots:

```text
RepositorySession
      |
      +--> RepositorySnapshot
      |      +-- RepositoryIdentity
      |      +-- RecordIndex
      |      +-- canonical source records
      |      +-- content-addressed read facts
      |
      +--> validate(snapshot)
      +--> graph(snapshot)
      +--> analyze(snapshot, query)
      +--> plan(snapshot, MutationRequest)
                       |
                       v
                 MutationPlan
                       |
                 explicit apply
                       |
                       v
                 MutationResult
```

A `MutationPlan` is not permission and is not a loose patch. It is a typed proposed state transition over a specific content-addressed read set.

## WI053 surface actually available

### `repopact-types`

WI053 currently provides:

- lifecycle/provenance enums;
- `AcceptanceCriterion`;
- `PreflightMarker`;
- `WorkItem`;
- `EvidenceReference`;
- `RepositoryIdentity`;
- structured `Diagnostic` and `ValidationReport`.

WI054 should extend these shared domain types rather than define frontend-shaped duplicates in new crates.

### `repopact-repository`

WI053 currently provides:

- normalized repository root;
- repository/common-dir and linked-worktree identity;
- deterministic lifecycle-directory work-item discovery;
- evidence discovery and evidence IDs;
- contract discovery with embedded linked-worktree exclusion;
- record-relative reference resolution;
- portable path normalization.

This is sufficient to become the physical read layer, but it is not yet a full indexed domain snapshot. Graph/analysis code must not independently crawl the repository and rediscover its own semantics.

### `repopact-validation`

WI053 owns the supported Rust semantic recognizer and dashboard comparison. WI054 should refactor reusable projection logic if necessary, but must keep validation behavior equivalent. Mutation success is conditioned on the supported Rust post-state validator, while Python remains the external reference/parity oracle during this migration phase.

### `repopact-core`

The current façade is intentionally minimal and read-only. `validate(self)` consumes the core because WI053 needed only one-shot validation. WI054 should evolve this to a reusable session façade. That is an additive architectural evolution, not a defect in WI053.

## New crate boundaries

Add the smallest clean reusable boundaries:

```text
rust/
  crates/
    repopact-types/          # extend
    repopact-schema/         # reuse
    repopact-repository/     # extend session/snapshot/index
    repopact-validation/     # reuse/refactor projection helpers
    repopact-graph/          # new
    repopact-analysis/       # new
    repopact-mutation/       # new
    repopact-core/           # compose all of the above
```

Tauri remains absent. Do not put graph, analysis, or mutation semantics into `apps/repopact-cli` merely because a CLI is convenient for testing.

## Repository session and snapshot

The first implementation step is a reusable session/read model.

Candidate concepts:

```text
RepositorySession
RepositorySnapshot
RepositoryRevision / SnapshotToken
RecordIndex
RecordRef
RecordKind
PathState
ReadSet
```

The snapshot should make the records used by graph, analysis, planning, and validation explicit. It should be immutable for the lifetime of one plan/analysis operation.

A useful record index should cover at least:

- work items and complete work-item directory paths;
- evidence runs;
- decisions and policies via the existing controlled front-matter grammar;
- owner scopes and roles;
- invariants;
- frozen-surface entries;
- audit findings;
- audit registry contracts/scopes;
- applicable `AGENTS.md` contracts;
- `VERSION` / `RELEASE_LABEL` facts used by supported projections/validation;
- dashboard and any template inputs used by supported mutation planning.

Do not turn the repository crate into an unbounded semantic god object. It owns deterministic physical discovery and parsed canonical records; graph/analysis/mutation own their domain interpretations.

## Content-addressed plan preconditions

Decision 0040 is binding for this work.

A plan must record the facts it actually consumed. A candidate representation is:

```text
ReadFact {
    path,
    expected: Present { digest, kind } | Absent
}

MutationPlan {
    repository_identity,
    read_set,
    plan_token,
    request,
    diagnostics,
    relationship_impacts,
    file_operations,
    generated_impacts,
    preview
}
```

The exact digest algorithm is an implementation choice, but it must be deterministic and cryptographically collision-resistant enough for state identity. Do not use mtimes, file sizes, branch names, or HEAD as the correctness token.

The read set should be precise rather than simply hashing the entire repository. A create-work-item plan, for example, legitimately reads all existing work IDs, owner/scope records, the work-item template, relevant governance, and dashboard source inputs. An unrelated source-code edit that the planner never read should not necessarily invalidate the plan.

An expected-absent path is a first-class precondition. This is required to prevent a create plan from overwriting a path that appeared after planning.

## Graph semantics

The graph is a projection of canonical facts, not an inference engine.

Recommended node kinds include:

- Repository;
- WorkItem;
- AcceptanceCriterion;
- EvidenceRun;
- Scope;
- Role;
- Decision;
- Policy;
- Contract;
- Invariant;
- FrozenSurfaceEntry;
- AuditFinding.

Recommended canonical edges include where derivable:

- WorkItem `depends_on` WorkItem;
- WorkItem `contains` AcceptanceCriterion;
- AcceptanceCriterion `supported_by` EvidenceRun;
- EvidenceRun `supports_work_item` WorkItem;
- WorkItem `owned_by` Scope;
- WorkItem `affects` Scope;
- Role `owns/allows` Scope as defined by owners records;
- Decision `supersedes` Decision;
- Policy `applies_to` declared targets;
- AuditFinding `concerns` Scope;
- AuditRegistryScope `governed_by_contract` Contract;
- record/path `constrained_by` applicable nested contract;
- candidate path `intersects` FrozenSurfaceEntry where matching is deterministic.

Do **not** create an authoritative graph edge from arbitrary Markdown prose merely because a token resembles a decision/work ID. Textual mentions may later be exposed as explicitly non-authoritative search candidates, but they are not canonical graph relationships.

Each edge must carry source context sufficient to answer "why does RepoPact say these are related?"

## Analysis semantics

Analysis needs an explicit distinction between fact, constraint, and suggestion.

Candidate shape:

```text
AnalysisFinding {
    kind,
    classification: Fact | Constraint | Suggestion,
    message,
    basis: [SourceRef],
    related_records,
    optional remediation
}
```

This prevents a deterministic heuristic from becoming authority merely because it is machine-generated.

Initial analyzers should include:

1. next available work-item ID;
2. exact scope overlap among active/proposed/non-terminal work;
3. owner-scope mismatch and affected-scope validity;
4. reverse dependencies;
5. dependency-state conflicts;
6. dependency-cycle preview for candidate changes;
7. acceptance/evidence gaps;
8. unresolved audit findings for relevant scopes;
9. applicable contract chain for candidate paths;
10. frozen-surface intersections for candidate paths;
11. provenance/completion concerns;
12. structurally related work based on explicit scopes/dependencies/paths supplied to the analyzer.

"Related decisions" is limited to canonical relationships or explicitly labeled search candidates. The current work-item schema has no decision-link field; WI054 must not silently invent one or parse arbitrary prose into normative relationships.

## Initial mutation request surface

Keep the first mutation authority focused on work items.

Recommended request variants:

```text
CreateWorkItem
EditWorkItem
TransitionWorkItem
```

`EditWorkItem` should be typed rather than accepting arbitrary JSON Patch. Supported edits may include:

- title;
- owner scope;
- affected scopes;
- dependencies;
- provenance where current governance permits it;
- acceptance-criterion additions/edits/state/evidence links;
- updated date as a derived consequence.

Immutable identity fields such as `id` and `created` should not be generic edits. Preflight should not be casually mutable after creation. Status changes belong to `TransitionWorkItem`, because status is both JSON state and filesystem location.

Decision/evidence writes are **not** required for WI054 closeout. The graph may read them. Deferring their mutations avoids accidentally weakening evidence immutability or introducing front-matter rewrite semantics before needed.

## Create-work-item parity

Python already defines `new_work_item`, so Rust creation should prove parity with that existing surface for equivalent inputs:

- next numeric ID semantics;
- slug behavior;
- lifecycle directory placement;
- canonical JSON shape/format;
- preflight marker semantics;
- local-template-first behavior and packaged/template fallback;
- README stamping;
- dashboard regeneration.

If WI054 intentionally improves an existing behavior instead of matching it, that is a semantic change and needs its own governed decision/test expectation rather than being hidden as a Rust difference.

## Lifecycle transitions

The formal model is explicit: any lifecycle state may transition to any other state. Reopening completed work is allowed. The validator determines whether the post-state is conformant.

Therefore:

- do not invent a narrower transition matrix;
- move the **entire work-item directory**, not only `work-item.json`;
- update JSON `status` consistently;
- preserve README, AGENTS, audit companions, directives, and any other directory content;
- never silently drop acceptance/evidence/history data;
- require the resulting supported repository state to validate before success.

A transition into completed with pending criteria must therefore fail the mutation's post-validation and roll back, even though an ordinary manual filesystem move could temporarily create that invalid state.

## Dashboard and SPEC projections

Work-item create/edit/transition affects the dashboard, so Rust dashboard generation/application is part of WI054's supported write set. WI053 already contains Rust dashboard rendering for validation; refactor that into one reusable projection implementation rather than copying it into mutation code.

Initial work-item mutations do not change SPEC source inputs. Do not port the whole SPEC generator merely to make WI054 look broader. Add SPEC mutation generation only if an actually supported WI054 mutation changes inputs that require it.

Derived artifacts remain outputs. The planner shows their changes as generated impacts; callers do not edit them directly.

## Frozen surface boundary

The current Python frozen check is diff-based and supports explicit human acknowledgement. WI054 is not the WI050 protected approval migration.

The Rust planner should:

- report protected path/symbol intersections it can deterministically establish;
- surface the reason and source frozen-surface entry;
- refuse native apply for a protected mutation unless an already-governed operator-approval mechanism is explicitly available to the core.

Do not reduce this to a caller-supplied `approved: true` boolean and call it security. WI054 must not manufacture operator authority.

## Apply and recovery

Multi-file mutation cannot be honestly called atomic merely because each individual rename is atomic. Treat the requirement as **atomic where possible, recoverable as a complete operation**.

Required properties:

1. verify repository identity and read-set preconditions immediately before apply;
2. acquire whatever short-lived process coordination the implementation needs without claiming it prevents arbitrary external editors from writing;
3. compute/stage the full write set before replacing governed paths;
4. preserve preimages needed for rollback;
5. apply in deterministic order;
6. regenerate owned projections from the candidate state;
7. validate the post-state;
8. on any failure, restore preimages and remove staged artifacts;
9. report success only after the supported post-state is valid;
10. test injected failure at multiple operation boundaries.

Do not introduce a permanent `.repopact` transaction database, hidden daemon state, or new repository-local journal format without a separate durable decision. An ephemeral recovery mechanism is sufficient for WI054 if it satisfies the failure-injection tests.

## Concurrency and TOCTOU

A read-set token protects against staleness before apply. It does not magically lock external editors.

Implementation should re-check target preimages as close as practical to replacement and fail/rollback if a target changed during apply. Tests should cover an external edit between plan and apply, and at least one injected drift point during apply.

Do not silently merge external content.

## Parity strategy

There are two legitimate proof classes.

### Existing Python-equivalent surfaces

Use executable Python/Rust before-after comparison for:

- create work item;
- dashboard rendering/regeneration;
- any other operation that actually has a Python reference implementation.

Compare canonical file trees/content, not success messages.

### New Rust-native mutation surfaces

For typed edits/transitions with no Python command equivalent:

- materialize a canonical before fixture;
- apply the Rust mutation;
- compare to a checked-in canonical after fixture;
- run the Python reference validator on the after state;
- run the Rust validator on the same state;
- require both to accept/reject consistently as specified.

Do not build duplicate Python mutation code solely for testing.

## Core API evolution

`RepoPactCore` should become a reusable façade over a session rather than a one-shot validator. Candidate API families:

```text
open
identity
snapshot
validate
build_graph
analyze
plan_mutation
apply_mutation
```

Tauri commands are not part of this work. The core APIs should be serializable enough for WI055 to wrap without exposing internal filesystem handles or arbitrary writes.

## Explicit non-goals

- Tauri application or frontend code;
- PyO3/native Python packaging;
- Python CLI cutover;
- general JSON Patch/filesystem patch API;
- decision mutation;
- evidence mutation;
- provider-backed AI analysis;
- WI050 admission/guard/IPC/platform migration;
- frozen-surface approval synthesis;
- persistent runtime database/journal architecture;
- changing canonical schemas merely to make Rust APIs easier.

## Recommended implementation order

1. Baseline Python + WI053 Rust/conformance results at the activation commit.
2. Extend shared record/source types.
3. Add `RepositorySession` / immutable snapshot / read-set primitives.
4. Port controlled front-matter parsing and remaining records needed for graph/analysis.
5. Add `repopact-graph` and canonical graph tests.
6. Add `repopact-analysis` and fact/constraint/suggestion result types.
7. Add `repopact-mutation` request/plan/result/read-set structures with **plan-only, no writes**.
8. Prove plan determinism and stale-token behavior.
9. Implement create-work-item candidate generation and Python parity.
10. Implement typed edit candidate generation and canonical fixtures.
11. Implement lifecycle directory transition candidate generation and fixtures.
12. Reuse/refactor Rust dashboard projection as an owned generated impact.
13. Implement recoverable apply with failure injection and rollback.
14. Add post-write Rust validation and cross-implementation after-state validation tests.
15. Expose the proven APIs through `repopact-core`; keep CLI additions minimal/testing-oriented.
16. Run full Python, Rust, legacy conformance, WI050 corpus, mutation parity, stale-plan, recovery, linked-worktree/reference, and frozen-surface checks.
17. Record closeout evidence and only then complete WI054.

## Stop/escalation conditions

Codex should stop and report before committing a hard-to-reverse design if implementation appears to require any of the following:

- canonical schema changes;
- frozen-surface changes;
- a new persistent repository-local runtime state/journal format;
- WI050 approval/admission/guard semantics;
- generic decision/evidence overwrite support;
- a permanent Python/Rust dual mutation authority for the same surface;
- a lifecycle transition model contradicting the formal model;
- a mutation plan correctness rule based primarily on Git HEAD or timestamps;
- a Tauri-specific dependency in reusable core crates.

## Exit condition

WI054 is closable when the Rust core can construct an explainable canonical graph, perform deterministic repository analysis, plan supported work-item mutations against content-addressed read sets, reject stale plans, preview exact durable/generated effects, apply supported transformations recoverably, regenerate its owned dashboard projection, and prove the supported transformations with the correct parity class and both validators—without claiming Tauri, Python cutover, WI050 migration, or unsupported mutation authority.
