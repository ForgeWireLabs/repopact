# Codex Execution Directive — WI053

**Coding agent:** Codex  
**Authoritative work item:** WI053  
**Architecture umbrella:** WI052  
**Delivery mode:** direct to `main`, preserving concurrent work; no force-push/reset/history rewriting.

## Mission

Implement WI053's bounded Rust foundation so RepoPact has a reusable Rust repository/schema/validation core that is exercised by the existing Python alternate-implementation conformance runner.

Do **not** start the Tauri UI, governed mutation authority, Python cutover, or WI050 security-substrate migration.

## Required reading before edits

1. root `AGENTS.md`;
2. applicable tooling contracts;
3. `work/active/052-rust-core-and-tauri-2-user-workbench-foundation/README.md`;
4. `work/active/052-rust-core-and-tauri-2-user-workbench-foundation/architecture-inventory.md`;
5. this WI053 `README.md` and `work-item.json`;
6. `CONFORMANCE.md`, `conformance/manifest.json`, and `repopact/run_conformance.py`;
7. `repopact/repo_model.py` and `repopact/validate_repo.py`;
8. decisions governing record-relative references and linked-worktree semantics;
9. WI050 boundaries before touching anything related to admission/enforcement.

## Start-state rules

- Fetch latest `origin/main` and verify the WI052/WI053 activation commit is present.
- Work from a clean tree.
- Inspect concurrent changes before every push; preserve them.
- Do not force-push, hard-reset shared history, or rewrite completed work.
- Run the existing Python baseline before introducing Rust so failures are distinguishable from regressions.
- Run frozen-surface checks before considering any protected path. `.github/workflows/**` is frozen; do not modify it in this execution unless a separate human approval is already recorded.

## Implementation order

### Phase 1 — workspace and contracts

Create the top-level `rust/` Cargo workspace and the smallest clean crate graph supporting:

- `repopact-types`;
- `repopact-schema`;
- `repopact-repository`;
- `repopact-validation`;
- `repopact-core`;
- a minimal validation CLI application.

Keep dependencies minimal and justified. Avoid Tauri, PyO3, async/runtime frameworks, databases, networking stacks, or provider SDKs unless a WI053 acceptance criterion actually requires them.

### Phase 2 — types and canonical schema boundary

Implement typed records required by validation. Keep the JSON schemas canonical.

If schemas are embedded into the binary/package, make that embedding mechanical from the repository schema files and test against the source so embedded copies cannot drift silently.

Normalize schema diagnostics into RepoPact-owned diagnostic structures.

### Phase 3 — repository model

Port semantics, not Python syntax.

At minimum reproduce:

- lifecycle/status directory model;
- ignored-path behavior;
- work-item/evidence/contract discovery;
- deterministic traversal/order;
- Git linked-worktree detection/common-dir handling where applicable;
- normalized path handling on Windows and Unix-like systems;
- record-relative reference semantics already established by RepoPact decisions.

Do not implement a broad filesystem crawler and then special-case conformance fixtures. The repository model must make sense on an actual adopted checkout.

### Phase 4 — structured diagnostics and validator

Implement stable diagnostic identity internally. Preserve expected human message substrings for the legacy conformance corpus.

Port validator behavior in small coherent rule groups. For each group:

1. identify the Python reference rule;
2. identify its current conformance case(s) or add a focused parity test when the corpus does not cover the relevant repository primitive;
3. implement Rust behavior;
4. compare Python/Rust results;
5. only then continue.

Do not weaken the Python reference or fixture expectations to make Rust pass.

### Phase 5 — canonical dashboard projection required by validation

Implement only the projection/read comparison needed to reproduce stale/missing-dashboard validation.

Do not add a general Rust write API under WI053. WI054 owns governed mutation/generation authority.

### Phase 6 — alternate implementation CLI

Provide a stable validation command callable by the existing runner, conceptually:

```text
<rust-validator> validate --root <repo>
```

Expected behavior:

- valid -> exit 0;
- invalid -> non-zero and deterministic diagnostics;
- unsupported semantic surface -> non-zero and explicit unsupported diagnostic;
- validation never writes the repository.

### Phase 7 — conformance proof

Run the **existing** alternate implementation interface, not a custom substitute. The important form is:

```text
python -m repopact.run_conformance --command "<rust-validator> validate --root {repo}"
```

The current runner materializes fixtures, validates them using Python as the reference, invokes the supplied command, and requires the alternate implementation to agree on accept/reject and the expected reject message.

The runner also reports the separate WI050 admission corpus. Keep that corpus green, but do not port it.

### Phase 8 — focused architecture parity

Add tests beyond the currently published validator corpus for the repository primitives most dangerous to reimplement incorrectly:

- linked Git worktrees;
- record-relative references;
- path normalization/separator behavior;
- ignored directories;
- deterministic discovery;
- no-write validation.

## Do not do these things

- Do not add Tauri.
- Do not build a React/Vue/Svelte frontend.
- Do not implement mutation planning/apply yet.
- Do not switch Python to call Rust yet.
- Do not add PyO3/maturin native-wheel packaging yet.
- Do not migrate WI050 admission/enforcement/guard/IPC/platform-service code.
- Do not create a Rust-specific RepoPact schema authority.
- Do not bypass the published conformance runner.
- Do not change expected fixtures/messages merely to make the Rust implementation green unless you first prove and record a defect in the canonical/reference behavior.
- Do not modify frozen CI paths without separate operator approval.
- Do not claim Rust is canonical at closeout.

## Validation before each meaningful push

At minimum run the repository-required Python checks plus relevant Rust checks. Use the repository's exact supported commands where available. Expected baseline includes:

```text
python -m pip install -e ".[dev]"
repopact validate
python -m unittest discover -s tests -v
```

Also run:

```text
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo check --manifest-path rust/Cargo.toml --workspace
cargo test --manifest-path rust/Cargo.toml --workspace
```

Then run the published conformance runner against the built Rust validator command.

If an environment cannot run a required platform-specific check, record that fact; do not convert an unrun check into a pass.

## Commit strategy

Prefer small coherent commits that keep `main` valid, for example:

1. Rust workspace + types/schema boundary;
2. repository model + focused topology tests;
3. validator/diagnostics + dashboard projection;
4. CLI + alternate conformance integration;
5. closeout/evidence/reconciliation only after all required checks are green.

This is guidance, not a requirement to preserve a bad split if implementation shows a safer atomic boundary.

## Stop/escalate conditions

Do not patch around these. Record the conflict and stop that slice if:

- a required Rust behavior contradicts a binding RepoPact decision/invariant;
- the Python reference and published conformance corpus disagree materially;
- satisfying WI053 would require weakening a WI050 security guarantee;
- a necessary change enters a frozen surface without operator approval;
- a dependency would materially dictate future Tauri/Python architecture beyond WI053;
- a canonical on-disk contract appears ambiguous enough that two conforming implementations could disagree.

## Completion report required

Report:

- final `main` SHA;
- commits created;
- exact Rust workspace/crates/CLI delivered;
- Python files changed, if any, and why;
- legacy conformance result count;
- WI050 admission corpus result count;
- Rust test counts;
- Python test counts;
- linked-worktree/reference parity results;
- frozen-surface result;
- exact unsupported/non-parity surfaces remaining;
- whether WI053 is actually ready for acceptance-criterion evidence/closeout;
- any architecture finding that should amend WI052, WI054, WI055, or WI056.
