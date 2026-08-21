# 05 — Claim/Evidence Matrix (Phase 5)

Classifies each claim from `01-paper-claims.md` against the ForgeWire evidence
in `04-forgewire-case-timeline.md`. Categories: SUPPORTS, PARTIALLY SUPPORTS,
NARROWS, CONTRADICTS, OUT OF SCOPE, INCONCLUSIVE.

## C1 — Checkpoint-based, not runtime-gated composition

**Classification: SUPPORTS, with a scope clarification.**

The paper never claims the checkpoint fires on any particular cadence — it
explicitly says enforcement is at "validation, CI, review, and generated
reports," leaving *when* those run outside the model. The accumulation of
`repopact validate`-reported errors during the WI230 window is fully
consistent with C1 as literally stated: no invalid *commit-boundary*
rejection ever occurred because no checkpoint ever ran, and the moment a
checkpoint *did* run (WI230's own closeout, then the WI236/237 session), it
reported every state it examined accurately — including, correctly, a large
class of validator false positives introduced by a version-specific defect
later found already fixed upstream (see C6, `03-version-delta.md`). C1 is
not contradicted. But this is a case where the claim's truth is compatible
with an outcome (a reported error count in the hundreds, silent for at least
34 days per the GA-1→WI230 gap — see `04-forgewire-case-timeline.md` for the
reported-vs-confirmed breakdown) that most readers would intuitively read
the paper's abstract-level framing
("keeps project state as typed, version-controlled records... checked at
commit and CI boundaries") as ruling out. The gap is not in C1's logic; it is
in what a reader infers from "checked at commit and CI boundaries" without
separately being told those boundaries must themselves be wired to invoke the
check. Recommendation for Phase 10: this is a **documentation/framing**
narrowing, not a model defect — C1 is formally sound but reads as a stronger
guarantee than it is.

## C2 — T5 Monitor non-bypass, tagged `[ci]`

**Classification: NARROWS.**

T5's `[ci]` discharge tag ("machine-checked on every run") is the specific
claim this incident bears on most directly. "Every run" of *what* — the
validator, or the CI pipeline? ForgeWire's CI pipeline ran on every push (unit
tests, lint, etc. all executed) but never invoked `repopact validate`. If
`[ci]` means "the validator is checked whenever CI runs," the claim is false
for the entire pre-WI236 ForgeWire history: CI ran constantly, the validator
never ran as part of it. If `[ci]` means "when the validator does run inside
some CI system, that instance of it correctly discharges the check," the
claim survives but says less than a reader expects from the tag "machine-
checked on every run" placed next to a claim about admission at CI
boundaries. The RepoPact-repo evidence (`03-version-delta.md`, WI-032/decision
0031) makes the same higher-level point from RepoPact's own first-party
operation, through a different mechanism than ForgeWire's: RepoPact's `main`
has no branch protection (a checkpoint-*effectiveness* gap, in the vocabulary
defined below) and its own governance workflow has been dispatching and
failing due to a billing lock (a checkpoint-*invocation* gap) — as distinct
from ForgeWire, whose CI ran successfully but was never wired to call the
CLI at all (a checkpoint-*coverage* gap). All three are instances of
enforcement closure not holding, by three different mechanisms. This is not
a contradiction of the theorem's logic (T5 is conditionally true: *if* the
checkpoint runs, it correctly decides `sk ∈ R`); it narrows what `[ci]`
should be understood to certify. **Recommendation:** the discharge-tag
taxonomy (`[def]/[ci]/[fix]/[conj]`) would benefit from distinguishing
"decided correctly whenever the checkpoint actually executes" (what T5/`[ci]`
establishes) from checkpoint coverage, invocation, and effectiveness (none of
which T5 establishes) — see the enforcement-closure definition below.

## C3 — I_derive_dash, one-tree dashboard fixpoint (2.2.0)

**Classification: SUPPORTS, strongly.**

This is the one claim in the corpus that *predicts and matches* the observed
robustness. Because dashboard-staleness detection was moved into the
one-tree validator (no CI dependency), every dashboard-staleness check this
session ran — local, no CI, sometimes minutes after an edit — caught
staleness exactly as designed, repeatedly, across the WI236/237 session and
again in the standalone fixture reproduction for Phase 6. This is direct,
repeated, positive confirmation of a specific, falsifiable claim, and it is
also indirect evidence *for* the broader thesis: the one invariant RepoPact
moved out of the CI-dependency class is the one invariant this incident could
not find a gap in. This is worth stating plainly in any write-up: the fix
that generalizes best is "move enforcement into the one-tree validator," not
"trust CI to invoke the one-tree validator."

## C4 — Concrete-record adoption trilemma / provenance typing

**Classification: OUT OF SCOPE.**

No adoption/migration event occurred during WI230 or WI236/237 — the
repository was already RepoPact-governed. The trilemma concerns bringing a
*naive* tree under the pact; this incident concerns a tree that had already
been brought under the pact and then drifted. The `waived`/disclosed-
retroactive-preflight conventions this session used (and that
`REPOPACT-ADOPTION.md` documents from the original adoption) are consistent
applications of the resolved trilemma's honesty discipline, but they are not
new evidence about the trilemma itself. Recorded as out of scope rather than
"supports," because citing it as support would overstate what this incident
actually tested.

## C5 — H12/S5: drift visibility, "measured honestly against F-011"

**Classification: NARROWS, and identifies a coverage gap in the operational
definition of "drift" itself.**

H12 and S5's mutation set (M1–M15) model drift along one axis: *a mutation
happens, then we measure whether/how-fast the next validator run catches
it* — i.e., **detection efficacy conditional on invocation**. This
presupposes a validator run happens on some cadence close enough to the
mutation for "latency in edits/commits" to be a meaningful unit. WI230's
incident exercises two axes S5 does not currently score:
**invocation latency or absence** (how long, by what mechanism, before
`validate` is run at all against a drifted state — S5's `latency` field
assumes a nonzero run-cadence to measure against, not an extended
zero-invocation window) and **accumulated drift/error volume over time**
(how much divergence, of what kinds, compounds across such a window). There
was no meaningful run-cadence to measure detection latency against during
(at minimum) the 34 days between GA-1 (2026-07-15, ForgeWire independently
found at 39 `repopact validate`-reported errors by RepoPact's own gap-audit
process) and WI230's start (2026-08-18, repo-wide reported errors at 297
before WI230's own partial repair, per its closeout evidence). This is a
*volume and duration* phenomenon that M1–M15's per-mutation, single-event
framing does not model. It is the closest real-world instance of mutations
M4/M5/M7/M9 (the harness's own labeled "blind spots") that this case study
found, but at a scale the mutation set's per-event scoring model was not
designed to characterize — and the reported figure needs its own
decomposition, not a single "N genuine violations" number: of the 269
errors `repopact validate` reported at the WI237 starting state, 171 (~64%)
are a version-specific validator false positive (the worktree-walk defect,
already fixed on RepoPact's `origin/main` three weeks before this incident —
`03-version-delta.md`), while the remainder, plus WI230's own separately
reported 26 record-level errors, are independently confirmed governance
discrepancies (see `04-forgewire-case-timeline.md` for the itemization and
what "confirmed" means for each). **This narrows H12/S5 rather than
falsifying it**: RepoPact's silent-staleness rate, measured the way S5
measures it (detection efficacy conditional on invocation, per discrete
mutation), may well beat convention files — but the incident this case
study examines is evidence about the other two, currently unscored, axes:
how long an invocation gap can persist, and how far reported and confirmed
drift can diverge and accumulate before any checkpoint fires. See
`08-pactbench-coverage-gap.md` for the concrete gap this identifies in the
benchmark's task inventory.

## C6 — F-008 (gitignore swallows evidence) vs. the worktree false-positive mirror

**Classification: NARROWS the fixed defect's scope, and separately supports
a version-currency finding (corrected from this phase's first pass).**

F-008 and its fix (`adopt` running `git check-ignore` on written records) are
squarely about *governed content becoming invisible*. The WI236/237 worktree
finding is the structural mirror: *non-governed content becoming falsely
visible*, via `repo_model.IGNORED_PARTS` having no concept of untracked
content, `.gitignore`, or git-worktree checkouts. Confirmed by direct source
inspection (`02-repopact-2.2-enforcement-model.md`) to be present in the
pinned `2.2.0` — **but already fixed on RepoPact's actual `origin/main`**
three weeks before this incident (`03-version-delta.md`, corrected). This
reclassifies the bulk of this finding: it is not evidence that RepoPact left
the gap open, but evidence of a *version-currency* gap (fix exists, adopter's
pin lagged it) — itself independent supporting evidence for the paper's own
GA-1/`fleet_verify.py` concern about adopters running stale pins. The
underlying mechanism (`IGNORED_PARTS` as a literal path-segment allowlist,
not genuine git-tracked-status awareness) remains narrowed rather than
closed — a worktree under a different directory name would still reproduce
this in *both* 2.2.0 and current `origin/main`, so a residual "brownfield
validator correctly distinguishes governed from ungoverned filesystem
content" gap does still stand, just smaller than the first-pass finding
claimed. 171 of 269 errors (~64%) in this incident came from this mechanism —
a materially large fraction of the total, meaning this is not a minor edge
case for an adopter whose agent tooling uses `git worktree` (a documented,
common agentic-coding
pattern, including this very session's own use of `EnterWorktree`-style
tooling).

## C7 — F-011/GA-1: longitudinal upgrade drift, "recurring in the wild"

**Classification: SUPPORTS the finding's honesty, PARTIALLY SUPPORTS its
completeness.**

GA-1 already, independently and a month earlier, caught ForgeWire drifting
(39 errors, 2026-07-15) and named it explicitly as "the F-011 class recurring
in the wild" — this is a genuinely strong, self-critical instance of the
paper's own falsification discipline (this is not a case of RepoPact hiding
an inconvenient recurrence; its own gap-audit process surfaced it and said so
plainly). It **supports** the paper's claim to honest, ongoing self-
falsification. But GA-1's proposed remedy ("run `doctor` upgrades... capture
the episode... new findings register entries") does not, by the evidence this
case study located, appear to have prevented a *larger* recurrence one month
later (297 errors by WI230's start) — whether `doctor` was run and failed to
hold, or was never run at all, is genuinely **[INFER]**-flagged as unresolved
in `04-forgewire-case-timeline.md` item 6. This matters for classification:
if `doctor` was run and the drift still reaccumulated, that is evidence
narrowing `doctor`'s effectiveness as a standing remedy (it repairs a
snapshot; it does not change the invocation-frequency problem). If `doctor`
was never re-run, that is a pure recurrence of the *invocation* gap (C1/C2),
not a `doctor`-effectiveness finding. This case study cannot discharge which,
and flags it as the single most valuable follow-up question for Phase 10/
future work rather than guessing.

---

## The "specified / executable / invoked / effective" question, refined

The commissioning brief's original four-way question is refined here after
maintainer review, replacing the earlier "invoked vs. effective" pairing with
a three-way split under an explicit admission-boundary definition — the
prior draft's "invoked"/"effective" language is retired in favor of the more
precise terms below.

**Governance specified** and **governance executable** are retained as
originally stated and are not in dispute: L0 (the record store — a declared
invariant, scope, or gate exists as a typed record) is cleanly distinct from
L2/L3 (a mechanism exists that *could* check the specification — the
validator binary, `check-frozen`, `doctor`). The typed enforcement lattice
is precisely the model's treatment of *executable*: it classifies which
invariants are machine-decidable from one tree, which need a diff, which
need human review (§3.4/§5 of `formal-model.md`/`paper.md`).

**Enforcement closure** is the property this case study is really probing,
and is defined here precisely rather than left as "invoked" and "effective"
loosely paired:

> Enforcement closure is the property that every transition promoting
> repository state across a governed admission boundary is necessarily
> evaluated by the applicable checkpoint, and a nonconformant state cannot
> cross that boundary merely because the checkpoint was absent, unavailable,
> ignored, or misconfigured.

This is deliberately **not** a claim that RepoPact runs continuously or on
every edit — that would contradict the model's own checkpoint-based (not
precondition-based) design, which this case study does not dispute (see C1).
Enforcement closure is a property of the *admission boundary* (a commit
landing on a protected branch, a release being published, a work item
transitioning to `completed`), not of every intermediate edit. Three
independent sub-properties compose it, and this incident's evidence shows
each can fail separately:

- **A. Checkpoint coverage** — every governed admission path routes through
  the applicable checker at all. ForgeWire's mechanism was a **pure coverage
  failure**: CI ran successfully on every push, but no workflow step ever
  called `repopact validate`/`check-frozen` — the admission path (merge to
  `main`) had no route through the checker, so invocation and effectiveness
  were never even at issue.
- **B. Checkpoint invocation** — the checker actually executes for the
  candidate state, given that coverage exists. RepoPact's own repository has
  coverage (`.github/workflows/governance.yml` does call the validator) but
  fails **invocation**: the workflow dispatches and is rejected by GitHub
  within seconds due to an account-level billing lock (`03-version-delta.md`,
  live-reconfirmed via `gh run list`).
- **C. Checkpoint effectiveness** — a failing result actually prevents the
  governed promotion. RepoPact's own repository additionally fails
  **effectiveness**, independent of the billing lock: `main` has no branch
  protection (`404 Branch not protected`, live-reconfirmed via `gh api`), so
  even a green — or a correctly-failing — governance run would not itself
  block a merge. Work item 032 AC-3 is, precisely, a request for evidence of
  effectiveness ("a deliberately invalid test branch proves the gate rejects
  drift before merge").

This yields a materially more precise comparison than "identical gap":
ForgeWire's mechanism was A only; RepoPact's own repository's mechanism is B
*and* C, with A already present. Both are enforcement-closure failures at the
same higher level of description; neither is the same failure mechanism as
the other, and conflating them would overstate the parallel.

**Checkpoint availability** (whether the underlying infrastructure a
checkpoint depends on — a CI provider, a runner, network access — is up at
all) is related to but distinct from invocation: RepoPact's billing lock is
best read as an availability failure that *causes* an invocation failure
(the checker cannot execute because its execution environment refuses to
start it), whereas a coverage failure (ForgeWire's case) would persist even
with perfect availability, and an effectiveness failure (RepoPact's missing
branch protection) would persist even with perfect availability and perfect
invocation. Availability is not treated as a fourth independent axis here;
it is one possible *cause* of an invocation failure, kept distinct from
coverage and effectiveness because either of those can fail even when
availability is perfect.

**Substrate neutrality.** GitHub branch protection is one possible mechanism
for effectiveness (C), not the definition of it. A repository-local canonical
runner that developers and agents are disciplined to invoke before every
commit (as ForgeWire's WI237 built), a self-hosted CI system, a release gate
that refuses to publish on a failing check, or a future Fabric-style runner
enforcing admission before a distributed control plane accepts a change,
could each supply coverage, invocation, or effectiveness through entirely
different substrates. This case study does not conclude GitHub-specific
mechanisms (branch protection, required status checks) are the only or the
recommended path — only that *some* mechanism providing all three properties
is what "enforcement closure" requires, and that RepoPact's current model
does not name or require any of them.

**Finding.** RepoPact's own work item 032 / decision 0031 is a live,
first-party demonstration that this three-way distinction is not merely
academic: its four acceptance criteria decompose almost exactly onto
coverage/invocation/effectiveness plus an honesty constraint — AC-1
(provision *a* checkpoint), AC-2 (it runs on every change — invocation),
AC-3 (it is *required*, proven by a negative test — effectiveness), AC-4
(don't claim more than what's actually true). RepoPact's own engineering
team has, independently and operationally, needed to decompose invocation
and effectiveness as separate acceptance criteria from mere executability,
without that distinction yet appearing in the formal model or paper.

## The bootstrap / enforcement-closure question

"RepoPact governs declared CI gates, but if CI does not invoke RepoPact, what
guarantees that RepoPact itself participates in the governed work loop?"

Based on direct inspection (`02-repopact-2.2-enforcement-model.md`): **this is
an undocumented boundary condition, not an explicitly modeled assumption, not
a solved problem, and not something the corpus claims to solve.**

- It is not *explicitly modeled*: no `I_*` predicate, invariant, or theorem in
  `formal-model.md` states or discharges "a checkpoint providing coverage,
  invocation, and effectiveness exists for every governed admission boundary."
  The closest adjacent language ("RepoPact does not require this to be
  enforced as a runtime gate" — `paper.md` §3.2) describes the *absence* of a
  runtime precondition as a deliberate design choice, not a treatment of the
  closure question.
- It is not *explicitly assumed external* in the sense of being named and
  set aside (contrast: L5's external-ingestion gap, which §7.4 of the paper
  names directly and repeatedly as a stated, acknowledged limit). No
  equivalent explicit statement exists for "who/what guarantees the
  admission boundary has coverage, invocation, and effectiveness."
- It is a **genuine design/theoretical gap that RepoPact's own engineering
  practice has already run into and is actively working**, evidenced
  concretely by work item 032 / decision 0031 in RepoPact's own repository.
  That RepoPact needed to open a dedicated, still-`blocked` work item to
  answer "what guarantees our own checkpoint actually gates our own repo" is
  itself strong evidence that this is not a solved or merely-assumed
  question — it is live, unresolved, and now doubly evidenced (once in
  ForgeWire via a coverage failure, once in RepoPact's own repository via
  invocation and effectiveness failures).

**Recommendation for Phase 10**: name the primitive *enforcement closure*
per the admission-boundary definition above, and treat coverage, invocation,
and effectiveness as its three composing, independently-failable
sub-properties. RepoPact's kernel (L0–L5) currently assumes enforcement
closure rather than modeling or guaranteeing it. ForgeWire's WI230 incident
and RepoPact's own WI-032 are two independent field demonstrations that
enforcement closure does not hold automatically merely because a repository
is "RepoPact-governed" in the L0 sense — and that it can fail through
different sub-properties in different repositories, which is itself evidence
that the three-way decomposition, not a single "invoked or not" binary,
is the right level of description.
