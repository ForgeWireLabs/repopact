# Tooling Agent Contract

## Scope

This subtree owns the Python compatibility CLI, retained Python workflows,
derived-report generation, and bootstrap/record-stamping tools. Canonical Rust
semantic behavior for migrated surfaces is invoked through `repopact-engine`;
the Python validator remains an explicit comparator or retained-surface
implementation, never a hidden fallback.

## Constraints

- Prefer the Python standard library. Declared, operator-approved dependencies are
  permitted: `jsonschema` validates records against repository-local
  `schemas/*.json` with packaged `repopact/schemas/*.json` as the upstream
  fallback (decision `0003`). Pin new dependencies in `requirements.txt`.
- Validators return nonzero on errors and produce deterministic diagnostics.
- Schemas are authoritative for record *structure*; the canonical validator is
  authoritative for cross-record *semantics* (references, lifecycle, cycles)
  on migrated Rust-supported surfaces. The Python implementation remains a
  named comparator.
- Generators may overwrite only files under `audits/reports/`.
- Tests must cover every rule that can block a lifecycle transition.

## Required checks

```powershell
python -m pip install -e ".[dev]"
repopact validate --root <supported-repository>
python -m repopact.run_conformance --legacy-python
python -m unittest discover -s tests -v
```

## Traceability

Maintain `repopact/_audit/inventory.md` and `repopact/_audit/alignment-report.md`
when enforcement behavior changes.
