# The Repository as the Operating System for Agentic Work

**A repository-native governance kernel for durable collaboration between humans and coding agents**

Jeremy Shows — ForgeWire Labs
Draft, 2026-06-24. Target: arXiv cs.SE preprint → workshop (BoatSE / ICSE-FSE ideas or tool track).

---

## Abstract

Coding agents lose the load-bearing state of a project at every conversation boundary —
the intent behind a change, the guarantees it must preserve, the decisions already made,
and the proof that anything worked. We call this **session amnesia** and argue it is a
memory and authority problem, not a reasoning one. We present **RepoPact**, a
repository-native governance kernel that keeps this state as typed, version-controlled
records organized in six layers (L0–L5): a record store, a per-work-item lifecycle
automaton, an invariant monitor checked at commit/CI boundaries, a typed enforcement
lattice, a derive layer, and an adoption boundary. Its distinguishing primitive is the
**binding invariant** — a declared guarantee with a rationale, an escalation path, and,
where its logical type permits, a machine enforcer — the unit an agent must not silently
weaken. We give the kernel an operational semantics in which a reference validator is the
characteristic function of the *conformant* repositories; show that brownfield adoption
obeys a **trilemma** (total, faithful, closed — pick two) bounding what migration can
promise; and explain why an invariant's logical *kind* predicts its enforcement mechanism.
We evaluate reflexively and adversarially against pre-registered falsification criteria on
the *packaged* product: two defects cracked the architecture and were fixed and
re-verified, while a battery of adversarial cases were caught. We pre-register a six-study
comparative benchmark suite — guarantee-violation detection, cross-session recovery and
efficiency, multi-agent coordination, context-token economy, drift, and security — and
commit to reporting results honestly, including the cases RepoPact does *not* catch.

**Keywords:** agentic software engineering; coding agents; repository governance; software
invariants; durable agent memory; evidence-gated workflows; brownfield adoption; benchmark
pre-registration.

---

## 1. Introduction

A coding agent is handed a task, loads context, edits code, and finishes a session. The
next session — perhaps a different agent, or the same one after a context reset — starts
without the intent behind the last change, the guarantees the change was supposed to
preserve, the decisions already made, or the proof that anything actually worked. We call
this the **session-amnesia problem**. It is not a reasoning failure; it is a *memory* and
*authority* failure, and it compounds as more of the work is delegated to agents.

The ecosystem's current answers each capture a fragment. `AGENTS.md` and its kin
(`CLAUDE.md`, editor rule files) give an agent instructions; agent-memory frameworks give
it a runtime store beside the process; ADRs record decisions; CI gates and policy-as-code
enforce one rule at a time. None of them makes the *binding* part of a project —
the guarantees an agent must not silently weaken — both explicit and machine-checkable,
and recoverable by a third party from the repository alone.

This paper makes four contributions:

1. **A repository-native model** (§3) in which durable agentic memory and governance live
   as typed records in the version-controlled tree, organized as a six-layer kernel
   (L0–L5). The lifecycle finite-state machine is one layer (L1), composed with an
   invariant monitor (L2) checked at commit/CI boundaries.
2. **The binding invariant as a first-class primitive**, and a *typed invariant lattice*
   (§3.3) showing why an invariant's logical kind (state / transition / temporal /
   relational / meta) predicts whether and how a machine can enforce it.
3. **An adoption trilemma** (§3.5): no migration of a RepoPact-naive repository can be
   simultaneously total, faithful, and closed under the conformance language. RepoPact
   relaxes *closed*, producing a sound fresh pact plus a reported worklist rather than a
   false acceptance — which bounds the brownfield claim honestly.
4. **A reflexive, adversarial evaluation** (§5–§6): the architecture is used to govern its
   own evaluation, and pre-registered falsification criteria are run against the *packaged*
   product. We report what cracked and what held, and pre-register a comparative benchmark
   suite for the value claims.

RepoPact is open source (Apache-2.0) and is the work-governance layer of a larger
inspectable-infrastructure stack (ForgeWire Labs); this paper concerns RepoPact alone,
which is independently useful.

## 2. Background and related work

**Agent context files.** `AGENTS.md` has become the de facto agent-instruction standard
(tens of thousands of repositories; foundation governance; native support across major
coding agents). It is deliberately schema-free Markdown: *instructions*, not an enforced
contract. RepoPact is positioned as the enforcement/evidence layer above it and *consumes*
it during adoption; it does not compete with it. `CLAUDE.md` and editor rule files occupy
the same instruction niche.

**Agent memory and state.** External stores, retrieval-augmented generation, scratchpads,
and runtime memory frameworks (e.g. MemGPT/Letta, Mem0, Zep, LangMem) give an agent memory
*beside the process*. That memory is generally not versioned, not auditable as part of the
code's history, and not recoverable by a third party from the repository alone. RepoPact's
memory is the repository: diffable, reviewable, and recoverable without the agent.

**Agent governance architectures.** Recent work proposes layered or security-oriented
governance that operates at *runtime* — for example a Layered Governance Architecture of
execution sandboxing, LLM-judge intent verification, zero-trust inter-agent authorization,
and immutable audit logging (arXiv:2603.07191), and six-layer reference architectures for
agentic software development (arXiv:2604.26275). These are runtime controls or descriptive
taxonomies. RepoPact differs on *substrate* (the version-controlled repository rather than
the running agent) and on *unit* (the binding invariant as an evidence-gated, git-versioned
primitive). The two compose: a runtime intent-checker and a repository-native invariant
monitor guard different boundaries.

**Decision records and policy-as-code.** ADRs record that a decision was made; conventional
commits structure history; OPA/Conftest, CI gates, and architecture fitness functions each
enforce one slice; developer portals (Backstage, Cortex, OpsLevel) score services at
organizational scale. RepoPact composes with these and unifies *enforcer + rationale +
escalation* in the repository itself, gating work completion on evidence.

## 3. The model

The kernel comprises six layers (Figure 1), ordered by how much of the repository's
environment they touch (L0–L3 internal, L4 derives, L5 crosses the boundary to state the
repo does not contain).

| Layer | Name | Object |
|---|---|---|
| L0 | Record store | the typed tree `s` |
| L1 | Lifecycle FSM | per-work-item automaton `M_w` |
| L2 | Invariant monitor | predicate `I`; conformant language `R` |
| L3 | Enforcement lattice | invariants typed → tiered enforcers |
| L4 | Derive layer | projections `π` (dashboard, SPEC) |
| L5 | Adoption boundary | migration of naive/external reality → the pact |

### 3.1 State (L0)

A repository state is a finite typed record store
`s = ⟨ver, Inv, Frz, Own, Reg, C, W, E, D, P, A⟩`: version, invariants, frozen surface,
owners/scopes, audit registry, contracts (`AGENTS.md` nodes), work items, evidence runs,
decisions, policies, and audit findings, each with a fixed source location in the tree. A
work item `w` carries a status `σ(w) ∈ {active, blocked, deferred, completed}` and a
lifecycle directory `dir(w)`; their *agreement* is an invariant, not an assumption. The
state space is infinite (record sets are unbounded), so the repository is an
infinite-state transition system; finite-state structure is confined to the per-item
lifecycle (§3.2).

### 3.2 The lifecycle automaton (L1)

Per work item, the lifecycle is a finite automaton (Figure 2) over `{active, blocked,
deferred, completed}` with `active` initial. Transitions are total — any state may move to any state,
because degradation must be explicit (a blocked or deferred item is first-class, not a
failure) — with a single guard on edges into `completed`: every acceptance criterion is
non-pending, and every satisfied criterion links real evidence. Crucially, the guard is
**not** a runtime gate: a `git mv` can move an item into `work/completed/` with pending
criteria. The resulting state is simply *non-conformant*, and the commit is rejected at the
checkpoint (§3.3). L1 supplies reachable control points; L2 decides admissibility.

### 3.3 The invariant monitor (L2) and the conformant language

The reference validator computes a finite set of atomic violations `Viol(s)`. Define
`I(s) ≡ Viol(s) = ∅` and `R = {s : I(s)}`. `R` is the set of **conformant** repositories,
and the validator is its characteristic function `χ_R`. `I` decomposes into atomic
predicates, each tied to a specification rule and a code site: structural schema validity,
identifier/path/status agreement, referential integrity, the acceptance predicate
(completed ⟹ no pending criterion; satisfied ⟹ linked evidence), dependency acyclicity,
optional disjoint-scope concurrency, and an orphan check (no planning artifact invisible to
the ledger).

The **monitor non-bypass** property is the safety backbone: for *any* edit trace
(arbitrary filesystem edits, not just CLI actions), the CI checkpoint admits a commit iff
the resulting state is in `R`. No invalid state passes a checkpoint.

### 3.4 The typed invariant lattice (L3)

RepoPact's invariants are not of one logical kind, and the kind predicts the enforcer:

| Type | Example invariant | Enforcer |
|---|---|---|
| state | completed ⟹ no pending criterion; satisfied ⟹ evidence | validator (one tree) |
| state (fixpoint) | derived artifacts equal their projection | CI dashboard-diff |
| transition (2-state) | frozen-surface change ⟹ operator approval | diff-time `check-frozen --base` |
| temporal / historical | completed work is never rewritten to look cleaner | human review + git |
| relational / refinement | a deeper contract refines, never weakens, its parent | human review |
| meta (coverage) | no critical state lives only in conversation | human + partial orphan check |

As the type ascends from state to transition to temporal to relational to meta, the
enforcer moves from the validator to a diff-time checker to human review — because that is
what is decidable on a single tree. A two-state property (frozen surface) needs a diff; a
historical property (no history rewrite) needs the git trace; a refinement property needs
an order on contracts. This typing is itself a contribution: it explains the enforcement
mechanisms and tells us what a fourth mechanism would have to be.

### 3.5 The adoption boundary (L5) and the trilemma

Constructor (`init`), invariant-preserving (`new`, guarded completion, `doctor --fix`),
and derive/read actions behave well with respect to `R`. **Migration** actions (`adopt`,
`import-plan`) do not, and cannot (Figure 3). A migration over a RepoPact-naive (or partially
external) project faces three requirements:

- **Total** — defined on any input tree;
- **Faithful** — maps existing signals to records without fabricating records or discarding
  signals;
- **Closed** — every output lies in `R`.

No migration satisfies all three, because input trees contain configurations `R` forbids
(a nested contract naming an unestablished scope; a roadmap with cyclic blocked-by edges; a
checklist item marked done with no evidence). Forcing such a tree into `R` requires either
inventing the missing record — e.g. synthesizing an evidence run, which manufactures false
proof and violates the evidence invariant — or discarding the offending signal, which
breaks faithfulness. RepoPact relaxes *closed*: it retains total and faithful, emits the
best sound records it can, and reports the residue as a validator-generated worklist,
yielding a *fresh pact* rather than a false acceptance. A future **provenance type** on
records (`concrete` vs `inferred`/`provisional`) is the principled escape: an inferred
record labels itself as reconstructed rather than proven, remaining faithful while letting
adoption approach `R` more closely (§7).

## 4. Reference implementation

The validator is the reference semantics: it *defines* `R` as its accept set, so it is the
characteristic function by construction; for an *alternative* implementation, recognizer
soundness and completeness becomes a theorem to be tested against a fixture corpus. The
CLI surface (`init`, `new`, `validate`, `dashboard`, `spec`, `check-frozen`, `adopt`,
`import-plan`, `doctor`) maps onto the action taxonomy of §3.5. Records are validated in
two layers — JSON Schema for structure, the validator for cross-record semantics — and the
dashboard and specification blocks are *derived*, never hand-authored (derive-over-declare).
A published, versioned **conformance suite** (a manifest mapping each fixture to the rule
it exercises, plus a runner) lets a third-party implementation claim conformance; this is
the artifact that makes "RepoPact is a standard" runnable rather than rhetorical.

## 5. Evaluation method

The evaluation has two parts. The first is complete; the second is pre-registered with
results forthcoming.

**5.1 Reflexive, adversarial falsification (complete).** We set hypotheses H1–H7 and
explicit falsification criteria *before* the runs. The subject under test is RepoPact *as
distributed* — the installed wheel in a clean environment, not the source checkout, because
testing the checkout would test the authors' machine rather than the product an adopter
receives. A throwaway but genuinely working CLI adopts the packaged product and is driven
through the happy path and through each falsification case. Every defect is fed back into
RepoPact using RepoPact's own machinery (an audit finding, a work item, a decision), which
is itself a test of recoverability (H6). Hypotheses span adoptability (H1), surface closure
(H2), evidence-gated completion (H3), binding authority (H4), state integrity (H5),
recoverability (H6), and brownfield adoptability (H7).

**5.2 Comparative benchmarks (pre-registered; results forthcoming).** Reflexive
falsification shows whether the architecture *catches what it claims* on one governed
subject; it does not measure whether governing a repository *changes agent behaviour*
versus not. A companion protocol pre-registers six studies with a held-constant design
(source, task, model, harness fixed; only the governance layer varies):

- **S1 (H8)** *PactBench* — guarantee-violation catch/escalate rate, reported as a
  confusion matrix with a false-stop control;
- **S2 (H9)** cross-session recovery and efficiency on long-horizon beds (SWE-bench
  Verified, SWE-EVO);
- **S3 (H10)** multi-agent coordination through the repository vs a shared scratchpad;
- **S4 (H11)** context-provisioning token economy across regimes (full-prompt,
  convention-file, RAG, summarized/external memory, on-demand fetch, RepoPact, hybrids),
  reported as a cost-vs-success Pareto frontier and a scaling curve;
- **S5 (H12)** drift detection / staleness vs convention files;
- **S6 (H13)** security — invariant enforcement (S6a, a security slice of PactBench) and
  context-file injection resistance (S6b).

Task sets and per-task expected outcomes are committed before runs; disconfirming results
are recorded with equal weight; fairness threats (caching/pricing for S4; baseline
strawman; task realism and defensive scoping for S5–S6) are tracked explicitly.

## 6. Results

**6.1 What cracked.** Two defects falsified hypotheses partially and were fixed and
re-verified from a rebuilt package. **F-001** (H2): a documented command crashed on an
`init`-fresh repository because it read a specification file the bootstrap does not seed —
the output was not closed under the tool's own surface; resolved by making the command fail
cleanly with guidance, recorded in a decision. **F-002** (H4): the frozen-surface check
diffed only committed ranges, so a *working-tree* edit to a protected file produced a false
"all clear" before commit; resolved by unioning the committed range with uncommitted
changes, so local edits are caught while CI still sees exactly the branch's commits.

**6.2 What held.** A series of adversarial cases were correctly caught (recorded as
evidence *for* the design): a criterion marked satisfied with no evidence is rejected
(F-003); a work item whose status disagrees with its directory is rejected (F-004); a
committed change to a protected path forces an explicit acknowledgement (F-005); and a
reader reconstructed an entire work item — intent, decision, and passing proof — from the
tree alone, with no chat history (F-006).

**6.3 Brownfield adoption.** `adopt` converted a real 4,569-commit application into a
conformant RepoPact non-destructively, mapping nested contracts, CODEOWNERS scopes, and CI
workflows (F-007); however, this subject is RepoPact's *progenitor* and is therefore
**confirmatory, not independent** (§8, T1). Independent adoption of an unrelated OSS project
(F-009) and of a different-domain commercial app (F-012) reached conformant RepoPact,
partially closing the generality gap. Real brownfield adoption surfaced structural defects
that only a real repository would: a pre-existing `.gitignore` silently un-tracking
governance records (F-008, fixed by an adopt-time warning), a hollow work ledger beside an
existing tracker (F-010, fixed by `import-plan`), and — most relevant to the drift study —
an older adopter drifting *invalid* as the standard evolved with nothing detecting or
guiding the upgrade (F-011, fixed by `doctor`). F-011 is recorded honestly as a limit:
longitudinal drift is not auto-detected at edit time.

**6.4 Comparative results.** Forthcoming. The benchmark suite (§5.2) is pre-registered;
this draft will be updated with the cost-vs-success Pareto frontier and the per-request
scaling curve (S4; Figures 4–5), the catch-rate confusion matrix and injection-resistance
figures (S1/S6; Figure 6), and the drift latency/staleness numbers (S5) as runs complete
across ≥2 model families.

## 7. Discussion

**Cost versus recoverability.** RepoPact is ceremony, and ceremony has a cost. The claim is
not that it is always worth it, but that it is worth it precisely when a guarantee is
load-bearing and an agent is fast, literal, and forgetful — increasingly the default. The
S4 study counts ceremony overhead *against* RepoPact, so an efficiency gain that is
cancelled by governance cost is reported as a non-result.

**The L5 boundary.** Much of the memory that makes coordinated agentic work possible —
plans with live updates, design intent, prior results, the history of *why* — lives outside
the repository, in trackers and documents that were never committed. `adopt` builds the
best sound starting point reachable from inside the repo and no more; the gap between the
pact and the project's true state is real, unfabricable drift, and the correct response is
to *state* it, not to paper over it with invented records. Two L5 directions follow:
external ingestion (bring trackers/docs across the boundary as evidence-bearing records)
and provenance-typed records that ratchet from `inferred` to `concrete` as evidence
arrives.

**When repository-native governance is the right tool.** When state must survive the
session, when guarantees must not be silently weakened, and when a third party (human or
agent) must be able to reconstruct what happened from the artifact itself. When those do
not hold, an instruction file is enough.

## 8. Threats to validity

- **T1 — Reflexivity (central).** RepoPact was distilled from one project's practices; that
  project adopting cleanly is confirmatory, not independent. Mitigation: adopt repositories
  with no lineage to RepoPact and report holds or cracks (partially done: F-009, F-012;
  still open: an independent repo with CODEOWNERS *and* nested contracts).
- **T2 — Single evaluator/session.** All evidence is from one operator/agent pair; every
  run links a raw capture and exact commands for third-party reproduction.
- **T3 — Scale/domain narrowness.** The greenfield subject is trivial and the brownfield
  subjects are individual apps; the conformance fixtures and further adopters broaden
  coverage.
- **T4 — "Holds" is absence-of-failure.** A caught adversarial case is not a proof of no
  bypass; the hypothesis list is falsifiable and additive.
- **T5 — Benchmark selection/curation.** Pre-registration and multi-model runs guard S1–S6.
- **T6 — Baseline fairness.** The baseline carries a genuine convention file over identical
  source; ceremony cost counts against RepoPact.
- **T7 — Token-measurement fairness.** Caching, tokenizer, and pricing are controlled;
  raw, cache-adjusted, and USD figures reported per request and per task.
- **T8 — Drift/security realism and scoping.** Mutations and security tasks are realistic,
  defensive, sandboxed; RepoPact's own records are evaluated as an attack surface (no
  immunity claim); RepoPact composes with runtime guards rather than replacing them.

## 9. Conclusion and future work

The session-amnesia and silent-weakening problems are memory and authority problems, and
the version-controlled repository is the substrate that already has the properties their
solution needs: durability, diffability, third-party recoverability, and a natural
commit/CI checkpoint. RepoPact turns that substrate into a six-layer governance kernel
whose distinguishing primitive — the binding invariant — is what an agent must not silently
break. We gave the kernel an operational semantics, identified an adoption trilemma that
bounds brownfield honesty, and evaluated the architecture reflexively and adversarially,
reporting both the defects that cracked it and the cases it caught. The comparative value
claims are pre-registered and forthcoming. Future work: run the benchmark suite; mechanize
the historical and refinement invariants (a trace semantics over git; a refinement order on
contracts); and cross the L5 boundary with external ingestion and provenance-typed records,
toward a repository that holds more of the working memory coordinated agents need.

---

## Ethics and responsible disclosure

The security study (S6) is **defensive and benign-by-construction**: tasks are sandboxed,
operate on synthetic fixtures, and involve no live targets or real exploit development.
Injection corpora are synthetic and contained. RepoPact's *own* records are evaluated as an
attack surface — we make no immunity claim — and RepoPact is framed as composing with
runtime guards, not replacing them. The evaluation involves no human subjects; agent runs
respect model-provider terms. Defects found in RepoPact during evaluation are recorded
openly as findings (F-001…F-013) rather than quietly patched.

## Data and artifact availability

RepoPact is open source under Apache-2.0. The formal model (`formal-model.md`), the
experiment protocol and its amendment (`protocol.md`), the comparative benchmark protocol
(`benchmark-protocol.md`), the threats register (`threats-to-validity.md`), the findings
register (`findings.md`), and the raw proving-ground captures (`captures/`) are in the
repository. Benchmark task sets (`benchmarks/`) are **pre-registered**: committed before
runs, with corrections issued as new task ids rather than silent edits. A versioned
conformance suite (work item 019) lets a third party reproduce the recognizer result.

## Appendices

- **A. Typed invariant lattice** — the seven invariants, their logical type, and enforcer
  (§3.4; full table in `formal-model.md` §5).
- **B. Findings register** — F-001…F-013 with severities and citing captures
  (`findings.md`).
- **C. Conformance** — the recognizer theorem and the fixture corpus (`formal-model.md` §6;
  conformance suite, work item 019).
- **D. Benchmark protocol** — studies S1–S6, hypotheses H8–H13, falsification criteria, and
  fairness threats (`benchmark-protocol.md`, `protocol.md` amendment, `benchmarks/`).

### E. Figures and tables (planned)

The draft references these; mockups are filled with real data as runs complete. Draft
mockups for all six (ASCII for the structural figures, illustrative data for the plots) live
in [`figures.md`](figures.md); Figure 6 is emitted directly by the benchmark harness.

- **Figure 1.** The six-layer kernel (L0–L5): object per layer and how much of the
  environment each touches. *(§3)*
- **Figure 2.** The work-item lifecycle automaton: the four states, the guarded edge into
  `completed`, and checkpoint composition with the L2 monitor. *(§3.2)*
- **Figure 3.** The adoption trilemma: total / faithful / closed as overlapping
  requirements, with RepoPact relaxing *closed* (the fresh-pact + worklist region). *(§3.5)*
- **Table 1.** The typed invariant lattice: each invariant's logical kind → enforcer
  (validator / diff-time / human). *(§3.4)*
- **Table 2.** Studies S1–S6 → hypothesis → primary metric → status (complete / forthcoming).
  *(§5.2)*
- **Table 3.** Reflexive findings F-001…F-013: hypothesis, severity, outcome
  (cracked / holds), citing capture. *(§6)*
- **Figure 4 (mock→real).** S4 cost-vs-success Pareto frontier across context regimes
  (C0–C9 + C2+C3). *(§6.4)*
- **Figure 5 (mock→real).** S4 scaling curve: per-request *context* tokens vs accumulated
  project state, per regime — the headline figure. *(§6.4)*
- **Figure 6 (mock→real).** PactBench confusion matrix, baseline vs RepoPact, with the
  false-stop control. *(§6.4)*

(Note: figure/table numbering will be reconciled at typesetting; the prose currently cites
Figures 1–6 informally.)

## References (informal)

- AGENTS.md — agent instruction standard (Linux Foundation Agentic AI Foundation).
- Layered Governance Architecture for autonomous agent systems — arXiv:2603.07191.
- Agentic AI in the software development lifecycle (six-layer reference architecture) —
  arXiv:2604.26275.
- Controlled benchmarks of agent memory in coding agents (efficiency-gain methodology).
- SWE-bench Verified; SWE-EVO (long-horizon software evolution).
- Internal: `formal-model.md`, `protocol.md`, `benchmark-protocol.md`,
  `threats-to-validity.md`, `findings.md`, and the proving-ground captures.
