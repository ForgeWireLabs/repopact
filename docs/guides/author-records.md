# Guide: Author records

*Diataxis mode: how-to (task-oriented).*

Every durable record has a packaged template in
[`repopact/templates/`](../../repopact/templates/) and a schema in
[`repopact/schemas/`](../../repopact/schemas/). `repopact init` and `adopt` copy
those resources into the repository's conventional `templates/` and `schemas/`
directories. Stamp the common records with `repopact new`.

## Work item

```
repopact new work-item "Short imperative title"
```

Creates `work/active/NNN-slug/` with `work-item.json` and a README. Set at least
one acceptance criterion stating an observable outcome. As work proceeds, keep the
README current — show the evolution, do not overwrite it.

## Decision (ADR)

```
repopact new decision "Adopt X over Y"
```

Use a decision when a choice is hard to reverse and its rationale will outlive the
work item. Record the alternatives you rejected — that is the part git cannot
reconstruct. Never edit a superseded decision; set its status and link forward.

## Policy

```
repopact new policy "Durable rule name"
```

Use a policy for a continuous operating rule that is not itself a binding invariant
(no escalation gate). Policies are where hard-won operating lessons live.

## Evidence run

Copy [`repopact/templates/evidence-run.json`](../../repopact/templates/evidence-run.json) to
`evidence/runs/<id>.json`. Record each command and its exit code. Evidence is
immutable: a rerun creates a new manifest. Link the run from the acceptance
criterion it satisfies.

## Audit finding

Copy the shape from
[`audits/findings/001-schemas-were-not-enforced.json`](../../audits/findings/001-schemas-were-not-enforced.json).
A finding records observed drift and its reconciliation; if the fix is real work,
it spawns a work item.

## Verify

```
repopact validate
```
