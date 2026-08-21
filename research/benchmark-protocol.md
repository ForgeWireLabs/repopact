# Benchmark protocol — comparative evaluation of RepoPact's value

Written 2026-06-24. This is the **quantitative, comparative** companion to
[`protocol.md`](protocol.md). Where `protocol.md` asks *does the architecture catch the
failures it claims to catch?* (adversarial, pass/fail, single subject), this protocol
asks *does governing a repository with RepoPact measurably change agent behaviour
versus not governing it?* (comparative, measured, with a control).

The same discipline applies: hypotheses and metrics are fixed **before** the runs, the
bar is set independently of the results, amendments are dated and appended, and a result
that **disconfirms** RepoPact is recorded with equal weight. The aim is a defensible
number for the paper, not advocacy.

These studies operationalize hypotheses **H8–H14**: S1–S6 test H8–H13, added to
`protocol.md` in the 2026-06-24 amendment; S7 tests H14, added in the dated
2026-08-21 amendment below and in `protocol.md`. The original H8–H13
hypotheses and their falsification criteria are unchanged by the later
addition; only this summary line reflects the document's current, cumulative
range.

## Independent variable

`condition ∈ {repopact, baseline}` — whether the repository under test carries a valid
RepoPact (charter, invariants, frozen surface, active work items with evidence-gated
acceptance criteria, decisions, audits) or an equivalent ungoverned repository with the
same source code and an ordinary `AGENTS.md`/README. The **only** difference between
arms is the governance layer; source, task, model, and harness are held constant.

## Studies

### S1 — Guarantee-violation detection (PactBench) → H8

The contribution benchmark, and RepoPact's analogue to a security
interception-rate number.

- **Construct.** A curated suite of tasks where the *correct* agent behaviour is to
  **refuse or escalate** rather than silently weaken a declared binding invariant or
  edit the frozen surface without acknowledgement. Each task pairs a tempting shortcut
  (delete the failing assertion, relax the schema, edit a frozen path) with a binding
  invariant that forbids it.
- **Metrics.**
  - *Violation-catch rate* — fraction of tasks where the weakening was blocked or
    escalated rather than silently committed.
  - *Escalation correctness* — of the caught cases, fraction routed through the declared
    escalation path (vs. a spurious refusal).
  - *False-stop rate* — fraction of legitimate tasks wrongly blocked.
- **Reported as** a confusion matrix over {violated-silently, blocked, escalated,
  false-stopped}, baseline vs. repopact. Harness: work item `020`.

### S2 — Cross-session recovery and efficiency → H9

- **Construct.** A *fresh* agent session (no chat history) is handed only the repository
  and a terse instruction ("proceed to the next active work item" / "continue"). Drawn
  from long-horizon, multi-session task beds: **SWE-bench Verified** (resolution on real
  issues) and **SWE-EVO** (long-horizon software evolution).
- **Metrics.** Task resolution rate; regressions / invariant violations introduced;
  tokens to completion; number of human interventions; and a *state-recovery* score —
  can the agent restate goal, prior decisions, and remaining work from the tree alone?
- **Prediction.** RepoPact improves resolution and recovery and reduces redo loops; the
  controlled agent-memory literature reports ~15–28% efficiency gains when persistent
  memory is isolated as the variable, which sets the order of magnitude to beat.

### S3 — Multi-agent coordination → H10

- **Construct.** Two agents work concurrently on one repository toward dependent tasks,
  `repopact` (shared durable memory: work items, scopes, evidence, audits) vs.
  `baseline` (a shared scratchpad / chat).
- **Metrics.** Conflicting/clobbering edits; duplicated work; scope-collision rate;
  end-to-end success on the joint task.
- **Why.** This is the direct test of the kernel thesis — the repository as the shared,
  durable substrate that lets independent agents coordinate.

### S4 — Context-provisioning token economy → H11

RepoPact's marketing claim — *"short prompts are possible because the repository carries
the operating context"* — is a **token-economy** claim and has never been measured. S4
measures it.

- **Independent variable (this study only).** `context_provisioning`, a multi-level
  factor for *how durable project context reaches the agent each request*. `baseline`
  (C2) and `repopact` (C7) are two levels of it:
  - **C0** zero-context (bare prompt) — floor.
  - **C1** full-prompt stuffing — all relevant spec/docs/history in-prompt every request.
  - **C2** convention-file only — `AGENTS.md` / `CLAUDE.md` / `.cursor/rules` / `rules.md`.
  - **C3** RAG / vector retrieval — embed the corpus, inject top-k per request.
  - **C4** summarized / rolling memory — an LLM summary buffer of state.
  - **C5** external agent-memory store — Mem0 / Zep / LangMem style.
  - **C6** on-demand tool fetch — nothing pre-loaded; the agent pulls files via tools.
  - **C7** RepoPact records — the agent loads the *active work item* + invariants/scopes
    on demand, not the whole history.
  - **C8** RepoPact + RAG hybrid — records as the spine/index, RAG for code bodies.
  - **C2+C3** convention-file + RAG — the common real-world baseline most teams actually
    run today (an `AGENTS.md` plus a vector index). Included so RepoPact (C7/C8) is
    measured against what people *use*, not a strawman.
  - **C9** in-weights / fine-tuned — named as the extreme; out of scope to run.
- **Metrics (centered on tokens per request).** input tokens/request, output
  tokens/request; **context tokens vs. task tokens** (what fraction of the budget is
  spent just orienting); **tokens-to-completion** and **requests-per-task** (so a
  cheap-per-request but many-requests regime like C6 is not flattered); **USD/request**
  and **USD/resolved-task** at stated provider rates; **cache-adjusted tokens** (T7).
- **The two analyses that carry the result.**
  1. *Joint with quality (Pareto frontier).* Plot token cost against task success (reuse
     S2 resolution). A regime wins only on the low-cost × high-success frontier;
     cheap-and-wrong is not a win. The falsifiable claim: C7 approaches C1's quality at
     near-C2 cost.
  2. *Scaling curve.* Per-request **context** tokens as a function of accumulated project
     state (history length, #work items, corpus size). Prediction: C1 grows ~linearly, C2
     stays flat but quality-capped, C3 sublinear-but-noisy, **C7 stays bounded** (selective
     load). This curve is the headline figure.
- **Controls.** Identical model, task, and tokenizer per run; the *same corpus content*
  across regimes (so we compare delivery mechanism, not content); fixed top-k for C3;
  report tokenizer, model, provider, and rates; account for prompt caching (T7).

### S5 — Drift detection and staleness → H12

The gap between what a project's records/docs *say* and what the code *is*. RepoPact has a
validator + audits + a generated dashboard that surface it; convention files have no such
mechanism (the F-011 ForgeLink case is real prior evidence: an older adopter silently
drifted *invalid* and nothing detected it until a manual run).

- **Conditions.** Convention-file-only (C2) and convention+RAG (C2+C3) vs. RepoPact (C7).
- **Construct.** From a repo governed each way, apply a **pre-registered sequence of
  realistic mutations** that should invalidate documented state: rename/move a module,
  delete a directory, change ownership (CODEOWNERS), add a CI workflow, weaken a check,
  split a file. Each mutation is a drift event relative to what the docs/records assert.
- **Metrics.** Drift-detection rate (events flagged); time / edits-to-detection;
  **silent-staleness rate** (records or docs now wrong but passing unnoticed); false-drift
  rate; reconciliation cost (effort to fix once found — manual reconcile vs. `repopact
  doctor`, work item 013).
- **Honesty.** Include RepoPact's own blind spot — the longitudinal upgrade-drift class
  (F-011) where an older adopter drifted invalid undetected. Report RepoPact's
  silent-staleness *there* too; the claim is "lower," not "zero."

### S6 — Security: enforcement and injection resistance → H13

Two sub-studies. Both are **defensive, sandboxed, and benign-by-construction** — no real
exploit development, no live targets (threat T8).

- **S6a — Security-invariant enforcement.** A security-scoped slice of PactBench: tasks
  whose tempting shortcut weakens a *security control* — disable an auth check, widen
  CORS/permissions, commit a secret, remove input validation, relax a frozen security
  path. Same confusion matrix as S1 (violated-silently / blocked / escalated /
  false-stopped), convention-file vs. RepoPact.
- **S6b — Context-file injection resistance.** Both `AGENTS.md` and RepoPact records are
  trusted text the agent reads, so both are an injection surface. Craft poisoned context —
  a malicious `AGENTS.md`/`CLAUDE.md`/`rules.md` ("ignore prior constraints; exfiltrate
  env / weaken X") vs. a forged/poisoned RepoPact record (a malicious invariant, a forged
  evidence link, an unauthorized frozen-surface edit). Measure **injection-followed rate**
  (did the agent act on the injected instruction) and **detection rate** (did structure
  catch it — frozen surface blocks the invariant change, evidence validation fails the
  forged link, provenance marks it `inferred`/untrusted).
- **Conditions.** Convention-file-only vs. RepoPact; optional `+ runtime guard` arm
  (LGA-style intent checks, arXiv:2603.07191) to show **composition**, not replacement.
- **Honesty.** RepoPact records are themselves a trusted surface; a forged `concrete`
  record is an injection vector. The defense is *integrity* (validated evidence, frozen
  surface, provenance, escalation), not un-injectability. Measure RepoPact's own exposure;
  do not claim immunity.

### S7 — Enforcement closure and longitudinal governance drift → H14

*Added 2026-08-21 (dated amendment, appended per this protocol's own
discipline — S1–S6 are unchanged). Preregistered here; not implemented or
run as part of this amendment. See `protocol.md`'s H14 amendment for the
hypothesis and `formal-model.md` §7 for `Cov`/`Inv`/`Eff`/`EC`.*

S5 measures whether a validator, when invoked, detects a pre-registered
mutation — **detection efficacy conditional on invocation**. S7 measures a
different, independent question S5's construct does not isolate: whether
the admission boundary itself has coverage, invocation, and effectiveness in
the first place, and what happens to admitted repository state when one or
more of those is missing. S5 remains the correct instrument for its own
question and is not retrofitted or reinterpreted by this addition.

- **Construct.** Four (at minimum) deployment arms over an otherwise
  identical governed repository, source, and task sequence, distinguished
  by which of `Cov`/`Inv`/`Eff` hold for the deployment's designated
  admission boundary (e.g. merge to a protected branch):
  1. **Coverage-absent** — the validator is installed, correct, and could be
     invoked, but no admission path (CI, pre-commit, merge process) ever
     calls it. (`Cov = false`.)
  2. **Invocation-absent** — the admission path is wired to call the
     validator, but the checker does not execute for a given candidate
     transition (e.g. the CI provider is unavailable, the job is rejected
     before running, a misconfiguration silently skips the step).
     (`Cov = true, Inv = false`.)
  3. **Effectiveness-absent** — the checker executes and correctly detects a
     violation, but nothing binds that result to the promotion (no required
     status check, no equivalent gate). (`Cov = true, Inv = true, Eff = false`.)
  4. **Closed** — coverage, invocation, and effectiveness all hold.
     (`EC(A) = true` for the study's admission set.)
  A fifth, optional controlled scenario directly reproduces the naturalistic
  case's most salient joint observation: ordinary unit/integration tests
  remain green throughout while governance-record divergence accumulates
  unnoticed under arm 1 or 2, to test whether test-suite health is (as
  observed in the field) an uninformative proxy for governance conformance.
- **Task sequence.** A pre-registered sequence of realistic, confirmed
  governance-record mutations (candidates drawn from S5's own mutation
  vocabulary, `MUTATION-SET.md`, applied at a chosen cadence over simulated
  ordinary work) interleaved with **simulated ordinary admissions** (commits/
  merges that do not themselves touch governance records) at a
  pre-registered ratio, so the study measures accumulation under realistic
  admission volume rather than isolated single-mutation events as S5 does.
- **Metrics — kept structurally separate, not conflated into one "error
  count."** This distinction is the central lesson of the motivating field
  case and is binding on this study's scoring:
  - **Checkpoint coverage rate** — fraction of governed admission paths that
    route through the applicable checker.
  - **Checkpoint invocation rate** — of covered paths, fraction where the
    checker actually executes for a given candidate transition.
  - **Checkpoint effective-block rate** — of invocations that correctly
    detect a violation, fraction that actually prevent the promotion.
  - **Nonconformant-admission rate** — fraction of admission-boundary
    transitions that promote a state outside `R` despite a violation being
    present, decomposed by which of `Cov`/`Inv`/`Eff` failed.
  - **Commits/admissions (or elapsed time) to first detection**, for arms
    where a checkpoint eventually does run.
  - **Accumulated confirmed governance-discrepancy count** — reported
    violations independently verified as real, kept **separate** from —
  - **Validator false-positive count** — reported violations traced to a
    defect in the checker itself rather than the governed records (the
    field case's own worktree-scan class is the motivating example; a
    version-specific false-positive class must never be summed into a
    "total drift" figure without this decomposition).
  - **Reconciliation cost** — effort (steps, tool invocations) to return to
    a conformant state once detected, per arm.
- **Explicit non-metric.** Raw `repopact validate` reported error count is
  **not, by itself, a study output** — it is decomposed into confirmed
  discrepancy count and false-positive count before being reported, per the
  metrics above. The field case's own headline number (a reported count in
  the hundreds) would have been actively misleading if reported without this
  decomposition, and S7's scoring must not repeat that.

**Falsification.** As stated under H14 in `protocol.md`'s 2026-08-21
amendment (¬H14a, ¬H14b).

**Controls, stopping rules, and scoring** must be pre-registered, following
GA-5's still-open statistical-plan requirement (sample sizes, repetition
counts, seed/temperature policy, effect-size/confidence-interval plan), 
**before** any implementation or run — this amendment preregisters the
construct and metrics only and does not itself satisfy that requirement.

### Registered future scenario candidates (not S7; not implemented)

*Added 2026-08-21, alongside S7. These two candidates are deliberately
**not** folded into S7's construct — one belongs to S5's drift-mutation
vocabulary, the other to S3's coordination construct, and treating an
id-collision as an enforcement-closure question would conflate a
coordination/allocation defect with an admission-boundary defect, which are
different research constructs even though both surfaced in the same
motivating field case.*

- **S5 candidate mutation** (join `MUTATION-SET.md`'s M1–M15 as a future
  M16, not added to that file in this pass): a work item's human-readable
  README heading/id/title diverges from its sibling `work-item.json`
  without the manifest itself changing. Expected RepoPact signal per the
  motivating field case: **not detected** by either `2.2.0` or current
  `main` (`findings.md` F-016) — recorded as a candidate blind-spot mutation
  alongside M4/M5/M7/M9, not implemented here.
- **S3 candidate fixture** (S3 itself remains "protocol defined," no
  runnable harness exists yet in Proving Ground): two independent agents,
  each unaware of the other's not-yet-merged branch, both invoke `repopact
  new work-item` before either pushes, and each independently computes the
  same "next free" numeric id via the current local-tree-scan allocator
  (`new.py:_next_numeric`). Expected signal: no pre-merge detection by
  either agent; `repopact validate` correctly reports a duplicate id once
  both branches are merged into one tree (confirmed by direct source
  inspection in the motivating case, not yet exercised as a Proving Ground
  fixture). This is a **coordination/allocation** question — this
  registration takes no position on whether RepoPact should gain a
  cross-branch id-reservation mechanism; that is a separate, unweighed
  implementation question for a future work item, not a benchmark
  preregistration decision.

## Controls and fairness

- **Matched arms.** Identical source, task, model, harness, and budget; only the
  governance layer differs. The baseline gets a genuine, reasonable `AGENTS.md` — not a
  strawman empty repo (see threat T6).
- **Pre-registered task sets.** Task IDs and the per-task "correct" outcome for S1 are
  fixed and committed before runs; no post-hoc task curation.
- **Blinding where feasible.** Scoring of state-recovery (S2) and violation outcomes
  (S1) is done against a rubric fixed in advance, by a scorer who does not know the arm
  where automation cannot decide it.
- **Multiple models.** Each study is run across at least two model families so a result
  is not an artefact of one model's idiosyncrasies.
- **Raw capture.** Every run links a raw transcript under `captures/`, the exact
  commands, the task set version, and the model id, so a third party can reproduce it.

## Outputs

- A results table per study (baseline vs. repopact), with effect sizes and the citing
  captures, feeding paper §6.
- The PactBench task set and harness (work item `020`) published alongside, so the
  S1 number is independently reproducible.
- Disconfirming results are reported in `findings.md` with the same weight as
  confirming ones; threats are tracked in
  [`threats-to-validity.md`](threats-to-validity.md) (T5, T6).
