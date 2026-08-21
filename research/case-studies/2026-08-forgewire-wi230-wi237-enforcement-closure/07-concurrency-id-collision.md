# 07 — The WI236 Concurrent Work-Item ID Collision (Phase 7)

## What happened (from `04-forgewire-case-timeline.md` item 14)

While this session's WI236 ("Canonical Local CI and RepoPact Enforcement")
was being developed locally on top of `main`, a separate process
independently committed and pushed `cb91fea6`/`fa78d397` ("Propose WI236
causal state runtime substrate" — an unrelated item) to `origin/main` under
the identical id, 236. Neither process was aware of the other's work at the
time each ran `repopact new work-item ...`. The collision surfaced only at
`git push` time (`git status -sb` reporting "ahead 4, behind 2"), i.e., after
both work items had already been fully authored, committed, and (on the
other side) pushed.

## Would RepoPact 2.2.0 detect duplicate IDs after merge?

**Yes, definitively — but only after both directories coexist in the same
tree.** `validate_repo.py`'s `validate_work` (confirmed at
`validate_repo.py:349` in the installed 2.2.0 package) maintains a `seen: dict`
of ids as it iterates `discover_work_items`, and reports `"duplicate id also
used by {seen[item.item_id]}"` the moment two work items share an id in one
scan. Had this session merged `origin/main`'s `236-causal-state-runtime-
substrate/` directory alongside a *still-236-numbered* local directory
(rather than renumbering first), `repopact validate` would have caught it
immediately on the next run, with a precise, unambiguous diagnostic. This is
confirmed by direct source inspection, not merely inferred from schema
design.

## Does RepoPact provide any pre-merge reservation/allocation mechanism?

**No — confirmed absent, not merely undocumented, in both 2.2.0 and current
dev HEAD** (`03-version-delta.md`). `new.py`'s `_next_numeric` (identical
logic in both versions) computes the next id as
`max(existing numeric prefixes in (root/"work").glob("*/*/work-item.json")) + 1`,
scanned from **the local filesystem at the moment `new` is invoked**. There is
no `git fetch`, no query against `origin`, no server-side counter, no file
lock, no distributed-consensus mechanism, and no "reserve an id" verb in the
CLI surface (`init, adopt, import-plan, new, validate, dashboard, spec,
check-frozen, doctor` — confirmed against both the 2.2.0 CLI dispatcher and
current dev HEAD's `cli.py`). Two agents, each starting from a local view of
`main` that does not yet contain the other's not-yet-pushed commit, will
independently and correctly compute the same "next free" id from their own
honest, current-at-the-time view. This is not a bug in the sense of
incorrect logic — the function does exactly what a single-actor, single-
clone workflow needs — it is an absent concern for the multi-clone,
multi-agent case the paper's own S3 (multi-agent coordination, H10) and the
kernel thesis ("the repository as the shared, durable substrate that lets
independent agents coordinate," `benchmark-protocol.md` S3) explicitly claims
to address.

## Is the collision a Git coordination issue outside RepoPact's scope, or a RepoPact gap?

**Both, and the distinction matters.** Git itself did exactly what it is
supposed to do: it refused the naive fast-forward push (local was behind by
2 commits) and required a fetch/merge/rebase, which is the standard,
correct, general-purpose collision-avoidance primitive Git provides for
*any* concurrent-edit conflict, RepoPact-specific or not. Git's mechanism
caught that *something* had diverged; it has no idea that the divergence was
specifically a RepoPact work-item id collision, nor could it — Git operates
on file content and paths, not RepoPact's record semantics, and the two
work-item directories had *different* slugs
(`236-canonical-local-ci-and-repopact-enforcement` vs.
`236-causal-state-runtime-substrate`), so there was no path-level conflict
for Git to flag either — a plain `git merge` of the two branches would have
succeeded cleanly at the Git level, landing both directories side by side,
each internally well-formed, and it was **only** `repopact validate`
(a RepoPact-specific semantic check, run deliberately by this session before
committing the merge) that would have caught the resulting duplicate id.
Git's generic conflict machinery is therefore necessary but not sufficient:
it will not surface a RepoPact-specific id collision when the two work items
happen to use different directory slugs (as they did here), which is the
common case (`new` slugifies the *title*, and two independently-chosen
titles for the same id are generally different strings). The gap that
matters is specifically RepoPact's: **id allocation has no cross-clone
coordination, so its own core record-identity invariant (`I_ID`: unique ids
per record type) is only checked reactively, after the fact, and only if
someone thinks to run `validate` after a merge** — which this session did,
by habit built during the WI236/237 work itself, not because anything
required it.

## Does concurrent-agent durable work suggest RepoPact needs stronger ID allocation semantics?

**Yes, on the evidence of this one incident**, with the important caveat that
this is a single data point (T2/T3 in `09-threats-to-validity.md` apply
directly). The kernel thesis is explicitly that the repository is "the
shared, durable substrate that lets independent agents coordinate"
(`paper.md` §7.1; `benchmark-protocol.md` S3/H10). A coordination substrate
whose most basic identity-allocation primitive (assigning a new work item a
number) has no cross-agent awareness is coordinating *despite* a real gap in
exactly the primitive the thesis rests on, not *because* the primitive
handles it. This case study does not attempt to design the fix (out of
scope per the phase instructions) but notes the shape of the options a
future work item would need to weigh: a centralized/remote id-issuing
service (adds an external dependency the repository-native philosophy
otherwise avoids); a collision-tolerant id scheme (e.g., content-addressed
or slug-primary identifiers, sidestepping numeric collision entirely, at the
cost of the existing numeric-ordering convention); or a lighter-weight
mitigation — `new` performing a `git fetch` and warning if `origin`'s HEAD
has moved since the last local fetch, without fully solving distributed
allocation. This is exactly S3/H10's territory (`benchmark-protocol.md`):
"conflicting/clobbering edits; duplicated work; scope-collision rate" are
already the S3 metrics, and a work-item id collision is a specific,
concrete instance of a "scope collision" that the pre-registered study
does not yet appear to explicitly enumerate as a scored mutation (see
`08-pactbench-coverage-gap.md`).
