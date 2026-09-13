# 037 — Reconcile the adopter fleet to the RepoPact 3.x package boundary and repair Proving Ground

> **Status**: Complete — refreshed, activated, and reconciled 2026-09-13.
> **Owners**: tooling-owner (lead); governance-owner and evidence-owner affected.
> **Depends on**: `029`, `036`, `038`.

## Intent

Restore current-release coherence across the declared adopter fleet after the
RepoPact 3.x package boundary shipped. The migration moves normal adopters to
the exact `repopact==3.0.2` package, repairs Proving Ground's S5 harness to use
the supported CLI boundary, and removes Moto's obsolete vendored core while
retaining its local ROM-lab safety checks.

This is a 3.x boundary reconciliation, not a repetition or rewrite of the
completed 2.x rollouts.

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

## Scope refresh — 2026-09-13

WI037 was originally created for the 2.3 adopter-fleet reconciliation described
above. RepoPact has since shipped the breaking 3.x package boundary and the
current public release is `3.0.2`. ForgeWire independently moved to
`repopact==3.0.0`; that historical migration is not retroactively counted as
WI037 work.

Decision `0029` makes the old assumption that adopters carry or invoke vendored
flat RepoPact modules obsolete. The supported contract is one installed
`repopact` package, the `repopact ...` console surface, and
`python -m repopact.cli ...` when explicit interpreter binding is useful.

The underlying obligation remains: restore coherent supported RepoPact
consumption across the declared public adopter fleet, preserve every genuine
Moto-local safety rule as a local extension, and repair Proving Ground's S5
path without resurrecting 2.x shims or vendored core tooling.

## Decisions

Package publication and ecosystem rollout remain separate phases (WI-029).
GitHub Actions remains billing-locked (WI-032), so local repository-native gates
and immutable remote-head verification are required without claiming CI
restoration.

## Scope

- Update only stale adopters and preserve unrelated downstream changes.
- Repair Proving Ground's supported package imports and cross-repository links.
- Migrate Moto from vendored RepoPact core to the installed package plus a
  separately owned local validation extension without dropping safety rules.
- Update the canonical fleet manifest only after adopter default branches
  represent their new consumption contracts.
- Re-run each adopter's native gates and the upstream fleet verifier.
- Verify against immutable remote default-branch heads.

## Acceptance criteria

- [x] **AC-1** — Fleet consumption: all declared public adopters use the
  intended current supported RepoPact release/boundary on their public default
  branch; normal consumers use the exact supported PyPI package pin and legacy
  vendored-core consumption is removed rather than falsely declared current.
- [x] **AC-2** — Proving Ground: supported `repopact.*`/CLI interfaces,
  authoritative cross-repository research links, and passing governance,
  benchmark, fixture, PactBench, and S5 drift checks from its declared
  dependency environment.
- [x] **AC-3** — Deterministic fleet verification: the canonical adopter
  manifest and verifier pass against immutable public default-branch heads and
  accurately describe each repository's real consumption architecture; no
  contract points at upstream files that no longer exist.
- [x] **AC-4** — Evidence: every modified adopter has repository-native
  validation and dated evidence, with no overstatement of hosted CI or
  cross-platform execution.

## Evidence and closeout

The parent reconciliation record is
[`20260913-037-adopter-fleet-3-0-2-reconciliation`](../../../evidence/runs/20260913-037-adopter-fleet-3-0-2-reconciliation.json).
It links the native evidence from ForgeLink, SkillForge, ForgeWire, Moto, and
Proving Ground, plus the immutable-head fleet verification.

ForgeWire's native evidence remains `partial`: its unrelated pre-existing
RepoPact work item 274 and audit debt, plus missing application dependencies,
still block that repository's full local validation. The package-boundary
migration itself is verified, and no hosted CI or cross-platform execution is
claimed.
