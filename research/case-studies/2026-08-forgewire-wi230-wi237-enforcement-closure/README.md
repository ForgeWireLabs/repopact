# Case study: ForgeWire WI230 → WI237, and the "enforcement closure" gap

A naturalistic (not pre-registered) field observation of RepoPact's
progenitor/reflexive adopter, ForgeWire, drifting to 269–297 unreconciled
`repopact validate` errors while fully governed and while its test suite
stayed green, followed by a deliberate intervention (WI236/237) that
reconciled the drift to zero and built a canonical local-CI runner. Produced
across 11 documents (`00`–`10`), each phase answering a specific question set
out in the commissioning brief; read `10-preliminary-conclusions.md` first
for the synthesis, or `00-evidence-freeze.md` for the raw git state each
subsequent document builds on.

**Status: preliminary, not yet reviewed or incorporated into the paper,
findings register, or formal model.** No claim in this case study should be
cited as an accepted RepoPact finding until the owning maintainer(s) review
it, per the process this case study itself recommends in `10`.

## Headline findings

1. **Enforcement closure is not currently a modeled property.** RepoPact's
   kernel distinguishes governance that is *specified* from governance that
   is *executable*, but has no named primitive for governance that is
   *invoked* (actually run) or *effective* (actually binding) as distinct
   from executable. WI230's drift (39 errors on 2026-07-15, per RepoPact's
   own `gap-audit-2026-07.md` GA-1; 297 by 2026-08-18) accumulated entirely
   because the validator was never wired into ForgeWire's CI, not because the
   validator failed when run. See `05-claim-evidence-matrix.md`'s discussion
   of the specified/executable/invoked/effective distinction, and the
   proposed `enforcement closure` primitive in `10`, item 7.
2. **RepoPact's own repository independently exhibits the identical gap.**
   `origin/main` has no branch protection (`404 Branch not protected`,
   reconfirmed live via `gh api` while writing this case study) and its
   Governance-validation workflow has been failing in single-digit seconds on
   every push for weeks (billing lock; reconfirmed via `gh run list`). Work
   item `032` / decision `0031` — still `blocked`, all four acceptance
   criteria `pending` — is RepoPact's own first-party attempt to close this
   for its own repository. See `03-version-delta.md`.
3. **The worktree-walk false-positive class (171 of ForgeWire's 269 errors,
   ~64%) was already fixed upstream** (`0096d70`, PR #7, merged 2026-07-28)
   before this incident — ForgeWire's pin had simply not moved past 2.2.0.
   This reclassifies most of the incident's error volume as a *version-
   currency* gap rather than a standing design gap, though the fixed
   mechanism (a literal directory-name allowlist) is narrowed, not closed
   structurally: a worktree under any other name reproduces it in both 2.2.0
   and current `origin/main`. See `03-version-delta.md`'s correction note.
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
