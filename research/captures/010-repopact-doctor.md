# Capture 010 — repopact doctor

Raw transcript. 2026-06-16. `repopact doctor` resolves F-011 (drift over time).
Tests: 44/44.

## Drifted synthetic repo: diagnose -> fix -> valid

```
# bootstrap a valid repo, then break it: delete root AGENTS.md, add a stale registry
# scope (docs/), add an unregistered nested contract (service/AGENTS.md) with an
# incomplete _audit dir.

$ doctor.diagnose(repo)
  ERROR [no-root-contract]   root AGENTS.md is missing (required contract)
  ERROR [registry-stale]     registry scope 'docs' points at a path/contract that does not exist
  ERROR [audit-incomplete]   missing service/_audit/README.md
  ERROR [audit-incomplete]   missing service/_audit/alignment-report.md
  ERROR [contract-unregistered] nested contract service/AGENTS.md is not registered

$ doctor.fix(repo)
  ~ created root AGENTS.md
  ~ registered contract service/AGENTS.md
  ~ dropped 1 stale registry scope(s)
  ~ stubbed service/_audit/README.md
  ~ stubbed service/_audit/alignment-report.md

$ doctor.diagnose(repo) -> []        # clean
$ validate(repo)        -> PASSED
```

## Real ForgeLink: healthy, with the right caution

```
$ python scripts/doctor.py --root C:/Projects/ForgeLink
WARN  [schema-differs] schema work-item.schema.json differs from the installed RepoPact
                       - review by hand (may be an intentional local extension; not auto-fixed)
0 error(s), 1 warning(s), 0 validation issue(s).
```

ForgeLink's `work-item.schema.json` carries the team's `preflight` extension. `doctor`
flags the difference for review but **does not** overwrite it — `--fix` only *adds
missing* schemas, never replaces a differing one. This safety rule was added precisely
because the first ForgeLink run showed a blunt refresh would have clobbered `preflight`.

## Dogfooding caught a doctor bug

Running `doctor` on RepoPact itself first flagged two `AGENTS.md` files under
`tests/fixtures/` as "unregistered contracts" — but the validator passes on RepoPact.
`doctor`'s contract discovery (`adopt_repo.find_nested_contracts`) was broader than the
validator's `repo_model.iter_contracts`, which excludes `fixtures`. Fixed by delegating
`find_nested_contracts` to `iter_contracts`, so `adopt`/`doctor`/validator agree on what
a contract is. After the fix, `doctor` on RepoPact reports healthy.

## Resolves F-011

The manual, twice-repeated ForgeLink reconciliation is now one command
(`repopact doctor --fix`). doctor's one acknowledged limit: it cannot tell an *ahead*
schema from a *stale* one — both "differ" — so it conservatively flags for review.
