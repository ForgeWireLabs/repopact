# Record Types

| Record | Owns | Must not own |
| --- | --- | --- |
| Charter | Principles, non-goals, orientation | Mechanically checkable rules |
| Invariant (`invariants.json`) | Binding guarantees, escalation, enforcer | Implementation plans |
| Frozen surface (`frozen-surface.json`) | Protected paths/symbols and why | Lifecycle state |
| `AGENTS.md` | Authority, boundaries, required checks | Current roadmap status |
| Decision record (`decisions/`) | A material choice and rejected alternatives | Execution logs, lifecycle state |
| Policy (`governance/policies/`) | A durable operating rule and its rationale | One-off choices, escalation gates |
| Work item | Intent, decisions-in-flight, dependencies, acceptance | Raw execution logs |
| Evidence run | Commands, results, artifacts, environment | Product priorities |
| Audit registry | Coverage, ownership, review cadence | Implementation plans |
| Audit finding (`audits/findings/`) | Observed drift, risk, reconciliation | Canonical architecture |
| Dashboard | Derived overview | Source-of-truth state |

Derived reports may be regenerated. Source records must be edited intentionally.

## Choosing between a decision, a policy, and an invariant

- **Decision**: a point-in-time choice. Its rationale outlives the work item that
  produced it, so it lives in `decisions/`, not buried in a closed work item.
- **Policy**: a continuous operating rule with no escalation gate (e.g. "derived
  artifacts are generated"). It encodes a hard-won lesson.
- **Invariant**: a continuous rule that is *binding* — weakening it requires
  operator approval. Invariants are the smallest, most-protected set.
