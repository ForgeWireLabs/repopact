# Capture 011 — SkillForge Academy adoption (real-world test)

Raw transcript. 2026-06-16. Subject: **SkillForge Academy** (local dir
`apex-a-plus-academy`), a real offline-first Tauri/Vite/TS certification-learning app.
Branch `adopt-repopact`. A third real adopter, in a different domain from forgewire and
ForgeLink, with its own homegrown repo-native coordination system.

## Pre-existing planning (not RepoPact)

- root `AGENTS.md` ("SkillForge Academy Agent System" — explicitly "a repo-native
  foundation that can later be mapped into another orchestration tool without losing
  history": RepoPact's own thesis, arrived at independently).
- `todos/TODO-001..007-*.md` (flat files), `tracking/` (decisions/milestones/risks/
  status/work-log), an existing `audits/` (free-form audit markdown), sectioned
  `ROADMAP.md`, `.github/workflows/release.yml`. No CODEOWNERS, no nested contracts.

## Full lifecycle: adopt -> import-plan -> doctor

```
$ adopt --target .            # 24 records; skipped existing AGENTS.md
Adopted repository validates as a conformant RepoPact.
  (release.yml -> governance/policies/001-ci-windows-release.md + INV-2 + frozen surface)

$ import-plan --target .
  + work/active/001-true-cert-factory ... 007-multi-cert-plan-reconciliation-audit   # todos/TODO-00N-*
  + work/completed/202..209-*          # ROADMAP "Shipped" section -> completed/waived
  + work/active/210..214-*             # ROADMAP "Next" section -> active
work/ ledger imported; repository validates as a conformant RepoPact.
# 14 active, 9 completed

$ doctor --target .
repopact doctor: healthy - no drift detected; repository validates.
```

`todos/`, `tracking/`, `audits/` markdown all preserved untouched.

## What this test exercised / surfaced

- **Independent, different-domain adopter** (cert-learning app, not agent infra) → full
  `adopt`+`import-plan`+`doctor` lifecycle to conformant, non-destructively.
- **Flat `TODO-NNN-*.md` todos** → motivated a one-line importer fix: `_split_num` now
  strips a leading tracker prefix (`TODO-`, `TASK-`, `ISSUE-`), so `TODO-001-foo` maps to
  `001-foo` instead of an auto-allocated id. Regression-tested.
- **Coexistence with a homegrown audit/tracking system**: RepoPact's `audits/registry.json`
  sits beside the team's `audits/*.md`; `tracking/` is left for the team to migrate (or
  not). Recorded as **F-012 (holds)**.
