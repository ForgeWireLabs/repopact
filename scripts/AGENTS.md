# Tooling Agent Contract

## Scope

This subtree owns governance validation, derived-report generation, and the
bootstrap/record-stamping tools. It may read all repository records but must not
mutate source records as a side effect.

## Constraints

- Prefer the Python standard library. Declared, operator-approved dependencies are
  permitted: `jsonschema` validates records against `schemas/*.json` (decision
  `0003`). Pin new dependencies in `requirements.txt`.
- Validators return nonzero on errors and produce deterministic diagnostics.
- Schemas are authoritative for record *structure*; the validator is authoritative
  for cross-record *semantics* (references, lifecycle, cycles).
- Generators may overwrite only files under `audits/reports/`.
- Tests must cover every rule that can block a lifecycle transition.

## Required checks

```powershell
pip install -r requirements.txt
python scripts/validate_repo.py
python -m unittest discover -s tests -v
```

## Traceability

Maintain `scripts/_audit/inventory.md` and `scripts/_audit/alignment-report.md`
when enforcement behavior changes.
