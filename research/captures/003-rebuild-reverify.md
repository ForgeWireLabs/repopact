# Capture 003 — hardening, rebuild, and re-verification

Raw transcript. 2026-06-15. RepoPact branch `007-proving-ground-hardening`.

## Tests after the fixes (in the checkout)

```
$ python -m unittest discover -s tests
Ran 30 tests in 62.099s
OK
# includes:
#   test_cli_spec_fails_cleanly_without_spec_file (F-001)
#   test_check_frozen_detects_working_tree_change (F-002)
```

## Dogfood: work item 007 closeout + 1.0 bump

```
$ python scripts/validate_repo.py
Repository governance validation passed.            # 007 completed, evidence linked

$ python scripts/check_frozen_surface.py --root . --base main
No frozen-surface changes detected.                 # no protected paths touched

# VERSION 0.1.0 -> 1.0.0; SPEC version block regenerated
$ grep -A1 "generated:version" SPEC.md
<!-- generated:version -->
This document specifies **RepoPact 1.0.0**.
```

## Rebuild and re-verify from the wheel (in the proving ground)

```
$ python -m build --wheel
Successfully built repopact-1.0.0-py3-none-any.whl

# in repopact-proving-ground:
$ pip install --force-reinstall --no-deps repopact-1.0.0-py3-none-any.whl

$ repopact spec --root .
No SPEC.md at C:\Projects\repopact-proving-ground. `spec` regenerates the derived
blocks of an existing SPEC.md ... an adopter repository does not need one.
EXIT=1                                              # F-001 fixed (was a traceback)

# weaken protected governance/invariants.json in the WORKING TREE, uncommitted:
$ repopact check-frozen --root . --base HEAD
Frozen-surface changes detected (INV-6 requires operator approval):
  governance/invariants.json: Invariants are the pact; weakening requires operator approval.
EXIT=1                                              # F-002 fixed (was "no changes")

$ git checkout -- governance/invariants.json
$ repopact validate --root .
Repository governance validation passed.
```

Both defects are fixed in the *packaged* product, not merely the source tree.
