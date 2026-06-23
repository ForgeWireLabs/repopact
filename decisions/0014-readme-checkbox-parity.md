---
id: 0014
title: README Checkbox Parity With the Work-Item Manifest
status: accepted
date: 2026-06-22
supersedes: []
---

# 0014: README Checkbox Parity With the Work-Item Manifest

## Context

A work item's `work-item.json` is the source of truth for acceptance-criterion
state, but many teams also mirror those criteria in the README as a Markdown
checklist (`- [ ] **AC-1** ...`). The validator checked the manifest thoroughly
but never looked at the README's checkboxes, so the two could silently disagree:
a criterion marked `satisfied` in the manifest while still shown unchecked in the
README, or the reverse. A reader trusting the README would be misled, and the
green validation gave no warning.

This was first observed in a downstream consumer (ForgeLink), where a hand-edited
README left several `satisfied` criteria unchecked while validation still passed.

## Decision

`validate_repo.py` adds a **README ↔ manifest checkbox parity** check
(`validate_readme_checkbox_parity`). Where a work-item README uses the
`- [ ] **ID** ...` checklist convention, every manifest criterion must have a
checkbox whose state matches the manifest:

- `satisfied` → `[x]`,
- `pending` → `[ ]`,
- `waived` → either (left flexible).

A manifest criterion with no checkbox in a convention-using README is also an
error. The check is **gated on the convention being present**: items whose READMEs
describe criteria in prose (including RepoPact's own templates and work items) are
unaffected. The manifest remains authoritative; this only stops the README from
contradicting it.

## Consequences

- Repos that adopt the checklist convention cannot pass validation while their
  README disagrees with the manifest.
- Repos that do not use checkboxes see no change.
- The default work-item README template stays prose-based; adopting checkboxes is
  opt-in, and adopting them opts into the parity check.
