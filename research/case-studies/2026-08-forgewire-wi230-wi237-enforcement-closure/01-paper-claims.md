# 01 — Paper Claims Inventory (Phase 2)

Source: `C:\Projects\repopact\research\{paper.md, formal-model.md, protocol.md,
benchmark-protocol.md, findings.md, threats-to-validity.md,
gap-audit-2026-07.md}`, read in full. No claim below is reinterpreted to fit
the ForgeWire incident; that comparison happens in `05-claim-evidence-matrix.md`.

Format per claim: id, paraphrase, source, what would support it, what would
falsify/narrow it, required preconditions.

---

## C1 — The lifecycle/invariant split is checkpoint-based, not runtime-gated

**Paraphrase.** RepoPact does not evaluate `g_done` (or any invariant) as a
runtime precondition on an edit. A user or agent can put the tree into an
invalid state with an ordinary `git mv`/file edit. RepoPact's enforcement
point is the *checkpoint* — validation, CI, review, generated reports — not
the edit itself.

**Source.** `paper.md` §3.2 ("Composition is checkpoint-based rather than
precondition-based... RepoPact's enforcement point is the checkpoint:
validation, CI, review, and generated reports"); `formal-model.md` §3
("Composition is checkpoint-based rather than precondition-based... L1
transitions freely; L2 decides admissibility at the commit boundary").

**Supporting observation.** Any case where an invalid edit is made, then
caught the next time `validate`/CI/`doctor` actually runs.

**Falsifying/narrowing observation.** A case where the checkpoint itself is
never exercised for an extended period, so "the point of enforcement" never
fires at all — this doesn't falsify C1 as *stated* (the paper never claims
the checkpoint fires automatically on a fixed cadence), but it exposes that
C1's safety property is conditional on *something* invoking the checkpoint,
which the model does not itself guarantee. This is the crux of the
"enforcement closure" question in `05-claim-evidence-matrix.md`.

**Preconditions required by the claim.** A checkpoint (validate, CI, review,
or a generated report someone reads) actually runs at some point before the
consequences of an invalid state compound. The paper does not state a bound
on how long a repository may go without a checkpoint running.

---

## C2 — T5, Monitor non-bypass, tagged `[ci]`

**Paraphrase.** "For any edit trace `s0 → s1 → ... → sk`, the checkpoint
admits `sk` iff `sk ∈ R`." Tagged `[ci]` = machine-checked on every run, and
`[conj]` for the adversarial negation (¬H4/¬H5).

**Source.** `paper.md` §3.3 and Appendix B, T5; `formal-model.md` §6, T5.

**Supporting observation.** A CI run that rejects a bad commit; a local
`validate` that rejects a bad state before push.

**Falsifying/narrowing observation.** A commit history in which CI is wired
(workflow files exist, reference "governance") but never actually invokes
`repopact validate`/`check-frozen`, so no commit is ever admitted-or-rejected
by the checkpoint — the checkpoint doesn't run, so T5's "admits iff conformant"
guarantee is vacuous for every commit in that window, not merely untested. A
literal reading of `[ci]` ("machine-checked on every run") presupposes CI
*performs* the check; a CI pipeline that exists but does not call the
validator is not covered by `[ci]`'s discharge, and the formal model does not
name this gap explicitly. See RepoPact's own `research/gap-audit-2026-07.md`
GA-3 and work item `032` (`C:\Projects\repopact\work\blocked\032-...`) for a
first-party instance of exactly this: RepoPact's own `main` branch has no
branch-protection requiring the governance workflow to pass, and the workflow
itself was billing-locked for over a month.

**Preconditions required by the claim.** CI (or an equivalent checkpoint) is
(a) actually configured to invoke the validator, and (b) actually running,
and, per work item 032's own AC-3, (c) *required* by branch protection or an
equivalent merge gate — otherwise a failing checkpoint doesn't block anything
either.

---

## C3 — I_derive_dash: dashboard fixpoint is now validator-enforced, not CI-only (RepoPact 2.2.0)

**Paraphrase.** As of 2.2.0, `I_derive_dash(s) = exists(dashboard) and
read(dashboard) = π_dashboard(s)` is checked by the one-tree validator itself,
not only by a CI diff step. "This makes CI a redundant execution venue for the
dashboard fixpoint rather than its only enforcer."

**Source.** `paper.md` §3.5; `formal-model.md` §5 ("Since RepoPact 2.2.0,
dashboard equality is decided directly by the one-tree validator").

**Supporting observation.** `repopact validate` rejecting a stale dashboard
with no CI involved at all — reproduced directly in this case study (every
`repopact dashboard --root .` regeneration step in the ForgeWire WI236/237
session was required independently of CI, and `repopact validate` caught
staleness locally multiple times).

**Falsifying/narrowing observation.** None found. This is the one derive-layer
claim where RepoPact deliberately moved a check *out* of the CI-dependency
class and into the one-tree validator — i.e., RepoPact's own maintainers
already narrowed the CI-dependency surface for exactly one invariant (INV-7's
dashboard half) after presumably recognizing CI-availability risk. `spec`
projection equality (`SPEC.md`) remains generator/CI-checked only, per the
same section — narrower coverage than the dashboard case.

**Preconditions.** None beyond running `repopact validate` at all (locally or
in CI) — this is precisely why it is the strongest claim in the corpus:
it does not depend on CI being wired correctly.

---

## C4 — The concrete-record adoption trilemma and its provenance-typed resolution (T6a/T6b)

**Paraphrase.** No brownfield migration can be simultaneously total, faithful,
and closed under a concrete-only record language. RepoPact 2.0 resolves this
by adding `inferred`/`provisional` provenance types, so migration can be all
three under the provenance-aware language `R_p`, while `completed` work still
requires `concrete` evidence (P2/P3).

**Source.** `paper.md` §3.6, §4.3 (F-014's `waived` convention), Appendix B
T6a/T6b; `formal-model.md` §4 ("Adoption cannot preserve R: a trilemma");
`findings.md` F-010 (import-plan; completed items marked `waived`, "never as
`satisfied` with fabricated evidence").

**Supporting observation.** ForgeWire's own `REPOPACT-ADOPTION.md` states
exactly this convention verbatim ("Completed todos import as `status:
completed` with their acceptance criterion `waived`... never as `satisfied`
with fabricated evidence") and the WI236/237 session independently applied
the identical pattern when backfilling 8 unregistered work directories and
adding disclosed (non-backdated) preflight markers to 5 more — i.e., this is
a *live, still-operative* convention in the wild, not a one-time adoption
artifact.

**Falsifying/narrowing observation.** None found in this case study. This
claim was not under test by the WI230/236/237 incident — no adoption event
occurred during it; the trilemma concerns *migration*, and this incident is
about *post-adoption drift*, a different L1/L2 question. Out of scope for
this case study's central question (see `05-claim-evidence-matrix.md`).

**Preconditions.** A migration path (`adopt`/`import-plan`) is used, as
opposed to hand-authoring records — which is itself a distinct concern from
whether an *already-adopted* repository's validator is invoked going forward.

---

## C5 — H12 / S5: RepoPact lowers silent-staleness rate and detection latency for drift, "measured honestly against its own known blind spot" (F-011)

**Paraphrase.** RepoPact should surface documented-state-vs-code divergence
at commit/CI boundaries with *bounded* staleness; convention-file regimes
have no detection mechanism at all (detection ≈ 0 until a human notices).
RepoPact's own longitudinal-upgrade blind spot (F-011, and mutation M9 in the
drift harness) is explicitly reported rather than hidden.

**Source.** `protocol.md` H12/¬H12 (falsification #12: "RepoPact's
silent-staleness rate is no lower for a drift class (cf. the longitudinal
F-011 case)"); `benchmark-protocol.md` S5; `findings.md` F-011;
`C:\Projects\repopact-proving-ground\benchmarks\drift\MUTATION-SET.md`
(mutations M4, M5, M7, M9, explicitly labeled RepoPact blind spots/partials).

**Supporting observation.** Every mutation *other than* M4/M5/M7/M9 has a
concrete, named validator predicate that fires when `validate` is *run*.

**Falsifying/narrowing observation.** This is the central hit. WI230's
269–297 accumulated errors are not a case of the validator running and
missing something (which is what M1–M15 model — detection accuracy on
invocation). They are a case of the validator *not being invoked at all* as
part of ordinary workflow for an extended period, while unrelated
architecture work proceeded and tests stayed green. This is a *different*
mechanism from F-011 (which is version-drift: an old adopter's records go
invalid as the *standard* evolves) and from M9 (same mechanism, modeled as a
mutation). WI230's mechanism is: the standard did not change, the repository
was never RepoPact-naive, but the checkpoint was simply not exercised as part
of the loop that produced the 269–297 errors' worth of drift. See
`08-pactbench-coverage-gap.md` for the precise gap this exposes in the
mutation set, and `05-claim-evidence-matrix.md` for the classification.

**Preconditions.** H12 is stated as a claim about behavior "under RepoPact" —
which implicitly presumes RepoPact's validator is exercised at the boundaries
being compared. The claim does not have a stated precondition that the
validator must actually run for the comparison to be meaningful, which is
itself worth flagging (see `05`).

---

## C6 — F-008 (adopter `.gitignore` silently un-tracks `evidence/runs/*.json`)

**Paraphrase.** A pre-existing `.gitignore` rule intended for unrelated
runtime data (`runs/`) can silently match RepoPact's own evidence directory,
so the repo validates locally but breaks on a fresh clone or in CI, where the
ignored evidence is absent. "The most dangerous failure is one that passes
locally and breaks for everyone else." Fixed: `adopt` now runs
`git check-ignore` on every record it writes and warns.

**Source.** `paper.md` §6.1; `findings.md` F-008.

**Supporting observation.** ForgeWire's `REPOPACT-ADOPTION.md` documents
exactly this incident (finding F-008 upstream) and its fix (`!evidence/runs/`
negation).

**Falsifying/narrowing observation — the mirror-image gap.** F-008 is about
*governed* content becoming silently *invisible* to `git`/CI (undercounting).
The WI236/237 session found the structural inverse: *non-governed* content —
local `git worktree` checkouts under `.claude/worktrees/**` — becoming
falsely *visible* to `repopact validate`'s raw filesystem walk
(`repo_model.iter_contracts`'s hardcoded `IGNORED_PARTS` set:
`.git, __pycache__, node_modules, .venv, .pytest_cache, build, dist,
fixtures` in the pinned 2.2.0 ForgeWire consumed — no `.claude` or worktree
awareness), producing 171 of the 269 errors from AGENTS.md files inside
untracked worktree checkouts that were never part of the governed tree and
never affected CI. **Correction**: this specific gap was already fixed on
RepoPact's actual `origin/main` (`0096d70`, "exclude worktrees/ scratch
checkouts from contract scanning," merged 2026-07-28) three weeks before this
incident — ForgeWire's pin simply hadn't moved past 2.2.0. See
`03-version-delta.md` for the corrected classification: roughly two-thirds of
this incident's error volume is a version-currency gap, not a standing
RepoPact design gap, though the underlying mechanism (a literal path-segment
allowlist rather than genuine git-tracked-status awareness) is narrowed for
the one directory name this incident used, not closed structurally — a
worktree checked out under any other name would still reproduce the same
false positives in both 2.2.0 and current `origin/main`. F-008's mitigation
(`git check-ignore` at write-time) does not address this direction either
way: the validator's own directory walk has no concept of "untracked, not
part of any CI checkout" the way `git ls-files` or `.gitignore` awareness
would provide. See `03-version-delta.md` and `05-claim-evidence-matrix.md`.

**Preconditions.** The adopter's local disk state (worktrees, ignored dirs)
diverges from what CI actually checks out — true for any adopter that uses
`git worktree` for agent sessions, which the paper does not discuss as a
distinct adopter usage pattern.

---

## C7 — F-011 / GA-1: longitudinal upgrade drift is a documented, still-recurring blind spot

**Paraphrase.** An older adopter drifted invalid as the *standard* evolved
(stale registry paths, a missing root contract) and nothing detected or
guided the upgrade; `doctor` was built in response. RepoPact's own gap audit
(`gap-audit-2026-07.md`, GA-1) later found this recurring in the wild: on
2026-07-15, ForgeWire itself failed current validation with 39 errors
("16 × unknown affected_scope, preflight-marker drift, others" — "This is the
F-011 longitudinal-drift class recurring in the wild").

**Source.** `findings.md` F-011; `gap-audit-2026-07.md` GA-1 and its
2026-07-26 update.

**Supporting observation.** GA-1's own framing: "an opportunity: run `doctor`
upgrades on both and capture the episode as fresh T7/ratchet evidence... If
`doctor` cannot repair the unknown-scope class, that is a new honest finding
about `doctor` coverage."

**Falsifying/narrowing observation.** GA-1 predates the WI230 incident by
over a month (2026-07-15 vs. WI230's 2026-08-18 start) and documents *the
same repository* (ForgeWire) drifting invalid *twice*, independently, with
RepoPact's own research process as the discovery mechanism both times (not
ForgeWire's own CI). This means WI230/236/237 is not a first occurrence but
at minimum a second, later, larger recurrence (39 → 297 errors) of the exact
pattern RepoPact's own maintainers had already named and partially addressed
with `doctor`. Whether `doctor --fix` was actually run on ForgeWire between
GA-1 (2026-07-15) and WI230 (2026-08-18), and what its effect was, is not
established by this case study and is flagged as an open question in
`04-forgewire-case-timeline.md`.

**Preconditions.** `doctor` repairs a bounded class of drift (stale registry
paths, missing preflight markers, etc.) but does not address the invocation
gap itself — `doctor` still has to be *run* by someone, on some cadence, for
this mitigation to apply, which is the same "checkpoint must be invoked"
precondition as C1/C2.
