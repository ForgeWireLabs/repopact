# Work Item 056 — Python Compatibility and Canonical Rust-Core Cutover

**Status:** Active

**Owner:** work-coordinator

**Affected scopes:** work, tooling, governance, docs, evidence

**Depends on:** WI053, WI054, WI055

## Purpose

WI056 performs the deliberate cutover from the migration state in which Python is the reference implementation and Rust is an alternate implementation, to a state in which the already-proven semantic surfaces have one canonical Rust implementation and Python is a compatibility client for those surfaces.

This is a surface-by-surface authority transition, not a claim that every historical RepoPact command has been rewritten in Rust.

Decision 0042 is binding for this work item.

## Proven prerequisite state

WI053 proved:

- Rust repository/schema/validation behavior against the published conformance corpus;
- linked-worktree, ignored-path, and record-relative-reference parity;
- structured diagnostics and explicit unsupported behavior.

WI054 proved:

- shared repository session/snapshot/index semantics;
- relationship graph and deterministic analysis;
- typed work-item create/edit/transition mutation;
- content-addressed stale-plan detection;
- recoverable apply/rollback;
- dashboard projection parity.

WI055 proved:

- those APIs support a real non-Python client without duplicated semantics;
- the desktop can consume typed Rust view models, graph/analysis, validation, and mutation;
- Rust-owned session/plan/watcher state works under an actual application boundary.

Those milestones are sufficient to begin canonical authority cutover for their proven surfaces.

## Canonical surface after WI056

The intended canonical Rust semantic authority after WI056 is:

- repository discovery/modeling used by validation and supported native clients;
- canonical-schema-backed validation and structured diagnostics;
- canonical dashboard rendering/writing;
- relationship graph;
- deterministic repository analysis;
- typed work-item create;
- typed work-item edit for proven fields;
- typed lifecycle transition;
- content-addressed mutation planning/apply/recovery semantics.

Python must no longer be presented as a co-canonical implementation for those behaviors.

## Explicitly retained Python authority / deferred surfaces

Unless separately proven during WI056, these remain Python-authoritative or otherwise outside the Rust cutover:

- repository `init` bootstrap mechanics;
- brownfield `adopt`;
- `import-plan`;
- `doctor` and `doctor --fix`;
- `takeover`;
- general SPEC generated-block mutation;
- generic decision/policy creation or mutation;
- generic evidence mutation;
- `check-frozen` diff-time Git inspection;
- adopter fleet verification;
- release closeout/build orchestration except where packaging must be updated for the engine artifact;
- WI050 admission/approval/guard/enforcement/IPC/platform/protected-service authority.

Where a retained Python operation needs a final repository-validity decision, it should call the canonical Rust validation path rather than silently treating the old Python validator as final product authority.

## Compatibility architecture

Python compatibility is a local process protocol:

```text
user / automation
      |
      v
`repopact` Python console entrypoint
      |
      v
Python compatibility client
      |
      | one request / process
      | versioned JSON on stdin/stdout
      v
`repopact-engine` Rust executable
      |
      v
repopact-core / repository / validation / graph / analysis / mutation
```

Tauri and Rust-native clients continue to call the same Rust crates directly. The process boundary is for compatibility and language-neutral integration; it is not a second semantic implementation.

## Protocol requirements

The machine protocol must be independent of human CLI formatting.

Every request/response must carry enough identity to reject ambiguity:

- protocol name;
- protocol major version;
- request id;
- engine/product version;
- operation;
- repository root where applicable;
- structured parameters/result;
- structured diagnostics/errors.

Required first-class behavior:

- handshake/capabilities;
- fail-closed unsupported protocol major;
- fail-closed packaged product-version mismatch;
- deterministic JSON output;
- no presentation-string scraping;
- no network, socket, daemon, or repository-local runtime database;
- process-bounded timeout/cancellation behavior;
- semantic failures distinguished from transport/protocol failures.

The engine must accept typed semantic operations rather than raw filesystem operations.

## Mutation protocol rule

The stdio boundary must not undo the safety gained in WI054/WI055.

Do not expose:

- arbitrary JSON Patch;
- arbitrary paths/content writes;
- caller-authored `MutationPlan.file_operations`;
- a serialized executable mutation plan that is returned by an untrusted caller and blindly executed.

For compatibility commands that mutate state, the engine receives high-level typed intent and performs plan/apply internally against its own snapshot.

If a read-only preview operation is added, the preview is informational. A later mutation request must be independently planned/revalidated by the engine unless an integrity-preserving cross-process plan mechanism is separately governed.

## Python command migration

At minimum migrate the public command paths already backed by proven Rust behavior:

- `repopact validate`;
- `repopact dashboard`;
- `repopact new work-item ...`;
- `repopact work propose ...`;
- `repopact work amend-proposal ...` where the existing proposed-only constraint is preserved in canonical Rust behavior.

Do not silently change command syntax/exit semantics unless existing behavior is demonstrably inconsistent with the canonical model and the correction is recorded.

The old Python implementations may remain as test/reference code, but these migrated public paths must not choose the Python engine as a hidden fallback when the Rust engine is missing or incompatible. Missing/incompatible engine is an explicit compatibility failure.

## Conformance authority flip

The existing conformance corpus is valuable and should remain language-neutral.

WI056 should reorganize execution so:

1. the Rust engine is the canonical implementation whose accept/reject behavior is checked against the published fixture expectations;
2. the historical Python validator may still be run as an independent regression comparator;
3. failure of the legacy Python comparator does not silently make Python canonical again;
4. any intentional divergence requires a governed specification/conformance change rather than implementation preference.

Do not delete independent Python parity tests merely to make the cutover look cleaner.

## Python packaging

Decision 0042 selects a binary-wheel process model rather than PyO3.

The preferred packaging route is a Maturin mixed Rust/Python project using `bin` bindings so the Python distribution contains:

- the `repopact` Python compatibility package;
- the existing `repopact` console script;
- a platform-specific `repopact-engine` executable installed in the same Python environment.

The wheel should remain independent of a particular CPython extension ABI; it will nevertheless be platform-specific because it carries a native executable.

Expected release families include, as actually supported/proven:

- Windows x64;
- Linux x86_64 with an appropriate manylinux compatibility target;
- macOS x86_64 and/or arm64 as supported by the release plan.

Do not claim an unexecuted platform as validated.

## Engine discovery

The Python client must prefer the engine installed in the same environment as the Python package/console entrypoint.

An explicit developer/operator override may be supported for tests/source checkout use, but it must:

- be opt-in;
- identify the exact executable path;
- run the handshake/version checks;
- never silently downgrade to human CLI text parsing.

Do not use an arbitrary PATH result as an unverified semantic authority.

## Version coupling

The engine and Python package belong to one RepoPact release line.

At runtime:

- protocol major compatibility is mandatory;
- packaged product/engine version must match the Python package expectation unless an explicit development mode says otherwise;
- mismatch diagnostics must show both sides;
- no hidden fallback to legacy Python semantics is permitted.

The implementation should avoid coupling this identity to every internal Cargo crate version if the repository's canonical `VERSION` / `RELEASE_LABEL` records already provide the correct product identity.

## Release-build migration

The current release verifier assumes exactly one `py3-none-any` wheel. That assumption must be deliberately replaced for the new distribution model.

The revised release build must verify, at minimum:

- expected Python package import root;
- absence of the old flat-module collision problem;
- canonical schemas/templates required by retained Python commands;
- presence and exact name of the Rust engine executable/script in the built wheel;
- platform/Python/ABI tags appropriate to a binary-application wheel;
- package/engine version coupling;
- artifact SHA-256;
- repeat-build reproducibility at the level actually achievable and claimed;
- clean committed source requirement;
- source distribution completeness for Rust path dependencies.

Frozen `.github/workflows/**` remain out of scope without separate approval.

## Fresh-install proof

Closeout must include a clean-environment proof, not only execution from the source checkout.

On the available host:

1. build the release wheel/sdist through the new packaging path;
2. create a fresh virtual environment with no checkout on `PYTHONPATH`;
3. install the built wheel without requiring a Rust compiler/network fetch during wheel installation;
4. prove `repopact` resolves its packaged engine;
5. prove handshake/version identity;
6. run migrated commands against scratch repositories;
7. run at least validation plus create/dashboard compatibility through the installed Python CLI;
8. prove missing/incompatible engine failures are explicit and do not fall back to Python semantics.

Source-build proof should separately demonstrate the documented Rust/Maturin requirement.

## Rollback

The cutover changes executable/package authority, not canonical repository data formats.

Rollback therefore means reinstalling a previously compatible RepoPact package/runtime or switching the development checkout to the previous revision. It must not require:

- rewriting Git history;
- downgrading work/evidence records;
- rewriting completed work;
- converting repositories back to a Python-specific format.

Any repository-format change discovered to be necessary is outside this implicit rollback model and requires separate governance.

## WI050 boundary

WI050 remains quarantined from this migration.

Admission/approval/guard/enforcement behavior continues through the existing Python/protected-provider path. The Rust engine must not claim those operations merely to make the implementation language look uniform.

The Python CLI may therefore intentionally contain both:

- compatibility-client routes to canonical Rust semantics;
- explicitly Python-owned WI050/security routes.

That is not dual authority because the surfaces are disjoint and documented.

## Public-claim rule

After WI056, permitted language is equivalent to:

> RepoPact uses a canonical Rust governance engine for its proven repository validation, analysis, graph, dashboard, and typed work-item mutation surfaces. The Python package provides the compatibility CLI and retains selected legacy/security operations that have not yet migrated.

Do not claim:

- every RepoPact feature is Rust;
- WI050 is Rust-enforced;
- generic decision/evidence mutation exists;
- Linux/macOS packages were executed if they were not;
- a release has been published if only local artifacts were built.

## Closeout standard

WI056 closes only when:

- the protocol/engine exists and is tested;
- migrated Python commands consume Rust authority;
- no migrated command silently falls back to Python semantics;
- conformance authority is explicitly Rust-first;
- packaging includes the correct Rust engine artifact;
- a fresh-wheel compatibility smoke passes on the available host;
- release-build verification understands platform wheels;
- baseline Python/Rust/desktop/conformance/WI050 tests remain green;
- rollback and version mismatch behavior are exercised;
- known Python-only/deferred surfaces are listed exactly;
- public documentation matches the actual cutover.

WI052 may be considered for umbrella closeout only after WI056 closes and its original acceptance criteria are reconciled against the completed WI053-WI056 evidence.
