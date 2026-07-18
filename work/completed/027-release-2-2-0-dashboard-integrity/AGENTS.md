# RepoPact 2.2.0 Release Agent

## Scope

- Own the 2.2.0 version surfaces, release decision, validation, distribution build,
  git-tag/PyPI publication evidence, ForgeLink formal-release adoption, and closeout.

## Required checks

- `python scripts/validate_repo.py`
- `python -m unittest discover -s tests -v`
- `python scripts/run_conformance.py`
- regenerate dashboard and specification, then require a clean diff
- build sdist and wheel and verify their version metadata

## Constraints

- Preserve immutable completed history for work item 026.
- Do not weaken canonical dashboard validation to ease release.
- Do not claim publication from a local build or tag alone.
- GitHub Actions publishing is billing-blocked for this release. Direct PyPI upload is
  permitted only from the exact locally validated tag artifacts, without recording
  credentials in repository evidence.
