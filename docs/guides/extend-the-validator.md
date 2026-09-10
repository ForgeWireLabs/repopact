# Guide: Extend the validator

*Diataxis mode: how-to (task-oriented).*

The canonical validator is the packaged Rust engine, invoked by `repopact
validate`. The Python module `repopact/validate_repo.py` remains an explicit
legacy comparator and supports retained Python workflows during the transition.
Both split work in two layers (decision [`0003`](../../decisions/0003-validate-records-against-json-schemas.md)):

- **Packaged schemas** (`repopact/schemas/*.json`) are authoritative for record
  *structure* and are copied to adopter repositories as `schemas/*.json`.
- **The canonical engine** is authoritative for cross-record *semantics* on
  migrated surfaces.

Put each new rule in the right layer.

## Add a structural rule

Edit the relevant schema. To add a required field to evidence runs, add it to
`repopact/schemas/evidence-run.schema.json`'s `required`. No Python change is needed; records
are validated against the schema via `jsonschema`.

## Add a semantic rule

For a rule JSON Schema cannot express (a cross-reference, a lifecycle constraint,
or a graph property), add the migrated semantic rule to the appropriate Rust
crate and return a structured diagnostic. Keep diagnostics deterministic and
path-scoped. Update `validate_repo.py` only when maintaining the explicit legacy
comparator or a retained Python-only surface; it is not a hidden fallback for
the public command.

```python
def validate_my_rule(root: Path, problems: list[Problem]) -> None:
    ...
    problems.append(Problem(path, "clear, specific message"))
```

Wire it into the relevant Rust validator path and add a comparator update when
parity evidence requires it.

## Always add a test

Every rule that can block a lifecycle transition needs a test in
`tests/test_validate_repo.py` that mutates a copy of the repo and asserts the error
fires. This is required by the tooling contract.

## Reflect it in the SPEC

If the rule changes conformance, update the relevant §4 prose in `SPEC.md` (the
catalog and invariant tables regenerate themselves). A backward-incompatible change
needs a MAJOR version bump and a decision record.
