# Work Ledger

Each work item is a directory containing:

- `README.md`: intent, decisions, plan, and closeout narrative.
- `work-item.json`: lifecycle state used by validators and dashboards.
- Optional local artifacts that are too specific to belong in central evidence.

Directory names use `NNN-kebab-case`. IDs are permanent and never reused.

The directory containing a work item is authoritative for its lifecycle state.
The JSON status must agree with it.

Lifecycle states:

- `proposed`: captured but unauthorized candidate work.
- `active`: authorized or in progress.
- `blocked`: accepted/current but unable to proceed.
- `deferred`: accepted but intentionally postponed.
- `completed`: evidence-closed and delivered.
