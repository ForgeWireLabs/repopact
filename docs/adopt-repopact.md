# Adopt RepoPact

*Diataxis mode: tutorial (learning-oriented). Follow it top to bottom once.*

By the end you will have a RepoPact-governed repository with one work item, one
evidence record, and a passing validator — the full loop, end to end.

## 1. Bootstrap

```
git clone https://github.com/ForgeWireLabs/repopact
python repopact/scripts/init_repo.py --target ./my-project
cd my-project
pip install -r requirements.txt
python scripts/validate_repo.py
```

You now have a valid seed: a root contract, one invariant, one frozen-surface
entry, a scope/owner map, schemas, and empty lifecycle directories.

## 2. Make the governance yours

Open `governance/invariants.json` and write one invariant that matters for *your*
project — a guarantee you never want silently weakened. Add a protected path to
`governance/frozen-surface.json`. Re-run `python scripts/validate_repo.py`.

## 3. Capture a work item

```
python scripts/new.py work-item "Wire up the health check"
```

Edit the generated `work/active/NNN-wire-up-the-health-check/work-item.json`: state
a real acceptance criterion. Fill in the README narrative. Validate.

## 4. Do the work, then record evidence

Make your change. Copy `templates/evidence-run.json` to
`evidence/runs/<date>-health-check.json`, record the command you ran and its exit
code, and link it from the criterion (`"state": "satisfied"`, `"evidence": [...]`).

## 5. Close the loop

Move the work-item directory from `work/active/` to `work/completed/`, set
`status` to `completed`, and regenerate the dashboard:

```
python scripts/generate_dashboard.py
python scripts/validate_repo.py
```

The validator now confirms the work is complete *with proof*. That is the whole
idea: state lives in files, and completion requires evidence.

## Next

- [Concepts](concepts.md) — why it is shaped this way.
- [Define your governance](guides/define-your-governance.md) — roles, invariants,
  frozen surface for a real team.
- [SPEC.md](../SPEC.md) — the normative reference.
