# RepoPact Agent Contract

## Binding thesis

The repository is the durable coordination surface. Conversations may initiate
work, but authority, state, decisions, validation, and handoff must survive in
versioned files.

## Invariants

- Read every `AGENTS.md` from the repository root to the target path.
- The deepest applicable contract refines its parents but cannot weaken them.
- Every change belongs to one explicit owner scope.
- Cross-scope work must name a lead and affected scopes in its work item.
- Work is not complete until its acceptance criteria have linked evidence.
- Never rewrite completed work to make history appear cleaner.
- Audits report drift; they do not silently redefine the intended architecture.
- Machine-readable lifecycle state lives in JSON. Markdown explains why.

## Writable scopes

- **Governance owner**: `AGENTS.md`, `governance/`, `schemas/`, audit policy.
- **Work coordinator**: `work/`, status transitions, dependency records.
- **Evidence owner**: `evidence/`, validation manifests, reproducibility records.
- **Tooling owner**: `scripts/`, `tests/`, CI automation.

## Required checks

Run before proposing completion:

```powershell
python scripts/validate_repo.py
python -m unittest discover -s tests -v
```

Regenerate `audits/reports/dashboard.md` after changing governed structure or
work-item status.

## Change protocol

1. Capture intent in a work item.
2. Discover applicable contracts and ownership.
3. Record acceptance criteria before implementation.
4. Implement within scope.
5. Record validation in an evidence manifest.
6. Reconcile affected audit entries.
7. Move the complete work-item directory to `work/completed/`.
