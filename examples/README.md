# Examples

## The canonical example is this repository

RepoPact governs its own development, so the clearest worked example is RepoPact
itself: real invariants ([`governance/invariants.json`](../governance/invariants.json)),
a frozen surface ([`governance/frozen-surface.json`](../governance/frozen-surface.json)),
work items with acceptance criteria ([`work/completed/`](../work/completed/)),
evidence ([`evidence/runs/`](../evidence/runs/)), decisions
([`decisions/`](../decisions/)), and a reconciliation finding
([`audits/findings/`](../audits/findings/)). The repository is both product and
evidence.

## Generate your own minimal example

A committed second copy would rot against the real schemas and tooling (policy
`001`), so instead generate a fresh one on demand:

```
python scripts/init_repo.py --target ./example-repo
cd example-repo && python scripts/validate_repo.py
```

That produces the smallest valid RepoPact: a root contract, one invariant, one
frozen-surface entry, a scope/owner map, schemas, and empty lifecycle directories —
ready to grow. The [adoption tutorial](../docs/adopt-repopact.md) walks through
filling it in.
