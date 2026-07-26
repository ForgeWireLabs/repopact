# Tooling Agent Contract

## Scope

This subtree owns governance validation, derived-report generation, and the
bootstrap/record-stamping tools. It may read all repository records but must not
mutate source records as a side effect.

## Constraints

- Prefer the Python standard library. Declared, operator-approved dependencies are
  permitted: `jsonschema` validates records against repository-local
  `schemas/*.json` with packaged `repopact/schemas/*.json` as the upstream
  fallback (decision `0003`). Pin new dependencies in `requirements.txt`.
- Validators return nonzero on errors and produce deterministic diagnostics.
- Schemas are authoritative for record *structure*; the validator is authoritative
  for cross-record *semantics* (references, lifecycle, cycles).
- Generators may overwrite only files under `audits/reports/`.
- Tests must cover every rule that can block a lifecycle transition.

## Required checks

```powershell
pip install -r requirements.txt
python -m pip install -e .
repopact validate
python -m unittest discover -s tests -v
```

## Traceability

Maintain `repopact/_audit/inventory.md` and `repopact/_audit/alignment-report.md`
when enforcement behavior changes.
