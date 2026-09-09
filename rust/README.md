# RepoPact Rust foundation

This workspace is the WI053 alternate implementation foundation. It is a
read-only validation surface; the Python implementation and the canonical
on-disk JSON schemas remain the authority during this work item.

## Layout

- `repopact-types` — serializable domain records and structured diagnostics.
- `repopact-schema` — target-repository schema loading with mechanically
  embedded canonical fallbacks and normalized schema diagnostics.
- `repopact-repository` — normalized paths, deterministic discovery, ignored
  directories, linked Git worktrees, repository identity, and record-relative
  references.
- `repopact-validation` — supported semantic rules and dashboard comparison.
- `repopact-core` — reusable non-Tauri façade.
- `apps/repopact-cli` — `repopact-cli validate --root <repository>`.

Build and test with:

```powershell
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo check --manifest-path rust/Cargo.toml --workspace
cargo test --manifest-path rust/Cargo.toml --workspace
```

WI053 does not implement Tauri, mutation planning/application, Python
cutover, or WI050 admission/enforcement semantics. Those surfaces fail
explicitly when encountered rather than being guessed or silently accepted.
