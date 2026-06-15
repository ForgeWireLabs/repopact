# RepoPact Agent Contract

## Binding thesis

The repository is the durable coordination surface. Conversations may initiate
work, but authority, state, decisions, validation, and handoff must survive in
versioned files.

## The pact (binding invariants)

The invariants in [`governance/invariants.json`](governance/invariants.json) are
binding on every role. You may not weaken one without flagging it and obtaining
explicit human operator approval. Where an invariant names an `enforced_by` rule,
that check is the backstop; where it names `null`, escalation is the only
enforcement and the responsibility is yours. Principles (judgment) and invariants
(binding) are kept distinct — see [`governance/charter.md`](governance/charter.md).

## Frozen surface

Paths and symbols listed in
[`governance/frozen-surface.json`](governance/frozen-surface.json) must not be
changed without operator approval (`INV-6`). Run
`python scripts/check_frozen_surface.py --base <ref>` before proposing a change
that may touch them; only pass `--ack` after a human has approved.

## Invariants

- Read every `AGENTS.md` from the repository root to the target path.
- The deepest applicable contract refines its parents but cannot weaken them.
- Every change belongs to one explicit owner scope (the non-overlap principle).
- Cross-scope work must name a lead and affected scopes in its work item.
- Work is not complete until its acceptance criteria have linked evidence.
- Never rewrite completed work to make history appear cleaner.
- Audits report drift; they do not silently redefine the intended architecture.
- Machine-readable lifecycle state lives in JSON. Markdown explains why.
- Derived artifacts are generated, not hand-maintained (policy `001`).

## Roles and writable scopes

Roles and the scopes they may write are declared in
[`governance/owners.json`](governance/owners.json). Each role is bound to one or
more path scopes; keep each change within a single role's scope. Adopters redefine
these roles for their own repository — the contract is the structure, not this
particular role set.

- **Governance owner**: `AGENTS.md`, `governance/`, `schemas/`, `decisions/`.
- **Work coordinator**: `work/`, status transitions, dependency records.
- **Evidence owner**: `evidence/`, validation manifests, reproducibility records.
- **Tooling owner**: `scripts/`, `tests/`, CI automation.

## Durable records

- **Decisions** (`decisions/`): material, hard-to-reverse choices and rejected
  alternatives. Promote a decision here when its rationale outlives the work item.
- **Policies** (`governance/policies/`): durable operating rules and hard-won
  lessons that are not themselves binding invariants.

## Required checks

Run before proposing completion:

```powershell
python scripts/validate_repo.py
python -m unittest discover -s tests -v
```

Regenerate `audits/reports/dashboard.md` after changing governed structure or
work-item status. Run `python scripts/check_frozen_surface.py` when a change may
touch the frozen surface.

## Change protocol

1. Honor the pact: check invariants and the frozen surface first.
2. Capture intent in a work item.
3. Discover applicable contracts and ownership.
4. Record acceptance criteria before implementation; promote durable choices to
   `decisions/`.
5. Implement within scope.
6. Record validation in an evidence manifest.
7. Reconcile affected audit entries; regenerate the dashboard.
8. Move the complete work-item directory to `work/completed/`.
