# Audit Findings

A finding records a specific observed disagreement between the declared system and
reality — drift, a gap, or a risk — plus its reconciliation. Findings are the
output of an audit; they are not a second backlog.

Each finding is `NNN-kebab-slug.json` validated against
[`schemas/audit-finding.schema.json`](../../schemas/audit-finding.schema.json):

- `scope`: a scope id from `governance/owners.json`.
- `risk`: `low` | `medium` | `high`.
- `state`: `open` | `reconciled` | `accepted` (accepted = drift acknowledged and
  deliberately tolerated, with the reconciliation explaining why).

A finding whose fix is real work spawns a work item; the finding records the
observation, the work item does the change.
