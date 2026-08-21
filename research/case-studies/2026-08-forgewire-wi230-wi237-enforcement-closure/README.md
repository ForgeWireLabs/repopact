# Case study: ForgeWire WI230 → WI237, and the "enforcement closure" gap

A naturalistic (not pre-registered) field observation of RepoPact's
progenitor/reflexive adopter, ForgeWire, accumulating a repository-level
`repopact validate` reported-error count in the hundreds while fully
governed and while its test suite stayed green, followed by a deliberate
intervention (WI236/237) that reconciled the reported count to zero and
built a canonical local-CI runner. Produced across 11 documents (`00`–`10`),
each phase answering a specific question set out in the commissioning
brief; read `10-preliminary-conclusions.md` first for the synthesis, or
`00-evidence-freeze.md` for the raw git state each subsequent document
builds on.

**Status: preliminary, reviewed once for internal consistency
(`11-maintainer-review.md`), not yet incorporated into the paper, findings
register, formal model, PactBench, or the RepoPact implementation.** No claim
in this case study should be cited as an accepted RepoPact finding until the
owning maintainer(s) complete the promotion review `11` recommends.

**Terminology note, read before the headline findings below.** This case
study distinguishes four things that earlier drafts conflated: *reported
RepoPact validation errors* (the raw count `repopact validate` prints),
*version-specific validator false positives* (reported errors caused by a
defect in the validator itself, not by anything wrong in ForgeWire's
records — see finding 3), *confirmed governance discrepancies* (reported
errors independently traced to a real problem in a governed record, each
with its own specific repair), and *WI230-local confirmed governance
errors* (the subset of reported errors WI230's own closeout evidence
attributes to its own work-item record specifically). "269–297" below always
means the first of these unless a finding explicitly narrows it.

## Headline findings

1. **Enforcement closure is not currently a modeled property.** RepoPact's
   kernel distinguishes governance that is *specified* from governance that
   is *executable*, but has no named primitive for an admission boundary
   having *checkpoint coverage* (every governed path routes through the
   checker), *checkpoint invocation* (the checker actually executes), or
   *checkpoint effectiveness* (a failing result actually blocks the
   promotion) — three properties this case study found fail independently
   in the field. ForgeWire's reported-error count (39 on 2026-07-15 per
   RepoPact's own `gap-audit-2026-07.md` GA-1; 297 by 2026-08-18) accumulated
   through a **checkpoint-coverage** failure specifically: ForgeWire's CI ran
   on every push but no workflow step ever called the RepoPact CLI. This is
   one of at least three independently-contributing factors this case study
   separates (the other two are covered in findings 2 and 3 below); it is
   not offered as the sole cause of the full reported-error volume. See
   `05-claim-evidence-matrix.md`'s enforcement-closure definition and its
   coverage/invocation/effectiveness decomposition, and `10`, items 7–8.
2. **RepoPact's own repository independently exhibits the same higher-level
   enforcement-closure failure, through a different mechanism.**
   `origin/main` has no branch protection (`404 Branch not protected`,
   reconfirmed live via `gh api` while writing this case study — a
   **checkpoint-effectiveness** failure) and its Governance-validation
   workflow has been dispatching and failing in single-digit seconds on
   every push for weeks due to a billing lock (a **checkpoint-invocation**
   failure; reconfirmed via `gh run list`) — coverage itself is present
   (the workflow does call the validator). This is a different mechanism
   from ForgeWire's coverage failure, not "the identical gap." Work item
   `032` / decision `0031` — still `blocked`, all four acceptance criteria
   `pending` — is RepoPact's own first-party attempt to close this for its
   own repository. See `03-version-delta.md`.
3. **The worktree-walk false-positive class (171 of ForgeWire's 269
   *reported* errors at the WI237 starting state, ~64% of that count) was
   already fixed upstream** (`0096d70`, PR #7, merged 2026-07-28) before this
   incident — ForgeWire's pin had simply not moved past 2.2.0. This
   reclassifies most of that specific count as a *version-currency* gap
   rather than a confirmed governance discrepancy or a standing RepoPact
   design gap — a third, independent contributing category alongside
   enforcement closure (finding 1) and the remaining confirmed discrepancies
   (`04-forgewire-case-timeline.md` item 11). The fixed mechanism (a literal
   directory-name allowlist) is narrowed, not closed structurally: a
   worktree under any other name reproduces it in both 2.2.0 and current
   `origin/main`. See `03-version-delta.md`'s correction note.
4. **A work-item README's heading can silently disagree with its own
   manifest's `id`**, invisibly to `repopact validate` in both 2.2.0 and
   current `origin/main` — reproduced in an isolated throwaway fixture, not
   by relying on inspection alone. Preserved live in ForgeWire on purpose
   (`work/completed/237-.../README.md` still reads "# 236 —"). See
   `06-representation-drift.md`.
5. **Work-item id allocation has no cross-branch coordination**, confirmed
   identical between 2.2.0 and `origin/main`: two agents on two branches will
   independently compute the same "next free" id and can only be caught by
   `repopact validate` *after* both are merged into one tree — which is
   exactly what happened between this incident's own WI236 and an unrelated,
   concurrently-created WI236 on `origin/main`. See
   `07-concurrency-id-collision.md`.
6. **Six of eight incident features this case study exhibited have no
   PactBench task or drift-harness mutation.** See
   `08-pactbench-coverage-gap.md` for the scored breakdown and concrete,
   cheaply-buildable candidates.

## Index

| File | Phase | Content |
| --- | --- | --- |
| `00-evidence-freeze.md` | 1 | Exact git state, all four repos, at capture time |
| `01-paper-claims.md` | 2 | Every relevant paper/model claim, with support/falsification criteria |
| `02-repopact-2.2-enforcement-model.md` | 3 | What 2.2.0 actually enforces, by direct source read |
| `03-version-delta.md` | 3 | 2.2.0 vs. `origin/main`, corrected after an initial stale-branch error |
| `04-forgewire-case-timeline.md` | 4 | The incident, hard-evidence vs. inference marked throughout |
| `05-claim-evidence-matrix.md` | 5 | SUPPORTS/NARROWS/CONTRADICTS/OUT-OF-SCOPE per claim |
| `06-representation-drift.md` | 6 | The README/manifest id mismatch, reproduced in isolation |
| `07-concurrency-id-collision.md` | 7 | The WI236 collision, mechanism and scope |
| `08-pactbench-coverage-gap.md` | 8 | Proving Ground coverage of this incident's features |
| `09-threats-to-validity.md` | 9 | Why this case is uncontrolled, and why it's still worth having |
| `10-preliminary-conclusions.md` | 10 | Synthesis, answering the commissioning brief's 11 questions |
| `11-maintainer-review.md` | review | Correction pass: fixes, retained/narrowed claims, revised definition, promotion candidates |
