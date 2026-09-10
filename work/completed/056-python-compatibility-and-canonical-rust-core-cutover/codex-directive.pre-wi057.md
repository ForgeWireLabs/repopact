# Codex Execution Directive — WI056

**Coding agent:** Codex
**Authoritative work item:** WI056
**Architecture umbrella:** WI052
**Prerequisites:** WI053, WI054, WI055 completed
**Architecture decision:** Decision 0042
**Delivery mode:** direct to `main`, preserving concurrent work; no force-push/reset/history rewriting.

## Mission

Execute RepoPact's deliberate, surface-scoped cutover from Python-reference/Rust-alternate to canonical Rust semantics for the surfaces already proven by WI053/WI054, while preserving the existing Python `repopact` CLI as a compatibility client and keeping explicitly unmigrated/security-critical operations on their existing authority paths.

The target is **one semantic engine**, not "all code must be Rust."

Do not close WI056 because a Rust subprocess exists. The compatibility protocol, Python routing, conformance-authority flip, platform wheel packaging, clean install proof, version mismatch behavior, rollback story, and regression evidence are all required.

## Required reading before edits

Read in this order:

1. root `AGENTS.md` and `repopact/AGENTS.md`;
2. `work/active/052-rust-core-and-tauri-2-user-workbench-foundation/architecture-inventory.md`;
3. completed WI053, WI054, WI055 READMEs/evidence;
4. active WI056 `README.md`, `work-item.json`, and `architecture-review.md`;
5. Decisions 0040, 0041, 0042;
6. `repopact/__init__.py`, `repopact/cli.py`, `repopact/run_conformance.py`, `repopact/release_build.py`, `pyproject.toml`;
7. all Rust core crates and `rust/apps/repopact-cli`;
8. WI050 active work, Decision 0039, and security-related Python modules before touching any admission/approval/guard path.

## Start-state rules

- Fetch latest `origin/main` and verify Decision 0042 + WI056 activation are present.
- Begin from a clean worktree.
- Preserve concurrent work before each push.
- No force push, hard reset of shared history, or rewritten completed work.
- Keep Cargo target output outside the checkout.
- Establish the complete pre-change baseline before implementation.
- Do not modify `.github/workflows/**`, canonical schemas, invariants, or charter without separate operator-approved governance.

## Baseline

Record exact results for at least:

```text
python -m pip install -e ".[dev]"
python -m repopact.validate_repo
python -m unittest discover -s tests -v
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo check --manifest-path rust/Cargo.toml --workspace
cargo test --manifest-path rust/Cargo.toml --workspace
python -m repopact.run_conformance --command "<current-rust-validator> validate --root {repo}"
```

Also run the focused WI053 linked-worktree/reference suite and WI055 frontend type/binding/tests/build as appropriate. Preserve the WI050 8/8 admission corpus.

## Phase 1 — command authority inventory

Before moving code, produce a concrete table of every current `repopact` CLI command/subcommand with:

- current Python implementation module;
- whether it uses validation/generation/mutation semantics already proven in Rust;
- intended WI056 authority after cutover: Rust canonical, Python retained, or deferred;
- any final-validation dependency that should route to Rust even though the command itself remains Python;
- WI050/security involvement.

Use the WI056 README authority map as the default. Do not expand migration scope opportunistically.

## Phase 2 — protocol crate

Create a small reusable transport-contract crate, preferably `repopact-protocol` unless repository evidence supports a better name.

It should own only protocol DTOs/version/serialization rules, not governance semantics.

Define deterministic serializable types equivalent to:

```text
EngineRequest
  protocol
  protocol_version
  request_id
  operation
  root?
  params

EngineResponse
  protocol
  protocol_version
  request_id
  engine_version
  ok/transport_status
  semantic_result?
  diagnostics[]
  error?
  capabilities?
```

Use typed enums/DTOs where practical; avoid arbitrary stringly JSON internally.

Required protocol properties:

- protocol name constant;
- protocol major version constant;
- stable request-id echo;
- deterministic JSON encoding;
- structured diagnostics;
- explicit unsupported operation/version errors;
- product-version field;
- no human-output parsing contract.

## Phase 3 — `repopact-engine`

Create a dedicated machine executable, conceptually:

```text
rust/apps/repopact-engine
```

Do **not** turn the existing human/developer `repopact-cli` output into the protocol.

Initial runtime model:

```text
stdin: exactly one JSON request
 -> parse/validate envelope
 -> handshake/version checks
 -> invoke existing Rust core
 -> serialize exactly one JSON response to stdout
 -> exit
```

Non-protocol logs go to stderr only.

No daemon, socket, named pipe, HTTP server, background service, or persistent state.

### Minimum engine operations

Implement at least:

- `handshake` / `capabilities`;
- `validate`;
- `dashboard.write` or equivalent canonical dashboard operation;
- `work.create`;
- `work.propose`;
- `work.amend_proposal` preserving proposed-only behavior;
- read-only `graph`;
- read-only `analyze`.

If you add typed edit/transition operations, keep them semantic and backed by WI054 planning/apply.

Do not add generic arbitrary patch/write operations.

## Phase 4 — product-version identity

The engine needs RepoPact product identity that agrees with Python package identity.

Prefer a build-time/generated mechanism derived from canonical `VERSION`/`RELEASE_LABEL` behavior rather than pretending every internal Cargo crate version is the public RepoPact version.

Tests must cover:

- matching package/engine version;
- mismatched engine product version;
- mismatched protocol major;
- malformed request;
- unsupported operation;
- valid semantic rejection distinct from transport failure.

## Phase 5 — Python engine client

Add one compatibility module, conceptually `repopact/engine_client.py`.

It owns all process/protocol mechanics:

- same-environment engine location;
- optional explicit development override;
- handshake and capability cache if useful;
- exact product-version verification;
- protocol-major verification;
- request-id generation;
- JSON request/response;
- timeout/cancellation;
- deterministic errors;
- Windows executable suffix handling;
- no stdout presentation scraping.

### Discovery

Default to the engine installed into the **same Python environment's scripts directory**.

An explicit environment variable/CLI development override may point to a checkout-built engine, but it must still handshake and version-check unless the explicit dev mode documents why product-version looseness is needed.

Do not silently search and trust arbitrary PATH executables.

## Phase 6 — migrate `repopact validate`

This is the first authority flip.

The public Python CLI `validate` path must call the engine client and render structured Rust diagnostics into compatible human output.

Preserve practical exit semantics:

- conformant repository -> success;
- semantic validation failure -> user-facing validation failure/nonzero;
- engine/protocol/version failure -> explicit compatibility/runtime error/nonzero.

There must be **no automatic fallback** to `validate_repo.validate` for the public migrated command.

Keep the legacy Python validator available for dedicated tests/reference comparison for now.

## Phase 7 — conformance authority flip

Refactor the conformance harness so the product's canonical default is Rust-first rather than Python-reference-first.

Preserve the published fixtures and ability to test alternate implementations.

A good end-state is conceptually:

- fixture manifest/spec expectations define expected accept/reject;
- canonical Rust engine/validator is run by default;
- optional/explicit legacy Python comparator is run as regression evidence;
- WI050 admission corpus remains Python-owned and separately reported.

Do not make an implementation pass by changing fixture expectations to match it.

Record exactly how command-line compatibility for existing `--command` users changes, if at all.

## Phase 8 — migrate dashboard

Route public `repopact dashboard` to the Rust canonical projection/write path.

Keep Python `generate_dashboard.generate` as an independent parity oracle if useful, but remove it from product authority for the public command.

Run Python/Rust byte-parity fixtures after the change.

## Phase 9 — migrate proven work-item commands

Migrate:

- `repopact new work-item`;
- `repopact work propose`;
- `repopact work amend-proposal`.

Preserve existing CLI UX/paths/status semantics.

`work amend-proposal` must remain proposed-only. Implement that semantic precondition in Rust authority, not only Python presentation code.

Do not migrate `new decision` or `new policy`; those are generic narrative mutations outside WI054 proof.

Do not expose raw work-item JSON editing.

## Phase 10 — retained Python workflows and canonical validation

Inspect retained Python workflows that call `validate_repo.validate` after they mutate state, especially:

- `init`;
- `adopt`;
- `import-plan`;
- `doctor` where applicable.

Where doing so does not create recursion or alter their unique semantics, make their **final validity decision** consume the engine/Rust validator.

Keep their mutation implementation Python-owned.

Do not rewrite adopt/import/takeover/doctor into Rust in this item.

For unit tests that directly exercise legacy modules, direct Python validator calls may remain as reference tests.

## Phase 11 — packaging spike before replacing build backend

Before editing root packaging, prove Maturin can build the intended mixed Python + `bin` wheel from this repository layout with path-dependent Rust crates.

Do the spike in a controlled temporary/config branch/worktree or uncommitted local setup first if useful.

Prove the wheel can contain:

- Python package `repopact`;
- `repopact` Python console entrypoint;
- `repopact-engine` native executable as a wheel-installed script;
- schemas/templates needed by retained Python operations;
- no Tauri desktop executable;
- no obsolete flat modules.

If Maturin cannot cleanly support the layout without architectural compromise, stop and report before inventing a custom wheel format.

## Phase 12 — migrate root packaging

If the spike passes, migrate `pyproject.toml` to the selected Maturin mixed/bin build arrangement.

Requirements:

- keep project name `repopact`;
- preserve package version behavior;
- preserve Python >=3.11 unless separately governed;
- preserve `repopact` console script;
- install `repopact-engine` in the environment;
- wheel must be platform-specific but not CPython-extension-ABI-specific;
- sdist contains all Rust source/path dependencies required to build;
- source build requirements are documented.

Do not add PyO3 merely because Maturin is the backend.

## Phase 13 — release verifier migration

Update `repopact/release_build.py` and tests to understand the new artifact identity.

Delete the obsolete requirement that the wheel be named exactly `py3-none-any`.

Replace it with stricter relevant checks:

- expected project/version;
- valid platform wheel tag;
- Python tag compatible with supported Python line;
- no CPython extension ABI dependency where the bin packaging does not require one;
- exact `repopact` package root;
- engine executable present in scripts payload;
- engine product-version handshake matches artifact metadata after install;
- schemas/templates count/content expectations;
- no flat root modules;
- no desktop app payload;
- SHA-256 evidence;
- sdist contains required Rust workspace sources and Python package;
- clean-tree and reproducibility checks remain.

Do not weaken reproducibility silently. If Rust/Windows linking creates a genuine reproducibility obstacle, isolate and document it and stop before reducing the invariant without governance.

## Phase 14 — fresh-wheel proof

Build release artifacts from a clean committed/source-exported state according to the updated release builder.

On Windows, create a **new isolated venv** with no source checkout injected.

Install the built wheel.

Prove:

```text
repopact --help
repopact validate --root <scratch-valid-repo>
repopact dashboard --root <scratch-repo>
repopact new work-item "..." --root <scratch-repo>
repopact work propose "..." --root <scratch-repo>
```

Also prove engine handshake from the installed environment and exact package/engine version coupling.

The wheel install itself must not require Rust/Cargo.

Then deliberately remove/rename the engine or point to an incompatible test engine and prove the Python CLI fails explicitly without executing the legacy Python implementation.

## Phase 15 — source-build proof

Separately prove the sdist/source installation story.

Record:

- required Rust version/toolchain;
- Maturin requirement;
- whether network access is needed for Cargo dependencies when not cached;
- command used;
- resulting engine/package identity.

Do not describe sdist installation as compiler-free.

## Phase 16 — Windows package evidence and cross-platform status

The available host is Windows.

Produce actual Windows wheel/install/runtime evidence.

For Linux/macOS:

- provide deterministic build commands/configuration;
- use compile/cross-build checks only if technically meaningful and available;
- record runtime/package execution as **unexecuted** unless truly run;
- do not modify frozen CI workflows merely to obtain a matrix.

Production signing/notarization remains separate unless already available.

## Phase 17 — documentation and claims

Update user/developer docs to explain:

- Rust is canonical for the exact migrated surfaces;
- Python CLI is a compatibility client for those surfaces;
- which commands remain Python-owned;
- engine discovery/version behavior;
- wheel/source install requirements;
- platform evidence boundaries;
- WI050 remains on the existing protected Python/provider path;
- no "all Rust" or "rewrite complete" claim.

Do not publish a new release as part of WI056 unless separately directed and all release governance is satisfied.

## Phase 18 — regression and closeout gates

Run, record exact counts/results, and fix regressions:

### Python

```text
python -m pip install -e ".[dev]"
python -m unittest discover -s tests -v
```

### Repository validation

Use canonical Rust validation for the product command and keep explicit legacy-Python parity checks as named regression evidence.

### Rust

```text
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo check --manifest-path rust/Cargo.toml --workspace
cargo test --manifest-path rust/Cargo.toml --workspace
```

### Conformance

- 20/20 legacy validation fixtures through canonical Rust;
- legacy Python comparator result recorded;
- 8/8 WI050 admission corpus remains Python-owned/green;
- linked-worktree/reference focused parity remains green.

### Desktop

From `rust/apps/repopact-desktop`:

```text
npm ci --ignore-scripts
npm run types:check
npm run typecheck
npm test -- --run
npm run build
```

Run desktop adapter Rust tests as part of workspace tests.

### Packaging

- release builder on clean committed source;
- fresh-wheel install smoke;
- engine missing/mismatch tests;
- sdist/source-build proof;
- artifact hashes.

### Frozen surface

```text
python -m repopact.check_frozen_surface --base <WI056-activation-base>
```

No `.github/workflows/**` changes.

## Authority assertions to test explicitly

Add tests proving:

1. public `repopact validate` invokes the Rust engine and does not call legacy Python validation;
2. public `dashboard` invokes Rust authority;
3. migrated work commands invoke Rust authority;
4. missing engine fails closed/no fallback;
5. product-version mismatch fails closed/no fallback;
6. protocol-major mismatch fails closed/no fallback;
7. malformed engine output fails closed/no fallback;
8. retained Python commands remain routed to their documented implementation;
9. WI050 commands do not go through `repopact-engine`;
10. engine protocol rejects admission/guard/raw-path/raw-plan operations;
11. installed wheel resolves its own environment's engine;
12. legacy Python reference code is reachable only through explicit tests/internal imports, not migrated public dispatch.

## No-go list

Do not:

- introduce PyO3 as the canonical compatibility seam;
- parse existing Rust CLI human output;
- silently trust arbitrary PATH engine binaries;
- silently fall back to Python on engine failure;
- expose raw filesystem writes or executable caller-supplied mutation plans;
- port WI050 admission/guard/approval/enforcement;
- rewrite schemas to ease protocol design;
- add a daemon/network service;
- add persistent repository-local protocol state;
- publish a release without explicit direction;
- modify frozen workflows;
- claim Linux/macOS execution that did not occur.

## Stop/escalate conditions

Stop and report before proceeding if:

- Maturin `bin` mixed packaging cannot preserve the Python package + console entrypoint + engine executable cleanly;
- release reproducibility would have to be weakened;
- a migrated command depends on meaningful Python semantics not present in Rust proof;
- protocol security would require persistent secrets/state or raw plan round-tripping;
- WI050 behavior would be bypassed/weakened;
- canonical record format changes become necessary;
- current release/version governance contradicts the planned artifact identity;
- implementation would require frozen-surface changes without approval.

## Completion report

Return a detailed final report containing:

- final `main` SHA and commits;
- exact canonical Rust surface after cutover;
- exact retained Python-authority surface;
- protocol crate/binary paths;
- protocol version/envelope/operations;
- engine product-version derivation;
- Python engine discovery rules;
- migrated CLI commands and compatibility behavior;
- no-fallback evidence;
- conformance authority change and results;
- old Python reference-code status;
- Maturin/package configuration;
- wheel filename/tags/contents;
- fresh wheel install results;
- source-build results;
- release-builder reproducibility/hashes;
- Windows runtime/package evidence;
- Linux/macOS status;
- full Python/Rust/frontend test counts;
- WI050 8/8 status;
- linked-worktree/reference status;
- frozen-surface result;
- rollback proof;
- exact known deferred/non-parity surfaces;
- whether WI052 is ready for umbrella closeout.

Do not close WI056 until the machine record, evidence, dashboard, and documentation are reconciled and the worktree is clean/pushed.
