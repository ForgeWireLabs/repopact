# 037 — Reconcile the 2.3 adopter fleet and Proving Ground package boundary

> **Status**: Proposed — recorded by WI-033 reconciliation; not yet authorized
> for cross-repository implementation.
> **Owners**: tooling-owner (lead); governance-owner and evidence-owner affected.
> **Depends on**: `029`, `036`.

## Intent

Restore current-release coherence after live WI-033 verification found that
ForgeLink, ForgeWire, and RepoPact Proving Ground still declare RepoPact 2.2.0
while the current public release is 2.3.0. SkillForge and the checksum-backed
Moto vendored consumer already pass the fleet contract.

The same verification found that Proving Ground's benchmark documents link to a
nonexistent local `research/` tree and its S5 harness imports removed flat module
names (`init_repo`, `validate_repo`) instead of the supported `repopact.*`
package boundary. PactBench's 24-task deterministic selftest still passes.

This item is a new 2.3 reconciliation, not a repetition or rewrite of the
completed 2.2.0 rollout.

**Cross-reference (2026-08-21, work item 041, narrow — no criterion here is
satisfied and no status changes as a result).** ForgeWire independently
completed its own migration to the current public release
(`repopact==3.0.0`, ForgeWire work item 238) outside of, and not counted
toward, this item's AC-1. That migration's field evidence exposed two
separate upstream product-correctness limitations, now findings F-015
(updated) and F-017 (new) in `research/findings.md`, with proposed
implementation work items `042` and `043`. Those are product defects in
RepoPact itself, not adopter-fleet reconciliation, and this item's scope —
migrating the remaining stale adopters and repairing Proving Ground — is
unchanged and not taken over.

## Decisions

Package publication and ecosystem rollout remain separate phases (WI-029).
GitHub Actions remains billing-locked (WI-032), so local repository-native gates
and immutable remote-head verification are required without claiming CI
restoration.

## Scope

- Update only stale adopters and preserve unrelated downstream changes.
- Repair Proving Ground's supported package imports and cross-repository links.
- Re-run each adopter's native gates and the upstream fleet verifier.
- Preserve Moto's stronger overlay/checksum parity proof.

## Acceptance criteria

- [ ] **AC-1** — all five public default branches declare the intended release.
- [ ] **AC-2** — Proving Ground's links and package boundary are repaired and its
  benchmark checks pass.
- [ ] **AC-3** — deterministic fleet verification passes, including Moto parity.
- [ ] **AC-4** — per-repository evidence is complete and CI restoration is not
  overstated.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
