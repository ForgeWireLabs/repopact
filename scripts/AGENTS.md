# Tooling Agent Contract

## Scope

This subtree owns governance validation and derived-report generation. It may read
all repository records but must not mutate source records as a side effect.

## Constraints

- Use the Python standard library unless a dependency provides clear value.
- Validators return nonzero on errors and produce deterministic diagnostics.
- Generators may overwrite only files under `audits/reports/`.
- Tests must cover every rule that can block a lifecycle transition.

## Required checks

```powershell
python scripts/validate_repo.py
python -m unittest discover -s tests -v
```

## Traceability

Maintain `scripts/_audit/inventory.md` and `scripts/_audit/alignment-report.md`
when enforcement behavior changes.

