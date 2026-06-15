# Contributing to RepoPact

RepoPact governs its own development, so contributing means using it. The
repository is the source of truth; conversations only initiate work.

## Before you start

Read [`AGENTS.md`](AGENTS.md) (the contract), then
[`governance/charter.md`](governance/charter.md) and
[`governance/workflow.md`](governance/workflow.md). New to the model? Follow the
[adoption tutorial](docs/adopt-repopact.md).

## The loop

1. **Capture intent.** `python scripts/new.py work-item "Short title"`, then fill in
   the README and at least one acceptance criterion.
2. **Stay in scope.** Keep each change within one role's scope
   (`governance/owners.json`). Honor the invariants and the frozen surface.
3. **Implement.**
4. **Prove it.** Record an evidence run under `evidence/runs/` and link it from the
   acceptance criterion.
5. **Reconcile.** Regenerate derived artifacts; do not hand-edit them.
6. **Transition.** Move the work-item directory to `work/completed/`.

## Required checks

```
pip install -r requirements.txt
python scripts/validate_repo.py
python -m unittest discover -s tests -v
python scripts/generate_dashboard.py
python scripts/generate_spec.py
```

CI runs the same gates and fails if a derived artifact is stale or validation does
not pass.

## Touching the frozen surface

Changes to paths or symbols in
[`governance/frozen-surface.json`](governance/frozen-surface.json) require
maintainer approval (INV-6). Call it out explicitly in your PR.

## Decisions

If your change makes a material, hard-to-reverse choice, add a decision record
(`python scripts/new.py decision "..."`) so the rationale outlives the work item.

## Conduct

By participating you agree to the [Code of Conduct](CODE_OF_CONDUCT.md). Questions
and ideas are welcome in [Discussions](https://github.com/ForgeWireLabs/repopact/discussions).
