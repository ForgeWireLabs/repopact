# 041 — Record WI238 Upstream Findings — Worktree Release Lag and Doctor Path Resolution

> **Status**: ✅ Completed
> **Owners**: governance-owner (lead).
> **Depends on**: none.

## Intent

ForgeWire's completed WI 238 (upgrading its RepoPact pin from `2.2.0` to the exact
public `3.0.0` release) surfaced two upstream RepoPact issues as a byproduct of a
downstream migration, not RepoPact-side implementation work. This work item is a
recording/triage pass only: verify the current state of RepoPact `main`, source the
field evidence directly from ForgeWire's own durable evidence records, update the
existing worktree finding (F-015) with new release-lag evidence rather than duplicating
it, verify `source_of_truth:` resolution semantics against the actual corpus before
recording a new finding (F-017), preserve a durable field-evidence capture, and record
two `proposed` (not `active`) implementation work items. No RepoPact implementation
change, no release, and no ForgeWire change are in scope here.

## Decisions

- **Update F-015, do not duplicate it.** WI 238's worktree reproduction is the same
  defect class F-015 already covers, on the current public release rather than a stale
  pin. Recorded as new evidence within F-015, with the finding's own interpretation
  corrected to distinguish adopter version lag (the original incident) from release lag
  (this reproduction) — a real distinction, not a synonym for "version drift."
- **Verify `source_of_truth:` semantics before recording F-017, not assume ForgeWire's
  reading.** Decision 0016 and `takeover.py`'s `rewrite_inbound_references` are the only
  existing specification and implementation of this field in the corpus, and both treat
  it as record-relative — the same rule Markdown links follow. `doctor.py`'s
  root-relative resolution is the outlier, confirmed against that precedent, not assumed
  from ForgeWire's usage alone.
- **Split, not combine, the two proposed fix work items (042, 043).** Issue A is a
  contract-discovery/filesystem-walk problem; Issue B is a path-resolution-semantics
  problem in a different module (`doctor.py` vs. `repo_model.py`), with different root
  causes and different acceptance criteria. Combining them would blur two independently
  reviewable and independently implementable pieces of work.
- **No paper or formal-model change.** Neither issue contradicts an existing claim in
  `paper.md` or `formal-model.md`; both are findings-register-level implementation
  defects, not architectural claims requiring revision.

## Scope

- `research/findings.md` — F-015 updated (register row + detailed section) with WI 238's
  release-lag reproduction; new finding F-017 added (doctor `source_of_truth`
  path-resolution bug).
- `research/captures/016-forgewire-wi238-3-0-0-field-evidence.md` — new durable
  field-evidence capture.
- `work/proposed/042-structural-git-worktree-awareness-in-contract-discovery/` — new
  proposed work item (Issue A).
- `work/proposed/043-define-and-enforce-source-of-truth-path-resolution-semantics/` —
  new proposed work item (Issue B).
- No change to RepoPact implementation code, no release, no change to work item 037, no
  ForgeWire file touched.

## Closeout

All 9 acceptance criteria are satisfied, evidenced by
`evidence/runs/20260821-041-wi238-upstream-findings-recording.json`. Findings F-015
(updated) and F-017 (new) are in `research/findings.md`; the durable field-evidence
capture is `research/captures/016-forgewire-wi238-3-0-0-field-evidence.md`; the two
proposed implementation work items are `042` and `043`; the narrow work-item-037
cross-reference is in that item's README with no criterion satisfied and no status
change. `repopact validate`, `dashboard`, `spec`, `check-frozen`, 20/20 conformance, and
the full 138-test unit suite (2 legitimate skips) all pass. No RepoPact implementation
code, release, or ForgeWire file was touched.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
