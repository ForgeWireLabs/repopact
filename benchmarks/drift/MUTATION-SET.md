# Drift mutation set (S5 / H12)

A **drift event** is a change to a project that makes its documented state diverge from
the code. S5 applies a pre-registered sequence of realistic mutations to a repository
governed two ways — **convention-file-only** (C2: `AGENTS.md`/`CLAUDE.md`/`rules.md`) and
**RepoPact** (C7) — and measures, for each, whether the divergence is *detected*, *when*,
and what it costs to reconcile.

The honest baseline expectation: convention files have no detection mechanism at all, so
their detection rate is ≈ 0 until a human happens to notice. The interesting numbers are
therefore **time/edits-to-detection**, **silent-staleness rate**, and **reconciliation
cost** — and RepoPact's *own* blind spot (M9, the F-011 longitudinal class), where
RepoPact also fails to auto-detect until `repopact doctor` is run.

## Scoring (per mutation, per arm)

- `detected` — did anything surface the drift automatically? (RepoPact: a `validate`/CI
  failure or a `doctor` finding; convention files: nothing automated.)
- `latency` — edits or commits between the mutation and detection (∞ if never).
- `silent_staleness` — is a record/doc now factually wrong while the gate stays green?
- `false_drift` — was something flagged as drift that wasn't?
- `reconciliation_cost` — steps to return to a consistent state (manual reconcile vs.
  `repopact doctor [--fix]`).

`expected_repopact` records the predicted RepoPact signal so the run can confirm or
falsify it (a mutation RepoPact was predicted to catch but didn't is a finding, recorded
with equal weight — ¬H12).

## The mutations

| ID | Mutation | Drift it induces | Expected RepoPact signal | Expected convention-file signal |
|---|---|---|---|---|
| **M1** | Rename/move a module a contract or scope path references | scope/contract path now dangles | `I_contract` / `I_ref` violation at validate/CI | none |
| **M2** | Delete a directory the audit registry points to | registry references a missing scope | `validate_contracts` error (cf. F-011) | none |
| **M3** | Remove a CODEOWNERS entry without updating `owners.json` | ownership doc disagrees with reality | `I_ref` (role/scope) violation | none |
| **M4** | Add a CI workflow not reflected as a policy/invariant | enforcement exists, governance silent | **partial blind spot** — not caught unless re-adopted; candidate finding | none |
| **M5** | Weaken/remove a check an invariant references | invariant claims a guarantee no longer enforced | caught only if the check is on the frozen surface (INV-6); else blind spot | none |
| **M6** | Split a file named in a frozen-surface entry | frozen-surface glob now stale | `check-frozen` diff behaviour changes; entry may dangle | none |
| **M7** | Regress code under a `completed` work item (acceptance drift) | completed criterion no longer true | **blind spot** — completion is checkpoint-time, not re-verified; candidate finding | none |
| **M8** | Add a nested `AGENTS.md` without registering it | unregistered contract | `I_contract` (nested contract not registered) | none |
| **M9** | Bump the standard/schema version so an older record shape goes stale (the F-011 class) | records drift *invalid* under evolved rules | **not auto-detected at edit time**; surfaced only by running `doctor`/`validate` after upgrade — RepoPact's documented blind spot | none |
| **M10** | Rename a scope id in `owners.json` without updating dependent records | records reference a missing scope | `I_ref` (owner/aff/role.scopes ⊆ Σ) | none |
| **M11** | Delete an evidence run referenced by a satisfied criterion | satisfied criterion points at missing evidence | `I_ref` / `I_accept` | none |
| **M12** | Introduce a blocked-by cycle between two work items | dependency graph is no longer a DAG | `I_acyclic` | none |
| **M13** | Hand-edit a derived artifact (dashboard) | derived file ≠ `generate(s)` | INV-7 fixpoint via CI dashboard-diff | none |
| **M14** | Change a frozen-surface file without acknowledgement | protected path changed without approval | INV-6 via `check-frozen` (diff-time) | none |
| **M15** | Move a work item to `completed/` without updating its status | status disagrees with directory | `I_ID` (status = directory) | none |

## Notes

- M4, M5, M7, M9 are deliberately included as RepoPact **blind spots or partials**, not
  wins. S5 must report them honestly; overstating RepoPact's drift coverage would be
  exactly the dishonesty the project's thesis rejects.
- The structured, machine-readable list is in [`mutations.json`](mutations.json); the
  harness applies them in order and records the scoring fields per arm.
- Reconciliation cost for RepoPact references `repopact doctor [--fix]` (work item 013,
  decision 0011), the command that exists *because* F-011 showed manual reconcile was the
  only option for the M2/M9 classes.
