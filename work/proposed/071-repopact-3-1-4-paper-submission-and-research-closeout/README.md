# 071 — RepoPact 3.1.4 paper submission and research closeout

> **Status**: 📋 Proposed
> **Owners**: governance-owner (lead).
> **Depends on**: [`070`](../../active/070-repopact-3-1-3-downloadable-workbench-installers/) (paper should cite the 3.1.3-or-later release-candidate tree), [`063`](../../active/063-durable-repository-orientation-graph-and-incremental-semantic-index/) (ROG-028/034/038 close out here).

## Intent

Two independent threads that share a release train:

1. **arXiv submission.** `research/arxiv/` is built and current but the
   durable record has said `ARXIV NOT SUBMITTED` since the paper existed.
   This item prepares a submission-ready package against the 3.1.3-or-later
   tree and hands the actual submission (account, endorsement, final
   approval, upload) to the operator — that step cannot be agent-performed.
2. **WI063 closeout.** 37 of 40 ROG acceptance criteria are already
   satisfied. The remaining three (responsive Workbench layout, the R1-vs-S8
   comparative evaluation, and closeout evidence enumeration) are what stand
   between WI063 and `work/completed/`.

Out of scope: WI068 (embedded mobile Git backend) — unstarted, unrelated
feature work, not release hygiene.

## Decisions

- The paper refresh happens against 3.1.3 (or later), not 3.1.2, so the
  installer-download story WI070 ships is reflected in the paper rather than
  citing a package surface that changed the week before submission.
- ROG-034's comparative evaluation must show correctness parity or
  improvement alongside any search-cost reduction; a cost win with a
  correctness regression is not an acceptable result to publish.

## Scope

- `research/arxiv/` (main.tex, references.bib, figures/, main.pdf).
- `research/paper.md` (source of truth the arXiv package is generated from).
- `work/active/063-.../work-item.json` (ROG-028/034/038 → satisfied, then
  status → completed and directory moved).
- `work/active/066-.../README.md`, `work/active/021-public-launch/` (additive
  reconciliation only, per Decision precedent set in work item 069).
- `evidence/runs/` — comparative-evaluation results, closeout evidence,
  arXiv submission confirmation.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
