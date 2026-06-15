# Guide: Extend the validator

*Diataxis mode: how-to (task-oriented).*

The validator (`scripts/validate_repo.py`) splits work in two layers (decision
[`0003`](../../decisions/0003-validate-records-against-json-schemas.md)):

- **Schemas** (`schemas/*.json`) are authoritative for record *structure*.
- **The validator** is authoritative for cross-record *semantics*.

Put each new rule in the right layer.

## Add a structural rule

Edit the relevant schema. To add a required field to evidence runs, add it to
`schemas/evidence-run.schema.json`'s `required`. No Python change is needed; records
are validated against the schema via `jsonschema`.

## Add a semantic rule

For a rule JSON Schema cannot express (a cross-reference, a lifecycle constraint, a
graph property), add a check in `validate_repo.py` that appends a `Problem`. Keep
diagnostics deterministic and path-scoped.

```python
def validate_my_rule(root: Path, problems: list[Problem]) -> None:
    ...
    problems.append(Problem(path, "clear, specific message"))
```

Wire it into `validate()`.

## Always add a test

Every rule that can block a lifecycle transition needs a test in
`tests/test_validate_repo.py` that mutates a copy of the repo and asserts the error
fires. This is required by the tooling contract.

## Reflect it in the SPEC

If the rule changes conformance, update the relevant §4 prose in `SPEC.md` (the
catalog and invariant tables regenerate themselves). A backward-incompatible change
needs a MAJOR version bump and a decision record.
