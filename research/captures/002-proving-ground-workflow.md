# Capture 002 — proving-ground adoption and workflow

Raw transcript. 2026-06-15. Proving ground: `C:/Projects/repopact-proving-ground`,
RepoPact installed from the v0.1.0 wheel. Subject app: `unitconv` (8 passing tests).

## Adopt from the wheel

```
$ python -m venv .venv
$ .venv/Scripts/python.exe -m pip install repopact-0.1.0-py3-none-any.whl
$ repopact init --target .
Bootstrapped a valid RepoPact at C:\Projects\repopact-proving-ground

$ python -m unittest discover -s tests
Ran 8 tests in 0.000s
OK
$ python unitconv.py temp 100 C F
212.0000
$ python unitconv.py length 1 mi km
1.6093
```

## F-003 (H3) — completion is evidence-gated

```
# AC-1 set to "satisfied" with evidence: []
$ repopact validate --root .
ERROR work\active\001-...\work-item.json: criterion AC-1 is satisfied without evidence
Validation failed with 1 error(s).

# produced evidence run 20260615-202635-unitconv-tests from the real test run,
# linked it to AC-1 and AC-2, both satisfied:
$ repopact validate --root .
Repository governance validation passed.
```

## F-004 (H5) — status is a filesystem fact

```
$ mv work/active/001-... work/completed/001-...      # status still "active"
$ repopact validate --root .
ERROR work\completed\001-...\work-item.json: status 'active' does not match directory 'completed'
Validation failed with 1 error(s).

# set status: completed
$ repopact validate --root .
Repository governance validation passed.
$ repopact dashboard --root .
Generated audits\reports\dashboard.md
$ git commit -m "feat: unitconv CLI governed by RepoPact (work item 001)"   # f3be631
```

## F-002 / F-005 (H4) — frozen surface

```
# governance/invariants.json (protected) weakened in the WORKING TREE, uncommitted:
$ repopact check-frozen --root . --base HEAD
No frozen-surface changes detected.        # <-- F-002: working-tree change invisible
EXIT=0

# after committing the change:
$ repopact check-frozen --root . --base HEAD~1
Frozen-surface changes detected (INV-6 requires operator approval):
  governance/invariants.json: Invariants are the pact; weakening requires operator approval.
EXIT=1                                      # <-- F-005: committed change caught

$ repopact check-frozen --root . --base HEAD~1 --ack
... Operator approval acknowledged (--ack). Proceeding.
EXIT=0

# invariant restored, re-validated, committed (4b4d3d3)
$ repopact validate --root .
Repository governance validation passed.
```

## F-006 (H6) — recoverability from the tree alone

The committed tree contains, with no external context:

- `work/completed/001-temperature-and-length-conversion/README.md` — intent and the
  pivot-unit decision.
- `.../work-item.json` — acceptance criteria, each linked to evidence.
- `evidence/runs/20260615-202635-unitconv-tests.json` — the proof (8 tests, exit 0).
- `audits/reports/dashboard.md` — derived status.

A fresh reader (or agent) can reconstruct what was done, why, and with what proof,
without the conversation that produced it.
