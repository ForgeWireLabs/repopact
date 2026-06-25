---
id: 0021
title: Preflight mandatory by default, and provenance-typed records (2.0)
status: accepted
date: 2026-06-24
supersedes: ["0018"]
---

# 0021: Preflight mandatory by default, and provenance-typed records (2.0)

## Context

Decision [`0018`](0018-generalized-opt-in-preflight-marker.md) made the preflight marker a
generalized but **opt-in, default-off** feature, on the grounds that an always-on default
would immediately fail every existing adopter with marker-less work items. Two things have
changed:

1. The operator now wants preflight to be **mandatory**: no work should begin until a work
   item is created and propagated through the pact. Opt-in undercuts the guarantee.
2. The formal model's **provenance typing** (formal-model.md §4/§7) — the principled escape
   from the adoption trilemma — is ready to move from future work into the kernel. Provenance
   also supplies the grandfathering mechanism that makes a mandatory default safe.

These ship together as **2.0** (a breaking change to default validation behavior).

## Decision

### A. Preflight is mandatory by default

- The validator's `_preflight_required` defaults to **enabled**: with no config, a marker is
  required on every work item.
- `repopact new` stamps the marker (and concrete provenance) automatically — creating the
  item *is* the preflight act.
- **Grandfathering** keeps it non-destructive: a repo exempts its pre-2.0 items via
  `required_from_id` / `required_from_date`. RepoPact's own `owners.json` sets
  `required_from_id: 23` (items 000–022 predate the mandate). `adopt` and `doctor` set
  `required_from_date` to the adoption/upgrade date so existing adopters are grandfathered on
  upgrade rather than broken.

### B. Provenance-typed records

- Records carry an optional `provenance ∈ {concrete, provisional, inferred}` (work items,
  acceptance criteria, evidence runs). **Absent = concrete**, so every existing record stays
  valid. Order: `inferred < provisional < concrete`.
- L2 rules (`validate_provenance`):
  - **P1 admission** — `provisional`/`inferred` items are *valid* states. This is the
    trilemma escape: `adopt` can be **Closed** (in R) and **Faithful** (labels reconstruction)
    at once, instead of relaxing closure.
  - **P2 completion requires concrete** — a `completed` item must be concrete and every
    satisfied criterion backed only by concrete evidence (keeps INV-3 honest; no completing
    on reconstructed proof).
  - **P3 consistency** — a `concrete` item may not rest on non-concrete evidence; it must
    declare itself provisional/inferred until the evidence is ratcheted.
- `adopt` emits reconstructed records as `inferred`/`provisional`; `doctor` **ratchets**
  `provisional → concrete` as real evidence arrives (conservative, monotone).

## Alternatives considered

- **Keep opt-in (0018 as-is).** Rejected: the operator wants a hard guarantee, not a
  convention.
- **Always-on with no grandfather.** Rejected: retroactively invalidates every adopter and
  RepoPact's own 000–022. Grandfather thresholds + provenance avoid this.
- **Boolean `provisional` instead of a 3-value enum.** Rejected: loses the
  inferred-vs-provisional distinction the consistency rule and the trilemma argument rely on.
- **Provenance in a side ledger.** Rejected: splits provenance from the record, weakening
  faithfulness and in-place diffability.

## Consequences

- **Breaking (2.0).** An adopter who upgrades and re-validates without declaring a preflight
  epoch will be told to; `doctor` performs the one-step migration (sets `required_from_date`).
  Schema changes (`schemas/**`) touch the frozen surface and ship with operator approval.
- `adopt` becomes Closed + Faithful via provisional typing — the trilemma is resolved in the
  implementation, not only the model.
- The mandatory marker means the work ledger can never silently trail the code: there is no
  "work first, record later".
