# WI056 Sol Architecture Review

Date: 2026-09-10
Reviewer role: architecture pass before Codex implementation
Coding agent: Codex

## Executive conclusion

WI053-WI055 provide enough evidence to begin a controlled canonical-Rust cutover, but the cutover must remain surface-scoped. The correct compatibility seam is a versioned Rust executable protocol, not a CPython extension boundary and not human CLI text parsing.

The implementation should create one new machine boundary and then route only proven Python command surfaces through it. The legacy Python implementation remains useful as an independent regression oracle and for explicitly unmigrated commands, but it stops being product authority for migrated semantics.

## Current proven Rust layers

The Rust workspace already provides:

- `repopact-types`
- `repopact-schema`
- `repopact-repository`
- `repopact-validation`
- `repopact-graph`
- `repopact-analysis`
- `repopact-mutation`
- `repopact-core`
- `repopact-desktop-api`
- a developer/human `repopact-cli`
- the Tauri desktop transport/client

`RepoPactCore` is the semantic façade. WI056 should not duplicate repository/validation/mutation rules in a protocol crate or Python client.

## Why not PyO3 as the primary seam

PyO3 is technically viable and current stable-ABI support can reduce the Python-version matrix. It remains an extension-module architecture, however, and therefore makes CPython ABI families part of the compatibility story. The project does not need in-process Python calls for performance: its public Python contract is the console entrypoint, and the operations are repository-scale rather than micro-call workloads.

The process seam has stronger architectural properties here:

- language-neutral;
- usable by future non-Python clients;
- no CPython object model in the semantic core;
- process isolation for crashes/timeouts;
- easy fail-closed version handshake;
- same Rust crates remain used by Tauri and native Rust callers;
- no need to expose internal Rust types as a stable Python ABI.

## Packaging finding

Maturin currently supports `bin` bindings and mixed Rust/Python layouts. A Rust binary can be installed as a Python wheel script. This permits a Python-version-neutral protocol boundary while accepting the necessary platform-specific wheel matrix.

This is preferable to inventing a custom wheel-rewrite layer around setuptools.

The current release verifier is explicitly incompatible with this end state because it requires the wheel name `repopact-<version>-py3-none-any.whl`. WI056 must migrate that verifier deliberately.

## Canonical authority map

### Move to Rust authority

- repository discovery/modeling as consumed by migrated validation and native clients;
- JSON-schema-backed validation;
- structured validation diagnostics;
- dashboard projection;
- graph;
- deterministic analysis;
- typed work-item create;
- typed supported work-item edits;
- lifecycle transition;
- WI054 plan/apply/stale/recovery semantics.

### Remain Python or deferred

- init/adopt/import-plan/takeover/doctor unique mutation semantics;
- general SPEC generation;
- generic decision/policy mutation;
- evidence mutation;
- check-frozen Git-diff enforcement;
- fleet verification and release orchestration except packaging changes required by WI056;
- WI050 admission/approval/guard/enforcement and platform services.

The Python CLI is intentionally hybrid at command-dispatch level, but there is no overlap in canonical authority.

## Protocol shape

Add a small crate such as `repopact-protocol` containing only transport DTOs/version rules. Add a binary such as `repopact-engine` that:

1. reads one request from stdin;
2. validates protocol/version/request identity;
3. invokes `repopact-core` or the proven lower-level Rust crate;
4. writes exactly one structured response to stdout;
5. writes optional non-protocol logging only to stderr;
6. exits.

Do not make it a daemon in WI056.

### Minimum operations

- `handshake` / `capabilities`
- `validate`
- `dashboard.render` and/or `dashboard.write`
- `work.create`
- `work.propose`
- `work.amend_proposal`
- read-only `graph`
- read-only `analyze`

A generic arbitrary mutation operation is not required. If additional typed work edit/transition protocol operations are exposed, they must accept semantic request DTOs and invoke Rust planning internally.

## Mutation integrity across the process boundary

Do not send a Rust `MutationPlan` to Python and later accept that object back for execution. That would reproduce the authority-roundtrip problem solved in WI055.

For one-shot compatibility commands, the engine should receive typed intent, create its own snapshot/plan, and apply internally. A preview may be returned as information, but a later apply must re-plan from intent unless a separately proven integrity mechanism exists.

## Python compatibility client

Create a narrow module, conceptually `repopact.engine_client`, responsible for:

- locating the same-environment engine;
- optional explicit development override;
- handshake;
- exact product-version check;
- protocol-major check;
- request IDs;
- JSON serialization/deserialization;
- process timeout/cancellation;
- structured error translation;
- no parsing of human CLI output.

The migrated command dispatcher should depend on this client, not subprocess logic scattered through `cli.py`.

## Engine discovery trust model

Preferred order:

1. explicit test/development override when intentionally configured;
2. executable installed into the current Python environment's scripts directory by the package;
3. fail explicitly.

Do not silently search an arbitrary system PATH and execute whatever happens to be named `repopact-engine` without version verification.

Every discovered engine must handshake before semantic use.

## Version model

There are two identities:

- RepoPact product/package version;
- machine protocol major version.

Protocol-major mismatch is always incompatible.

The Python package and its packaged engine should match product version exactly. A checkout-only developer override may deliberately relax exact product version only through an explicit development switch, never as an automatic fallback.

Prefer deriving engine product identity from the repository's existing version/release records at build time rather than forcing every internal Cargo crate to adopt public product semver immediately.

## Migrated Python CLI behavior

At minimum:

### `validate`

Python sends `validate`, renders the returned structured diagnostics in the historical user-facing style, and maps semantic validity to the existing process exit code.

The legacy Python validator may still be invoked by dedicated regression tests, but not as automatic fallback.

### `dashboard`

Python requests canonical Rust dashboard write/render. The old Python generator remains a parity oracle if useful.

### `new work-item`

Python converts existing CLI fields to a typed Rust create request. `new decision` and `new policy` remain Python because generic narrative mutation was not proven by WI054.

### `work propose`

Route to canonical Rust creation with `proposed` lifecycle status.

### `work amend-proposal`

Preserve the existing proposed-only restriction. That constraint should be checked in the Rust semantic operation, not trusted to a Python precheck alone.

## Retained Python commands that need validation

Commands such as `init`, `adopt`, `import-plan`, or `doctor` may retain their unique Python mutation logic. If they perform a final conformance check, that final check should call canonical Rust validation through the compatibility client.

Do not rewrite these complex surfaces into Rust inside WI056 unless all semantics and acceptance evidence are deliberately expanded first.

## Conformance transition

The current conformance runner historically uses Python as the reference and an alternate `--command` implementation.

After cutover:

- fixture expectations/spec are the authority;
- Rust engine is the default canonical implementation tested against them;
- Python legacy validation runs as an explicit compatibility/regression comparator;
- alternate-implementation testing remains possible without implying Python is canonical.

Preserve the 20-case legacy corpus and 8-case WI050 corpus. WI050 remains Python-owned and should continue to be reported separately.

## Packaging implementation direction

Prefer Maturin mixed Rust/Python `bin` packaging over PyO3.

Requirements:

- keep Python package `repopact`;
- keep console command `repopact`;
- package `repopact-engine` native binary in platform wheel;
- Python >=3.11 remains the supported Python line unless separately changed;
- wheel install must not need Rust;
- sdist/source install may require Rust + Maturin and must say so;
- include canonical schemas/templates required by retained Python behavior;
- do not accidentally package Tauri desktop binaries in the Python wheel;
- do not expose the developer Rust CLI as the compatibility engine unless the protocol is truly separated from human output.

## Release verifier migration

`release_build.py` currently verifies one pure wheel and one sdist. Update it so its structural assertions match the new package instead of weakening them.

Prove:

- platform wheel name/tags are valid;
- wheel contains exactly the intended Python root;
- wheel contains `repopact-engine` in the correct scripts payload/location;
- engine handshake product version equals wheel metadata;
- no obsolete flat modules;
- schemas/templates remain present where intended;
- clean-source build still required;
- two builds are compared for reproducibility and any platform-specific normalization is documented rather than silently ignored.

## Platform claims

The development host is Windows. WI055 produced Windows Tauri artifacts but did not execute Linux/macOS.

WI056 may close with Windows packaging/install/runtime proof if the work-item contract says cross-platform execution is recorded to the available level, but it must not claim Linux/macOS runtime validation without evidence. It should leave deterministic build instructions/configuration for those platforms and record them as unexecuted if CI remains intentionally disabled.

Do not modify `.github/workflows/**` merely to obtain the matrix; those paths remain frozen.

## Rollback

No durable schema migration should be required by WI056. Therefore rollback is executable/package rollback.

A previous package can be reinstalled without changing governed repository records. If implementation discovers a need for a new durable protocol marker in adopted repositories, stop and govern that separately rather than smuggling it into the cutover.

## WI050 quarantine

The new engine protocol must not expose `admission`, `approval`, `guard`, protected-service, lease, operator-key, or enforcement operations.

Python continues to own those command routes. This is an intentional authority partition, not a migration failure.

## Implementation phases

1. Establish baseline and inventory every CLI branch by authority.
2. Add protocol DTO/version crate and engine binary.
3. Add engine handshake/version tests and machine protocol tests.
4. Add Python engine client with deterministic discovery/version checks.
5. Migrate `validate`.
6. Flip conformance/default authority and keep Python legacy comparator explicit.
7. Migrate dashboard and proven work-item command routes.
8. Route final validation of retained Python workflows through Rust where safe.
9. Migrate packaging to Maturin `bin` mixed package and update release verifier.
10. Build/install fresh Windows wheel and prove no-Rust wheel installation.
11. Prove sdist/source build requirements.
12. Update documentation/claims and record explicit non-parity.
13. Run full Python/Rust/frontend/desktop/conformance/WI050/frozen regression gates.
14. Close WI056 only after fresh-installed CLI tests pass.

## Stop/escalate conditions

Stop and record an architecture amendment if implementation would require:

- changing canonical schemas/invariants/charter;
- adding repository-local runtime/protocol state;
- accepting raw filesystem operations over the engine protocol;
- making a network service mandatory;
- porting WI050 authority;
- changing frozen CI workflows;
- silently dropping an existing Python command;
- requiring a durable repository format migration for compatibility;
- describing unexecuted Linux/macOS packaging as proven.

## After WI056

When WI056 closes, review WI052 as an umbrella. If its acceptance criteria are now satisfied by WI053-WI056 evidence, close WI052 with a synthesis evidence record rather than leaving the migration umbrella permanently active.
