# 024 — Provenance-typed records (concrete / provisional / inferred)

> **Status**: Complete 2026-06-29. Evidence
> [`20260629-provenance-typed-records-closeout`](../../../evidence/runs/20260629-provenance-typed-records-closeout.json).
> **Owners**: governance-owner (lead); tooling-owner (validator, adopt, doctor).
> **Depends on**: none. Part of the 2.0 release (with [023](../023-preflight-mandatory/)).

## Intent

Implement the provenance typing the formal model marked as future work (§4/§7): records carry
a `provenance` of **concrete** (proven; default), **provisional** (reconstructed by tooling),
or **inferred** (from weak signals). This is the principled escape from the **adoption
trilemma** — `adopt` emits provisional/inferred records, so the result is **Closed** (valid,
in R) *and* **Faithful** (reconstruction is labelled, not fabricated as proof). `doctor`
ratchets `provisional → concrete` as real evidence arrives.

## Decisions

[`0021`](../../../decisions/0021-preflight-mandatory-and-provenance.md): provenance enum,
default concrete; L2 rules P1 (admission), P2 (completion requires concrete), P3
(consistency); adopt emits non-concrete provenance; doctor ratchets.

## Scope

- `schemas/work-item.schema.json`, `schemas/evidence-run.schema.json` — `provenance` enum.
- `scripts/validate_repo.py` — `validate_provenance` (P1/P2/P3) + evidence-provenance map.
- `scripts/adopt_repo.py` — seed reconstructed records as `inferred`/`provisional`.
- `scripts/doctor.py` — ratchet provisional → concrete.
- `tests/`, `benchmarks/` — admission / ratchet / consistency suite.

## Acceptance criteria

- **AC-1** — provenance enum across record schemas, default concrete.
- **AC-2** — L2 admission / completion-requires-concrete / consistency.
- **AC-3** — adopt emits provisional; doctor ratchets; tests cover it.

## Closeout

All acceptance criteria are satisfied. Schemas carry provenance types, the
validator enforces admission/completion/consistency, `adopt` emits
provisional/inferred records, and `doctor` ratchets provisional work when
concrete evidence is present. Moto One Hyper provides live adopter evidence for
the provisional adoption + inferred evidence shape.
