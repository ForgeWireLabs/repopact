# 031 — Canonical research metadata and evidence trace repair

> **Status**: Active
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

- [ ] **AC-1** — create the missing F-014/capture 013 evidence chain.
- [ ] **AC-2** — define canonical metadata for repeated research facts.
- [ ] **AC-3** — repair every known contradiction listed in GA-8.
- [ ] **AC-4** — regress repeated-fact drift and reconcile closed/open research gaps.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
