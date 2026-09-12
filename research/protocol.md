# Experiment protocol: adversarial evaluation of RepoPact

Written 2026-06-15, before the proving-ground runs, so the bar is set independently
of the results. Amendments are dated and appended, never silently rewritten.

## Research question

Does a repository-native governance layer, including binding invariants, scoped authority,
evidence-gated work items, and a filesystem state machine, actually make agent
work *durable and recoverable*, or does it merely add ceremony that drifts out of
sync with the real project? Concretely: can an external adopter pick up the
**published package**, govern a real project with it, and have the architecture catch
the failures it claims to catch?

## Subject under test

RepoPact as distributed: the `repopact` package installed from a clean environment,
not a source checkout. Testing only the checkout would test the author's machine, not
the product an adopter receives.

## Hypotheses

- **H1: Adoptability.** An adopter can install the package and reach a *valid*
  governed repository with the documented commands, with no access to RepoPact's
  own source tree.
- **H2: Closure.** Every command RepoPact advertises succeeds, or fails cleanly,
  on the repository its own `init` produces. The tool's output is closed under the
  tool's own surface.
- **H3: Gated completion.** A work item cannot be marked complete unless its
  acceptance criteria are backed by evidence runs that actually exist.
- **H4: Authority is binding.** Changes to the frozen surface are detected and
  blocked without operator acknowledgement; invariants cannot be silently weakened.
- **H5: State integrity.** A work item's declared status must match its directory;
  dependency cycles, unknown dependencies, and concurrent conflicting scopes are
  rejected.
- **H6: Recoverability.** A reader, human or agent, with *only* the repository and
  no chat history can reconstruct what was done, why, and with what proof.
- **H7: Brownfield adoptability.** An *existing, RepoPact-naive* project with its
  own CODEOWNERS, CI workflows, nested contracts, and history can be brought under
  RepoPact non-destructively into an honestly represented governed state. Added
  2026-06-15 after the operator identified greenfield-only proof as insufficient.

## Falsification criteria

The architecture is **disproven, in whole or part**, if any of:

1. The package cannot bootstrap a valid repo without the source checkout (not H1).
2. A documented command crashes or corrupts state on an `init`-fresh repo (not H2).
3. The validator accepts a completed work item whose criteria lack real evidence (not H3).
4. A frozen-surface or invariant change passes the gates unacknowledged (not H4).
5. A status/directory mismatch, dependency cycle, or scope conflict is accepted (not H5).
6. Reconstructing project state from the tree alone requires information that lives
   only outside the repository (not H6).
7. An existing real-world repository cannot be adopted without either discarding
   reachable governance state, fabricating proof, or clearly reporting structural
   residue that cannot yet be represented conformantly (not H7).

A finding that the architecture *holds* under an adversarial case is recorded with
equal weight. The aim is calibration, not advocacy.

## Method

1. **Package.** Build wheel and sdist; install into an isolated venv; confirm seed
   data resolves from the install location. Run 001.
2. **Adopt.** Create a small but genuinely working project and adopt RepoPact into it
   from the installed package. Make real commits.
3. **Exercise, happy path.** Drive `init -> new -> implement -> evidence -> validate ->
   transition -> dashboard` for real work items with real acceptance criteria.
4. **Exercise, adversarial.** Deliberately attempt each falsification case above and
   record whether the architecture catches it.
5. **Reconcile.** Feed every defect back into RepoPact as an audit finding, work item,
   or decision using RepoPact's own machinery, which is itself a test of H6.
6. **Decide release readiness.** Release claims are limited to what the surviving
   evidence supports.

## Instruments and outputs

- The proving-ground repository and its RepoPact evidence runs as primary data.
- RepoPact `audits/findings/` entries for defects found in the subject.
- Raw transcripts in [`captures/`](captures/), one per run, referenced by ID.
- [`findings.md`](findings.md) as the analyzed register and [`run-log.md`](run-log.md)
  as the chronological record.

## Amendment 2026-06-24: comparative value hypotheses (H8-H13)

The hypotheses above ask whether the architecture *catches what it claims* on a single
governed subject. They do not measure whether governing a repository with RepoPact
**changes agent behavior** relative to not governing it. That comparative question is
added here and operationalized in [`benchmark-protocol.md`](benchmark-protocol.md). The
independent variable is `condition in {repopact, baseline}`, holding source, task, model,
and harness constant.

- **H8: Guarantee enforcement is measurable.** On a pre-registered suite of tasks whose
  correct outcome is to refuse or escalate weakening a binding invariant or frozen
  surface, a RepoPact-governed agent blocks or escalates the weakening at a materially
  higher rate than the same agent on an ungoverned repo. Benchmark: PactBench.
- **H9: Durable state improves recovery and efficiency.** A *fresh* session given only
  the repository resolves long-horizon tasks at a higher rate, with fewer regressions
  and lower redo cost, under RepoPact than baseline, and can reconstruct goal,
  decisions, and remaining work from the tree alone.
- **H10: The repo is a coordination substrate.** Two concurrent agents on one
  RepoPact-governed repository produce fewer conflicting or duplicated edits and higher
  joint-task success than on a shared scratchpad.
- **H11: Context efficiency.** Across context-provisioning regimes, RepoPact sits on a
  better frontier of per-request token cost versus task success and its per-request
  context cost grows more slowly with accumulated project state than full-context
  stuffing. Study: S4.
- **H12: Drift visibility.** Documented project state that diverges from code is surfaced
  with lower silent-staleness and lower time-to-detection under RepoPact than under
  convention-file-only regimes, measured honestly against known blind spots. Study: S5.
- **H13: Security enforcement and injection resistance.** RepoPact catches or escalates
  silent weakening of security-relevant invariants at a higher rate than convention-file
  only and lowers the rate at which injected or forged context drives unsafe actions,
  while treating RepoPact records themselves as a trusted attack surface rather than
  claiming immunity. Study: S6.

**Falsification, added 2026-06-24.** The comparative claim is disproven, in whole or part,
if any of:

8. RepoPact shows no meaningful improvement, or a regression, in violation-catch rate
   over a fair baseline at a comparable false-stop rate (not H8).
9. RepoPact does not improve recovery or reduce redo cost on long-horizon work, or only
   does so by adding ceremony cost that cancels the gain (not H9).
10. Two agents coordinate no better through the repository than through the baseline
    coordination substrate (not H10).
11. RepoPact is Pareto-dominated on token cost versus task success, or its context cost
    scales no better than full-prompt stuffing (not H11).
12. Baseline convention files surface drift as well as RepoPact, or RepoPact's silent
    staleness is no lower for the tested classes (not H12).
13. RepoPact gives no security-invariant preservation improvement over convention files,
    or its injected-context-followed rate is no lower under fair conditions (not H13).

Disconfirming results are recorded with the same weight, and the corresponding threats
to validity are re-examined before reporting a headline number.

## Amendment 2026-08-21: enforcement closure (H14)

A naturalistic, post-hoc field observation in ForgeWire and a separate instance in
RepoPact's own repository motivated a new prospective hypothesis. The observation that
motivated the hypothesis is not evidence *for* it. It was not pre-registered, has no
baseline arm, and does not itself confirm H14. H14 remains untested until S7 runs.

- **H14: Enforcement closure.** A RepoPact deployment that establishes checkpoint
  coverage, checkpoint invocation, and checkpoint effectiveness (`formal-model.md` §7:
  `Cov`, `Inv`, `Eff`) at its governed admission boundaries admits a materially lower
  rate of known-nonconformant repository states across those boundaries than an otherwise
  identical deployment that has an executable, correct validator but lacks one or more
  of coverage, invocation, or effectiveness. This is a claim about admission-time
  outcomes, not about whether the validator decides correctly when it runs.

**Falsification.** H14 is disproven, in whole or part, if:

14. A deployment lacking one or more of coverage, invocation, or effectiveness shows no
    higher rate of known-nonconformant state crossing governed boundaries than a deployment
    with all three established, once the friction and failure modes of establishing
    closure are counted fairly.

H14 is deliberately narrower than "RepoPact prevents governance drift." It isolates the
admission-boundary mechanism while holding validator decision correctness constant.

## Amendment 2026-09-12: governance continuity (H15)

The September 2026 implementation work made an implicit property of RepoPact explicit.
The canonical Rust engine, compatibility CLI, and Tauri Workbench are different clients of
the same repository-native governed state. That architectural convergence suggests a
question that H6 and H9 do not fully isolate.

H6 asks whether a reader can recover a particular project's history and proof from the
tree. H9 asks whether durable state improves agent task recovery and efficiency after a
session reset. **H15 is a structural interoperability claim:** does the governed state
survive a change of worker, machine, or legitimate client without depending on hidden
predecessor-local authority?

No S8 run had been performed when this amendment was committed. F-006 is prior bounded
evidence for one recovery case and therefore cannot count as confirmation of H15.

Define the represented governance projection `G(s)` and current conformance violations
`Viol(s)` as in `formal-model.md`. Then:

- **H15: Governance continuity.** For load-bearing state RepoPact claims is repository
  authoritative, a fresh legitimate worker starting from a clean clone and the versioned
  RepoPact semantics can recover the same represented governance projection and current
  known violations without access to the predecessor's private chat, local database,
  cache, machine state, or provider memory. Equivalent supported clients should not
  disagree materially about that governed state.

H15 is not an omniscience claim. A fact that never crossed the L5 adoption boundary is
outside the recoverable projection. It is also not a claim that every handoff is cheap.
The S8 study separately measures orientation cost.

**Falsification.** H15 is disproven, in whole or part, if:

15. A clean-clone handoff requires predecessor-private state to recover a load-bearing
    fact RepoPact claims is repository authoritative; a supported client produces a
    materially contradictory governance projection from the same versioned tree; a known
    repository violation becomes falsely clean solely because the worker or client
    changed; or RepoPact preserves state only by imposing orientation cost so large that
    the claimed continuity is practically unusable under the registered S8 task set.

S8 in `benchmark-protocol.md` operationalizes H15. Its primary correctness measures are
field-level governance recovery, violation recovery, false-clean rate, authority-state
error, and cross-client disagreement. Its secondary efficiency measures include
orientation time, input tokens, tool calls, file reads, and repository-wide search or grep
operations. These efficiency measures are deliberately registered before the proposed
repository-orientation graph is designed, so a future graph implementation cannot choose
its evaluation metric after seeing its own results.
