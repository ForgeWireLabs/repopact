# Work Item 052 — Rust Core and Tauri 2 User Workbench Foundation

**Status:** Proposed

**Owner:** work-coordinator

**Affected scopes:** work, tooling, governance, docs, evidence

## Intent

Evolve RepoPact from a CLI-centric Python reference implementation into a language-neutral governance platform with a reusable Rust core and a modern Tauri 2 desktop workbench, without creating two permanent and conflicting semantic authorities.

The desktop application is not the architectural endpoint. It is the first major native consumer of a broader RepoPact engine. The target is a reusable domain/runtime layer that can eventually serve desktop, CLI, Python compatibility, IDE/plugin, agent, and other integrations while the durable RepoPact standard remains defined by repository-native specifications, schemas, invariants, conformance fixtures, and governed decisions.

This work item establishes the foundation and migration contract. It does **not** authorize an unbounded whole-product rewrite, immediate retirement of the Python package, or a premature port of the still-active WI050 admission/enforcement substrate.

## Problem statement

RepoPact today has a useful but increasingly CLI-shaped implementation boundary. The Python package owns record discovery, work-item semantics, frontmatter/JSON parsing, validation, doctor behavior, frozen-surface checks, dashboard/spec generation, fleet verification, admission/enforcement behavior, adapters, and other repository operations. That has been effective for bootstrapping the product, but a serious user-facing application exposes several architectural constraints:

1. A desktop UI cannot safely become a second implementation of RepoPact semantics in TypeScript.
2. A Rust-only UI backend that independently reimplements a subset of Python semantics would create two authorities that can accept, reject, or mutate the same repository differently.
3. A permanent subprocess/sidecar architecture would preserve one semantic authority, but it would keep the desktop dependent on process invocation and would not advance the broader native/core direction.
4. Immediate PyO3 conversion would solve duplication eventually but would force cross-platform native Python packaging before the Rust implementation has proven semantic equivalence.
5. RepoPact already has a versioned conformance corpus suitable for proving an alternate implementation, so migration can be staged and falsifiable rather than trust-based.
6. Work-item creation/editing is not merely CRUD. RepoPact can structurally analyze overlapping work, dependencies, scopes, frozen surfaces, related decisions, evidence requirements, provenance, and downstream impact. A first-class domain API is required to make those operations safe and useful.
7. RepoPact's security direction increasingly includes typed policy, cryptographic verification, local IPC, protected services, and cross-platform host behavior. Rust is a natural long-term executable substrate, but security-critical migration must follow proof rather than run ahead of it.

The architectural goal is therefore **one standard, one eventual canonical executable core, multiple consumers, and a temporary migration overlap that is mechanically constrained by conformance**.

## Product direction

RepoPact should become a governance workbench rather than a Markdown editor.

A user should be able to open an adopted repository and understand:

- repository health;
- active, blocked, proposed, deferred, and completed work;
- dependency chains;
- decisions and what references them;
- evidence and which criteria it satisfies;
- frozen-surface implications;
- unresolved findings;
- applicable contracts and scopes;
- provenance and stale review state;
- repository drift;
- likely overlap with new work;
- what a proposed edit would change before it is applied.

Creation and editing should be schema-driven and governance-aware. The UI should explain why an operation is valid or invalid and should preview the exact durable repository changes that will occur.

## Architectural thesis

The following layers must remain distinct:

```text
RepoPact standard
  SPEC + schemas + invariants + governed decisions + conformance corpus
                           |
                           v
                  reusable Rust core
        +------------------+------------------+
        |                  |                  |
        v                  v                  v
  Tauri desktop      Rust/native APIs   compatibility surfaces
                                             |
                                             +-- Python CLI/package
                                             +-- future Rust CLI
                                             +-- IDE/plugin adapters
                                             +-- agent integrations
```

The standard remains language-neutral. Rust becomes the intended canonical executable semantics only after staged parity gates are satisfied. Tauri is a consumer, not the owner of governance rules. Python remains the reference implementation during migration and stays independently usable throughout this foundation phase.

## Migration principle: conformance before authority

RepoPact already supports versioned conformance for alternate implementations. This work item must use that capability as the migration arbiter.

The migration state is:

```text
Phase A
Python = reference implementation
Rust   = incomplete alternate implementation
Conformance = arbiter

Phase B
Python = reference implementation
Rust   = conformant for declared supported surfaces
Cross-implementation parity = required for supported mutations

Phase C
Rust   = candidate canonical executable core
Python = compatibility consumer or explicitly retained reference subset
Cutover requires separately governed decision and packaging plan
```

No code path may silently treat "implemented in Rust" as equivalent to "RepoPact conformant."

Unsupported Rust semantics must be explicit. The implementation must fail closed or route to an explicitly permitted reference path; it must not guess or mutate repository state under partially implemented rules.

## Proposed Rust workspace

The exact path names may be refined by implementation, but the architectural separation should resemble:

```text
rust/
  Cargo.toml

  crates/
    repopact-types/
    repopact-schema/
    repopact-repository/
    repopact-validation/
    repopact-graph/
    repopact-analysis/
    repopact-mutation/
    repopact-core/

    # reserved for later migration, not full WI052 scope
    repopact-admission/
    repopact-enforcement/

  apps/
    repopact-desktop/
```

The purpose of splitting these concerns is not crate-count for its own sake. It prevents UI concerns, filesystem concerns, schema mechanics, mutation mechanics, and security-critical enforcement from collapsing into one Tauri backend crate.

### `repopact-types`

Own typed, serializable domain structures and stable enums/value objects.

Candidate types include:

- `WorkItem`;
- `DecisionRecord`;
- `EvidenceRun`;
- `AcceptanceCriterion`;
- `LifecycleState`;
- `ScopeId`;
- `DependencyRef`;
- `Provenance`;
- `RepositoryIdentity`;
- `RecordIdentity`;
- `ValidationDiagnostic`;
- `MutationRequest`;
- `MutationPlan`;
- `MutationResult`.

These Rust types make the implementation safer; they do not replace the language-neutral JSON/Markdown contracts.

### `repopact-schema`

Own:

- canonical schema loading;
- schema-version awareness;
- schema validation;
- serialization/deserialization boundaries;
- normalized schema diagnostics.

Canonical RepoPact JSON schemas remain authoritative artifacts. Rust structs must not become an undocumented replacement schema.

### `repopact-repository`

Own the repository read model:

- RepoPact-root discovery;
- nested contract discovery;
- canonical record discovery;
- ignored paths;
- linked Git worktree handling;
- repository/common-dir identity;
- path normalization and resolution;
- record loading;
- indexed repository snapshots;
- incremental refresh inputs.

The UI must not recursively interpret the repository itself.

A candidate shape is:

```text
RepositorySession
  |
  +-- RepositoryIdentity
  +-- RepositorySnapshot
  +-- RecordIndex
  +-- ContractIndex
  +-- RelationshipIndex
  +-- DiagnosticSet
```

### `repopact-validation`

Own semantic validation for Rust-supported surfaces.

Diagnostics must be structured rather than requiring clients to parse prose:

```text
Diagnostic
  code
  severity
  message
  record
  path
  field
  related_records[]
  suggested_actions[]
```

Human-readable text remains important, but machine-readable identity and context are required so desktop and CLI surfaces can render equivalent results.

### `repopact-graph`

Expose RepoPact's existing implicit graph explicitly.

Relationships may include:

```text
Work Item
  -> depends_on -> Work Item
  -> governed_by -> Decision
  -> affects -> Scope
  -> satisfies/produces -> Evidence
  -> contains -> Acceptance Criterion
  -> constrained_by -> Contract / invariant / frozen surface

Decision
  -> referenced_by -> Work Item / docs / evidence

Evidence
  -> supports -> Acceptance Criterion / work item
```

The graph must support impact queries and visualization without inventing authority absent from canonical records.

### `repopact-analysis`

Provide deterministic, explainable analysis used during creation and editing.

Initial analyzers should evaluate, where derivable:

- next available work-item identifier;
- active/proposed work with overlapping scopes;
- direct file/scope overlap when candidate paths are known;
- dependency candidates;
- reverse dependencies;
- related decisions;
- frozen-surface intersection;
- unresolved audit findings;
- applicable contracts;
- evidence gaps;
- lifecycle conflicts;
- provenance/review issues;
- structurally similar governed work.

Each result should identify the repository facts that caused it. "Confidence" alone is not a sufficient explanation.

### `repopact-mutation`

All governed writes from native clients flow through a typed mutation boundary.

The expected flow is:

```text
client edits form
      |
      v
MutationRequest
      |
      v
plan mutation
      |
      +-- resolve current repository identity/state
      +-- validate lifecycle/authority constraints
      +-- compute affected durable records
      +-- compute derived artifact changes
      +-- produce diagnostics
      +-- render preview/diff
      |
      v
MutationPlan
      |
      | explicit apply
      v
transactional/recoverable write
      |
      +-- canonical serialization
      +-- derived artifact regeneration
      +-- post-write validation
      |
      v
MutationResult
```

The frontend must never become the authority for direct `work-item.json`, decision, evidence, dashboard, or other governed-file writes.

## Tauri 2 desktop foundation

The desktop application should be modern, native-feeling, fast, and fully driven by RepoPact semantics rather than generic file editing.

Initial application navigation should include:

- Repository / workspace switcher;
- Dashboard / repository health;
- Work;
- Decisions;
- Evidence;
- Dependency/relationship graph;
- Validation / diagnostics;
- Analysis;
- Settings.

A candidate high-level layout:

```text
+------------------------------------------------------------------+
| RepoPact                                repository: example-repo |
+----------------+-------------------------------------------------+
| Dashboard      | Repository Health                               |
| Work           |                                                 |
| Decisions      | Proposed | Active | Blocked | Problems          |
| Evidence       |                                                 |
| Graph          | Recent governance changes                       |
| Validation     | Outstanding diagnostics                          |
| Analysis       |                                                 |
| Settings       |                                                 |
+----------------+-------------------------------------------------+
```

The visual design should prioritize dense but understandable engineering/governance information, strong hierarchy, keyboard navigation, responsive layouts, dark/light themes, and accessible state representation. Decorative complexity must not replace information architecture.

## Work-item browser and editor

The work view should support:

- lifecycle filtering;
- owner/scope filtering;
- dependency filtering;
- search;
- active/blocked emphasis;
- acceptance-criterion progress;
- evidence linkage state;
- related decision count;
- frozen-surface implications;
- history/context navigation.

A work-item detail view should expose:

```text
Overview
Acceptance Criteria
Dependencies
Scope / impact
Evidence
Decisions
Relationships
Validation
History
Raw representation (advanced/read-only by default)
```

### Creation workflow

"New work item" should be a governed workflow rather than an empty text editor.

Candidate stages:

1. Intent
2. Repository analysis
3. Scope and ownership
4. Dependencies
5. Acceptance criteria
6. Evidence plan
7. Governance/frozen-surface review
8. Preview
9. Create

Before creation, RepoPact should surface deterministic findings such as:

- likely overlap with an active item;
- a candidate dependency that is only proposed;
- an affected frozen surface;
- an existing decision governing the same subsystem;
- a scope ownership mismatch;
- unresolved findings relevant to the intended work.

The user remains the authority for the durable work record. Analysis assists the decision; it does not silently create authority.

### Editing workflow

Edits should be schema-driven and field-aware. The client should request a mutation plan and show:

- fields changed;
- files changed;
- lifecycle transition;
- generated files affected;
- relationship changes;
- downstream impact;
- warnings/errors;
- canonical diff.

Only then should the client apply the mutation.

## Decision UX

Decisions are not generic notes. The desktop should expose:

- status/state;
- rationale;
- alternatives;
- supersession/replacement;
- work items governed by the decision;
- other references;
- impact of proposed edits.

Before a material decision edit, the analysis layer should identify records that reference it and any acceptance criteria or documentation that may be invalidated.

This work item does not authorize changing decision semantics merely to satisfy a UI design.

## Evidence UX

Evidence is durable proof, not ordinary editable content.

The evidence view should make clear:

- associated work item / criteria;
- run status;
- commands/checks;
- artifacts/references;
- timestamps/provenance;
- immutability/durability expectations;
- validation status.

Completed evidence must not be presented as an ordinary freeform editor. Corrections/additions should follow RepoPact's existing durable-history rules.

## Filesystem watching

Live repository updates are desirable, but watching belongs below the UI boundary.

A candidate flow:

```text
filesystem events
      |
      v
RepositoryWatcher
      |
      +-- debounce/coalesce
      +-- canonicalize path
      +-- classify affected record/index
      +-- incremental reload
      +-- revalidate affected surface
      |
      v
RepositoryChanged event
      |
      v
Tauri presentation layer
```

The frontend should consume typed events or refreshed snapshots, not implement recursive filesystem semantics itself.

The watcher must account for editor save patterns, rename/replace writes, Git branch changes, worktree changes, bursty generated-artifact updates, and self-originated mutation events without causing feedback loops.

## Tauri security boundary

The webview/frontend must not receive generic authority merely because RepoPact is a local developer tool.

Expose narrow operations such as:

- `open_repository`;
- `repository_snapshot`;
- `list_work_items`;
- `get_work_item`;
- `analyze_work_item`;
- `plan_work_item_create`;
- `plan_work_item_update`;
- `apply_mutation`;
- `validate_repository`;
- `get_relationship_graph`.

Avoid broad frontend capabilities equivalent to:

- arbitrary filesystem write;
- unrestricted shell execution;
- arbitrary command invocation;
- unconstrained path traversal.

Repository-scoped capability and path restrictions should be explicit.

This boundary is independent of WI050's stronger protected agent-admission substrate; the desktop must not claim WI050 enforcement merely because its own webview is constrained.

## Optional assisted analysis boundary

RepoPact can benefit from LLM/agent assistance, especially for:

- candidate acceptance criteria;
- work decomposition;
- duplicate/overlap explanations;
- dependency suggestions;
- evidence-plan suggestions;
- decision-impact summaries.

However, no AI provider belongs in the authority kernel.

The architecture should permit:

```text
deterministic RepoPact analysis
             |
             +-- optional assisted-analysis adapter
                         |
                         +-- local model
                         +-- hosted model
                         +-- downstream product/runtime
```

Assisted output is proposal-only.

An assistant may not, by itself:

- transition proposed work to active;
- waive an acceptance criterion;
- approve a frozen-surface change;
- fabricate operator authorization;
- alter completed evidence/history;
- bypass validation;
- convert uncertain inference into recorded fact without provenance.

Provider configuration and any downstream secrets must remain outside durable RepoPact governance records unless an existing canonical record explicitly requires them.

## Python compatibility during this phase

The Python package remains independently usable while WI052 is being implemented.

WI052 must **not** require immediate PyO3 conversion.

That preserves the current simple Python distribution while the Rust core is incomplete and avoids forcing manylinux/macOS/Windows native-wheel release engineering before semantic parity has been established.

Later cutover alternatives include:

### Native Python binding

```text
Python CLI/package
      |
      v
PyO3 binding
      |
      v
Rust core
```

Advantages:
- in-process;
- single engine;
- direct typed binding.

Costs:
- platform/Python-specific native wheels;
- more complex release matrix;
- source-build concerns.

### Stable local executable/protocol

```text
Python CLI/package
      |
      v
stable repopact-core executable/protocol
      |
      v
Rust core
```

Advantages:
- Python package can remain largely pure;
- language-neutral consumer boundary.

Costs:
- process/protocol/versioning boundary;
- error/streaming semantics;
- executable distribution.

### Reduced Python reference surface

A final option may retain Python only for explicitly declared compatibility/reference functions while normal user execution moves to Rust.

No option is selected permanently by WI052. A separately governed architectural decision is required after the Rust core has enough evidence to evaluate real packaging and maintenance costs.

## WI050 admission/enforcement boundary

WI050 is active and security-critical. Its protected admission/enforcement substrate includes policy, leases, cryptography, protected guard/service behavior, IPC, process identity, platform backends, and fail-closed semantics.

WI052 must not opportunistically rewrite that surface in Rust merely because a Rust workspace now exists.

Instead:

- reserve clean extension/module boundaries for future migration;
- permit read-only inspection/status integration in the desktop when it uses existing canonical behavior;
- do not claim Rust enforcement parity absent dedicated tests;
- do not weaken existing enforcement to simplify desktop integration;
- require separate governed work once WI050 semantics are stable enough to port safely.

This prevents the GUI initiative from becoming an accidental security rewrite.

## Cross-implementation parity gates

For each Rust-supported semantic surface, define executable parity categories:

### Read parity

Given the same repository, Python and Rust identify the same governed records, canonical identities, lifecycle states, dependencies, scopes, and relevant paths.

### Validation parity

For the declared conformance version/surface:

- accept fixtures are accepted;
- reject fixtures are rejected;
- expected primary diagnostics are equivalent;
- unsupported semantics are explicit.

### Graph parity

Relationships derived from canonical records match the same source facts and do not invent undocumented edges.

### Mutation-plan parity

Given the same repository and supported mutation request, Rust predicts the canonical durable changes and rejects transitions that the reference semantics reject.

### Mutation-result parity

When a Rust mutation is enabled for a supported surface, the resulting repository validates under the reference implementation and produces canonical derived artifacts.

### Failure parity

Invalid or unsupported operations fail without partial governed-state mutation.

These gates are more important than raw implementation percentage.

## Initial mutation scope

WI052 should start with a deliberately bounded native mutation set, likely around work-item operations, because that is the primary desktop need and has strong schema/conformance coverage.

Candidate first supported operations:

- create proposed work item;
- update ordinary proposed work-item fields;
- edit pending acceptance criteria;
- add/remove dependencies where lifecycle rules allow;
- validate without mutation.

Lifecycle transitions that imply authorization, frozen-surface approval, completed-history mutation, or other higher-risk semantics should remain reference-only until parity is specifically proven.

The exact enabled set must be documented and tested; unsupported operations must be visible in both API and UI.

## Generated artifacts

Derived artifacts remain generated, not hand-maintained.

A mutation plan must know which generators are implicated. Applying a supported mutation must regenerate required derived artifacts through RepoPact-owned generation logic and include them in the preview/result.

The desktop must not contain its own dashboard renderer whose output can drift from the canonical generator.

During migration, generation may remain Python-backed for surfaces not yet ported, but the dependency must be explicit and covered by a compatibility test.

## Repository identity and worktrees

RepoPact has already encountered linked-worktree and path-resolution correctness issues. The Rust repository model must not regress them.

Tests should cover:

- ordinary checkout;
- nested current working directory;
- linked Git worktree;
- ignored/unrelated nested repositories;
- canonical root discovery;
- relative record references;
- record-relative `source_of_truth`-style semantics where applicable;
- case/path behavior on supported platforms.

Repository identity must be stable enough that file watching, mutation planning, and future enforcement integration cannot accidentally act on the wrong checkout/root.

## Performance expectations

The purpose of Rust is not merely benchmark performance, but the native core should be designed for responsive desktop use.

Targets should be measured rather than guessed. The implementation should record representative timings for:

- cold repository open/index;
- warm snapshot retrieval;
- incremental refresh after one work-item edit;
- full validation;
- work-item analysis;
- relationship-graph query.

Optimization must not trade away semantic correctness or produce stale results without clear state.

## Accessibility and UX quality

The desktop is intended as a real user-facing product, not an internal debug panel.

Required design concerns include:

- keyboard navigation;
- visible focus;
- screen-reader labels for controls/status;
- non-color-only status communication;
- scalable typography;
- light/dark appearance support;
- readable dense tables;
- clear empty/error/loading states;
- confirmation for destructive/high-impact operations;
- recoverable drafts for unsaved form state;
- explicit repository identity in mutation-confirmation flows.

## Observability and diagnostics

Native APIs should make failures diagnosable.

Errors should distinguish at least:

- repository not adopted/not found;
- unsupported RepoPact version;
- parse/schema failure;
- semantic validation failure;
- stale repository snapshot;
- mutation conflict;
- filesystem/permission failure;
- generator failure;
- post-write validation failure;
- unsupported Rust surface;
- internal error.

The UI should present a concise explanation while preserving structured technical detail for debugging/evidence.

## Testing strategy

WI052 requires tests at multiple layers.

### Rust unit tests

Cover typed parsing, lifecycle/state helpers, path handling, diagnostics, graph relationships, and mutation-plan primitives.

### Fixture/conformance tests

Run Rust behavior against canonical RepoPact fixtures for supported rules.

### Cross-implementation differential tests

Materialize the same repository/fixture and compare Python and Rust results for supported operations.

### Mutation safety tests

Prove:

- preview is stable for unchanged input;
- stale snapshots/conflicts are rejected;
- failure does not leave partial durable state;
- unsupported operations do not mutate;
- generated-artifact changes are included;
- post-write reference validation succeeds.

### Tauri command/API tests

Prove the frontend command surface maps to domain APIs and cannot bypass mutation planning through a generic file-write command.

### UI tests

At minimum cover repository open, work-item browse, create workflow, validation error rendering, mutation preview, and repository-change refresh behavior.

### Platform smoke tests

The foundation should be exercised on Windows, Linux, and macOS where CI/available runners permit. Missing native platform evidence must be reported honestly.

## Architecture decision requirements

Implementation will likely cross material/hard-to-reverse boundaries. Durable decision records should be created before closeout for choices such as:

- canonical Rust-core cutover criteria;
- workspace/crate public boundaries if they become compatibility surfaces;
- cross-language compatibility protocol/binding if selected;
- mutation transaction/recovery model;
- stable diagnostic identity/versioning if exposed publicly;
- desktop distribution/update trust model once packaging is implemented.

The work item may investigate alternatives, but durable product commitments should not live only in this README.

## Follow-on boundaries

WI052 is a foundation and must remain closable.

The following are intentionally expected to become separate work unless they remain trivially small:

### Full semantic parity and canonical-core cutover

Port remaining validators/generators/doctor/fleet semantics, reach defined compatibility coverage, choose Python integration strategy, and transfer canonical executable authority.

### Production desktop completion and distribution

Installer/signing/update channels, polished platform integrations, complete editing surfaces, release packaging, crash reporting/privacy policy where applicable, and product release criteria.

### Advanced repository analysis and assisted planning

Deeper graph analysis, work decomposition, optional LLM/provider adapters, explainable similarity ranking, and richer planning workflows.

### Admission/enforcement Rust migration

Only after WI050 semantics and platform proofs are stable enough for a security-focused port with its own threat model, differential testing, and cutover gates.

The existence of those future boundaries is deliberate. WI052 must not remain open until every RepoPact Python module or every imagined desktop feature has been rewritten.

## Non-goals

- Do not rewrite all RepoPact Python code in one work item.
- Do not make Tauri or TypeScript the semantic authority.
- Do not make the desktop a generic unrestricted filesystem editor.
- Do not require PyO3 before parity evidence exists.
- Do not permanently maintain two independent complete engines.
- Do not silently diverge from the published conformance suite.
- Do not redefine canonical schemas merely to make Rust typing convenient.
- Do not change completed historical records to simplify the new model.
- Do not weaken WI050 enforcement or claim it has been ported.
- Do not make an LLM/provider mandatory for ordinary RepoPact operation.
- Do not hard-code ForgeWire-specific product authority into RepoPact.
- Do not reserve future work-item IDs in advance; follow-on work receives IDs when actually created.
- Do not declare the Python implementation retired at WI052 closeout.

## Implementation sequence

A recommended implementation order is:

1. Record any required durable architecture decision before hard-to-reverse implementation.
2. Add Rust workspace and core domain types with no Tauri dependency.
3. Implement schema loading/parsing and repository discovery/index foundation.
4. Implement structured diagnostics.
5. Add Rust conformance runner integration for the first supported rules.
6. Add relationship graph/query layer.
7. Add deterministic work-item analysis primitives.
8. Add mutation request/plan model without write authority.
9. Differentially test planning against Python/reference behavior.
10. Enable the smallest safe work-item mutations with atomic/recoverable apply and reference post-validation.
11. Add Tauri 2 application shell consuming only typed Rust APIs.
12. Implement repository dashboard/browse/validation/graph surfaces.
13. Implement schema-driven proposed work-item creation/editing against mutation plans.
14. Add watcher/incremental refresh behind the repository layer.
15. Harden Tauri capabilities and negative-test generic write/shell bypasses.
16. Measure performance and reconcile cross-platform behavior.
17. Update docs/roadmap and record closeout evidence plus explicitly deferred surfaces.

Parallelization is permitted only where owner scopes and implementation dependencies remain clear.

## Exit condition

WI052 is complete when RepoPact has a proven Rust/Tauri foundation that is useful on its own, does not create a second semantic authority, and establishes executable gates for future migration.

Closeout does **not** require that every Python module has been ported.

A successful closeout should truthfully allow a statement similar to:

> RepoPact now has a reusable Rust domain/repository foundation and Tauri 2 desktop workbench for the declared supported surfaces. Those surfaces are conformance/differentially tested against the Python reference implementation. Native mutations pass through a typed preview/apply boundary and validate under canonical RepoPact semantics. Python remains supported, and the remaining canonical-core cutover, production packaging, advanced assistance, and admission/enforcement migration are explicitly deferred to separately governed work.

Anything stronger requires the corresponding evidence.
