# 10 — Preliminary Conclusions (Phase 10)

No edits to `research/paper.md`, `findings.md`, `formal-model.md`, RepoPact
source, or ForgeWire were made to reach these conclusions. This is the
strongest defensible reading of the evidence assembled in `00`–`09`, stated
plainly including where it is unfavorable to RepoPact.

## 1. What did WI230 demonstrate RepoPact CAN do?

When actually invoked, RepoPact's validator correctly and completely
enumerated what it was designed to check, given the record state and its own
implementation at the time. The reported error counts require the
reported/confirmed/false-positive distinction established in
`04-forgewire-case-timeline.md` item 11: of the 269 errors reported at the
WI237 starting state, ~98 were **confirmed governance discrepancies**
(unregistered work directories, missing preflight markers, invalid scope
references, non-concrete evidence citations, one provenance error, one
stale-dashboard error), each independently repaired and traced to a commit —
this is real, positive evidence the validator correctly flags genuine drift
when run. But 171 (~64%) were a **version-specific validator false
positive** (the `.claude/worktrees` walk defect in the pinned 2.2.0, already
fixed on RepoPact's `origin/main` before this incident — `03-version-
delta.md`), not confirmed governance discrepancies at all. The correct
statement is therefore: RepoPact's validator, when invoked, correctly
surfaced ~98 real discrepancies *and* one class of its own version-specific
false positives, in the same reported count, undifferentiated until manual
investigation separated them — itself a finding, not a caveat to bury. WI230's
own closeout evidence (`0 WI230 errors, 26 at the start` — a separate,
WI230-local reported count) shows the validator can be run mid-stream,
on-demand, by an agent choosing to check its own work, and correctly gate
that agent's own closeout — this is C3/C1 working exactly as designed, for
the *portion* of the repository someone chose to check. The dashboard-
staleness fixpoint (`I_derive_dash`, RepoPact 2.2.0's own headline
improvement) worked flawlessly every single time it was exercised in this
incident and in the WI236/237 session, with zero CI involvement required.
This remains real, repeated, positive evidence for the specific claims C1
and C3 make — narrowed by the false-positive finding for C6, not undermined
by it.

## 2. What did WI230 demonstrate RepoPact DOES NOT guarantee merely by being adopted?

That confirmed governance discrepancies will be caught before they
accumulate — because nothing guarantees the validator is actually *run*.
Being "under RepoPact" in the sense of having valid, conformant
`governance/*.json` files, a `work/` ledger, and an `evidence/runs/`
directory did not, by itself, cause the validator to execute at any point
during the (at minimum) 34-day period between GA-1's discovery (39 reported
errors) and WI230's own starting state (297 reported errors, an
undifferentiated mix of confirmed discrepancies and — per the same
false-positive class identified above, likely already present at some
level during this window too, though this case study did not independently
verify the false-positive/confirmed split at the 39- or 297-error snapshots
specifically, only at the 269-error WI237 starting state). Adoption
established L0 (the record store exists and can be checked); it did not
establish that anything in the ordinary commit/CI loop *would* check it.
This is the central, unfavorable-to-a-naive-reading finding of this case
study, and a structurally related failure recurred independently in the
evidence gathered here: once in ForgeWire (this incident, a **checkpoint-
coverage** failure — CI ran but never called RepoPact) and once in
RepoPact's own repository (WI-032/decision 0031: a **checkpoint-invocation**
failure — CI billing-locked — compounded by a **checkpoint-effectiveness**
failure — no branch protection). Both are enforcement-closure failures at
the same higher level of description; they are not the same failure
mechanism (see `05-claim-evidence-matrix.md`'s coverage/invocation/
effectiveness decomposition), and this document does not claim they are.

## 3. What did WI237 change?

It made invocation structural rather than optional: one canonical
implementation (`scripts/ci.py fast|full|closeout`) that developers, agents,
and (after commit `5f05b3e2`) hosted CI all invoke identically; a pre-commit
hook running the `fast` profile; and — the more important structural change
— it moved the *decision to invoke* out of individual judgment and into a
named, documented rule (decision 0009, `docs/contributing/agent-workflow.md`:
fast before commit, full before push, closeout before marking a work item
complete). It did not change RepoPact itself in any way (no code in
`C:\Projects\repopact` was touched by this intervention); it changed
ForgeWire's *practice* around an unchanged tool.

## 4. What evidence suggests operationalized enforcement improves outcomes?

The negative-control demonstrations run in the WI236/237 session
(`scripts/ci.py fast` correctly failing when a forbidden import was
deliberately reintroduced; `check-frozen` correctly failing without `--ack`
on a real, not staged, frozen-surface change; a fabricated stale-HEAD
evidence run being detectably inconsistent with the actual current
`git rev-parse HEAD`) show the *new* arrangement's gates are not merely
declared but demonstrably fire and block when exercised — which is precisely
the property (coverage, invocation, *and* effectiveness together, not merely
executability — see `05-claim-evidence-matrix.md`'s definition) that was
missing before. This is real, if narrow and freshly-produced, evidence that
the specific gap identified in this case study (enforcement closure) is what
was actually closed, not a peripheral improvement.

## 5. What still escaped after operationalization?

Two things. First, the `.claude/worktrees` false-positive class — fixed in
ForgeWire only by removing the stale worktrees, not by any RepoPact upgrade
(ForgeWire's own pin stayed at 2.2.0). **Correction to this phase's first
pass**: RepoPact had already fixed the underlying `IGNORED_PARTS` gap on its
own `origin/main` three weeks earlier (`03-version-delta.md`) — so what
"escaped" here is more precisely a version-currency gap (ForgeWire's pin
lagging an available fix) than a standing RepoPact defect, though the fixed
mechanism is narrowed to one literal directory name, not closed structurally,
per the same document. Second, the README/manifest heading mismatch
(deliberately left unfixed, per this phase's instructions, and would remain
invisible to `repopact validate` even if "fixed" by hand, since nothing
checks it) — this one **is** confirmed absent in both 2.2.0 and current
`origin/main`. Neither is closed by operationalizing invocation — both are
gaps in *what the validator checks* (or, for the first, *which release the
adopter is on*), not gaps in *whether it runs*. Operationalizing invocation
(WI237) and widening validator coverage/pin currency (the two remaining
gaps) are independent axes;
this case study closed the first without touching the second.

## 6. Does any paper claim need narrowing?

Yes — two, both identified precisely in `05-claim-evidence-matrix.md`:

- **T5's `[ci]` discharge tag** ("machine-checked on every run") should be
  understood as "checked whenever the checkpoint is actually invoked," not
  as "guaranteed to be invoked on every run of CI" — the current tag
  conflates these, and this case study (plus RepoPact's own WI-032) shows
  the conflation is not merely theoretical.
- **H12/S5's drift-visibility claim** should be understood as measured along
  one axis — *detection efficacy conditional on invocation*, over discrete,
  promptly-checked mutations — not over *invocation latency/absence* or
  *accumulated reported-error volume over an extended zero-invocation
  window*, which this case study's evidence (a multi-week accumulation
  reaching a reported count in the hundreds, only part of which — see item 1
  — turned out to be confirmed governance discrepancy rather than a
  version-specific false positive) exercises and the current framing's
  honest acknowledgment of F-011 as a blind spot does not yet score.

## 7. Is there a new hypothesis or primitive suggested by "enforcement closure"?

Yes, refined during maintainer review into an admission-boundary definition
rather than a continuous-checking one (the earlier draft's "guaranteed to
actually run... on a bounded cadence" phrasing risked reading as conflicting
with RepoPact's own checkpoint-based, not precondition-based, design — see
C1 — which this case study does not dispute). This case study now proposes:

> **Enforcement closure** is the property that every transition promoting
> repository state across a governed admission boundary is necessarily
> evaluated by the applicable checkpoint, and a nonconformant state cannot
> cross that boundary merely because the checkpoint was absent, unavailable,
> ignored, or misconfigured.

— composed of three independently-failable sub-properties: **checkpoint
coverage** (every governed admission path routes through the applicable
checker), **checkpoint invocation** (the checker actually executes for the
candidate state), and **checkpoint effectiveness** (a failing result actually
prevents the promotion). Full definitions and the ForgeWire-vs-RepoPact-repo
contrast (a coverage failure vs. an invocation-plus-effectiveness failure) are
in `05-claim-evidence-matrix.md`. The kernel model (L0–L5) currently assumes
this triad holds rather than modeling, guaranteeing, or naming it as a
distinct concern from L2's invariant-monitor predicate `I`. A candidate
formal treatment: an additional layer or cross-cutting property, provisionally
`L2.5`, stated as three composable predicates over the admission-boundary
transition relation rather than a single meta-invariant, since this case
study's own evidence shows the three sub-properties fail independently and a
single boolean "closure holds/fails" would lose that distinction. This is
offered as a hypothesis for future work, not a proven necessity; RepoPact's
own WI-032 is independent, first-party evidence that its authors are already
converging on needing exactly this three-way distinction operationally, ahead
of it being named in the formal model.

## 8. Does "declared gate" need to be distinguished from "exercised/effective gate"?

Yes, unambiguously, on the evidence gathered here — and more precisely than
a single invoked/effective pair. `05-claim-evidence-matrix.md`'s refined
analysis found the current corpus distinguishes *specified* (a record
exists) from *executable* (a mechanism could check it) cleanly, via the
typed enforcement lattice — but has no first-class vocabulary for
*checkpoint coverage*, *checkpoint invocation*, or *checkpoint
effectiveness* as distinct from *executable*, and this case study's own two
field instances (ForgeWire: coverage absent; RepoPact's own repository:
coverage present, invocation and effectiveness both absent) show these three
need to stay separate rather than collapsing into one "invoked or not"
binary. RepoPact's own work item 032 acceptance criteria (AC-1 through AC-4)
are, read closely, almost exactly a decomposition onto coverage, invocation,
and effectiveness plus an honesty constraint — evidence the three-way
distinction is operationally necessary even where it isn't yet theoretically
named.

## 9. Is representation drift a RepoPact bug, a model boundary, or a documentation concern?

A **model boundary, evolving by narrow patches, not (yet) a general
principle** — supported directly by the version delta (`03-version-
delta.md`): decision 0014 (checkbox parity) and decision 0028 (root README
version line) are each real, working, narrowly-scoped fixes for a specific
discovered instance of "prose disagrees with a typed record," added at
different times as each instance was discovered. Neither is a bug (each does
exactly what it claims); the boundary is that no general "any prose
restating a source record must match it" check exists, so a third instance
(work-item README headings) remains open, and by the same pattern, others
likely exist unfound. This is best framed as a genuine model/coverage gap
with a clear, low-risk remediation path already implied by RepoPact's own
prior two fixes (add decision 00XX, gate on the heading-pattern convention,
following exactly the precedent of 0014/0028) — not a design flaw requiring
rethinking derive-over-declare, which already states the right principle;
work-item READMEs simply haven't been brought under it yet.

## 10. What should be added to PactBench?

Directly implied by `08-pactbench-coverage-gap.md`'s six absent and two
partial items, in order of how cheaply each could be built from evidence
already on hand:

1. A drift mutation (M16 candidate) for "edit a work-item README's
   heading/id/title without updating the sibling manifest" — directly
   modeled on the reproduced fixture in `06-representation-drift.md`, which
   already demonstrates the exact scenario and expected (non-)detection.
2. An S3 fixture for "two agents, unaware of each other, each run
   `repopact new work-item` before either pushes" — directly modeled on
   `07-concurrency-id-collision.md`'s mechanism, which is fully understood
   and trivially reproducible.
3. A longitudinal/duration-based drift scenario distinct from M1–M15's
   per-event framing: govern a repository, then simulate N days/commits of
   ordinary work with zero validator invocations, then measure the
   accumulated violation count and its composition — closer to what actually
   happened in ForgeWire than any existing single-mutation task.
4. A "gate exists but is invoked-and-ineffective" scenario (item 7 in
   `08`): a CI/pre-commit step that runs but fails for a reason unrelated to
   the tested violation (miscalibrated rule, missing toolchain, billing
   lock) — RepoPact's own WI-032 history is a ready-made, already-documented
   real instance that could be adapted directly rather than synthesized.

## 11. What additional observations would we need before claiming causal improvement?

At minimum: (a) a second, unrelated adopter's longitudinal history exhibiting
the same "adopted-but-uninvoked" pattern, to address T1's reflexivity threat
for this specific finding, since currently both instances (ForgeWire and
RepoPact's own repo) are ForgeWireLabs-controlled; (b) an observation window
for ForgeWire's *post*-WI237 practice spanning a comparable duration to the
34+ days over which the pre-WI237 drift accumulated, to see whether the new
`fast`/`full`/`closeout` discipline actually holds under real, ordinary,
unsupervised work rather than only at the single point-in-time this case
study captured; (c) resolution of the one open evidentiary gap this case
study could not close — whether `doctor` was run (and failed to hold) or was
never re-run at all between GA-1 (2026-07-15) and WI230 (2026-08-18) — since
the two possibilities support materially different conclusions about
whether `doctor` itself needs strengthening or whether the pure invocation
gap is sufficient explanation on its own; and (d) a comparative baseline (per
T6's construct-validity concern) — this case study has no ungoverned "what
would have happened to this same repository without RepoPact" arm, so while
it demonstrates RepoPact-with-a-gap, it cannot speak to whether an
ungoverned or convention-file-only ForgeWire would have drifted worse, the
same, or been caught sooner by some other mechanism entirely.
