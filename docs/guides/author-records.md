# Guide: Author records

*Diataxis mode: how-to (task-oriented).*

Every durable record has a template in [`templates/`](../../templates/) and a
schema in [`schemas/`](../../schemas/). Stamp the common ones with `scripts/new.py`.

## Work item

```
python scripts/new.py work-item "Short imperative title"
```

Creates `work/active/NNN-slug/` with `work-item.json` and a README. Set at least
one acceptance criterion stating an observable outcome. As work proceeds, keep the
README current — show the evolution, do not overwrite it.

## Decision (ADR)

```
python scripts/new.py decision "Adopt X over Y"
```

Use a decision when a choice is hard to reverse and its rationale will outlive the
work item. Record the alternatives you rejected — that is the part git cannot
reconstruct. Never edit a superseded decision; set its status and link forward.

## Policy

```
python scripts/new.py policy "Durable rule name"
```

Use a policy for a continuous operating rule that is not itself a binding invariant
(no escalation gate). Policies are where hard-won operating lessons live.

## Evidence run

Copy [`templates/evidence-run.json`](../../templates/evidence-run.json) to
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
python scripts/validate_repo.py
```
