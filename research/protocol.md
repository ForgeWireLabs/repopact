# Experiment protocol — adversarial evaluation of RepoPact

Written 2026-06-15, before the proving-ground runs, so the bar is set independently
of the results. Amendments are dated and appended, never silently rewritten.

## Research question

Does a repository-native governance layer — binding invariants, scoped authority,
evidence-gated work items, and a filesystem state machine — actually make agent
work *durable and recoverable*, or does it merely add ceremony that drifts out of
sync with the real project? Concretely: can an external adopter pick up the
**published package**, govern a real (if trivial) project with it, and have the
architecture catch the failures it claims to catch?

## Subject under test

RepoPact as distributed: the `repopact` wheel installed from a clean environment,
not a source checkout. Testing the checkout would test the authors' machine, not
the product an adopter receives.

## Hypotheses

- **H1 — Adoptability.** An adopter can install the package and reach a *valid*
  governed repository with the documented commands, with no access to RepoPact's
  own source tree.
- **H2 — Closure.** Every command RepoPact advertises succeeds (or fails cleanly)
  on the repository its own `init` produces. The tool's output is closed under the
  tool's own surface.
- **H3 — Gated completion.** A work item cannot be marked complete unless its
  acceptance criteria are backed by evidence runs that actually exist.
- **H4 — Authority is binding.** Changes to the frozen surface are detected and
  blocked without operator acknowledgement; invariants cannot be silently weakened.
- **H5 — State integrity.** A work item's declared status must match its directory;
  dependency cycles, unknown dependencies, and concurrent conflicting scopes are
  rejected.
- **H6 — Recoverability.** A reader (human or agent) with *only* the repository —
  no chat history — can reconstruct what was done, why, and with what proof.
- **H7 — Brownfield adoptability.** An *existing, RepoPact-naive* project — with its
  own CODEOWNERS, CI workflows, nested contracts, and history — can be brought under
  RepoPact, non-destructively, into a state that validates. (Added 2026-06-15 after
  the operator identified greenfield-only proof as insufficient for "ready".)

## Falsification criteria

The architecture is **disproven, in whole or part**, if any of:

1. The package cannot bootstrap a valid repo without the source checkout (¬H1).
2. A documented command crashes or corrupts state on an `init`-fresh repo (¬H2).
3. The validator accepts a completed work item whose criteria lack real evidence (¬H3).
4. A frozen-surface or invariant change passes the gates unacknowledged (¬H4).
5. A status/directory mismatch, dependency cycle, or scope conflict is accepted (¬H5).
6. Reconstructing project state from the tree alone requires information that lives
   only outside the repository (¬H6).
7. An existing real-world repository cannot be adopted into a valid RepoPact without
   either discarding its existing governance or hand-authoring records (¬H7).

A finding that the architecture *holds* under an adversarial case is recorded with
equal weight; the aim is calibration, not advocacy.

## Method

1. **Package.** Build wheel + sdist; install into an isolated venv; confirm seed
   data resolves from the install location. (Run 001.)
2. **Adopt.** Create a tiny but genuinely working CLI (the proving ground) and
   adopt RepoPact into it from the installed package. Make real commits.
3. **Exercise — happy path.** Drive `init → new → implement → evidence → validate →
   transition → dashboard` for real work items with real acceptance criteria.
4. **Exercise — adversarial.** Deliberately attempt each falsification case above
   and record whether the architecture catches it.
5. **Reconcile.** Feed every defect back into RepoPact as an audit finding, a work
   item, or a decision — using RepoPact's own machinery, which is itself a test of H6.
6. **Decide 1.0.** Declare 1.0 only if the surviving evidence supports it.

## Instruments and outputs

- The proving-ground repository and its RepoPact evidence runs (primary data).
- RepoPact `audits/findings/` entries for defects found in the subject.
- Raw transcripts in [`captures/`](captures/), one per run, referenced by ID.
- [`findings.md`](findings.md) as the analyzed register; [`run-log.md`](run-log.md)
  as the chronological record.

## Amendment 2026-06-24 — comparative value hypotheses (H8–H13)

The hypotheses above ask whether the architecture *catches what it claims* on a single
governed subject. They do not measure whether governing a repository with RepoPact
**changes agent behaviour** relative to not governing it. That comparative question is
added here and operationalized in [`benchmark-protocol.md`](benchmark-protocol.md). The
independent variable is `condition ∈ {repopact, baseline}`, holding source, task, model,
and harness constant.

- **H8 — Guarantee enforcement is measurable.** On a pre-registered suite of tasks whose
  correct outcome is to refuse/escalate weakening a binding invariant or frozen surface,
  a RepoPact-governed agent blocks or escalates the weakening at a materially higher rate
  than the same agent on an ungoverned repo. (Benchmark: PactBench, work item `020`.)
- **H9 — Durable state improves recovery and efficiency.** A *fresh* session given only
  the repository resolves long-horizon tasks (SWE-bench Verified, SWE-EVO) at a higher
  rate, with fewer regressions and lower token cost, under RepoPact than baseline, and
  can reconstruct goal/decisions/remaining-work from the tree alone.
- **H10 — The repo is a coordination substrate.** Two concurrent agents on one
  RepoPact-governed repository produce fewer conflicting/duplicated edits and higher
  joint-task success than on a shared scratchpad.
- **H11 — Context efficiency.** Across context-provisioning regimes (full-prompt,
  convention-file-only, RAG, summarized/external memory, on-demand tool fetch, RepoPact,
  hybrids), RepoPact sits on the better frontier of **per-request token cost vs. task
  success** — approaching full-context quality at near-convention-file cost — and its
  per-request *context* tokens grow sublinearly with accumulated project state where
  full-prompt stuffing grows linearly. (Study: S4.)
- **H12 — Drift visibility.** Documented project state that diverges from the code is
  surfaced at commit/CI boundaries with bounded staleness under RepoPact, whereas
  convention-file regimes (`AGENTS.md` / `CLAUDE.md` / `rules.md`) accumulate *undetected*
  staleness (detection ≈ 0 until a human notices). RepoPact lowers silent-staleness rate
  and time-to-detection — measured honestly against its own known blind spot, longitudinal
  upgrade drift (F-011). (Study: S5.)
- **H13 — Security enforcement & injection resistance.** (a) RepoPact catches/escalates
  silent weakening of *security-relevant* invariants at a higher rate than
  convention-file-only; (b) its integrity structure (frozen surface, evidence validation,
  provenance, escalation) lowers the rate at which injected/forged context drives unsafe
  actions, versus free-form convention files — while acknowledging RepoPact records are
  themselves a trusted surface, protected by integrity checks, not immune to injection.
  (Study: S6.)

**Falsification (added 2026-06-24).** The comparative claim is **disproven, in whole or
part**, if any of:

8. The RepoPact arm shows no significant improvement (or a regression) in
   violation-catch rate over baseline at a fair false-stop rate (¬H8).
9. RepoPact does not improve resolution/recovery and does not reduce redo cost on the
   long-horizon beds — or only does so by adding ceremony cost that cancels the gain
   (¬H9).
10. Two agents coordinate no better through the repository than through a scratchpad
    (¬H10).
11. RepoPact is Pareto-dominated on token cost vs. task success — some regime reaches
    equal-or-better success at strictly lower tokens — or its per-request context tokens
    grow as fast as full-prompt stuffing as project state accumulates (¬H11).
12. Convention files surface drift as well as RepoPact, or RepoPact's silent-staleness
    rate is no lower for a drift class (cf. the longitudinal F-011 case) (¬H12).
13. RepoPact gives no security-invariant catch-rate improvement over convention files, or
    its injected/forged-context-followed rate is no lower — its records are gamed as
    easily as a poisoned `AGENTS.md` (¬H13).

A result that **disconfirms** RepoPact is recorded with equal weight, in `findings.md`,
and the relevant threat (T5/T6 in [`threats-to-validity.md`](threats-to-validity.md)) is
re-examined before the number is reported.

## Amendment 2026-08-21 — enforcement closure (H14)

A naturalistic, post-hoc field observation (an accepted case study of RepoPact's
progenitor adopter, ForgeWire, plus an independent instance in RepoPact's own
repository — see `findings.md`'s "Field-study synthesis: enforcement
closure" and `formal-model.md` §7) motivates a new prospective hypothesis.
Per this protocol's own discipline, the observation that motivates a
hypothesis is not evidence *for* it: the naturalistic case is not
pre-registered, has no baseline arm, and does not itself confirm what
follows. It is recorded here as motivation only; the hypothesis is
untested until S7 (`benchmark-protocol.md`) runs.

- **H14 — Enforcement closure.** A RepoPact deployment that establishes
  checkpoint coverage, checkpoint invocation, and checkpoint effectiveness
  (`formal-model.md` §7: `Cov`, `Inv`, `Eff`) at its governed admission
  boundaries admits a materially lower rate of known-nonconformant
  repository states across those boundaries than an otherwise identical
  deployment that has an executable, correct validator but lacks one or
  more of coverage, invocation, or effectiveness. This is a claim about
  *admission-time outcomes*, not about whether the validator decides
  correctly when run — that is T1/T5's claim, not H14's, and is not in
  dispute here.

**Falsification.** H14 is disproven, in whole or part, if any of:

14. A deployment lacking one or more of coverage, invocation, or
    effectiveness at its admission boundaries shows no higher rate of
    known-nonconformant states crossing those boundaries than a deployment
    with all three established — i.e., enforcement closure makes no
    measurable difference to admitted nonconformance (¬H14a); or a
    deployment with all three established still accumulates
    known-nonconformant admitted states at a rate statistically
    indistinguishable from one lacking them, once ceremony/friction costs
    of establishing closure are accounted for (¬H14b, the construct-validity
    mirror of T6).

H14 is deliberately scoped narrower than "RepoPact prevents governance
drift" — it isolates the admission-boundary mechanism (coverage, invocation,
effectiveness) as the independent variable, holding the validator's own
decision correctness (T1/T5) constant and out of dispute, consistent with
this protocol's practice of testing one mechanism at a time rather than a
compound claim.
