# RepoPact Rust foundation

This workspace contains RepoPact's canonical Rust semantic engine and the
language-neutral crates behind it. The Python package is a compatibility client
for migrated surfaces; canonical on-disk JSON schemas remain the record-format
authority.

## Layout

- `repopact-types` — serializable domain records and structured diagnostics.
- `repopact-schema` — target-repository schema loading with mechanically
  embedded canonical fallbacks and normalized schema diagnostics.
- `repopact-repository` — normalized paths, deterministic discovery, ignored
  directories, linked Git worktrees, repository identity, record-relative
  references, reusable sessions, immutable snapshots, indexed records, and
  content-addressed path facts.
- `repopact-validation` — supported semantic rules and dashboard comparison.
- `repopact-graph` — deterministic canonical relationship graph with
  source-backed nodes, edges, and queries.
- `repopact-analysis` — explainable fact/constraint/suggestion analyzers over
  one snapshot and graph.
- `repopact-mutation` — typed work-item create/edit/transition plan/apply
  boundary with read-set tokens, generated impacts, stale rejection, rollback,
  and post-validation.
- `repopact-core` — reusable non-Tauri façade for session, snapshot, validate,
  graph, analysis, plan, and apply operations.
- `apps/repopact-cli` — `repopact-cli validate --root <repository>` plus
  testing-oriented typed create/transition plan/apply commands.
- `repopact-protocol` — versioned request/response DTOs for the local engine
  boundary; it contains no repository semantics.
- `apps/repopact-engine` — one-request-per-process JSON engine for Python and
  other language-neutral clients. `repopact-engine` owns semantic validation,
  dashboard projection, graph/analysis reads, and typed work-item mutations.

Build and test with:

```powershell
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo check --manifest-path rust/Cargo.toml --workspace
cargo test --manifest-path rust/Cargo.toml --workspace
```

The WI054 mutation boundary is intentionally narrow: it accepts typed work-item
create, edit, and lifecycle-transition requests only. Plans contain the exact
read facts they consumed, including expected-absent paths, and must be applied
explicitly. Apply preserves preimages in ephemeral process memory, stages the
complete operation through the typed plan, regenerates the owned dashboard,
and reports success only after Rust post-validation.

The engine protocol does not implement Tauri, generic decision/evidence
mutation, persistent repository-local transaction state, or WI050
admission/enforcement semantics. Those surfaces fail explicitly or remain
Python-owned rather than being guessed or silently accepted. The Rust engine
requires the Rust toolchain for source builds; installing a platform wheel does
not require Cargo or a Rust compiler.
