# 031 — Canonical research metadata and evidence trace repair

> **Status**: Complete 2026-07-22. Evidence
> [`20260722-canonical-research-metadata-and-trace-repair`](../../../evidence/runs/20260722-canonical-research-metadata-and-trace-repair.json).
> **Owners**: governance-owner (lead); tooling-owner and evidence-owner affected.
> **Depends on**: `025`, `028`.

## Intent

Close the missing proposed-state evidence chain and prevent repeated research facts from
drifting independently. The paper currently narrates an episode that has no F-014/capture
013 record, while the threats, figures, and benchmark protocol contradict shipped 2.x
facts.

## Decisions

Research prose remains human-authored. Only repeated identifiers and counts with a real
machine source of truth become generated or cross-checked; semantic claims remain subject
to review rather than being reduced to token substitution.

## Scope

- F-014, capture 013, and their decision/work/release/adopter trace.
- Canonical research metadata and validation.
- Repairs to the threat register, figures, protocol, paper references, gap audit, and run log.

## Acceptance criteria

- [x] **AC-1** — create the missing F-014/capture 013 evidence chain.
- [x] **AC-2** — define canonical metadata for repeated research facts.
- [x] **AC-3** — repair every known contradiction listed in GA-8.
- [x] **AC-4** — regress repeated-fact drift and reconcile closed/open research gaps.

## Closeout

All acceptance criteria are satisfied by the linked evidence. F-014/capture 013 traces
the proposed-state episode; canonical metadata and the repository gate cross-check the
repeated facts; GA-8 is repaired; and the dated gap disposition preserves the remaining
open/operator-gated research work without overstating closure.
