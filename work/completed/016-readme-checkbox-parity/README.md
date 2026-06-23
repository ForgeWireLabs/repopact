# 016 — README checkbox parity with the work-item manifest

> **Status**: ✅ Complete
> **Owners**: tooling (lead).
> **Depends on**: none.

## Intent

Close a gap where a work-item README and its `work-item.json` could silently
disagree. Many teams mirror acceptance criteria in the README as a Markdown
checklist (`- [ ] **<criterion-id>** ...`); the validator checked the manifest but never the
checkboxes, so a `satisfied` criterion could sit unchecked (or vice versa) while
validation still passed. This was first hit in a downstream consumer (ForgeLink).

## What shipped

- `validate_repo.py` gains `validate_readme_checkbox_parity`: where a README uses
  the checklist convention, each manifest criterion must have a checkbox whose
  state matches (`satisfied` → `[x]`, `pending` → `[ ]`; `waived` flexible), and a
  manifest criterion with no checkbox is an error. The check is **gated** on the
  convention being present, so prose READMEs — including RepoPact's own templates
  and work items — are unaffected.
- Two tests in `tests/test_validate_repo.py` (flags a contradiction; silent
  without checkboxes).
- Recorded as decision `0014`; version bumped 1.5.0 → 1.6.0.

## Decisions

- README ↔ manifest checkbox parity, gated on the checklist convention — decision
  [0014](../../../decisions/0014-readme-checkbox-parity.md).

## Evidence

- `20260622-220000-readme-checkbox-parity` — validator green (parity silent on the
  repo's own prose READMEs) and the full validator test suite green with the two
  new parity tests.
