# Record Templates

Exemplars for each durable record type. Copy one, or stamp it with
`python scripts/new.py <type> "<title>"`, which fills the id and date and writes
it to the correct location.

| Type | Template | Lands in |
| --- | --- | --- |
| work item | `work-item.json` + `work-item.README.md` | `work/active/NNN-slug/` |
| decision | `decision.md` | `decisions/NNNN-slug.md` |
| policy | `policy.md` | `governance/policies/NNN-slug.md` |
| evidence run | `evidence-run.json` | `evidence/runs/<id>.json` |

Templates are reference material; they are not validated as live records.
