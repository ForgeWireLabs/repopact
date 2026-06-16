# Capture 009 — ForgeLink upgrade (older adopter → 1.2.0)

Raw transcript. 2026-06-16. Subject: the **real** `C:/Projects/ForgeLink`, which ran
an *older* RepoPact (the "watered-down" version). Branch `adopt-repopact-1-2-0`.

## Older adopter no longer validates under current rules

```
$ python C:/Projects/repopact/scripts/validate_repo.py --root C:/Projects/ForgeLink
ERROR .: missing root AGENTS.md
ERROR audits\registry.json: audit scope does not exist: docs
ERROR audits\registry.json: audit scope does not exist: todos/001-production-readiness
ERROR work\001-production-readiness\AGENTS.md: nested contract is not registered in audits/registry.json
Validation failed with 4 error(s).
```

Root cause: `registry.json` was written for a structure that changed — the root
`AGENTS.md` was never present, `docs/` was removed, and `todos/001` became
`work/001`. The records drifted out of conformance as the project and the standard
evolved. ForgeLink also had live WIP (`work/001-production-readiness/`, `.local/`, a
`requirements-repopact.txt` edit).

## Reconcile + refresh to 1.2.0

```
# add root AGENTS.md; repair registry (drop docs, point to work/001, register its
# contract); refresh vendored schemas/scripts/templates to 1.2.0 (schemas were
# already current; added adopt_repo, plan_import, generate_*, init, new,
# check_frozen, repopact_cli + templates/)

$ python scripts/validate_repo.py --root .
Repository governance validation passed.
```

Committed on a branch (`.local/` left untracked). The reconcile was hand-done; nothing
in the tooling guided it.

## Finding

**F-011 (major).** An older adopter drifted *invalid* under the evolved standard, and
nothing in RepoPact detected or guided the upgrade — the fix was manual. This is the
first evidence about **longitudinal** adoption (vs. one-shot `adopt`): the architecture
needs a reconcile/upgrade story (a `repopact upgrade`/`doctor` that diagnoses drift and
proposes fixes). Until then, upgrading an older adopter is a manual reconcile.
