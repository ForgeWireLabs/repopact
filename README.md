# RepoPact

RepoPact is a repository-native operating system for durable agent work.
It makes authority, planning, evidence, and architectural drift visible in the
filesystem so that a new contributor or agent can recover the state of the
project without depending on a prior conversation.

> Make the repository the pact: authority, intent, evidence, and history that
> survive every session.

## Core loop

```text
intent -> scoped authority -> work item -> implementation -> evidence -> audit -> history
```

## Five primitives

1. **Charter**: project thesis and invariants in `governance/`.
2. **Scopes**: layered `AGENTS.md` files near the code they govern.
3. **Work items**: narrative `README.md` plus machine-readable `work-item.json`.
4. **Evidence**: immutable run manifests under `evidence/runs/`.
5. **Reconciliation**: audits that compare declared architecture with reality.

## Quick start

```powershell
python scripts/validate_repo.py
python scripts/generate_dashboard.py
python -m unittest discover -s tests -v
```

Begin with [`AGENTS.md`](AGENTS.md), then read
[`governance/workflow.md`](governance/workflow.md).

## Status is a filesystem transition

Work moves between `work/active`, `work/blocked`, `work/deferred`, and
`work/completed`. Its `work-item.json` status must match its directory. Moving a
work item never deletes its reasoning, decisions, or evidence links.
