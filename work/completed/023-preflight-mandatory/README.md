# 023 — Make preflight mandatory by default (2.0)

> **Status**: Complete 2026-06-29. Evidence
> [`20260629-preflight-mandatory-closeout`](../../../evidence/runs/20260629-preflight-mandatory-closeout.json).
> **Owners**: governance-owner (lead); tooling-owner (validator + `new`).
> **Depends on**: none. Part of the 2.0 release (with [024](../024-provenance-typed-records/)).

## Intent

Flip preflight from opt-in/default-off (decision 0018) to **mandatory by default**: no work
should begin until a work item is created and propagated through the pact. Creating the item
with `repopact new` *is* the preflight act, so `new` stamps the marker automatically.

Recorded before implementation began — this item dogfoods the rule it introduces.

## Decisions

[`0021`](../../../decisions/0021-preflight-mandatory-and-provenance.md) (supersedes 0018):
preflight enabled by default; grandfather pre-2.0 items via `required_from_id` /
`required_from_date`; `new` stamps the marker; `adopt`/`doctor` set the grandfather date.

## Scope

- `scripts/validate_repo.py` — `_preflight_required` defaults enabled.
- `scripts/new.py` — stamps the preflight marker + concrete provenance.
- `governance/owners.json` — `preflight.enabled` with `required_from_id: 23` (grandfather).
- `scripts/adopt_repo.py`, `scripts/doctor.py` — set `required_from_date` on adopt/upgrade.
- `tests/` — default-on + grandfather coverage.

## Acceptance criteria

- **AC-1** — default-on; grandfather thresholds for existing items.
- **AC-2** — `new` stamps the marker; RepoPact grandfathers 000-022.
- **AC-3** — adopt/doctor grandfather on upgrade; tests cover it.

## Closeout

All acceptance criteria are satisfied. RepoPact validates with default-on
preflight, `repopact new` stamps concrete preflight markers, and the Moto One
Hyper adopter validates with `required_from_date` and post-adoption preflight
markers.
