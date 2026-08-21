# 043 — Define and Enforce source_of_truth Path-Resolution Semantics

> **Status**: 📋 Planning (proposed — not started)
> **Owners**: governance-owner (lead).
> **Depends on**: none.

## Intent

Finding F-017 (research/captures/016-forgewire-wi238-3-0-0-field-evidence.md) found
that `doctor.py`'s `_dead_source_of_truth` resolves a record's `source_of_truth:`
frontmatter against the repository root, while `takeover.py`'s
`rewrite_inbound_references` (decision 0016, the only existing specification of this
field) treats the same field as record-relative — an internal inconsistency, verified
against the corpus before being recorded, not assumed. This work item defines one
documented, enforced resolution rule and brings `doctor` into line with it. In scope:
`source_of_truth:` path-resolution semantics and `doctor`'s implementation of them. Out
of scope: any other frontmatter field's resolution rules, and any change to
`takeover.py`'s existing (already-correct, per this finding) behavior unless the AC-1
decision requires it.

## Decisions

Not yet made — this item is proposed, not started. The central decision (AC-1) is
whether to formally adopt record-relative resolution (matching the only existing
precedent) or something else, and how to handle the one existing bare-token test case
that currently only passes by coincidence with root-relative resolution.

## Scope

Not yet started. Expected to touch `repopact/doctor.py` (`_dead_source_of_truth`),
`tests/test_validate_repo.py`, and a new `decisions/` record for the resolution rule.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
