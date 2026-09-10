---
id: 0042
title: Canonical Rust engine with versioned stdio compatibility protocol
status: accepted
date: 2026-09-10
supersedes: []
---

# 0042: Canonical Rust engine with versioned stdio compatibility protocol

## Context

WI053 proved Rust repository/schema/validation parity, WI054 proved graph, deterministic analysis, typed work-item mutation, content-addressed stale-plan protection, recovery, and dashboard parity, and WI055 proved that those Rust APIs can support a real desktop client without duplicating governance semantics in the frontend.

RepoPact is therefore ready to leave the migration state in which Python is the reference implementation and Rust is described only as an alternate for those proven surfaces. The cutover must preserve the existing `repopact` command experience, avoid creating permanent Python/Rust dual authority, remain language-neutral, and keep WI050 admission/enforcement on its separately proven security path.

Two compatibility families were evaluated. PyO3 can provide efficient native bindings and stable-ABI wheels, but it makes Python's extension ABI and wheel matrix part of the compatibility boundary. RepoPact's public Python contract is the console entrypoint rather than importable implementation modules, and the Rust engine is already useful to Tauri and non-Python consumers. A process protocol is therefore the narrower and more reusable authority seam.

## Decision

1. For surfaces proven by WI053/WI054, RepoPact's canonical semantic implementation is the reusable Rust core: repository discovery/modeling, canonical schema-backed validation, dashboard projection, relationship graph, deterministic analysis, and typed work-item create/edit/lifecycle mutation.
2. Python remains the compatibility CLI and retains authority only for explicitly unmigrated operations. Importable Python implementation modules are not a supported public API and may remain as regression/reference or legacy implementation code, but migrated user-facing paths must not silently execute them as an independent canonical engine.
3. Python compatibility uses a versioned local **stdio JSON protocol** to a Rust executable named conceptually `repopact-engine`. The protocol is machine-oriented and separate from human CLI text output.
4. The initial protocol is one-request-per-process. Requests and responses carry a protocol identifier/version, request identifier, engine/product version, operation, structured parameters/results, and structured diagnostics/errors. Protocol-major incompatibility is fail-closed. Product-version mismatch is rejected by default for the packaged Python client.
5. The engine accepts semantic operations, not arbitrary filesystem patches or caller-supplied executable `MutationPlan` objects. Mutating operations receive typed intent and perform plan/apply internally through the WI054 core. The process boundary does not weaken Decision 0040 or Decision 0041.
6. No daemon, socket, network listener, persistent transaction database, or repository-local runtime state is introduced for Python compatibility. Cancellation/timeouts are process-scoped.
7. The Python client resolves the engine from its own installed environment/package delivery first and verifies the handshake before use. A development/operator override may be explicit, but silent execution of an arbitrary PATH binary is not the trust model.
8. Distribution uses platform-specific Python wheels containing the Rust engine executable while retaining the Python `repopact` package and console entrypoint. Maturin `bin` packaging is the preferred implementation path because it supports Rust executables as wheel-installed scripts without requiring a CPython extension module. Source distributions remain supported and may require Rust/Maturin to build the engine.
9. The existing release verifier's `py3-none-any` assumption is intentionally obsolete after this cutover and must be replaced with structural checks appropriate to a Python-version-neutral but platform-specific wheel containing the exact engine binary/version. Reproducibility and artifact SHA-256 evidence remain required.
10. The canonical authority flip is surface-by-surface. At WI056 activation, the intended migrated Python command paths are `validate`, `dashboard`, work-item creation through `new work-item`, proposed-work creation, and typed proposal/work-item edits or transitions where a corresponding Rust operation is proven. Graph/analysis may be exposed through the machine protocol for compatibility/integration without requiring a new Python human command.
11. `init`, `adopt`, `import-plan`, `doctor`, `takeover`, `spec`, generic decision/policy creation, `check-frozen`, adopter fleet/release orchestration, and WI050 admission/approval/guard/enforcement remain Python-authoritative or otherwise unmigrated unless WI056 independently proves and records a narrower migration. When these Python operations require repository validation, they should consume the canonical Rust validation path rather than treating the legacy Python validator as final authority.
12. The conformance story flips with authority. The Rust engine becomes the canonical implementation under test for migrated semantics. The legacy Python validator may remain an independent regression oracle during the transition, but the product must not describe it as co-canonical.
13. WI050 admission, guard, enforcement, IPC, platform backend, protected-service, cryptographic approval, and operator-authority semantics do not migrate through this protocol. Their current Python/protected-provider path remains authoritative until separately proven.
14. Existing durable repository formats are not changed by the cutover. Rollback is a package/runtime rollback to a previously compatible RepoPact release, not a rewrite of governed history or a downgrade of repository records.
15. This decision does not itself publish a release or modify frozen CI/release workflows. Version/release labeling and public platform claims must continue to follow existing release governance and the evidence actually executed.

## Compatibility contract

The protocol must support an explicit capability/handshake operation and structured operations sufficient for the migrated surfaces. A conceptual envelope is:

```json
{
  "protocol": "repopact-engine",
  "protocol_version": 1,
  "request_id": "...",
  "operation": "validate",
  "root": "...",
  "params": {}
}
```

Responses mirror the protocol/request identity and carry `engine_version`, success/semantic outcome, result data, and structured diagnostics. Validation rejection is a semantic result, not a transport parse failure. Python maps semantic outcomes to the historical CLI exit behavior.

Human-readable Rust CLI output is not a protocol. The Python client must never scrape or parse presentation strings to discover semantic state.

## Alternatives considered

- **PyO3/native extension as the compatibility boundary.** Rejected for the primary cutover seam. Stable ABI support reduces Python-version builds but still ties distribution to CPython ABI families, including separate considerations for free-threaded Python, while adding no value to non-Python clients.
- **Keep Python and Rust independently authoritative.** Rejected because semantic drift would become permanent and the architecture would no longer have one recognizable RepoPact engine.
- **Have Python invoke the existing human `repopact-cli` and parse text.** Rejected because presentation strings are not a stable machine contract.
- **Expose raw serialized `MutationPlan` across stdio and send it back for apply.** Rejected because it recreates the authority-roundtrip problem already avoided by the desktop boundary.
- **Download an engine from the network on first use.** Rejected as the default because offline installation and supply-chain provenance are first-class requirements.

## Consequences

RepoPact gains a single Rust semantic kernel for the surfaces already proven, while the familiar Python entrypoint becomes a compatibility client rather than a second validator/mutator. Python wheels become platform-specific because they carry a Rust executable, but remain Python-version-neutral at the protocol boundary. The same engine protocol can be used by future language integrations without creating CPython-specific authority.

The cost is a more explicit release matrix and version-coupled binary packaging. That cost is preferable to maintaining two semantic engines or binding the canonical compatibility architecture to one language runtime.
