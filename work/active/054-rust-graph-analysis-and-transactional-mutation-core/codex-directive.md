# Codex Execution Directive — WI054

**Coding agent:** Codex  
**Authoritative work item:** WI054  
**Architecture umbrella:** WI052  
**Prerequisite:** WI053 completed at/through `9facd78be758dd839f26c787e524af117d7a72a9`  
**Delivery mode:** direct to `main`, preserving concurrent work; no force-push/reset/history rewriting.

## Mission

Implement WI054's bounded Rust graph/analysis/mutation core on top of the proven WI053 repository/schema/validation foundation.

The milestone is not "we can write JSON." The milestone is that RepoPact can explain canonical relationships, deterministically analyze candidate work, plan exact work-item state transitions against content-addressed repository facts, reject stale plans, apply supported transformations recoverably, regenerate its owned projection, and prove the resulting state through the correct parity gates.

Do **not** start Tauri, Python cutover, generic decision/evidence mutation, or WI050 security-substrate migration.

## Required reading before edits

1. root `AGENTS.md` and applicable tooling contracts;
2. `work/active/052-rust-core-and-tauri-2-user-workbench-foundation/architecture-inventory.md`;
3. completed WI053 README/work-item/evidence;
4. this WI054 `README.md`, `work-item.json`, and `architecture-review.md`;
5. Decision 0040;
6. `SPEC.md` lifecycle/invariants sections and `research/formal-model.md` lifecycle/transition-system sections;
7. `rust/crates/repopact-types`, `repopact-repository`, `repopact-validation`, and `repopact-core`;
8. Python reference surfaces `repopact/new.py`, `generate_dashboard.py`, `generate_spec.py`, `frontmatter.py`, `check_frozen_surface.py`, and relevant validator helpers;
9. WI050 boundaries before touching anything that looks like operator approval, admission, leases, guard, IPC, platform backend, or protected service.

## Start-state rules

- Fetch latest `origin/main` and verify the WI054 activation commit/Decision 0040 are present.
- Work from a clean tree.
- Inspect concurrent changes before every push and preserve them.
- No force push, hard reset of shared history, or rewriting completed work.
- Establish the full Python and WI053 Rust/conformance baseline before implementation.
- Keep Cargo target/build output outside the repository as WI053 did.
- Do not change frozen `.github/workflows/**`, canonical schemas, invariants, or charter without an independently recorded operator-approved reason.

## Phase 1 — persistent session and snapshot

Before graph or mutation code, evolve the Rust façade into a reusable repository session/read model.

Implement concepts equivalent to:

- `RepositorySession`;
- immutable `RepositorySnapshot`;
- `RecordIndex` / typed record references;
- content-addressed path/read facts;
- repository/snapshot identity suitable for mutation planning.

Do not make graph, analysis, and mutation crates independently recurse through the filesystem.

The current `RepoPactCore::validate(self)` one-shot ownership shape may be refactored so validation can operate from the reusable session/snapshot without changing WI053 semantics.

## Phase 2 — extend canonical record model

Add only the record representations needed by WI054, such as:

- decision front matter;
- policy front matter if needed for graph/analysis;
- owner scopes/roles;
- invariants/frozen entries;
- audit findings/registry entries;
- contract references;
- reusable `RecordRef` / `SourceRef` identity.

Match the current deliberately small Python front-matter grammar. Do not introduce a broad YAML language into RepoPact just for convenience unless repository evidence proves it is required.

Canonical JSON schemas and existing Markdown/front-matter contracts remain the standard.

## Phase 3 — `repopact-graph`

Create a reusable graph crate. Canonical graph edges come only from structured/registered facts.

At minimum support deterministic source-backed relationships needed by WI054:

- work dependency and reverse dependency;
- work -> acceptance criteria;
- criteria -> evidence;
- evidence -> work;
- work -> owner/affected scopes;
- decision supersession;
- audit finding -> scope;
- contract applicability/registration;
- frozen/invariant constraints where derivable.

Every edge/query result must retain source record/path context.

Do not parse arbitrary Markdown prose into authoritative graph relationships. If you expose textual mention candidates, type them as non-authoritative search/suggestion results.

## Phase 4 — `repopact-analysis`

Create deterministic, explainable analysis over the same snapshot/graph.

Analysis result types must distinguish at least:

- facts;
- constraints/violations;
- suggestions/heuristics.

Each result includes its repository basis/source refs.

Implement the WI054 analyzers in coherent groups:

1. next available work ID;
2. scope validity/ownership and overlap;
3. dependency/reverse dependency;
4. candidate dependency cycle and lifecycle-state effects;
5. acceptance/evidence gaps;
6. unresolved findings relevant to scopes;
7. applicable contracts for candidate paths;
8. frozen-surface intersections for candidate paths;
9. provenance/completion concerns;
10. structurally related work where explicit facts make the relation explainable.

Do not make opaque similarity scores or LLM output authoritative.

## Phase 5 — `repopact-mutation` plan-only boundary

Create typed domain structures for:

```text
MutationRequest
MutationPlan
MutationResult
ReadFact / ReadSet
File/PathOperation
GeneratedImpact
MutationDiagnostic
```

Initial request variants should remain work-item focused, conceptually:

- `CreateWorkItem`;
- `EditWorkItem`;
- `TransitionWorkItem`.

Use typed edit fields. Do not expose arbitrary JSON Patch or arbitrary file write.

A plan contains:

- repository identity;
- exact content-addressed read set, including expected-absent paths;
- deterministic plan token;
- durable path/file operations;
- generated dashboard impact;
- graph/relationship impact where useful;
- diagnostics/blocked reasons;
- canonical preview/diff.

**Implement and test plan-only behavior before implementing apply.** Planning must not mutate the repository.

## Phase 6 — read-set correctness and stale plans

Decision 0040 is binding.

- Use a deterministic collision-resistant content digest for relevant path state.
- Plan token derives from repository identity + sorted read facts.
- Do not use Git HEAD, branch, mtime, or file size as the primary correctness precondition.
- An absent target is a real precondition.
- Re-read relevant facts before apply.
- Reject any material drift; do not silently re-plan or merge.

Test at least:

- target appeared after planning;
- work record changed after planning;
- governance/owner input used by planning changed;
- dashboard source input used by the plan changed;
- unrelated file not in the read set does not necessarily stale the plan;
- linked-worktree identity remains correct.

## Phase 7 — work-item creation parity

Python `repopact.new.new_work_item` is the existing reference surface. Prove Rust before/after parity for equivalent inputs.

Cover:

- next ID;
- slugging;
- selected lifecycle directory;
- work-item JSON contents and canonical serialization;
- preflight marker/date semantics;
- README/template stamping;
- local-template precedence/fallback behavior;
- dashboard regeneration.

Do not silently improve/change Python behavior under a parity claim. If an existing behavior is wrong and must change, isolate it as a separately justified semantic correction with focused tests and durable rationale.

## Phase 8 — typed work-item edits

Support only explicitly typed fields required by the future desktop workflow. Preserve immutable identity/history semantics.

At minimum reason carefully about:

- `id` — immutable;
- `created` — immutable;
- preflight — not generic editable data;
- `status` — transition operation, not ordinary field edit;
- title/owner/affected scopes/dependencies — typed edits;
- acceptance criteria/state/evidence — typed edits subject to existing validation;
- updated date — derived consequence of successful mutation.

For edit operations that have no Python command equivalent, use checked-in canonical before/after fixtures and both validators rather than adding duplicate Python mutation code.

## Phase 9 — lifecycle transitions

Follow the formal model, not an invented UI state machine.

- Any lifecycle state may move to any other lifecycle state.
- Reopening completed work is allowed.
- Moving into completed is only successful if the supported post-state validates.
- Move the **entire work-item directory**.
- Update machine `status` consistently.
- Preserve README, AGENTS/audit companions, directives, evidence links, and all other directory content.
- Never silently drop history to make a completed/reopened record cleaner.

Test transitions including:

- proposed -> active;
- active -> blocked/deferred;
- active -> completed with valid criteria;
- attempted completed transition with pending criteria -> apply failure + rollback;
- completed -> active reopen preserving contents/evidence;
- transition of a directory containing additional files.

## Phase 10 — dashboard projection ownership

Reuse/refactor WI053's Rust dashboard rendering so validation and mutation generation have one implementation.

Work-item mutations must plan and regenerate the dashboard when required.

Do **not** port all of `generate_spec.py` unless an actually supported WI054 mutation changes SPEC inputs. Initial work-item operations do not require a general SPEC mutation port.

Callers never directly edit generated dashboard content through the mutation API.

## Phase 11 — frozen-surface behavior

Planning should deterministically identify protected path intersections and explain the relevant frozen entry/reason.

WI054 does not own operator authentication/approval. Do not implement a caller-controlled `approved=true` or equivalent and present it as operator authority.

For this work item, protected mutation apply fails closed unless an already-governed approval mechanism is explicitly available without porting WI050.

## Phase 12 — recoverable apply

Implement the smallest cross-platform recoverable write strategy that satisfies Decision 0040 and WI054.

Required behavior:

1. re-check repository/read-set preconditions;
2. compute/stage the complete candidate write set;
3. preserve preimages needed for rollback;
4. apply deterministically;
5. regenerate owned projections;
6. run supported Rust post-validation;
7. on any failure, restore the pre-state/clean staging;
8. report success only after valid completion.

Use failure injection tests at multiple points.

Do **not** introduce a persistent `.repopact` transaction database, daemon, or repository-local journal format without stopping and escalating. Ephemeral staging/recovery is preferred for WI054.

Also test an external target change close to/during apply and fail/rollback instead of merging it.

## Phase 13 — parity/fixture harness

Use two proof classes.

### Existing Python surfaces

Before/after Python/Rust tree parity for:

- create work item;
- dashboard rendering;
- any additional surface that genuinely has a Python reference operation.

### New Rust-native operations

For typed edits/transitions:

- canonical before fixture;
- Rust plan/apply;
- exact expected after fixture;
- Python reference validation;
- Rust validation;
- no unexpected file changes.

Do not compare only console text or success codes.

## Phase 14 — core façade

Expose proven capabilities through `repopact-core` in a form suitable for WI055 to wrap later:

- open/session;
- identity/snapshot;
- validate;
- graph queries;
- analysis;
- plan mutation;
- apply mutation.

No Tauri dependency. CLI additions are optional/testing-oriented and must not become the semantic owner.

## Regression gates before closeout

Run and record at least:

- editable Python install;
- `repopact validate` / Python validator;
- complete Python unit suite;
- Rust fmt/check/workspace tests;
- existing Python-driven Rust legacy conformance 20/20;
- WI050 admission corpus 8/8;
- WI053 linked-worktree/reference tests;
- graph deterministic tests;
- analysis source/basis tests;
- plan no-write tests;
- stale read-set tests;
- create Python/Rust before-after parity;
- dashboard Python/Rust parity;
- edit/transition canonical before-after fixtures + both validators;
- complete-directory lifecycle move preservation;
- failure injection + rollback/recovery tests;
- no-write/no-partial-success assertions;
- frozen-surface check from activation base;
- generated dashboard current;
- clean final worktree.

## Stop/escalate before proceeding if

Implementation appears to require:

- canonical schema changes;
- frozen `.github/workflows/**`, invariants, or charter changes;
- new persistent repository-local runtime transaction state;
- WI050 operator approval/admission/guard/IPC/platform semantics;
- generic decision/evidence overwrite authority;
- Tauri/PyO3 work;
- a lifecycle transition matrix narrower than the formal model;
- mutation correctness based on HEAD/mtimes;
- graph authority inferred from arbitrary prose;
- permanent dual Python/Rust mutation implementations for a new surface.

## Commit discipline

Prefer coherent commits by phase rather than one enormous commit, but keep `main` valid after each push. Preserve concurrent work. If another session advances `main`, integrate normally and rerun affected gates. No force push/reset/history rewrite.

## Closeout report

Report:

- final `main` SHA and commits;
- crate/API changes;
- graph nodes/edges/queries delivered;
- analyzers delivered and how findings are classified/sourced;
- exact mutation request variants/fields supported;
- read-set/plan-token design;
- stale-plan test results;
- create-work-item Python/Rust parity results;
- dashboard parity results;
- canonical edit/transition fixture results;
- lifecycle directory-preservation results;
- recovery/failure-injection results;
- Python/Rust validator results;
- legacy conformance and WI050 corpus counts;
- linked-worktree/reference results;
- frozen-surface result;
- any Python source changes and why;
- exact unsupported mutation/graph/analysis surfaces;
- whether Decision 0040 or WI052 sequencing needs amendment;
- whether WI054 is genuinely ready for closeout.

Do not close WI054 just because graph code exists or because one mutation succeeds. The plan/staleness/recovery/parity/post-validation contract is the milestone.
