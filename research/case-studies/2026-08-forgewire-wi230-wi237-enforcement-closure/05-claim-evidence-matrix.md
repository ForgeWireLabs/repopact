# 05 — Claim/Evidence Matrix (Phase 5)

Classifies each claim from `01-paper-claims.md` against the ForgeWire evidence
in `04-forgewire-case-timeline.md`. Categories: SUPPORTS, PARTIALLY SUPPORTS,
NARROWS, CONTRADICTS, OUT OF SCOPE, INCONCLUSIVE.

## C1 — Checkpoint-based, not runtime-gated composition

**Classification: SUPPORTS, with a scope clarification.**

The paper never claims the checkpoint fires on any particular cadence — it
explicitly says enforcement is at "validation, CI, review, and generated
reports," leaving *when* those run outside the model. WI230's 269-error
accumulation is fully consistent with C1 as literally stated: no invalid
*commit-boundary* rejection ever occurred because no checkpoint ever ran, and
the moment a checkpoint *did* run (WI230's own closeout, then the WI236/237
session), it correctly reported every violation. C1 is not contradicted. But
this is a case where the claim's truth is compatible with an outcome (269
accumulated errors, silent for at least 34 days per the GA-1→WI230 gap) that
most readers would intuitively read the paper's abstract-level framing
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
0031) makes the same point from RepoPact's own first-party operation, not
just from ForgeWire: RepoPact's `main` has no branch protection and its own
governance workflow was non-functional for over a month, so RepoPact's *own*
repository was, by the same criterion, not actually discharging T5's `[ci]`
tag during that period either. This is not a contradiction of the theorem's
logic (T5 is conditionally true: *if* the checkpoint runs, it correctly
decides `sk ∈ R`); it narrows what `[ci]` should be understood to certify.
**Recommendation:** the discharge-tag taxonomy (`[def]/[ci]/[fix]/[conj]`)
would benefit from distinguishing "checked whenever invoked" from "guaranteed
to be invoked" — these are currently conflated under `[ci]`.

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

H12 and S5's mutation set (M1–M15) model drift as: *a mutation happens, then
we measure whether/how-fast the next validator run catches it.* This
presupposes a validator run happens on some cadence close enough to the
mutation for "latency in edits/commits" to be a meaningful unit. WI230's
incident is not well-described by that frame: there was no meaningful
run-cadence to measure latency against — the validator was simply outside the
loop that produced ordinary architecture work for (at minimum) the 34 days
between GA-1 (2026-07-15, ForgeWire at 39 errors) and WI230's start
(2026-08-18, repo-wide at 297 errors before WI230's own partial repair). This
is a *volume and duration* phenomenon (many independent violations compounding
across many records over an extended period with zero checkpoint invocations)
that M1–M15's per-mutation, single-event framing does not model. It is the
closest real-world instance of mutations M4/M5/M7/M9 (the harness's own
labeled "blind spots") that this case study found, but at a scale (269–297
simultaneous violations, ~64% later attributable to a distinct false-positive
mechanism) the mutation set's per-event scoring model was not designed to
characterize. **This narrows H12/S5 rather than falsifying it**: RepoPact's
silent-staleness rate, measured the way S5 measures it (per discrete
mutation), may well beat convention files — but the incident this case study
examines is evidence about a *different, currently unscored* dimension: total
duration a governed-but-uninvoked repository can silently diverge, and how
far it can diverge, before any checkpoint fires. See `08-pactbench-coverage-
gap.md` for the concrete gap this identifies in the benchmark's task
inventory.

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

## The "specified / executable / invoked / effective" question

The user's prompt asks whether current RepoPact theory collapses these four
distinct states. Based on the corpus read for this case study:

- **Governance specified** — a declared invariant, scope, or gate exists as a
  typed record (`governance/invariants.json`, CI workflow files, etc.). This
  is L0's record store.
- **Governance executable** — a mechanism exists that *could* check the
  specification (the validator binary, `check-frozen`, `doctor` are all
  executable). RepoPact's reference implementation guarantees this for
  everything in L2/L3's typed lattice.
- **Governance invoked** — the executable mechanism is actually run, on some
  occasion, by something (a human, a CI step, a pre-commit hook).
- **Governance effective** — invocation actually gates the outcome (a failing
  check blocks a merge/commit/release), as opposed to being invoked and
  ignored, or invoked with no enforcement teeth (e.g., CI runs but nothing
  requires it to pass).

**Finding: the model's vocabulary does distinguish *specified* from
*executable*** (this is exactly L0 vs. L2/L3, and the typed enforcement
lattice's whole point — some invariants are validator-decidable, some need a
diff, some need human review; §3.4/§5 of `formal-model.md`/`paper.md`). It
does **not** have a named layer, predicate, or invariant for **invoked** vs.
**effective** as distinct from **executable**. The closest the corpus comes
is the checkpoint-based composition language in `formal-model.md` §3
("Composition is checkpoint-based rather than precondition-based... the
checkpoint admits `sk` iff `sk ∈ R`") — but this describes what happens *at*
a checkpoint, taking the checkpoint's occurrence as given, rather than
modeling whether a checkpoint occurs at all, on what cadence, or whether its
result is binding. RepoPact's own work item 032 / decision 0031 is a live,
first-party demonstration that this is not merely an academic distinction:
its four acceptance criteria are effectively "AC-1: someone invokes/provisions
the checkpoint; AC-2: it runs on every change; AC-3: it is *required*
(effective), proven by a negative test; AC-4: don't claim more than what's
actually true" — i.e., RepoPact's own engineering team has, independently,
had to decompose exactly *invoked* and *effective* as separate acceptance
criteria from *executable*, in order to state what "restoring enforcement"
actually requires. **This case study's answer: the current theory does not
yet collapse specified/executable, but it does not yet have first-class
vocabulary separating invoked from effective from executable either — RepoPact
Engineering has needed that distinction operationally (WI-032) without it
yet appearing in the formal model or paper.**

## The bootstrap / enforcement-closure question

"RepoPact governs declared CI gates, but if CI does not invoke RepoPact, what
guarantees that RepoPact itself participates in the governed work loop?"

Based on direct inspection (`02-repopact-2.2-enforcement-model.md`): **this is
an undocumented boundary condition, not an explicitly modeled assumption, not
a solved problem, and not something the corpus claims to solve.**

- It is not *explicitly modeled*: no `I_*` predicate, invariant, or theorem in
  `formal-model.md` states or discharges "a checkpoint invoking the validator
  occurs within bound `Δ` of any edit" or similar. The closest adjacent
  language ("RepoPact does not require this to be enforced as a runtime gate"
  — `paper.md` §3.2) describes the *absence* of a runtime precondition as a
  deliberate design choice, not a treatment of the invocation-guarantee
  question.
- It is not *explicitly assumed external* in the sense of being named and
  set aside (contrast: L5's external-ingestion gap, which §7.4 of the paper
  names directly and repeatedly as a stated, acknowledged limit). No
  equivalent explicit statement exists for "who/what guarantees the
  checkpoint runs."
- It is a **genuine design/theoretical gap that RepoPact's own engineering
  practice has already run into and is actively working**, evidenced
  concretely by work item 032 / decision 0031 in RepoPact's own repository.
  That RepoPact needed to open a dedicated, still-`blocked` work item to
  answer "what guarantees our own checkpoint actually gates our own repo" is
  itself strong evidence that this is not a solved or merely-assumed
  question — it is live, unresolved, and now doubly evidenced (once in
  ForgeWire, once in RepoPact's own repository).

**Recommendation for Phase 10**: this deserves a named primitive — provisionally,
*enforcement closure*: the property that a checkpoint capable of deciding
`s ∈ R` is guaranteed to actually run, on a bounded cadence, with a binding
consequence, for every change that could move the repository out of `R`.
RepoPact's kernel (L0–L5) currently assumes enforcement closure rather than
modeling or guaranteeing it. ForgeWire's WI230 incident and RepoPact's own
WI-032 are two independent field demonstrations that enforcement closure does
not hold automatically merely because a repository is "RepoPact-governed" in
the L0 sense.
