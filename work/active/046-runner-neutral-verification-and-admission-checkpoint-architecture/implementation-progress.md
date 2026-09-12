# WI046 implementation progress — 2026-09-12

This record captures implementation state only. It is not closeout evidence and it does not mark any acceptance criterion satisfied without an executed evidence run.

## Landed architecture

- `governance/verification.json` is the repository-defined provider-neutral verification contract.
- Local execution is primary. Hosted CI and hosted CD remain independently opt-in and disabled by default under Decision 0043.
- Verification profiles are ordered argv/builtin steps; command execution uses `shell=False`.
- Profiles declare `coverage: host | complete`.
  - `host` proves required checks applicable to the executing host and reports non-applicable declared steps honestly.
  - `complete` remains incomplete when any required declared platform/capability did not execute.
- Aggregate output records required-step counts, declared platforms, capability states, and whether all declared required steps actually executed.
- `repopact verify ... --evidence-work-item NNN` can persist an actual invocation as immutable concrete evidence. The record captures profile, executor, platform, coverage, step states, executed exit codes, and best-effort Git commit/tree/dirty identity. An existing evidence id is never overwritten.
- Local release verification/build/inspect/publication remain separate. Publication requires explicit intent and external credentials.
- GitHub workflows are thin optional adapters over the local entry points rather than an independent semantic pipeline.
- Brownfield adoption records discovered hosted workflows as candidate adapters rather than inventing enforcement closure from workflow-file presence.

## Canonical Rust integration

`governance/verification.json` participates in the immutable Rust repository snapshot/read set. Changes to the contract therefore change the snapshot token used by every snapshot-backed product surface.

The WI046 verification-contract rules now execute inside the shared `repopact-validation` semantic boundary rather than being layered only in `RepoPactCore`. The shared validator checks the optional record against the embedded `verification-profile.schema.json` and adds semantic checks for:

- `default_profile` referring to a declared profile;
- unique step ids within a profile;
- repository-contained relative working directories, including existing symlink/reparse containment through the repository resolver;
- the closed provider-neutral placeholder set.

This closes the earlier validity split. The installed CLI/engine path, `RepoPactCore`, the desktop/Workbench validation projection, and mutation post-apply validation all consume the same `repopact-validation::validate` / `validate_snapshot` result. No caller maintains a second WI046 diagnostic layer.

The verification-contract implementation is isolated in `rust/crates/repopact-validation/src/verification.rs` and is invoked as a first-class always-executed shared validation phase. Focused Rust tests cover valid, malformed, missing-default, duplicate-step, cwd-escape, and unknown-placeholder behavior without requiring adopter-fleet state.

Two consumer-boundary regression tests also pin the parity architecture:

- `rust/crates/repopact-desktop-api/tests/verification_validation_parity.rs` requires the Workbench overview to expose a shared WI046 diagnostic from an invalid contract.
- `rust/crates/repopact-mutation/tests/verification_post_validation.rs` requires mutation post-apply validation to reject that same invalid contract.

These tests are staged but have not yet been executed in this record.

## Invariant and specification reconciliation

INV-7's machine-enforcement pointer no longer names GitHub Actions. It now identifies canonical dashboard validation plus the repository-defined local verification contract for SPEC freshness.

The generated SPEC catalog includes the verification contract and rule 14 defines its semantic boundary. The specification explicitly distinguishes local verification evidence from remote admission enforcement.

The conformance inventory includes `SPEC-4-verification-contract` and a negative fixture. The independent Python comparator has a WI046 compatibility validator so fixture isolation remains meaningful after the Rust cutover without restoring Python as product authority.

## Integration status

The previously recorded Workbench/mutation integration gap is structurally closed. There is now one Rust validity notion for WI046 across the known product callers, with direct consumer-boundary regression coverage staged to guard that architecture.

This is an implementation conclusion, not an execution claim. It still needs checkout-level proof that the workspace compiles, the new tests run, the Workbench path reports the expected diagnostic, mutation post-validation rejects the same invalid contract, and the conformance fixture passes through the canonical engine.

## Evidence still required

No test or release-readiness result is claimed by this record. Closeout still requires execution from an actual checkout, including at minimum:

1. Python WI046 verification/release tests;
2. Rust workspace tests including the shared verification-contract and cross-caller parity tests;
3. canonical RepoPact validation;
4. conformance;
5. SPEC/dashboard freshness checks;
6. executed parity proof that CLI/engine, Workbench, and mutation post-validation observe the same invalid verification contract;
7. the `ci` profile with repository-native WI046 evidence recording;
8. the `release` profile and local release preparation without publication;
9. negative hosted-switch/publication/enforcement cases required by the work item;
10. truthful platform/capability evidence for any release claim whose coverage exceeds one host.

Until those runs exist and are linked, WI046 remains active.
