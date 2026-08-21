# 11 — Maintainer Review / Correction Pass

A review pass over `00`–`10`, requested and scoped by the maintainer after
the preliminary case study (commit `279b714`) was accepted as the
evidence-gathering pass. Scope of this pass: `research/case-studies/
2026-08-forgewire-wi230-wi237-enforcement-closure/**` only. No changes were
made to `research/paper.md`, `research/formal-model.md`,
`research/findings.md`, PactBench, the RepoPact implementation, or
ForgeWire.

## 1. Corrections made

- **Error-count characterization, throughout.** The prior draft's
  "269–297 genuine violations," "none of which turned out to be a false
  accusation," and "drifted to 269–297 governance errors" language conflated
  four distinct things that are now kept separate everywhere in the case
  study: *reported RepoPact validation errors*, *version-specific validator
  false positives*, *confirmed governance discrepancies*, and *WI230-local
  confirmed governance errors*. The prior "~98 genuine errors" figure is
  retained, but is now explicitly qualified as resting on the itemization in
  commit `e02aec1e` and the fact that `repopact validate` reported zero
  errors after those specific named repairs — not on an independent
  per-error re-audit performed within this case study. See
  `04-forgewire-case-timeline.md` items 8–11 (rewritten), and the parallel
  correction in `01-paper-claims.md` C5/C6, `05-claim-evidence-matrix.md`
  C1/C5/C6, `10-preliminary-conclusions.md` items 1–2, and the `README.md`
  headline findings and terminology note.
- **Single-cause language removed.** No remaining passage attributes the
  full reported-error volume to any one cause. Three independently-
  contributing categories are now named everywhere the volume is discussed:
  an enforcement-closure failure (real discrepancies could persist because
  RepoPact was outside the admission loop), a version-currency failure
  (ForgeWire's pin lagged an upstream fix), and — as its own, separate,
  already-corrected finding — a validator-coverage defect (the 2.2.0
  worktree-walk false positive itself).
- **The RepoPact-own-repository comparison** no longer says "identical gap."
  Every instance now says "the same higher-level enforcement-closure failure
  class through a different mechanism," and spells out the mechanism
  difference explicitly: ForgeWire = checkpoint-coverage failure only
  (hosted CI functioned, never invoked RepoPact); RepoPact's own repository
  = checkpoint-invocation failure (billing-locked Actions) *and*
  checkpoint-effectiveness failure (no branch protection), with coverage
  itself present. See `01-paper-claims.md` C2, `03-version-delta.md`,
  `05-claim-evidence-matrix.md`, `10` item 2, `README.md` finding 2.
- **`03-version-delta.md`'s duplicated work-item-id-allocation bullet**
  removed; the two near-duplicate entries (one under the original
  "Unchanged" heading, one added during the earlier mid-session correction)
  are merged into one, confirmed directly against `origin/main`.
- **General sweep** for stale statements left over from the earlier
  mid-session `origin/main` correction: none found beyond the items above
  (checked every file for `entirely because`, `identical gap`/`identical
  class`, `269–297`/`269-297`, `false accusation`, `genuine violations`, and
  cross-checked the exact three banned strings verbatim — zero matches
  remain).

## 2. Claims retained as originally stated

- **C1** (checkpoint-based, not runtime-gated composition) — SUPPORTS,
  unchanged. This case study continues to find C1 formally sound; the
  correction sharpens *what accumulated and why* (three separate
  categories), not whether C1 itself holds.
- **C3** (`I_derive_dash`, the one-tree dashboard fixpoint) — SUPPORTS,
  strongly, unchanged. No part of the correction pass touched this finding;
  it remains the strongest positive result in the case study.
- **C4** (concrete-record adoption trilemma / provenance typing) — OUT OF
  SCOPE, unchanged.
- **C7** (F-011/GA-1 longitudinal drift) — SUPPORTS the finding's honesty,
  PARTIALLY SUPPORTS its completeness, unchanged. The open question (whether
  `doctor` was run and failed to hold, or never re-run, between GA-1 and
  WI230) remains genuinely unresolved and is repeated below.
- **Representation drift** (`06`) — the live ForgeWire README `236` /
  manifest `237` mismatch remains **unfixed**, exactly as instructed. The
  finding that RepoPact has targeted parity checks for some duplicated
  narrative facts (decision 0014, decision 0028) but no general policy is
  retained without a prescribed final implementation — `06` presents two
  possible directions (a narrow fixpoint at validate/dashboard time, or a
  further narrow convention-gated decision following the 0014/0028
  precedent) as *options for future work*, not a recommendation to build
  either now.
- **Concurrent ID allocation** (`07`) — retained exactly: duplicate ids are
  detected once both records coexist in one tree; RepoPact has no pre-merge,
  cross-branch allocation/reservation mechanism. No conclusion that RepoPact
  should become a distributed ID allocator is drawn; `07`'s own text
  explicitly lists that as one of several unweighed options a future work
  item would need to evaluate, not a recommendation.

## 3. Claims narrowed by this pass

- **C2 / T5's `[ci]` tag** — classification stays NARROWS, but the
  narrowing is now stated in terms of three composable properties
  (coverage, invocation, effectiveness) rather than a single vague
  "invoked vs. guaranteed to be invoked" pairing. T5's conditional admission
  logic itself is not violated by any evidence this case study found — only
  what `[ci]` should be read to certify is narrowed.
- **C5 / H12 and the S5 drift harness** — classification stays NARROWS, now
  decomposed into three explicit axes: detection efficacy conditional on
  invocation (what M1–M15 measure well), invocation latency or absence
  (partially instrumented, no scenario exercises it), and accumulated
  reported-error volume over an extended zero-invocation window (not
  modeled at all). The benchmark's current strength is squarely on the first
  axis; this case study's evidence bears on the other two.
- **C6 / F-008 vs. the worktree false positive** — already corrected in the
  preliminary pass, reconfirmed and tightened here: the worktree-walk defect
  is a version-currency finding for ~64% of the WI237-starting-state
  reported count, not a standing RepoPact design gap, though the underlying
  mechanism (literal directory-name allowlist, not genuine git-awareness) is
  narrowed rather than closed for any other directory name.

## 4. Revised enforcement-closure definition

Superseding the preliminary pass's "guaranteed to actually run, on a bounded
cadence" phrasing (which read as in tension with RepoPact's checkpoint-based
design), the definition used throughout `05` and `10` as of this pass is:

> **Enforcement closure** is the property that every transition promoting
> repository state across a governed admission boundary is necessarily
> evaluated by the applicable checkpoint, and a nonconformant state cannot
> cross that boundary merely because the checkpoint was absent, unavailable,
> ignored, or misconfigured.

Composed of three independently-failable sub-properties:

- **A. Checkpoint coverage** — every governed admission path routes through
  the applicable checker.
- **B. Checkpoint invocation** — the checker actually executes for the
  candidate state, given coverage.
- **C. Checkpoint effectiveness** — a failing result actually prevents the
  governed promotion.

Checkpoint *availability* (whether the infrastructure a checkpoint depends
on is up at all) is treated as a possible *cause* of an invocation failure,
not a fourth independent axis, since a coverage or effectiveness failure can
occur even with perfect availability. The definition is deliberately
substrate-neutral: GitHub branch protection is one possible mechanism for
effectiveness, not the definition of it; a repository-local canonical
runner, self-hosted CI, a release gate, or a future distributed-runner
admission check could each supply coverage, invocation, or effectiveness
through different substrates. Full derivation, and the worked ForgeWire
(coverage failure) vs. RepoPact-own-repository (invocation + effectiveness
failure) contrast, are in `05-claim-evidence-matrix.md`.

## 5. Unresolved questions

Carried forward from the preliminary pass, none closed by this review (this
was a correction pass, not a further evidence-gathering pass):

- Whether `doctor` was run and failed to hold, or was never re-run at all,
  between GA-1 (2026-07-15, 39 reported errors) and WI230's start
  (2026-08-18, 297 reported errors). This remains the single most valuable
  follow-up question — it determines whether the recurrence is primarily a
  `doctor`-effectiveness finding or a pure invocation-gap recurrence.
- Whether ForgeWire's 39-error (2026-07-15) and 297-error (2026-08-18)
  snapshots contain the same worktree-walk false-positive proportion found
  at the 269-error WI237 starting state (~64%), or a different mix — not
  independently verified at those earlier snapshots by this case study.
- Whether a second, unrelated (non-ForgeWireLabs) adopter's repository
  exhibits the same coverage/invocation/effectiveness failure pattern —
  needed to address T1's reflexivity threat for the enforcement-closure
  finding specifically, since both current instances (ForgeWire, RepoPact's
  own repository) are ForgeWireLabs-controlled.
- Whether ForgeWire's post-WI237 canonical-CI discipline holds over a
  duration comparable to the pre-WI237 drift window (34+ days) — the case
  study captured only a single point-in-time clean closeout.

## 6. Recommended promotion targets (not performed in this pass)

Presented as candidates for the owning maintainer(s) to weigh, in
ascending order of how much the corpus itself would need to change:

- **`findings.md`** — the worktree-walk false positive (now understood as a
  version-currency finding, already fixed on `origin/main`) and the
  work-item-README representation-drift gap are each shaped like
  register-ready findings (comparable in form to F-008/F-011): a specific,
  reproduced defect class with a clear before/after. The enforcement-closure
  observation itself is broader than a single finding and likely belongs
  as a synthesis rather than a single F-0XX entry.
- **`formal-model.md`** — the coverage/invocation/effectiveness
  decomposition is the most concrete candidate for a new named property
  (provisionally `L2.5` or a cross-cutting predicate set) alongside the
  existing L0–L5 layers; the T5 `[ci]` discharge-tag ambiguity this case
  study surfaced (checked-when-invoked vs. guaranteed-to-be-invoked) is a
  candidate for a footnote or tag refinement even independent of adopting
  the new property.
- **`paper.md`** — H12/S5's framing could be narrowed to state explicitly
  what it does and does not measure (detection efficacy conditional on
  invocation, not invocation latency or accumulated-drift volume); T1's
  reflexivity threat gains a second, independent (though still
  ForgeWireLabs-internal) instance worth citing (RepoPact's own WI-032).
- **PactBench / drift harness** — the concrete, low-effort candidates
  identified in `08-pactbench-coverage-gap.md`: a work-item-README
  heading/id mismatch mutation (directly modeled on the reproduced fixture
  in `06`), an S3 fixture for the two-agent id-collision scenario (directly
  modeled on `07`), a longitudinal zero-invocation drift scenario distinct
  from M1–M15's per-event framing, and a "checkpoint invoked but ineffective
  for an unrelated reason" scenario (RepoPact's own WI-032 history is
  already a documented real instance that could be adapted).
- **Implementation work** — none recommended for immediate action by this
  case study; `03-version-delta.md` and `07-concurrency-id-collision.md`
  deliberately stop short of prescribing a fix for the id-allocation gap,
  and `06-representation-drift.md` presents options rather than a spec for
  the README-heading gap. Work item 032 / decision 0031 already exists as
  RepoPact's own implementation-track response to the coverage/invocation/
  effectiveness gap for its own repository and needs no new item from this
  case study — only, if the maintainer agrees, a cross-reference to this
  case study as additional field evidence for its urgency.

**No promotion listed above has been performed.** This pass is a
correction/consistency pass over the case-study documents only.
