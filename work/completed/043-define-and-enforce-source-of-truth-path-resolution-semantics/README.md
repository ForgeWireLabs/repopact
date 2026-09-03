# 043 — Define and Enforce source_of_truth Path-Resolution Semantics

> **Status**: ✅ Completed
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

Decision [0034](../../../decisions/0034-source-of-truth-record-relative-resolution.md)
formally adopts record-relative resolution, matching decision 0016 and
`takeover.py`. The former bare-token test case was deliberately corrected: a bare
`AGENTS.md` from `decisions/9999-probe.md` means `decisions/AGENTS.md`, not the
coincident root contract.

## Scope

Implemented in decision [0034](../../../decisions/0034-source-of-truth-record-relative-resolution.md),
`repopact/doctor.py`, and focused regression coverage in
`tests/test_validate_repo.py`. `source_of_truth:` now resolves uniformly as
`declaring_record.parent / token`; bare names do not fall back to repository-root
coincidences, and `doctor --fix` remains non-destructive for stale pointers.

The post-release package/runtime identity invariant is also honored with the
VERSION-pinned development label `3.0.1-dev.1`, deriving deterministic PEP 440
metadata `3.0.1dev1` while leaving `VERSION` unchanged.

## Evidence and closeout

Evidence run [20260902-043-source-of-truth-resolution](../../../evidence/runs/20260902-043-source-of-truth-resolution.json)
records the old root-relative behavior, decision 0016/takeover precedent, nested
`../` and bare-token fixtures, the root-coincidence negative control, stale and
non-destructive-fix behavior, RepoPact/adopter-corpus checks, package identity,
and complete local validation. All six acceptance criteria are satisfied. F-017
is resolved without rewriting its historical capture; its disposition is recorded
in `research/findings.md`.

## Closeout

All six acceptance criteria are satisfied by linked concrete evidence. The complete
directory is now under `work/completed/`, and dashboard/SPEC outputs were regenerated
and validated after the lifecycle transition.
