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

These studies originally operationalized hypotheses **H8–H14**: S1–S6 test H8–H13,
added to `protocol.md` in the 2026-06-24 amendment; S7 tests H14, added in the dated
2026-08-21 amendment. A dated 2026-09-12 amendment below adds S8 for H15, governance
continuity. Earlier study definitions are not retroactively changed by the later additions.

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
- **Prediction.** RepoPact improves resolution and recovery and reduces redo loops.

### S3 — Multi-agent coordination → H10

- **Construct.** Two agents work concurrently on one repository toward dependent tasks,
  `repopact` (shared durable memory: work items, scopes, evidence, audits) vs.
  `baseline` (a shared scratchpad / chat).
- **Metrics.** Conflicting/clobbering edits; duplicated work; scope-collision rate;
  end-to-end success on the joint task.
- **Why.** This is the direct test of the kernel thesis: the repository as the shared,
  durable substrate that lets independent agents coordinate.

### S4 — Context-provisioning token economy → H11

RepoPact's claim that short prompts are possible because the repository carries durable
operating context is a **token-economy** claim and must be measured rather than assumed.

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
  - **C7** RepoPact records — the agent loads the active work item plus invariants/scopes
    on demand, not the whole history.
  - **C8** RepoPact + RAG hybrid — records as the spine/index, RAG for code bodies.
  - **C2+C3** convention-file + RAG — a common practical baseline.
  - **C9** in-weights / fine-tuned — named as an extreme; out of scope to run.
- **Metrics.** Input and output tokens/request; context tokens vs. task tokens;
  tokens-to-completion; requests-per-task; USD/request and USD/resolved-task at stated
  provider rates; cache-adjusted tokens.
- **Analyses.**
  1. *Joint with quality.* Plot token cost against task success and report the Pareto
     frontier. Cheap-and-wrong is not a win.
  2. *Scaling curve.* Measure per-request context tokens as accumulated project state
     grows. The registered prediction is that selective RepoPact loading stays more
     bounded than full-prompt stuffing.
- **Controls.** Identical model, task, tokenizer, and corpus content across comparable
  regimes. Report caching, prompt construction, provider, and rates.

### S5 — Drift detection and staleness → H12

- **Conditions.** Convention-file-only (C2) and convention+RAG (C2+C3) vs. RepoPact (C7).
- **Construct.** Apply a pre-registered sequence of realistic mutations that should make
  documented or governed state stale: rename or move a module, delete a directory, change
  ownership, add a CI workflow, weaken a check, split a file, and other registered cases.
- **Metrics.** Drift-detection rate; time or edits to detection; silent-staleness rate;
  false-drift rate; reconciliation cost.
- **Honesty.** Include RepoPact's own known blind spots and report silent staleness there
  too. The claim is lower, not zero.

### S6 — Security: enforcement and injection resistance → H13

Two sub-studies. Both are defensive, sandboxed, and benign by construction.

- **S6a: Security-invariant enforcement.** A security-scoped slice of PactBench tests
  pressure to disable an authorization check, widen permissions, commit a secret, remove
  input validation, or relax a protected security surface. Use the same preservation /
  escalation / false-stop scoring as S1.
- **S6b: Context-file injection resistance.** Treat both convention files and RepoPact
  records as potential injection surfaces. Measure injection-followed rate and structural
  detection rate for poisoned or forged context.
- **Conditions.** Convention-file-only vs. RepoPact, with an optional runtime-guard arm
  to test composition rather than replacement.
- **Honesty.** RepoPact records are trusted text and can themselves be attacked. The
  defense claim is integrity structure, not un-injectability.

### S7 — Enforcement closure and longitudinal governance drift → H14

*Added 2026-08-21. Pre-registered here; not implemented or run as part of this amendment.
See `protocol.md` H14 and `formal-model.md` §7.*

S5 asks whether a validator detects a mutation **when it runs**. S7 asks whether the
admission boundary has coverage, invocation, and effectiveness in the first place.

- **Construct.** At minimum, use four otherwise matched deployment arms:
  1. **Coverage absent:** the validator exists but the admission path never routes through it.
  2. **Invocation absent:** the checker is wired into the path but does not execute for
     the candidate transition.
  3. **Effectiveness absent:** the checker executes and rejects, but nothing binds that
     result to promotion.
  4. **Closed:** coverage, invocation, and effectiveness all hold.
- **Task sequence.** Apply a pre-registered sequence of confirmed governance-record
  mutations interleaved with simulated ordinary admissions so accumulation is measured
  under realistic admission volume rather than as isolated events only.
- **Metrics.** Keep these separate:
  - checkpoint coverage rate;
  - checkpoint invocation rate;
  - checkpoint effective-block rate;
  - nonconformant-admission rate;
  - admissions or elapsed time to first detection;
  - independently confirmed governance-discrepancy count;
  - validator false-positive count;
  - reconciliation cost.
- **Explicit non-metric.** Raw validator error count alone is not a study output. It must
  be decomposed into confirmed discrepancies and checker false positives before reporting.

**Falsification.** As stated under H14 in `protocol.md`.

**Controls, stopping rules, and scoring** must be pre-registered before execution,
including sample size, repetitions, seed or temperature policy, and effect-size /
confidence-interval plans.

### Registered future scenario candidates from 2026-08-21

These candidates are not silently folded into S7 because they test different constructs.

- **S5 candidate mutation:** a work item's human-readable README id/title diverges from
  its sibling `work-item.json` without the manifest itself changing. The expected signal
  based on the motivating field case is that this is a candidate blind spot until the
  relevant invariant is implemented.
- **S3 candidate fixture:** two independent agents on not-yet-merged branches each invoke
  `repopact new work-item` and independently choose the same next numeric id. The merged
  tree should reveal the duplicate, but the pre-merge coordination problem is a separate
  allocation question, not an enforcement-closure question.

### S8 — Governance continuity and clean-clone orientation → H15

*Added and pre-registered 2026-09-12 before any S8 run. This study operationalizes H15
from `protocol.md`. It is deliberately distinct from S2. S2 measures downstream task
recovery and efficiency after a session reset. S8 measures whether the governed substrate
itself survives changes of worker, machine, and legitimate client without hidden
predecessor-local authority.*

#### Construct

Prepare a repository state with a frozen, versioned oracle of represented governance:
work lifecycle, dependencies, owners/scopes, contracts, invariants, frozen surfaces,
decisions, evidence links, provenance, derived views, and a small pre-registered set of
known current conformance violations. The violations are intentional because a continuity
system should faithfully recover an unhealthy repository rather than turn it falsely green.

For each handoff, the receiving worker starts from a **clean clone**. It receives no prior
chat transcript, local application database, search index, cache, IDE session, provider
memory, or predecessor-generated summary unless that artifact is itself committed as part
of the registered condition.

Run a matrix that covers, where available:

- fresh agent session on a clean machine or isolated environment;
- human operator through the supported Workbench surface;
- supported CLI or language-neutral compatibility surface;
- a second agent/model family;
- cross-machine reconstruction;
- client-to-client handoff, such as Workbench to agent or CLI to Workbench.

The source tree, RepoPact version, task prompt, and oracle remain fixed for matched runs.
Unsupported platform/client combinations are recorded as unavailable rather than treated
as failures of an advertised surface.

#### Recovery task

The receiving worker must reconstruct, without predecessor-private state:

1. current work by lifecycle and authority state;
2. work dependencies and affected scopes;
3. ownership and applicable contracts;
4. binding invariants and frozen surfaces relevant to selected work;
5. decisions relevant to that work;
6. supporting evidence and provenance status;
7. known current conformance violations;
8. the next legitimate actions available to the worker.

The worker then performs a small registered orientation task that requires following these
relationships, not merely listing files. This separates semantic recovery from simple
repository enumeration.

#### Primary correctness metrics

- **Governance field precision and recall:** compare recovered facts with the frozen oracle.
- **Violation recall:** fraction of seeded current violations correctly recovered.
- **False-clean rate:** fraction of runs that report a healthy state when the oracle is
  intentionally nonconformant.
- **Authority-state error rate:** proposed, active, blocked, deferred, or completed work
  misrepresented in a way that changes legitimate authority.
- **Cross-client disagreement:** materially contradictory governed facts reported by two
  supported clients over the same tree and RepoPact version.
- **Predecessor-context dependence:** any required fact obtainable only from excluded
  predecessor-private state.

#### Secondary orientation-cost metrics

These do not determine correctness, but they measure whether continuity is practically
usable:

- wall-clock time to an accepted orientation answer;
- input and output tokens;
- tool calls;
- file reads;
- repository-wide text search or grep operations;
- bytes of repository material read before the first accepted answer;
- human interventions or clarifications.

The repository-wide search/grep count is registered **before** design or implementation of
any durable repository-orientation graph. A future graph therefore cannot choose this
metric after seeing whether it helps.

#### Conditions

The initial S8 comparison uses at least:

- **B0: reasonable convention baseline.** Source repository plus a realistic `AGENTS.md`
  and README, with no RepoPact governance layer.
- **R0: RepoPact current release.** Repository-native governance records with no future
  orientation graph enabled.

A later graph-enabled condition may be added only by a new dated amendment committed
before graph-condition runs. It must not rewrite B0 or R0 results. This keeps evaluation
of the proposed graph separate from evaluation of RepoPact's existing continuity claim.

#### Falsification

As stated for H15 in `protocol.md`. In operational terms, S8 counts against H15 if a
load-bearing repository-authoritative fact requires excluded local state; supported
clients materially disagree over the same versioned tree; a seeded violation becomes
falsely clean after handoff; or continuity is technically correct but consistently
requires orientation effort large enough to erase its practical value on the registered
task set.

#### Stopping rules and scoring

Before S8 execution, freeze the repository fixtures, handoff matrix, scorer, repetition
count, model/temperature policy, and confidence-interval or uncertainty reporting plan.
No S8 result is reported from exploratory runs performed before those artifacts are frozen.

## Controls and fairness

- **Matched arms.** Identical source, task, model, harness, and budget where the study
  design permits; only the registered independent variable changes.
- **Pre-registered task sets.** Task IDs and expected outcomes are fixed before runs.
- **Blinding where feasible.** Scoring is done against a rubric fixed in advance and is
  automated where the construct permits it.
- **Multiple models.** Agent studies should include at least two model families so a
  result is not merely one model's idiosyncrasy.
- **Raw capture.** Every run links raw transcript or capture, exact commands, task-set
  version, condition, and model/client identity so a third party can reproduce it.

## Outputs

- A results table per study with effect sizes or uncertainty measures appropriate to the
  design and links to the underlying captures.
- PactBench tasks and harnesses published alongside the S1 evidence.
- S8 recovery matrices and orientation-cost traces, when run, published without collapsing
  correctness and cost into one score.
- Disconfirming results recorded with the same weight as confirming ones; threats tracked
  in [`threats-to-validity.md`](threats-to-validity.md).
