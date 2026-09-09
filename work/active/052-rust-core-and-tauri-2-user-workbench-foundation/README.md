# Work Item 052 — Rust Core and Tauri 2 User Workbench Foundation

**Status:** Active

**Owner:** work-coordinator

**Affected scopes:** work, tooling, governance, docs, evidence

## Activation

WI052 was activated on 2026-09-09 after an architecture-first inventory of the current Python implementation, conformance corpus, repository semantics, and the still-active WI050 enforcement boundary.

This item is the program/architecture umbrella for RepoPact's transition from a CLI-centric Python reference implementation toward a language-neutral governance platform with a reusable Rust core and a Tauri 2 desktop workbench.

The original detailed proposal is preserved verbatim in [`proposal.md`](proposal.md). The implementation sequencing discovered at activation is recorded in [`architecture-inventory.md`](architecture-inventory.md).

## Current authority boundary

During this phase:

- RepoPact's specification, schemas, invariants, decisions, and conformance corpus remain language-neutral authority.
- The existing Python implementation remains the reference executable semantics.
- Rust is an alternate implementation until the relevant conformance/parity gates are executable and green.
- Tauri is a client of the Rust domain/core APIs, not an independent implementation of governance semantics.
- Unsupported Rust operations must fail explicitly rather than guess or mutate governed state.
- WI050 admission/enforcement remains on its existing security-critical path while WI050 is active; this program may reserve later Rust boundaries but must not silently port or weaken that substrate.

## Implementation decomposition

The architecture inventory split implementation into bounded subordinate work rather than allowing WI052 to become a whole-product rewrite:

- **WI053 — Rust Workspace, Repository Model, Schema and Validator Conformance** — active first implementation slice and current coding owner.
- **WI054 — Rust Graph, Analysis and Transactional Mutation Core** — proposed; depends on WI053.
- **WI055 — Tauri 2 Desktop Governance Workbench** — proposed; depends on WI053 and WI054.
- **WI056 — Python Compatibility and Canonical Rust-Core Cutover** — proposed; depends on WI053, WI054, and WI055.

WI052 remains active as the coordinating architecture item. Concrete implementation must be attributed to the subordinate item that owns that slice.

## Immediate milestone

The first milestone is deliberately narrower than a GUI:

> A Rust implementation can load the same supported RepoPact repository state as Python and reach the same accept/reject judgment, with the expected primary diagnostic, through the published alternate-implementation conformance interface.

Only after that read/validation foundation is proven may RepoPact move governed mutation authority into Rust.

## Non-goals for the current slice

- no Tauri UI implementation under WI053;
- no PyO3/native-wheel cutover;
- no claim that Rust is canonical merely because a crate exists;
- no direct frontend writes to governed files;
- no port of WI050 admission, guard, IPC, platform backend, or enforcement behavior;
- no provider-specific AI dependency in the authority kernel;
- no rewrite of completed RepoPact history.

## Closeout model

WI052 closes only when its subordinate architecture boundaries are implemented or explicitly deferred with evidence, the implemented Rust surface is stated precisely, known non-parity surfaces are recorded, and no documentation overstates the authority or completeness of the Rust migration.
