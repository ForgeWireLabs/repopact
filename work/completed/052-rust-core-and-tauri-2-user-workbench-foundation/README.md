# Work Item 052 — Rust Core and Tauri 2 User Workbench Foundation

**Status:** Completed

**Owner:** work-coordinator

**Affected scopes:** work, tooling, governance, docs, evidence

## Activation

WI052 was activated on 2026-09-09 after an architecture-first inventory of the current Python implementation, conformance corpus, repository semantics, and the still-active WI050 enforcement boundary.

This item is the program/architecture umbrella for RepoPact's transition from a CLI-centric Python reference implementation toward a language-neutral governance platform with a reusable Rust core and a Tauri 2 desktop workbench.

The original detailed proposal is preserved verbatim in [`proposal.md`](proposal.md). The implementation sequencing discovered at activation is recorded in [`architecture-inventory.md`](architecture-inventory.md).

## Current authority boundary

After WI053-WI057 and the WI056 cutover:

- RepoPact's specification, schemas, invariants, decisions, and conformance corpus remain language-neutral authority.
- Rust is canonical for the proven repository/session, validation, dashboard, graph, analysis, and typed work-item mutation surfaces.
- Python is the compatibility client for those surfaces and remains authoritative for explicitly retained workflows.
- Tauri is a client of the Rust domain/core APIs, not an independent implementation of governance semantics.
- Unsupported Rust operations fail explicitly rather than guessing or mutating governed state.
- WI050 admission/enforcement remains on its existing security-critical Python/protected-provider path and was not ported or weakened.

## Implementation decomposition

The architecture inventory split implementation into bounded subordinate work rather than allowing WI052 to become a whole-product rewrite:

- **WI053 — Rust Workspace, Repository Model, Schema and Validator Conformance** — completed.
- **WI054 — Rust Graph, Analysis and Transactional Mutation Core** — completed; depends on WI053.
- **WI055 — Tauri 2 Desktop Governance Workbench** — completed; depends on WI053 and WI054.
- **WI056 — Python Compatibility and Canonical Rust-Core Cutover** — completed; depends on WI053, WI054, WI055, and the completed WI057 substrate.

Concrete implementation remains attributed to the subordinate item that owns each slice; the completed WI056 evidence reconciles the umbrella boundary.

## Delivered milestone

The initial alternate-implementation milestone grew into a staged, evidence-gated delivery:

> The canonical Rust engine loads the supported RepoPact state, reaches the expected accept/reject judgment through the published conformance interface, serves the desktop workbench, and is the authority behind the migrated Python compatibility commands.

The 20/20 canonical conformance, 20/20 explicit Python comparator, 8/8 WI050 corpus, desktop checks, and package/install evidence are recorded in the WI056 closeout evidence.

## Retained boundaries after closeout

- no PyO3/native-extension compatibility seam;
- no generic decision/evidence/SPEC mutation in the Rust engine;
- no direct frontend writes to governed files;
- no port of WI050 admission, guard, IPC, platform backend, or enforcement behavior;
- no provider-specific AI dependency in the authority kernel;
- no claim of unexecuted Linux/macOS runtime/package validation;
- no rewrite of completed RepoPact history.

## Closeout model

WI052 closes only when its subordinate architecture boundaries are implemented or explicitly deferred with evidence, the implemented Rust surface is stated precisely, known non-parity surfaces are recorded, and no documentation overstates the authority or completeness of the Rust migration.

## Umbrella closeout

The original RUI-001 through RUI-022 criteria are reconciled as satisfied in
the umbrella work-item record. The implementation and regression details are
recorded by the completed WI053, WI054, WI055, and WI056 evidence records;
WI056's [`20260910-056-rust-engine-cutover`](../../../evidence/runs/20260910-056-rust-engine-cutover.json)
is the final authority/cutover and packaging closeout record.
