# Work Item 053 — Rust Workspace, Repository Model, Schema and Validator Conformance

**Status:** Completed

**Owner:** work-coordinator

**Affected scopes:** work, tooling, governance, docs, evidence

**Depends on:** WI052

## Intent

Create RepoPact's first executable Rust foundation and prove that it can interpret and validate the existing language-neutral RepoPact repository contract without becoming a second semantic authority.

This is the first implementation slice under WI052. It deliberately stops before governed mutation authority, Tauri UI work, Python cutover, or WI050 admission/enforcement migration.

## Why this slice comes first

The live architecture inventory showed that RepoPact validation depends on more than JSON schema decoding. The current reference implementation includes repository discovery, linked Git worktree recognition, ignored paths, work/evidence/contract indexing, record-relative references, lifecycle/dependency semantics, generated-dashboard integrity, and deterministic diagnostic behavior.

A Rust foundation that models only structs/schemas would not be able to judge a real RepoPact checkout and would encourage the UI or later callers to fill semantic gaps independently. WI053 therefore treats repository topology + validation conformance as one foundation.

## Authority model

Throughout WI053:

```text
RepoPact standard / schemas / decisions / conformance
                      |
                      v
       Python reference implementation
                      |
                conformance arbiter
                      |
                      v
          Rust alternate implementation
```

Rust is **not canonical** during this item. Passing Rust unit tests alone is insufficient. The published alternate-implementation conformance runner must invoke the Rust validator command against its materialized fixtures.

## In scope

### 1. Rust workspace

Create a top-level `rust/` workspace with separation comparable to:

```text
rust/
  Cargo.toml
  crates/
    repopact-types/
    repopact-schema/
    repopact-repository/
    repopact-validation/
    repopact-core/
  apps/
    repopact-cli/
```

The exact crate names may be refined if implementation demonstrates a materially better boundary, but Tauri dependencies are prohibited from the reusable domain crates.

### 2. Typed domain model

Implement the minimum typed domain surface needed for conformance and repository snapshots, including:

- lifecycle/status;
- provenance;
- work items;
- acceptance criteria;
- dependencies;
- scopes/record identity;
- evidence references/identity needed by validation;
- repository identity;
- structured validation diagnostics.

Rust types are implementation aids. They do not supersede the canonical JSON schemas.

### 3. Canonical schema use

The Rust implementation must consume/validate against RepoPact's existing language-neutral schemas or a mechanically equivalent embedded/package representation derived from those files.

Do not fork a second hand-maintained Rust schema definition.

Schema diagnostics should be normalized behind RepoPact-owned diagnostic identity/context rather than exposing a third-party JSON-schema library's unstable prose as the future client contract.

### 4. Repository model

Implement the repository semantics required by validation:

- RepoPact root handling;
- ignored-directory semantics;
- linked Git worktree recognition;
- work-item discovery by lifecycle directory;
- evidence discovery;
- applicable/nested `AGENTS.md` discovery where required by validation;
- repository/common-dir identity where required;
- normalized paths;
- established record-relative reference behavior;
- deterministic ordering.

Do not simplify linked worktrees to "ignore every directory containing `.git`" or treat all references as repository-root relative. Existing decisions/reference behavior must be preserved.

### 5. Validator

Implement the semantic checks needed to satisfy the published legacy conformance corpus.

The Rust validator should use structured diagnostics internally with a minimum model such as:

```text
code
severity
message
path/record
field (optional)
related_records (optional)
suggested_actions (optional)
```

Legacy expected-message substrings may be preserved in `message` for compatibility, while `code` becomes the stable native identity for later consumers.

### 6. Dashboard integrity needed by validator parity

Because stale/missing dashboard behavior is part of the existing validation contract, implement the canonical dashboard projection/comparison required for Rust validator parity.

This does **not** grant general governed mutation authority. WI054 will own the transactional generation/write boundary.

### 7. Alternate-implementation command

Provide a minimal Rust executable suitable for the existing runner, conceptually:

```text
repopact-rs validate --root <repo>
```

The executable must:

- exit `0` on accepted repositories;
- exit non-zero on rejected or explicitly unsupported repositories;
- emit deterministic diagnostics containing the expected compatibility message for existing reject fixtures;
- avoid modifying the repository during validation.

The existing Python conformance runner must be able to execute it through `--command "... --root {repo}"`.

### 8. Cross-platform semantics

Repository/path/worktree behavior must be designed for Windows, Linux, and macOS rather than depending on Unix-only path or process assumptions.

Executable unit/integration tests should cover portable path behavior. `.github/workflows/**` is frozen; WI053 does not authorize modifying frozen CI without separate human approval under the existing frozen-surface contract.

## Existing conformance target

The published corpus already isolates validator rules around lifecycle/dependency consistency, provisional/completed state, schema-invalid evidence, status-directory mismatch, unknown dependencies, cycles, active-scope policy, contracts, semantic versions, release labels, and missing/stale dashboards.

Codex must run the Rust command through the existing `repopact.run_conformance` alternate-command interface. It must not replace that gate with a Rust-only duplicate.

Note that the current runner also reports the separate WI050 admission corpus. WI053 is responsible for the legacy/alternate validator path only; existing admission behavior must remain green but is not ported to Rust here.

## Explicitly out of scope

- Tauri desktop application;
- work-item/decision/evidence mutation authority;
- graph/analysis engine beyond what validation requires;
- PyO3 or native Python wheels;
- replacing the Python CLI;
- claiming Rust canonical authority;
- porting `admission.py`, `enforcement.py`, `guard.py`, `guard_ipc.py`, `platform_backends.py`, `windows_guard_service.py`, or WI050 admission conformance;
- weakening diagnostics/conformance to make Rust easier to pass;
- modifying frozen CI without separate approval.

## Implementation sequence

1. Read root `AGENTS.md`, WI052, `architecture-inventory.md`, this WI, and applicable tooling contracts.
2. Confirm clean/up-to-date `main` and run current Python baseline validation/tests.
3. Establish the Rust workspace and domain/schema crate boundaries.
4. Implement repository discovery/topology semantics before broad validator rules.
5. Implement structured diagnostics and validator rules fixture-by-fixture against the reference behavior.
6. Add the minimal alternate-implementation CLI.
7. Run the existing conformance runner against that CLI continuously.
8. Add focused parity tests for linked-worktree and record-relative path decisions even where the current conformance corpus is not sufficient.
9. Keep Python reference behavior unchanged unless a real cross-implementation discrepancy reveals a proven Python defect; such a defect must be recorded rather than silently normalized.
10. Produce immutable evidence and update WI053 acceptance criteria only after required Python + Rust + conformance checks are green.

## Closeout standard

WI053 may close when RepoPact has a reusable non-Tauri Rust foundation that can load supported repositories and pass the published legacy validator conformance interface, with explicit unsupported surfaces, structured diagnostics, portable repository semantics, and no claim that governed mutation or canonical-core cutover has occurred.

WI054 must not assume mutation authority until WI053's conformance evidence is durable.

## Closeout

WI053 is complete on the published read/validation milestone. The durable
evidence record is
[`20260909-053-rust-validator-conformance`](../../../evidence/runs/20260909-053-rust-validator-conformance.json).
It records the Python baseline and post-change checks, Rust workspace tests,
the existing alternate-implementation conformance result (20/20 legacy cases
and 8/8 WI050 admission vectors), linked-worktree/reference parity, frozen
surface result, and exact remaining non-parity surfaces.

Rust remains an alternate implementation. Tauri, governed mutation,
transactional generation, Python cutover, and WI050 admission/enforcement
migration remain outside this work item.
