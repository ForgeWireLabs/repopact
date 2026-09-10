# WI052 Architecture Inventory and Migration Matrix

**Inventory date:** 2026-09-09  
**Base `main`:** `7ee3a9ca93f49f577d6150005abb52e5d1b475dd`  
**Purpose:** convert WI052's destination architecture into an implementation sequence grounded in the live RepoPact repository.

## Executive conclusion

The Rust migration must begin below the UI and below mutation authority.

The first useful Rust milestone is not "a Tauri shell" and it is not merely "Rust structs for the JSON schemas." RepoPact's validator depends on repository topology and semantic behavior including linked Git worktrees, ignored paths, record discovery, record-relative reference semantics, lifecycle/dependency rules, generated-dashboard integrity, and deterministic diagnostics. Those repository semantics must be present in the first Rust slice or the resulting core would not be capable of judging a real RepoPact checkout.

The migration therefore uses four bounded implementation waves:

1. **WI053:** Rust workspace + typed records/schema support + repository model + validator conformance.
2. **WI054:** relationship graph + deterministic analysis + transactional mutation/generation authority.
3. **WI055:** Tauri 2 desktop workbench as a narrow client of the proven Rust APIs.
4. **WI056:** Python compatibility strategy and explicit canonical-core cutover.

WI050's protected admission/enforcement substrate is not part of these first migration waves while WI050 remains active.

## Governing constraints discovered

The implementation must preserve these repository-level rules:

- Machine-readable lifecycle state is in JSON; Markdown explains intent/history.
- Each change belongs to an explicit owner scope.
- Work-item preflight is mandatory for current IDs.
- Completed work/history is durable and must not be rewritten for convenience.
- Derived artifacts are generated, not independently hand-authored sources of truth.
- Linked Git worktrees are structural repository entities and must not be accidentally rediscovered as nested RepoPact roots/records.
- Source-of-truth references use the repository's established record-relative semantics; Rust must not introduce a different path interpretation.
- Alternate implementations are judged through the versioned conformance corpus rather than by implementation-language preference.

## Current Python semantic surface

The current Python package is substantially broader than a Markdown parser. The migration boundary is classified below.

| Current surface | Current responsibility | Rust destination / disposition | Wave |
| --- | --- | --- | --- |
| `repo_model.py` | statuses, record discovery, ignored paths, contracts, evidence IDs, linked-worktree recognition | `repopact-repository` | WI053 |
| `frontmatter.py` | canonical frontmatter parsing support | `repopact-schema` / repository parsing boundary | WI053 |
| `validate_repo.py` | repository semantic validation and diagnostics | `repopact-validation` | WI053 |
| JSON schemas | language-neutral record contracts | remain canonical; loaded/validated by `repopact-schema` | WI053 |
| `generate_dashboard.py` | canonical dashboard projection used by validation | Rust projection needed for validator parity; generalized mutation-generation ownership follows | WI053/WI054 |
| `generate_spec.py` | generated specification projection | generation parity | WI054 |
| `check_frozen_surface.py` | protected/frozen-surface change checks | repository/analysis boundary | WI054 unless required by an earlier conformance fixture |
| `new.py` | governed work creation | mutation planning/application | WI054 |
| `plan_import.py` / `track_import.py` | import planning/tracking | compatibility/mutation consumers | WI054/WI056 |
| `adopt_repo.py` / `takeover.py` | adoption/takeover and reference rewriting | compatibility/mutation consumers; preserve record-relative semantics | WI054/WI056 |
| `doctor.py` | repository diagnostics/repair guidance | Rust diagnostics/analysis consumer after validator foundation | WI054/WI056 |
| `fleet_verify.py` | fleet/release verification | later compatibility/cutover consumer | WI056 |
| release/package helpers | release surfaces | later distribution/cutover | WI056 |
| `cli.py` | multiplexes validation, mutation, import, release, and WI050 operations | keep Python independently usable until WI056; do not reimplement command-by-command ad hoc | WI056 |
| `admission.py` | WI050 admission semantics | **defer**; reserved future Rust boundary only | post-WI050 |
| `enforcement.py` | WI050 enforcement semantics | **defer** | post-WI050 |
| `guard.py` / `guard_ipc.py` | protected guard and IPC | **defer** | post-WI050 |
| `platform_backends.py` | protected cross-platform host substrate | **defer** | post-WI050 |
| `windows_guard_service.py` | Windows protected service | **defer** | post-WI050 |
| admission conformance | WI050-specific policy/security corpus | remains additive and separate from initial Rust validator parity | post-WI050 |

## Existing conformance is the first arbiter

`repopact.run_conformance` already supports an alternate implementation command template using `{repo}`. For each legacy fixture it materializes an isolated repository, computes the Python reference result, invokes the alternate command, and requires:

- exit code `0` for accepted fixtures;
- non-zero exit for rejected fixtures;
- the expected primary diagnostic text in the alternate implementation output;
- fixture isolation on the Python reference side so unexpected secondary violations do not mask a test.

This gives WI053 a concrete interoperability target without changing the standard to accommodate Rust.

### Current validator fixture themes

The published corpus includes coverage for conditions such as:

- active dependency on proposed work;
- completed provisional work;
- completed work with pending criteria;
- dependency cycles;
- disjoint active-scope policy;
- invalid semantic versions;
- missing/stale dashboards;
- orphan work directories;
- release/readme/version drift;
- satisfied criteria without evidence;
- schema-invalid evidence;
- status/directory mismatch;
- unknown dependencies;
- unregistered contracts;
- valid proposed/provisional/release-label repositories.

WI053 must drive the Rust validator through this existing interface. It must not create a Rust-only test suite and call that conformance.

## Conformance gaps relevant to later waves

The existing alternate-implementation corpus is primarily a validator contract. It does not, by itself, prove parity for every Python operation.

Before mutation/cutover, additional parity fixtures are required for:

- canonical work-item creation;
- lifecycle moves and dependency edits;
- dashboard/spec generation where Rust assumes generation authority;
- transactional failure/no-partial-write behavior;
- decision/evidence relationship changes;
- record-relative reference rewriting used by adoption/takeover flows;
- diagnostics/doctor behavior claimed as compatible;
- Python CLI compatibility after Rust becomes the underlying authority.

Those belong to WI054/WI056 rather than inflating WI053.

## First Rust workspace boundary

WI053 should establish a workspace resembling:

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
    repopact-cli/        # minimal alternate-implementation/conformance entrypoint
```

Names can change if implementation evidence demonstrates a cleaner boundary, but the responsibilities may not collapse into Tauri or a monolithic desktop backend.

### `repopact-types`

Typed serializable domain structures only. No Tauri and no arbitrary filesystem logic.

Minimum useful types include lifecycle/status, provenance, work item, acceptance criterion, dependency, scope/record identity, evidence references, repository identity, and structured diagnostics.

### `repopact-schema`

Load/use the canonical RepoPact JSON schemas. Rust types do not replace those schemas. Schema errors need stable normalization so downstream clients do not parse library-specific prose forever.

### `repopact-repository`

Own repository identity and discovery:

- root/path normalization;
- ignored-directory behavior;
- linked-worktree detection;
- work-item discovery by lifecycle directory;
- evidence discovery;
- nested `AGENTS.md` discovery;
- repository/common-dir identity where required;
- record-relative reference resolution required by validated semantics.

### `repopact-validation`

Own Rust-supported semantic checks and structured diagnostics. Human-readable messages may preserve expected compatibility substrings while carrying stable codes/context for future native clients.

### `repopact-core`

A reusable façade composed from the lower crates. It must remain independent of Tauri. The first exposed operation is repository validation/snapshot loading, not governed writes.

### Minimal Rust CLI

Provide an executable suitable for the existing conformance runner, conceptually:

```text
repopact-rs validate --root <repository>
```

The command must return zero for valid supported repositories and non-zero for invalid/unsupported states, with deterministic diagnostics on stdout/stderr. The exact executable name may be refined, but the published conformance runner must be able to invoke it through `--command` without a custom bypass.

## Diagnostic strategy

The first Rust implementation should introduce structured diagnostics internally even where the legacy conformance test presently matches message substrings.

Minimum shape:

```text
code
severity
message
record/path
field (optional)
related records (optional)
suggested actions (optional)
```

Do not couple durable diagnostic identity to the wording required by old message-substring fixtures. Preserve compatibility text while creating stable machine identity for Tauri and later CLI consumers.

## Authority gates

Rust authority advances only through explicit gates.

### Gate 1 — Read parity

The same supported checkout yields equivalent discovered governed records, statuses, dependencies, contracts, evidence IDs, and relevant repository identity/path semantics.

### Gate 2 — Validation parity

The published legacy conformance corpus passes through the Rust command. Rejected fixtures contain the expected primary diagnostic. Unsupported surfaces fail explicitly.

### Gate 3 — Mutation parity

WI054 introduces before/after fixtures proving canonical mutations, generated projections, rollback/no-partial-write behavior, and post-write validation.

### Gate 4 — Client parity

The Tauri client uses only typed/narrow Rust operations for governed behavior. The frontend contains no independent RepoPact validator or direct governed-file mutation path.

### Gate 5 — Canonical-core cutover

WI056 records the binding decision, packaging model, compatibility strategy, rollback path, and complete known non-parity list before Rust is described as the canonical executable core.

## WI050 quarantine boundary

WI050 is still proving security-critical protected admission/enforcement behavior. The following are explicitly outside WI053–WI056 unless a later governed amendment says otherwise:

- admission-policy authority migration;
- signed authorization/lease semantics migration;
- guard process/service migration;
- guard IPC migration;
- protected platform backend migration;
- Windows protected service migration;
- changes that weaken existing fail-closed behavior;
- treating a desktop capability boundary as equivalent to WI050 protected enforcement.

A future Rust admission/enforcement migration should be its own security work after WI050's pending closeout surfaces are stable.

## Tauri boundary discovered

The future desktop should call narrow core operations such as repository open/snapshot, item reads, deterministic analysis, mutation planning, mutation apply, validation, and graph queries.

The frontend must not receive generic `read_any_file`, `write_any_file`, or unrestricted shell execution merely for convenience. Filesystem watching belongs in the Rust repository layer and should emit typed/coalesced repository-change events after reindex/revalidation.

This is independent of WI050 and does not make the desktop an enforcement authority.

## Optional assisted-analysis boundary

LLM assistance is deferred until deterministic analysis exists. A future adapter may suggest acceptance criteria, decomposition, dependencies, or evidence plans, but assistant output is proposal-only and cannot activate work, waive criteria, approve frozen-surface changes, fabricate authorization, or bypass validation.

No provider belongs in the RepoPact authority kernel.

## Subordinate work map

```text
WI052  Rust Core + Tauri program architecture (active umbrella)
  |
  +-- WI053  Rust workspace/repository/schema/validator conformance (active)
          |
          +-- WI054  graph + deterministic analysis + transactional mutation (proposed)
                  |
                  +-- WI055  Tauri 2 governance workbench (proposed)
                          |
                          +-- WI056  Python compatibility + canonical-core cutover (proposed)
```

The dependency chain is intentionally conservative. Later implementation may prove some UI shell work can proceed in parallel, but no UI path may assume mutation authority before WI054 proves it.

## Coding-agent boundary

Codex is the implementation agent for WI053.

Codex must start from the latest `main`, read the applicable agent contracts and this inventory, and implement only WI053's bounded Rust foundation. It must not begin Tauri, PyO3 cutover, or WI050 migration. Any newly discovered architecture conflict should be recorded against WI053/WI052 rather than patched around silently.

The durable execution directive is stored with WI053.
