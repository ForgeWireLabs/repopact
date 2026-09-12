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

`governance/verification.json` now participates in the immutable Rust repository snapshot/read set. Changes to the contract therefore change the snapshot token used by the canonical engine path.

`RepoPactCore::validate_snapshot` validates the optional verification record from that same snapshot against the embedded `verification-profile.schema.json` and adds semantic checks for:

- `default_profile` referring to a declared profile;
- unique step ids within a profile;
- repository-contained relative working directories, including existing symlink/reparse containment through the repository resolver;
- the closed provider-neutral placeholder set.

The installed Rust CLI and compatibility-engine `validate` operation traverse `RepoPactCore`, so these WI046 semantics are part of the normal canonical command path.

## Invariant and specification reconciliation

INV-7's machine-enforcement pointer no longer names GitHub Actions. It now identifies canonical dashboard validation plus the repository-defined local verification contract for SPEC freshness.

The generated SPEC catalog includes the verification contract and rule 14 defines its semantic boundary. The specification explicitly distinguishes local verification evidence from remote admission enforcement.

## Known integration gap

The desktop API still constructs its validation view by calling the lower-level `repopact-validation` crate directly instead of `RepoPactCore::validate_snapshot`. That pre-existing bypass means the installed CLI/engine path sees WI046 verification-contract diagnostics while the current Workbench validation view does not yet include this additional core-layer diagnostic set.

This must be reconciled before WI046 can claim one semantic validation surface across CLI and Workbench. The correction should route the desktop view through `RepoPactCore` or move the new verification-contract validator to a lower shared semantic layer without duplicating rules.

The mutation crate also performs its internal post-apply repository validation through `repopact-validation`. That existing lower-level boundary must be reviewed in the same reconciliation so WI046 does not create divergent validity notions.

## Evidence still required

No test or release-readiness result is claimed by this record. Closeout still requires execution from an actual checkout, including at minimum:

1. Python WI046 verification/release tests;
2. Rust workspace tests including the new snapshot-backed contract tests;
3. canonical RepoPact validation;
4. conformance;
5. SPEC/dashboard freshness checks;
6. the `ci` profile with repository-native WI046 evidence recording;
7. the `release` profile and local release preparation without publication;
8. negative hosted-switch/publication/enforcement cases required by the work item;
9. truthful platform/capability evidence for any release claim whose coverage exceeds one host.

Until those runs exist and are linked, WI046 remains active.
