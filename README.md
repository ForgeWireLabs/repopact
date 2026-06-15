# RepoPact

RepoPact is a repository-native operating system for durable agent work.
It makes authority, intent, evidence, and architectural drift visible in the
filesystem so that a new contributor or agent can recover the state of the
project without depending on a prior conversation.

> Make the repository the pact: authority, intent, evidence, and history that
> survive every session.

## Why "pact"

Conventions like `AGENTS.md`, ADRs, and issue trackers each capture a fragment of
project knowledge, but none of them makes the *binding* part explicit — the
guarantees an agent must not silently weaken. RepoPact's wedge is the **binding
invariant**: a declared guarantee with a rationale, an escalation path, and (where
possible) a machine enforcer. That, plus evidence-gated completion and a
filesystem state machine, is what turns a folder convention into a contract.

## Core loop

```text
intent -> scoped authority -> work item -> implementation -> evidence -> audit -> history
```

## Primitives

1. **Charter & invariants**: principles (judgment) and binding invariants
   (escalation-gated) in `governance/`.
2. **Frozen surface**: paths and symbols that require operator approval to change.
3. **Scopes & roles**: layered `AGENTS.md` files plus a role/scope map in
   `governance/owners.json`.
4. **Work items**: narrative `README.md` plus machine-readable `work-item.json`,
   with evidence-linked acceptance criteria.
5. **Evidence**: immutable run manifests under `evidence/runs/`.
6. **Decisions & policies**: durable choices (`decisions/`) and operating rules
   (`governance/policies/`) whose rationale outlives any single work item.
7. **Reconciliation**: audits and a *generated* dashboard that surface drift and
   review staleness rather than hand-maintaining it.

## Quick start

```powershell
pip install -r requirements.txt
python scripts/validate_repo.py
python scripts/generate_dashboard.py
python -m unittest discover -s tests -v
python scripts/check_frozen_surface.py --base origin/main
```

Records are validated against `schemas/*.json` (structure) and by the validator
(cross-record semantics). See decision
[`0003`](decisions/0003-validate-records-against-json-schemas.md).

Begin with [`AGENTS.md`](AGENTS.md), then read
[`governance/charter.md`](governance/charter.md) and
[`governance/workflow.md`](governance/workflow.md).

## Adopt into a new repository

```powershell
python scripts/init_repo.py --target ../your-repo   # seed a valid RepoPact
python scripts/new.py work-item "Title of the work" # stamp records from templates/
```

`init_repo.py` writes the minimal valid source records, copies the schemas and
tooling, and validates the result. Templates for every record type live in
[`templates/`](templates/). RepoPact is licensed under Apache-2.0 (see
[`LICENSE`](LICENSE) and decision [`0002`](decisions/0002-license-apache-2.0.md));
its spec version is in [`VERSION`](VERSION).

## Status is a filesystem transition

Work moves between `work/active`, `work/blocked`, `work/deferred`, and
`work/completed`. Its `work-item.json` status must match its directory. Moving a
work item never deletes its reasoning, decisions, or evidence links.

## Derive over declare

Anything computable from source records is generated, not authored by hand. The
dashboard and audit-freshness views are derived; the only hand-maintained files
are genuine sources (charter, invariants, frozen surface, schemas, owners, work
items, evidence, decisions, policies). See policy `001`.
