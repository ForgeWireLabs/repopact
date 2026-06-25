# 019 — Published conformance suite for the RepoPact standard

> **Status**: 🟢 Active
> **Owners**: tooling-owner (lead); governance-owner (schema/spec coupling).
> **Depends on**: none.

## Intent

Decision [`0019`](../../../decisions/0019-repopact-role-in-forgewire-labs-portfolio.md)
commits RepoPact to the **standard** posture rather than the product posture. A standard
obliges an artefact a third party can implement against and a way to claim conformance.
RepoPact already has `tests/fixtures/{valid,invalid}`, but they are *internal test data*,
not a *published conformance suite*: there is no manifest tying each fixture to the spec
rule it exercises, no runner that points at an arbitrary implementation, and no statement
of what "RepoPact-conformant" means.

This work item closes that gap so "RepoPact is a standard" is backed by something
runnable, not a claim.

## Decisions

Follows decision `0019` (standard posture → conformance is load-bearing). No new decision
required; this is the implementation of an existing one.

## Scope

- `conformance/` — the promoted suite: fixtures + `manifest.json` mapping each case to a
  spec rule / invariant id and its expected validator outcome.
- `scripts/run_conformance.py` — the runner; CI wiring under `.github/`.
- `CONFORMANCE.md` — the conformance statement, versioned against `VERSION`.
- Out of scope: changing the validator's semantics; this *describes and pins* current
  behaviour, it does not alter it.

## Acceptance criteria

- **AC-1** — versioned, documented conformance suite with a rule-mapping manifest and
  expected outcomes.
- **AC-2** — a runner usable against any implementation, wired into CI.
- **AC-3** — `CONFORMANCE.md` defines the conformance claim, tied to `VERSION`; all gates
  pass.

(State is tracked in [`work-item.json`](work-item.json); criteria are `pending` until
each is backed by an evidence run.)
