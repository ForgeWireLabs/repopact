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

These studies operationalize hypotheses **H8–H10**, added to `protocol.md` in the
2026-06-24 amendment.

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
