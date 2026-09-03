# 049 — Reconcile Post-3.0.2 Evidence-Timestamp Hotfix and Development Identity

> **Status**: ✅ Completed.
> **Owners**: governance-owner (lead); tooling-owner, evidence-owner, and work-coordinator affected.
> **Depends on**: WI045 (stable source/artifact identity), WI048 (3.0.2 release boundary).

## Intent

A runtime validator change landed on `main` after the public `v3.0.2` release commit without a RepoPact work item and without the `RELEASE_LABEL` required by decision 0032 for materially later package/runtime source at the same `VERSION`. The change, commit `f2c80b7dcdc54ff9f4753bc996ef0b6dfba539bf`, rejects evidence-run timestamps materially in the future and was motivated by a real ForgeWire WI260 field defect.

This work does **not** pretend that change was preflight-governed. The historical commit remains part of the record. WI049 begins now, before reconciliation work starts, to decide whether the landed hotfix should be retained or reverted, restore current development identity, and bring the retained behavior under durable RepoPact evidence and documentation without rewriting history.

## Required reconciliation

1. Reproduce the motivating future-timestamp defect and inspect the exact landed implementation/tests.
2. Decide explicitly whether the rule is generic and correct enough to retain. If not, revert it cleanly. If retained, record its semantics, clock-skew tolerance, timezone behavior, and limits.
3. Restore decision-0032 compliance for post-3.0.2 runtime source by adding a VERSION-pinned development `RELEASE_LABEL` before any further package/runtime work.
4. Record concrete evidence for the retained/reverted outcome and the governance breach itself; do not relabel the original hotfix as though a preflight work item existed.
5. Keep WI044 frozen-schema implementation paused until this baseline is valid and closed.

## Boundaries

- Do not cut a new stable release in WI049.
- Do not change `VERSION` or move `v3.0.2`.
- Do not implement WI044, WI046, or WI047 here.
- Do not weaken preflight, provenance, frozen-surface, or stable source/artifact identity rules to excuse the historical hotfix.

## Closeout

WI049 completed on 2026-09-03. The landed wall-clock comparison was replaced
with the deterministic, opt-in Git-recording chronology in decision 0037:
`timestamp_basis: "git-recording"` records normalize to UTC and must be no
later than their first recording commit plus five minutes; legacy backfills and
Git-free exports retain structural-only validation. The post-release runtime
identity is explicitly `VERSION=3.0.2`, `RELEASE_LABEL=3.0.2-dev.1`, with
derived package version `3.0.2dev1`. The stable `v3.0.2` tag is unchanged, no
stable release was cut, and WI044/WI046/WI047 remain unimplemented. Full
criterion evidence is [`20260903-049-evidence-timestamp-reconciliation`](../../../evidence/runs/20260903-049-evidence-timestamp-reconciliation.json).
