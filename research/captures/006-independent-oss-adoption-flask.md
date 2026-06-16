# Capture 006 — independent (clean-room) OSS adoption: Flask

Raw transcript. 2026-06-15. Subject: **pallets/flask**, a widely-used OSS project
with **no relationship to RepoPact** (it did not inspire the model). This is the
independent generality datum the threats doc (T1) flagged as missing — the
counterpart to the confirmatory forgewire adoption.

## Clone and adopt (with the F-008 guardrail active)

```
$ git clone --depth 50 https://github.com/pallets/flask $TEMP/oss-flask
$ ls $TEMP/oss-flask/.github/workflows
lock.yaml  pre-commit.yaml  publish.yaml  tests.yaml  zizmor.yaml
$ ls $TEMP/oss-flask/CODEOWNERS
(no CODEOWNERS)

$ python scripts/adopt_repo.py --target $TEMP/oss-flask
Created 23 record(s); skipped N existing file(s).
  + AGENTS.md                       # synthesized — Flask has none
  + governance/owners.json          # default governance scope (no CODEOWNERS)
  + governance/policies/001-ci-lock-inactive-closed-issues.md
  + governance/policies/002-ci-pre-commit.md
  + governance/policies/003-ci-publish.md
  + governance/policies/004-ci-tests.md
  + governance/policies/005-ci-github-actions-security-analysis-with-zizmor.md
  + audits/registry.json
  + evidence/runs/<ts>-adopt.json
  + work/completed/000-adopt-repopact/...
  + decisions/0001-adopt-repopact.md

Adopted repository validates as a conformant RepoPact.
```

No `.gitignore` collision warning fired (Flask has no `runs/` rule), so the F-008
guardrail correctly stayed silent — the clean case.

## What this establishes

A real, unrelated OSS project — different domain, different conventions, no
CODEOWNERS, no nested `AGENTS.md`, no lineage to RepoPact — was brought to a
**conformant RepoPact** by `adopt`, non-destructively. Its 5 CI workflows became
binding-gate policies; a root contract was synthesized; history became evidence.

This is **independent** evidence (unlike forgewire, the progenitor). It exercises the
"sparse + workflows" path: it does not prove the CODEOWNERS or nested-contract
mappings generalize (Flask has neither). Closing those would want a second
independent repo that does. Recorded as **F-009 (holds)**; T1 partially retired.
