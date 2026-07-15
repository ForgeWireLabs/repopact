# The Repository as the Operating System for Agentic Work

**A repository-native contract, evidence, and conformance layer for durable coding-agent work**

Jeremy Shows
ForgeWire Labs
Draft, 2026-06-25; revised 2026-07-15
Target: arXiv cs.SE preprint, then software engineering or agentic systems workshop submission

## Abstract

Coding agents lose the load-bearing state of a project at every conversation boundary: the intent behind a change, the guarantees it must preserve, the decisions already made, the authority boundaries that constrain it, and the evidence that anything worked. We call this **session amnesia** and argue that it is primarily a memory, authority, and governance problem, not merely a reasoning problem. We present **RepoPact**, a repository-native governance kernel that keeps project state as typed, version-controlled records in the filesystem. RepoPact organizes this state into six layers, L0 through L5: a record store, a per-work-item lifecycle automaton, an invariant monitor checked at commit and CI boundaries, a typed enforcement lattice, a derive layer, and an adoption boundary for bringing external or previously ungoverned project memory into the pact.

RepoPact's distinguishing primitive is the **binding invariant**: a declared guarantee with a rationale, an escalation path, and, where its logical type permits, a machine enforcer. A binding invariant is the unit an agent must not silently weaken. We give RepoPact an operational semantics in which the reference validator is the characteristic function of the conformant repository language. We further show that brownfield adoption exposes a concrete-record trilemma: a migration from a RepoPact-naive project cannot be simultaneously total, faithful, and closed if every emitted record must be asserted as concrete. RepoPact 2.0 resolves this by extending the record language with provenance types. A reconstructed record may be valid as `inferred` or `provisional` without pretending to be proven. Adoption can therefore be total, faithful, and closed under the provenance-aware conformant language, while completion and concrete claims remain evidence-gated.

We evaluate RepoPact reflexively and adversarially against pre-registered falsification criteria on the packaged product, not merely the source checkout. Defects that cracked the architecture were recorded as findings, fixed upstream, and re-verified from rebuilt packages. We also pre-register a comparative benchmark program for value claims: guarantee-violation detection, cross-session recovery and efficiency, multi-agent coordination, context-token economy, drift detection, and defensive security enforcement. The runnable benchmark artifacts now live in RepoPact Proving Ground, a separate public adversarial lab that consumes RepoPact from PyPI. Its PactBench suite includes pre-registered tasks, real fixtures, model-agnostic execution, deterministic plumbing checks, a subprocess interface for real agent runs, and drift-harness support. Comparative model results remain forthcoming and will be reported whether confirming or disconfirming.

**Keywords:** agentic software engineering; coding agents; repository governance; software invariants; durable agent memory; evidence-gated workflows; brownfield adoption; provenance typing; benchmark pre-registration; agent evaluation.

## 1. Introduction

A coding agent is handed a task, loads context, edits code, and finishes a session. The next session, perhaps a different agent, perhaps the same one after context reset, often starts without the intent behind the last change, the guarantees the change was supposed to preserve, the decisions already made, the authority boundaries that constrained the work, or the proof that anything actually worked. The repository may contain the resulting code, but not the operational memory that made the code safe to change.

We call this the **session-amnesia problem**. It is not simply that agents forget facts. Human collaborators also forget, rotate, misread, and lose context. The deeper problem is that load-bearing project state often lives outside the artifact that future work depends on. It is in chat logs, prompt transcripts, issue comments, private planning documents, local scratchpads, model context windows, and implicit operator knowledge. When that state is absent from the repository, a future human or agent can reproduce the files but not the governing intent.

This becomes more severe as coding agents become faster and more autonomous. A capable model can satisfy a local instruction while weakening a global guarantee. It can delete a failing assertion instead of satisfying it. It can mark work complete without linking evidence. It can edit a protected surface because the instruction file did not make the authority boundary enforceable. It can comply with a stale or injected instruction because there is no typed record distinguishing asserted, inferred, provisional, and proven state. The problem is not lack of intelligence. The problem is lack of durable, inspectable, enforceable project memory.

The ecosystem's current answers each capture a fragment.

`AGENTS.md`, `CLAUDE.md`, editor rules, and similar context files tell an agent how to behave. They are useful, but they are instructions, usually Markdown, with no built-in record typing, evidence semantics, lifecycle state, conformance language, or enforcement boundary. Agent-memory frameworks give models recall beside the process. They may improve continuity across calls, but the memory is usually outside the repository, weakly versioned, hard to audit as code history, and not independently recoverable by a third party from the artifact alone. ADRs record decisions, but they generally do not enforce acceptance criteria or bind work completion to evidence. CI and policy-as-code enforce particular checks, but they do not normally encode the rationale, authority scope, escalation path, work lifecycle, and evidence ledger as a unified project contract.

RepoPact starts from a different substrate: the version-controlled repository itself. The repository already has the properties a durable agent-work substrate needs. It is persistent. It is diffable. It is reviewable. It is replicated. It is naturally checked at commit and CI boundaries. It already carries the code, tests, history, and change-control mechanisms that software engineering trusts. RepoPact turns that repository into a governance kernel for agentic work. The phrase "repository as operating system" is used here in a restricted systems sense: RepoPact does not orchestrate all agent runtime behavior, but supplies the repository-native contract, evidence, lifecycle, provenance, and conformance layer through which durable work is admitted.

The central distinction is this:

> `AGENTS.md` tells an agent how to behave. RepoPact enforces and records whether the work respected the contract.

RepoPact does not replace `AGENTS.md`. It consumes it as a contract signal during adoption and positions itself above it as the layer that records authority, binds invariants, gates completion on evidence, tracks provenance, derives dashboards and specifications, and surfaces drift.

This paper makes five contributions.

1. **A repository-native governance model.** We model durable agentic memory and authority as typed records in the version-controlled tree, organized as a six-layer kernel, L0 through L5. The lifecycle finite-state machine is one layer, composed with an invariant monitor checked at commit and CI boundaries.

2. **The binding invariant as a first-class primitive.** A binding invariant is a declared guarantee with a rationale, an escalation path, and, where possible, a machine enforcer. It is the unit an agent must not silently weaken.

3. **A typed enforcement lattice.** We classify invariants by logical kind: state, state fixpoint, transition, temporal, relational, and meta. The kind predicts the enforcer. Some invariants are decidable on one tree. Some require a diff. Some require git history. Some require human review or future formalization.

4. **A brownfield adoption trilemma and a provenance-typed resolution.** In a concrete-only record language, no migration from a RepoPact-naive project can be total, faithful, and closed. RepoPact 2.0 resolves this by adding provenance-typed records. Reconstructed state can be valid as `inferred` or `provisional` without being falsely asserted as concrete, while completion remains gated on concrete evidence.

5. **A reflexive and adversarial evaluation program.** RepoPact is evaluated against its own claims using the packaged product, a public Proving Ground, adversarial findings, a formal model, a machine-checkable conformance suite, and pre-registered comparative benchmarks. Results that disconfirm RepoPact are in scope by design.

RepoPact is open source under Apache-2.0. It is the work-governance layer of the ForgeWire Labs inspectable-infrastructure stack, but this paper concerns RepoPact as an independently useful artifact.

## 2. Background and Related Work

### 2.1 Agent context files

Agent context files such as `AGENTS.md`, `CLAUDE.md`, `.cursor/rules`, and editor-specific rule files have become a practical standard for orienting coding agents. `AGENTS.md` alone, now stewarded under the Linux Foundation's agentic-AI umbrella, appeared in tens of thousands of public repositories within its first year. They are easy to read, easy to write, and easy to adopt. They tell the agent about coding style, test commands, project structure, preferred tools, and local conventions.

Their strength is also their boundary. They are advisory text. They do not, by themselves, provide record schemas, evidence-gated work items, lifecycle state, provenance typing, acceptance semantics, conformance fixtures, or machine-enforced frozen surfaces. A context file can say, "Do not weaken authentication." It cannot, by itself, prove that the work item preserved the authentication invariant, reject completion without evidence, or record that an adoption fact was reconstructed rather than proven.

RepoPact treats context files as input, not as competition. During adoption, nested `AGENTS.md` files are registered as contracts. Their scopes are mapped where possible. Their content remains available to agents. RepoPact adds the enforcement and evidence layer above them.

### 2.2 Agent memory and external state

External memory systems, retrieval-augmented generation, persistent scratchpads, vector stores, agent databases, and runtime memory frameworks — MemGPT and its Letta successor, Mem0, Zep, LangMem, and similar systems — give agents access to state beyond a single context window. These systems are valuable, especially for personalization, long-running assistants, and task continuity. However, they hold memory beside the agent process rather than inside the artifact of record. That creates several problems for software engineering:

1. The memory may not be versioned with code.
2. The memory may not be reviewed during pull requests.
3. The memory may not be reproducible by a third party.
4. The memory may not survive tool or provider changes.
5. The memory may not be visible to the next agent unless the same runtime is used.
6. The memory may preserve facts without preserving authority, acceptance criteria, or evidence.

RepoPact's memory is repository-native. It is intentionally boring in the software-engineering sense: files, schemas, records, git diffs, CI checks, generated dashboards, and explicit decisions. It is not meant to replace all external memory. It is meant to ensure that load-bearing project memory crosses the boundary into the artifact future work depends on.

### 2.3 Governance architectures and runtime controls

Recent agent-governance work often focuses on runtime control: sandboxing, intent verification, tool authorization, zero-trust agent-to-agent communication, LLM-judge review, audit logging, and policy enforcement around live execution. Layered governance architectures and six-layer agentic-SDLC reference models are representative: they organize runtime controls or provide taxonomies over the development lifecycle. These controls are important. They guard the running agent and the operational environment. But none of them is repository-native, and none makes the binding guarantee an evidence-gated, version-controlled primitive.

RepoPact guards a different boundary. It governs the repository as the durable substrate of work. Runtime controls ask whether an agent may perform an action now. RepoPact asks whether the repository records enough intent, authority, invariants, evidence, provenance, and history for the action to remain inspectable and recoverable later. The two approaches compose. A runtime sandbox can prevent dangerous execution. A repository-native invariant monitor can prevent silent weakening of a declared project guarantee.

### 2.4 Decision records, policy-as-code, and architecture fitness functions

ADRs record decisions. Conventional commits structure change history. OPA, Conftest, CI gates, and architecture fitness functions enforce particular policies. Developer portals and scorecard tools such as Backstage, Cortex, and OpsLevel evaluate services at organizational scale. Each enforces one slice, at service or CI granularity. RepoPact composes with these but differs in its unit of governance.

RepoPact unifies:

* the declared guarantee,
* the rationale,
* the authority boundary,
* the escalation path,
* the work item,
* the acceptance criteria,
* the linked evidence,
* the lifecycle transition,
* the provenance type,
* the generated dashboard,
* and the drift surface.

This is why the binding invariant is central. It is not merely a check. It is a guarantee embedded in a record system that future humans and agents can recover.

## 3. The Model

RepoPact is modeled as a layered governance kernel over a repository. The kernel comprises six layers:

| Layer | Name                | Object                        | Role                                         |
| ----- | ------------------- | ----------------------------- | -------------------------------------------- |
| L0    | Record store        | typed repository state `s`    | stores source records                        |
| L1    | Lifecycle FSM       | per-work-item automaton `M_w` | models work states                           |
| L2    | Invariant monitor   | predicate `I`; language `R`   | decides conformance                          |
| L3    | Enforcement lattice | typed invariants              | maps invariant kind to enforcer              |
| L4    | Derive layer        | projections `π`               | generates dashboard and spec artifacts       |
| L5    | Adoption boundary   | migration and external memory | brings naive or external state into the pact |

L0 through L3 are internal to the governed tree. L4 derives artifacts from the tree. L5 is the boundary where the repository meets state it does not yet contain, including trackers, design documents, conversation history, and legacy planning systems.

### 3.1 State, records, and provenance

A repository state is a finite typed record store:

```text
s = <ver, Inv, Frz, Own, Reg, C, W, E, D, P, A, Prov>
```

where:

| Symbol | Component                                   | Source location                          |
| ------ | ------------------------------------------- | ---------------------------------------- |
| `ver`  | semantic version string                     | `VERSION`                                |
| `Inv`  | declared invariants                         | `governance/invariants.json`             |
| `Frz`  | frozen surface definitions                  | `governance/frozen-surface.json`         |
| `Own`  | scopes, roles, and concurrency rules        | `governance/owners.json`                 |
| `Reg`  | audit registry                              | `audits/registry.json`                   |
| `C`    | contract files, including `AGENTS.md` nodes | `**/AGENTS.md`                           |
| `W`    | work items                                  | `work/<status>/<id-slug>/work-item.json` |
| `E`    | evidence runs                               | `evidence/runs/<id>.json`                |
| `D`    | decisions                                   | `decisions/<id-slug>.md`                 |
| `P`    | policies                                    | `governance/policies/<id-slug>.md`       |
| `A`    | audit findings                              | `audits/findings/<id-slug>.json`         |
| `Prov` | provenance typing over records              | record fields and validator semantics    |

Each record `r` has a provenance type:

```text
prov(r) ∈ {concrete, provisional, inferred}
```

A **concrete** record asserts that its claim is directly authored or backed by concrete evidence. A **provisional** record is valid but not complete. It is a transitional record, usually emitted during adoption or migration, and expected to be ratcheted later. An **inferred** record is reconstructed from available signals rather than directly proven. It is honest about its epistemic status.

This is the essential change in RepoPact 2.0. Validity no longer means "every record is fully proven." Validity means "every record is structurally and semantically well-formed, and every non-concrete claim is honestly typed." Completion remains stricter: completed work must rest on concrete evidence.

A work item is modeled as:

```text
w = (id, title, status, owner, affected_scopes, dependencies, AC, created, updated, prov)
```

where:

```text
status(w) ∈ {proposed, active, blocked, deferred, completed}
```

The five statuses are not merely progress markers. They encode **authority**. A `proposed` item is captured candidate work that has not been accepted: it records possible intent without granting implementation authority. An `active` item is accepted work authorized for design or implementation. `blocked` and `deferred` are accepted work that cannot or should not proceed right now, for a named reason. `completed` is delivered work whose acceptance criteria are evidence-closed.

The `proposed` state was added after 2.0.2 (decision 0023), when a downstream adopter exposed the gap: candidate work worth capturing durably had no honest home. Mapping it to `active` overstates authority. Mapping it to `blocked` misstates why it is not moving. Mapping it to `deferred` implies it was already accepted. The lifecycle needed a state for "recorded but not yet authorized," for the same reason the record language needed provenance types: the alternative to an honest type is a dishonest assertion.

and each acceptance criterion is:

```text
c = (criterion_id, status, evidence_links)

status(c) ∈ {pending, satisfied, waived}
```

A work item has both a declared status field and a lifecycle directory. Their agreement is not assumed. It is an invariant. If `work-item.json` says `completed` while the directory is `work/active`, the state is invalid. If a completed work item has pending criteria, the state is invalid. If a satisfied criterion links no evidence, the state is invalid.

The repository as a whole is an infinite-state transition system because the sets of work items, evidence runs, decisions, policies, findings, and contracts are unbounded. The finite-state structure is local to each work item.

### 3.2 The lifecycle automaton, L1

Per work item, RepoPact defines a lifecycle automaton:

```text
M_w = (Q, Λ, δ_w, Q0)

Q = {proposed, active, blocked, deferred, completed}
Q0 = {proposed, active}
```

`Λ` is the alphabet of lifecycle moves. A lifecycle move is a directory relocation plus a matching status rewrite. `Q0` is a set of initial states because work can be born two ways: as accepted work (`active`, the default of `new`) or as a captured candidate (`proposed`). The `proposed → active` move is the acceptance event — the point where recorded intent becomes implementation authority.

The transition relation is otherwise deliberately permissive. Work may move among the five states; reopening completed work is allowed; degradation is explicit rather than hidden.

Two edges carry semantics beyond relocation.

The first is acceptance. `proposed` grants no authority, and the invariant monitor enforces this at the dependency level: an `active` or `completed` work item may not depend on a `proposed` one, because that would treat unaccepted candidate work as accepted implementation authority. Authority flows through the acceptance edge, not around it.

The second is completion. Moving into `completed` is semantically guarded by:

```text
g_done(w, s) =
  every acceptance criterion is not pending
  and every satisfied criterion links evidence that exists
  and completed work is concrete
  and concrete completed work does not rest on non-concrete evidence
```

RepoPact does not require this to be enforced as a runtime gate. A user or agent can perform an ordinary file edit or `git mv` that puts the tree into an invalid state. RepoPact's enforcement point is the checkpoint: validation, CI, review, and generated reports. The lifecycle automaton supplies reachable control points. The invariant monitor decides whether the resulting repository state is admissible.

This separation matters. RepoPact is repository-native, not runtime-exclusive. It assumes agents and humans can edit files. Its claim is that invalid states are visible, rejectable, and auditable at the repository boundary.

### 3.3 The invariant monitor, L2

The reference validator computes a finite set of violations:

```text
Viol(s)
```

Define:

```text
I(s) ≡ Viol(s) = ∅
R = {s | I(s)}
```

`R` is the conformant repository language. The reference validator is the characteristic function of this language:

```text
validate(s) = accept ⇔ s ∈ R
```

The predicate `I` decomposes into atomic predicates:

| Predicate    | Meaning                                                                                      |
| ------------ | -------------------------------------------------------------------------------------------- |
| `I_ver`      | the version string is semantic                                                               |
| `I_struct`   | every record satisfies its schema                                                            |
| `I_contract` | root and nested contracts are present and registered                                         |
| `I_id`       | record identifiers match paths and are unique per type                                       |
| `I_status`   | work status agrees with lifecycle directory                                                  |
| `I_ref`      | dependencies, scopes, evidence links, decisions, owners, and findings refer to known records; authorized work does not depend on `proposed` work |
| `I_accept`   | completed work has no pending criteria; satisfied criteria link evidence                     |
| `I_acyclic`  | the work dependency graph is acyclic                                                         |
| `I_conc`     | active scopes are disjoint when concurrency enforcement is enabled                           |
| `I_orphan`   | planning artifacts do not exist outside the ledger                                           |
| `I_prov`     | provenance typing is valid and completion does not rest on non-concrete proof                |
| `I_derive`   | derived artifacts equal their projections where enforced                                     |
| `I_frozen`   | protected surfaces require acknowledgement at diff time                                      |

`I_prov` is the 2.0 addition that changes the adoption story. It can be summarized as:

```text
I_prov(s) =
  completed(w) implies prov(w) = concrete
  concrete(w) implies all evidence supporting satisfied criteria is concrete
  inferred(r) or provisional(r) implies r declares reconstruction/provenance metadata
  inferred/provisional records may be valid but may not masquerade as concrete proof
```

This separates validity from proof. A repository may be conformant while containing inferred or provisional records, because the language admits honestly typed reconstruction. But a completed work item cannot rely on those records as if they were concrete.

The monitor non-bypass property is the safety backbone:

```text
For any edit trace s0 -> s1 -> ... -> sk,
the checkpoint admits sk iff sk ∈ R.
```

The trace may be produced by RepoPact CLI actions, manual file edits, an agent, or arbitrary filesystem operations. RepoPact's claim is not that invalid edits cannot be made. Its claim is that invalid repository states are rejected or surfaced at the checkpoint, while valid states carry the evidence needed for later recovery.

### 3.4 The typed enforcement lattice, L3

RepoPact's invariants are not of one logical kind. Their logical kind predicts what can enforce them.

| Type                  | Example                                         | Enforcer                                  |
| --------------------- | ----------------------------------------------- | ----------------------------------------- |
| state                 | completed implies no pending criterion          | validator on one tree                     |
| state                 | satisfied implies linked evidence               | validator on one tree                     |
| state with provenance | completed implies concrete evidence             | validator on one tree                     |
| state fixpoint        | generated dashboard equals projection           | CI diff or generator check                |
| transition            | frozen-surface change implies acknowledgement   | diff-time checker                         |
| temporal              | completed work is not rewritten to look cleaner | git history plus review                   |
| relational            | nested contract refines parent contract         | human review, future refinement calculus  |
| meta coverage         | no critical state lives only in conversation    | human judgment plus partial orphan checks |

State invariants are decidable from a single tree. Transition invariants require two states, usually a base and head diff. Temporal invariants require a trace. Relational invariants require a semantic order over contracts. Meta invariants concern coverage and judgment: whether the repository contains enough of the working memory to make future work safe.

This lattice explains why RepoPact uses multiple enforcement mechanisms. A single JSON Schema cannot enforce a frozen-surface diff. A single validator pass cannot prove no history rewrite. A human review cannot scale to every referential-integrity check. RepoPact assigns enforcement according to logical type.

### 3.5 The derive layer, L4

RepoPact distinguishes source records from derived artifacts. Source records include invariants, policies, work items, evidence, decisions, owner maps, audit findings, and contract registrations. Derived artifacts include dashboards, generated specification blocks, and audit views.

The derive principle is:

```text
derive over declare
```

Anything computable from source records should be generated, not hand-maintained. This prevents a common drift class: a dashboard or status page that becomes a second, manually edited source of truth. In the model, derive functions are projections:

```text
π_dashboard(s) -> audits/reports/dashboard.md
π_spec(s) -> SPEC.md derived blocks
```

A derived artifact is valid only when it matches its projection from the source records. Where enforced, this is a state-fixpoint invariant.

### 3.6 The adoption boundary, L5, and the concrete-record trilemma

L5 is where RepoPact meets state it does not yet contain. Brownfield projects already have history, conventions, issue trackers, planning docs, nested context files, CI workflows, CODEOWNERS, roadmap files, and implicit team knowledge. Some of that state can be read from the repository. Some cannot.

A migration from a RepoPact-naive project faces three requirements:

1. **Totality.** The migration should be defined on arbitrary input trees.
2. **Faithfulness.** The migration should map existing signals without fabricating proof or discarding load-bearing state.
3. **Closure.** The migration output should lie in the conformant language.

In a concrete-only record language, no migration can satisfy all three.

The reason is structural. Input trees can contain facts that RepoPact would reject if asserted as concrete records. A roadmap may claim work is done but contain no evidence. A nested `AGENTS.md` may name a scope that no owner map establishes. A tracker import may contain cyclic dependencies. A legacy planning file may contain decisions with missing dates, missing rationale, or unknown authority. Forcing such input into a concrete conformant state requires either inventing missing proof, which breaks faithfulness, or discarding the offending signal, which also breaks faithfulness. If the migration keeps the signal faithfully and does not fabricate proof, the output is not closed under a concrete-only language.

This is the **concrete-record adoption trilemma**:

```text
total + faithful + closed cannot all hold when every emitted record is concrete
```

RepoPact 2.0 resolves this by changing the language. It introduces provenance-typed records. A migration can emit records that are valid as `inferred` or `provisional`, not falsely concrete. Such records can preserve what was observed while truthfully labeling the epistemic status of the reconstruction.

Under the provenance-aware conformant language `R_p`, adoption can satisfy:

1. **Totality**, because it can operate on arbitrary input trees.
2. **Faithfulness**, because it need not discard or fabricate signals.
3. **Closure**, because reconstructed facts are admitted as reconstructed, not asserted as proven.

This does not mean adoption magically knows the true project state. It means the record language can distinguish "known," "reconstructed," "provisional," and "not yet proven." The trilemma is not denied. It is resolved by adding the missing type distinction.

We therefore distinguish two languages:

```text
R_c = concrete-only conformant language
R_p = provenance-aware conformant language
```

In `R_c`, adoption is not closed if it remains total and faithful. In `R_p`, adoption can be closed because inferred and provisional records are valid members of the language, provided they are honestly typed and completion still requires concrete evidence.

This is the central 2.0 model change.

### 3.7 Action taxonomy

RepoPact actions partition by their relationship to conformance.

| Class                 | Actions                                         | Property                                                                                   |
| --------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Constructor           | `init`                                          | creates a valid governed repository from an empty target                                   |
| Record creation       | `new`                                           | creates a valid preflight work item                                                        |
| Derive/read           | `validate`, `dashboard`, `spec`, `check-frozen` | reads or regenerates projections without changing source records, except generated outputs |
| Lifecycle move        | status and directory transition                 | valid when post-state satisfies `I`                                                        |
| Repair                | `doctor --fix`                                  | conservative repair toward conformance                                                     |
| Migration             | `adopt`, `import-plan`                          | crosses L5; emits concrete, inferred, or provisional records                               |
| Diff-time enforcement | `check-frozen --base`                           | checks two-state frozen-surface invariants                                                 |

`doctor` is a repair and ratchet operator. It diagnoses drift and may fix safe classes of invalid state. In 2.0, it also ratchets provisional records to concrete when all required proof is concrete. The intended algebra is conservative and monotone:

```text
Viol(doctor(s)) ⊆ Viol(s)
doctor(s) = s if s is already healthy
doctor does not overwrite differing local source records as if they were disposable
```

This matters because repair can become fabrication if it silently replaces local intent. RepoPact's repair principle is to make safe additive fixes, remove known-invalid derived or stale registry entries where appropriate, and surface the rest as a worklist.

## 4. Reference Implementation

RepoPact's reference implementation consists of a CLI, schemas, templates, validator, dashboard generator, frozen-surface checker, adoption/import tools, doctor repair tool, conformance fixtures, and research records.

The validator defines the reference semantics. In the reference implementation, `R` is the validator's accept set. For an alternative implementation, recognizer soundness and completeness become testable claims against the conformance suite.

### 4.1 CLI surface

The public CLI includes:

| Command        | Purpose                                               |
| -------------- | ----------------------------------------------------- |
| `init`         | seed a valid RepoPact into a new repository           |
| `adopt`        | map existing repository signals into RepoPact records |
| `import-plan`  | import legacy planning or tracker-like material       |
| `new`          | create preflight work items from templates, as accepted (`active`) or candidate (`--status proposed`) work |
| `validate`     | validate schema and cross-record semantics            |
| `dashboard`    | generate dashboard views from source records          |
| `spec`         | generate or check specification projections           |
| `check-frozen` | enforce frozen-surface changes against a base         |
| `doctor`       | diagnose, repair, and migrate drift                   |

RepoPact 2.0 makes preflight mandatory. A work item must exist before implementation begins. Existing repositories are grandfathered through a preflight epoch and `doctor` migration path. This makes the "intent before execution" property visible in the tree.

### 4.2 Validation layers

Validation has two layers.

First, JSON Schema validates individual record structure. This catches malformed fields, invalid statuses, missing required properties, and type errors.

Second, the semantic validator checks cross-record constraints. This includes referential integrity, status-directory agreement, dependency cycles, evidence links, scope validity, concurrency rules, provenance rules, orphan work directories, and other non-local constraints.

This two-layer design keeps the standard implementable. Schema validation handles local form. Semantic validation handles the language.

### 4.3 Conformance

A standard without conformance is a convention. RepoPact therefore publishes a versioned conformance surface in the repository itself: `CONFORMANCE.md`, the fixture corpus under `conformance/`, the suite manifest at `conformance/manifest.json`, and the runner `scripts/run_conformance.py`.

The suite is organized around a minimal valid RepoPact repository plus invalid overlays. Each case declares the rule or invariant it exercises, the expected accept or reject outcome, and the diagnostic text or rule identity required for rejection. Lifecycle and authority semantics are part of the surface, not just record shape: when the `proposed` state was added, the suite gained both a valid fixture containing a proposed item and a rejection overlay in which active work depends on proposed work. A conformant implementation must reproduce the authority semantics, not merely parse the records. The reference implementation is checked with:

```powershell
python scripts/run_conformance.py
```

Alternative implementations are checked by passing a validator command template to the same runner. The runner materializes each fixture repository and invokes the supplied command against it.

This separates four claims that are easy to collapse in prose:

1. the paper's formal model,
2. the repository contract,
3. the machine-checkable conformance corpus,
4. and the evidence produced by concrete runs.

A third-party implementation need not copy the Python validator to claim RepoPact compatibility, but it must accept the conformant fixtures and reject the invalid overlays for the named RepoPact version.

### 4.4 Proving Ground split

RepoPact defines the contract language, validator semantics, conformance expectations, and research protocols. The runnable adversarial lab lives separately in **RepoPact Proving Ground**.

The split is deliberate.

RepoPact should not become a benchmark repository that grades itself in-place. It should define what must be measured and how results must be recorded. RepoPact Proving Ground hosts the runnable tasks, fixtures, harnesses, captures, and drift experiments. It consumes RepoPact from PyPI, which means it tests what an adopter receives rather than a local source checkout.

The ecosystem relation is:

```text
RepoPact defines the pact.
RepoPact Proving Ground tests whether the pact holds under agent pressure.
```

## 5. Evaluation Method

The evaluation has two parts.

The first is reflexive and adversarial. It asks whether RepoPact catches the failures it claims to catch on a governed subject. This part is complete for the current reflexive findings register, though it remains open-ended as new falsification cases are added.

The second is comparative. It asks whether governing a repository with RepoPact changes agent behavior compared with a fair baseline. This part is pre-registered, and the runnable benchmark infrastructure now exists for key studies, but real cross-model results remain forthcoming.

### 5.1 Reflexive adversarial falsification

The reflexive protocol sets hypotheses H1 through H7 and explicit falsification criteria before the runs. The subject under test is RepoPact as distributed: the packaged wheel installed into a clean environment. Testing only the source checkout would test the author's working tree, not the product an adopter receives.

The subject repository is a small but genuinely working project. It adopts RepoPact from the packaged product, creates real work items, links evidence, validates, transitions work, and records decisions. It is then attacked through adversarial cases that attempt to falsify the architecture.

The hypotheses are:

| Hypothesis | Claim                                                                  |
| ---------- | ---------------------------------------------------------------------- |
| H1         | an adopter can install the package and reach a valid governed repo     |
| H2         | advertised commands are closed over the initialized repository surface |
| H3         | completion is evidence-gated                                           |
| H4         | frozen-surface and invariant authority are binding                     |
| H5         | state integrity violations are rejected                                |
| H6         | a reader can recover project state from the tree alone                 |
| H7         | brownfield adoption is non-destructive, sound, and honestly typed      |

The falsification criteria are intentionally direct. The architecture is disproven in whole or part if the package cannot bootstrap, a documented command crashes, the validator accepts unproven completion, a protected surface can be changed without acknowledgement, status-directory mismatch is accepted, recovery requires chat history, or brownfield adoption can only work by fabricating or discarding state.

Every defect is fed back through RepoPact's own machinery: finding, work item, decision, evidence, and validation. This is itself a test of recoverability.

### 5.2 Comparative benchmarks

Reflexive falsification shows whether the architecture catches what it claims on one governed subject. It does not show whether RepoPact changes agent behavior relative to an ungoverned or convention-governed repository. The comparative protocol therefore introduces a held-constant design:

```text
condition ∈ {baseline, repopact}
```

Source, task, model, and harness are held constant. The governance layer varies.

The pre-registered studies are:

| Study | Hypothesis | Construct                                     | Primary outputs                                              | Current status                                                                     |
| ----- | ---------- | --------------------------------------------- | ------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| S1    | H8         | guarantee-violation detection, PactBench      | confusion matrix, catch/escalate rate, false-stop rate       | protocol defined; runnable artifacts in Proving Ground; real model results pending |
| S2    | H9         | cross-session recovery and efficiency         | resolution rate, recovery score, tokens, interventions       | protocol defined; results pending                                                  |
| S3    | H10        | multi-agent coordination                      | conflict rate, duplicated work, joint success                | protocol defined; results pending                                                  |
| S4    | H11        | context-token economy                         | cost-success Pareto frontier, scaling curve                  | protocol defined; results pending                                                  |
| S5    | H12        | drift detection and staleness                 | detection rate, latency, silent-staleness rate               | runnable drift harness in Proving Ground; comparative results pending              |
| S6    | H13        | security enforcement and injection resistance | security catch rate, injection-followed rate, detection rate | tasks included in PactBench scope; real model results pending                      |

The current benchmark maturity states are:

| State             | Meaning                                                                          |
| ----------------- | -------------------------------------------------------------------------------- |
| pre-registered    | hypothesis, construct, metrics, and falsification criteria are fixed before runs |
| implemented       | task files, fixtures, harnesses, or scorers exist                                |
| selftested        | deterministic plumbing runs validate the harness shape                           |
| operator-gated    | real model runs require keys, model access, or agent command wiring              |
| results available | model outputs and scored result bundles exist                                    |

As of this draft, the benchmark program should be described as pre-registered, partially implemented, and operator-gated for real cross-model results. Deterministic mock outputs are plumbing checks, not findings.

### 5.3 PactBench and Proving Ground

PactBench is the first named benchmark suite inside RepoPact Proving Ground. It measures whether agents silently weaken declared guarantees less often when RepoPact governance is present.

The core task pattern is:

1. A repository contains a guarantee.
2. A task creates pressure to take a shortcut.
3. The shortcut would violate the guarantee.
4. The correct behavior is to preserve, refuse, escalate, or produce valid evidence.
5. The scorer records whether the agent silently weakened the guarantee, blocked the weakening, escalated correctly, or false-stopped a legitimate task.

The current Proving Ground includes:

* pre-registered PactBench tasks,
* security and correctness classes,
* adversarial evidence-fabrication and context-injection cases,
* real fixtures rather than only mocks,
* a model-agnostic harness,
* a deterministic mock runner for pipeline selftests,
* a subprocess real-runner interface for actual agents,
* an S5 drift harness,
* and result-bundle expectations.

The Proving Ground's role is not to claim results before they exist. Its role is to make the benchmark executable and inspectable before model runs are performed.

## 6. Results

The reflexive findings register currently holds thirteen entries. Each cites a raw capture; severity reflects impact on an adopter, not effort to fix. "Holds" records an adversarial case the architecture correctly caught, kept as evidence *for* the design rather than discarded as a non-event.

| ID    | Hypothesis | Severity | Finding                                                                             | Resolution                          |
| ----- | ---------- | -------- | ----------------------------------------------------------------------------------- | ----------------------------------- |
| F-001 | H2         | major    | `spec` crashes on an `init`-fresh repository                                        | fixed, re-verified from rebuilt wheel |
| F-002 | H4         | minor    | `check-frozen` blind to working-tree edits                                          | fixed, re-verified from rebuilt wheel |
| F-003 | H3         | holds    | satisfied criterion without evidence rejected                                       | n/a                                  |
| F-004 | H5         | holds    | status–directory mismatch rejected                                                  | n/a                                  |
| F-005 | H4         | holds    | committed change to protected path requires acknowledgement                         | n/a                                  |
| F-006 | H1, H6     | holds    | full work item recovered from the tree alone, no chat history                       | n/a                                  |
| F-007 | H7         | holds*   | brownfield adoption of a real 4569-commit repository, non-destructive               | shipped (*confirmatory only)         |
| F-008 | H7         | major    | adopter's `.gitignore` silently un-tracks governance records                        | fixed: `adopt` warns                 |
| F-009 | H7         | holds    | clean-room adoption of an independent OSS repository (pallets/flask)                | n/a                                  |
| F-010 | H7         | major    | adoption left the work ledger hollow beside the team's real plan tree               | fixed: `import-plan`                 |
| F-011 | H7         | major    | older adopter drifted invalid as the standard evolved, undetected                   | fixed: `doctor`                      |
| F-012 | H7         | holds    | full lifecycle on an independent different-domain application                       | shipped                              |
| F-013 | H7         | holds    | governance-folder planning migrated and legacy tree retired without data loss       | shipped                              |

### 6.1 What cracked

Two early defects partially falsified hypotheses and were fixed.

**F-001, surface closure.** A documented command crashed on an `init`-fresh repository because it expected a specification file that bootstrap did not seed. This violated H2: the advertised surface was not closed over the tool's own initialized output. The fix made the command fail cleanly with guidance, and the decision was recorded.

**F-002, frozen-surface working-tree gap.** The frozen-surface checker originally diffed only committed ranges. A working-tree edit to a protected file could therefore produce a false "all clear" before commit. This violated the spirit of H4 for local preflight. The fix unioned committed ranges with uncommitted changes, so local edits are caught while CI still evaluates the branch's commits.

Three further majors emerged only under real brownfield adoption, which is why the evaluation insists on real repositories rather than synthetic fixtures alone.

**F-008, the swallowed record.** An adopter's pre-existing `.gitignore` rule (`runs/`, intended for runtime data) silently matched RepoPact's `evidence/runs/*.json`. The repository validated on the author's disk and would have failed on any fresh clone or in CI, where the ignored evidence is absent. This is the most dangerous failure shape: green locally, broken for everyone else. `adopt` now runs `git check-ignore` on every record it writes and warns with suggested negations.

**F-010, the hollow ledger.** Adoption produced a valid governed repository whose `work/` held one bootstrap item while the team's roughly seventy-five real plan items stayed in a legacy `todos/` tree. Nothing was invalid, but the ledger did not reflect the project, splitting planning across two trees. This motivated `import-plan`: legacy plan directories and checklist files import into `work/` by lifecycle, non-destructively and idempotently, with completed items marked `waived` rather than backed by fabricated evidence.

**F-011, longitudinal upgrade drift.** An older adopter drifted invalid as the standard evolved — stale registry paths, a missing root contract — and nothing detected or guided the upgrade. Validation catches drift when it runs, but an adopter who has no reason to re-run it gets no signal. This motivated `doctor` as a diagnose, repair, and migrate path, proven against the drifted repository itself.

These failures are important because they demonstrate that the evaluation was capable of cracking the architecture, and that some failure classes are only reachable through real adoption. None was hidden as implementation noise. Each was recorded as a finding, repaired, and re-verified.

### 6.2 What held

Several adversarial cases were correctly caught.

A criterion marked satisfied without evidence was rejected. A work item whose declared status disagreed with its directory was rejected. A committed change to a protected path required explicit acknowledgement. A reader was able to reconstruct a work item, its intent, decision context, and proof from the tree alone without chat history.

These are not proofs of global soundness. They are evidence that the stated mechanisms work for the tested classes. RepoPact treats "holds" as absence of failure under a defined adversarial case, not as proof that no bypass exists.

### 6.3 Brownfield adoption and the 2.0 shift

Brownfield adoption converted real projects into RepoPact-governed repositories non-destructively, mapping nested contracts, CODEOWNERS scopes, CI workflows, work ledgers, decisions, and existing planning signals where available.

The adoption evidence spans four subjects of increasing independence. The progenitor repository — 4569 commits, nineteen nested `AGENTS.md` contracts, seven CODEOWNERS-derived scopes, four CI workflows mapped to binding gates — adopted conformantly and non-destructively, but is confirmatory only: RepoPact was distilled from its practices, so it cannot witness generality (F-007, and threat T1). A clean-room open-source repository with no RepoPact lineage (pallets/flask) reached a conformant state through the sparse-signals path, the first independent datum (F-009). An independent application in a different domain and stack exercised the full lifecycle — adopt, plan import, doctor — end to end (F-012). And a governance-folder planning tree was migrated into the ledger with the legacy method retired by `takeover` without losing un-captured data (F-013).

The early adoption framing treated the concrete-record trilemma as a hard boundary. If a legacy project contained a done item with no evidence, adoption could preserve faithfulness only by relaxing closure and reporting the residue as a worklist. That was the correct model before provenance typing.

RepoPact 2.0 changes the model. Adoption can now emit provisional and inferred records. This allows the migration to remain honest without forcing false concrete proof. A reconstructed adoption work item can be valid as provisional. Inferred evidence can state that it was reconstructed from a scan rather than directly proven. `doctor` can later ratchet records to concrete when the required evidence exists.

This is not cosmetic. It turns the adoption result from "valid only after residue is resolved" into "valid with epistemic status preserved," where completion and concrete claims remain strict.

A prior adopter also revealed longitudinal upgrade drift: the standard evolved and the older adopted repository drifted invalid without automatic guidance. This became a finding and motivated `doctor` as an upgrade and repair path. The limitation remains important. RepoPact detects many drift classes at validation or CI boundaries, but not every longitudinal drift class is auto-detected before a check is run.

### 6.4 Standard evolution under adoption pressure

The `proposed` lifecycle state is itself a result, not merely a feature. A downstream public adopter surfaced candidate work that deserved durable capture but had not been accepted or authorized. The four-state lifecycle offered no honest mapping: every available state either overstated authority or misstated intent. The gap was resolved the way the pact requires — a decision record with the rationale, a schema change, CLI support, and conformance fixtures for both the valid and the forbidden configurations — and the dependency rule (authorized work may not depend on proposed work) entered the invariant monitor rather than remaining prose.

This matters for two reasons. First, it is external pressure: the defect was found by an adopter that is not the progenitor, in ordinary use rather than in a designed adversarial case. Second, it exercises the meta-claim. RepoPact argues that governance state must evolve through typed, recorded, machine-checked channels; the standard's own evolution followed exactly that channel, and a reader can recover the entire episode — motivation, decision, semantics, enforcement — from the tree.

### 6.5 Benchmark infrastructure status

Comparative model results are forthcoming. The benchmark suite is no longer merely a plan, however. The ecosystem now separates protocol from execution.

RepoPact's `research/` directory defines the protocol, hypotheses, metrics, falsification criteria, and threats to validity. RepoPact Proving Ground hosts runnable benchmark artifacts. PactBench, the guarantee-violation suite, is implemented there with pre-registered tasks, fixtures, harnesses, and a real-runner interface. The S5 drift harness is also implemented there. Deterministic selftests validate pipeline structure but do not constitute agent-behavior findings.

The paper will be updated with:

* S1 PactBench confusion matrices,
* S5 drift detection and staleness results,
* S6 security and injection-resistance results,
* S4 token-economy scaling curves,
* S2 recovery and efficiency numbers,
* and S3 multi-agent coordination measurements,

only after real runs are complete across the stated model families.

## 7. Discussion

### 7.1 Why repository-native governance

RepoPact's wager is that the repository is the correct substrate for load-bearing agent work state. The repository is not the only memory a project has, but it is the memory that future work must be able to inspect. It is already shared, replicated, reviewed, diffed, branched, and checked. It already carries authority through pull requests, branch protection, CI, owners, and history.

Putting governance state in the repository creates ceremony. The claim is not that ceremony is free or always worth it. The claim is that ceremony is justified when guarantees are load-bearing and work is delegated to fast, literal, forgetful agents. If a failure would be cheap, local, and reversible, an instruction file may be enough. If a failure would silently weaken an invariant future work relies on, a stronger contract is warranted.

### 7.2 Enforcement without pretending total automation

RepoPact does not claim every invariant can be fully machine-enforced. The typed enforcement lattice is a restraint mechanism. It distinguishes what is decidable from one tree, what requires a diff, what requires history, and what still requires human judgment.

This matters for credibility. A governance system that claims total automation over semantic project intent is likely overstating. RepoPact instead says:

* some invariants are machine-checkable now,
* some are diff-time properties,
* some are human-reviewed,
* some are open proof obligations,
* and the record should say which is which.

The benefit is not perfect enforcement. The benefit is explicit enforcement boundaries.

### 7.3 The provenance lesson

The adoption trilemma is one of RepoPact's strongest theoretical contributions because it explains a real brownfield problem. Legacy projects contain useful but messy signals. Treating every migrated statement as concrete proof is dishonest. Refusing to migrate until everything is proven blocks adoption. Dropping uncertain facts loses memory.

Provenance typing is the middle path. It lets the repository hold reconstructed state without lying about it. A provisional record can be useful. An inferred record can be valid. A concrete completion claim still requires proof.

This is directly relevant to agent systems. Agents often operate over partial context. A durable governance kernel should not force them to choose between silence and fabrication. It should give them a type for uncertainty.

The `proposed` lifecycle state is the same lesson applied to authority rather than proof. Before it existed, candidate work had to be either omitted from the ledger or asserted as accepted. Provenance typing says: do not force a record to claim more proof than it has. The proposed state says: do not force a record to claim more authority than it has. In both cases the kernel grew a type instead of tolerating a lie, and in both cases the pressure came from real adoption rather than from theory.

### 7.4 The L5 boundary remains real

RepoPact governs what crosses into the repository. It cannot govern what remains outside. Much of a project's real memory may still live in issue trackers, external documents, private chats, model transcripts, or human heads. Adoption can only reconstruct from reachable signals.

RepoPact 2.0 resolves the concrete-record trilemma for migrated records, but it does not solve omniscience. External ingestion remains future work. A tracker export, design document, or external roadmap must become a first-class evidence-bearing record before RepoPact can govern it directly. Until then, the correct behavior is to state the gap, not fabricate closure.

### 7.5 How RepoPact composes with runtime governance

RepoPact is not a sandbox, not an authorization server, not an LLM firewall, and not a runtime agent orchestrator. It composes with those systems. Runtime controls protect live actions. RepoPact protects durable project state.

A secure agentic development stack likely needs both. Runtime controls can stop dangerous tool use. RepoPact can stop or surface silent weakening of repository guarantees. Runtime audit logs can show what an agent did. RepoPact records whether the resulting work respected the contract.

### 7.6 When RepoPact is not the right tool

RepoPact is overkill for throwaway experiments, one-off scripts, private scratch work, or low-risk projects where the cost of governance exceeds the cost of repair. It is most appropriate when:

* multiple sessions or agents will touch the same work,
* project guarantees must survive context loss,
* authority boundaries matter,
* work completion must be evidence-backed,
* adoption or migration must be honest about uncertainty,
* and a future human or agent must reconstruct state from the repository alone.

When those conditions do not hold, an instruction file and tests may be enough.

## 8. Threats to Validity

### T1: Reflexivity

RepoPact was distilled from real practices in the author's agentic development workflow. That is a strength for relevance but a threat to generality. A project adopting the system that it helped inspire is confirmatory, not independent. Mitigation requires unrelated repositories, different domains, and third-party reproduction.

### T2: Single evaluator and operator effects

Early evidence comes from one operator and a small number of agent workflows. The process may reflect the operator's habits, model choices, and tolerance for ceremony. Raw captures, exact commands, and public artifacts reduce but do not remove this threat.

### T3: Scale and domain narrowness

The greenfield proving subject is intentionally small. Some brownfield subjects are larger, but the evaluation is not yet representative of large organizations, monorepos, regulated environments, or many-agent teams. The comparative studies are designed to broaden this, but results remain pending.

### T4: Holds are not proofs

A caught adversarial case shows that one attack was caught. It does not prove all variants are caught. RepoPact's evidence should be read as falsification-oriented engineering evidence, not mathematical proof of comprehensive safety.

### T5: Benchmark curation

PactBench and related suites could overfit to RepoPact's strengths. Pre-registration mitigates this by fixing task ids and expected outcomes before real runs. Fair baselines and disconfirming-result reporting are required.

### T6: Baseline fairness

A weak baseline would exaggerate RepoPact's value. The baseline must include reasonable convention files and, where appropriate, convention-plus-RAG regimes. Ceremony cost must count against RepoPact.

### T7: Token and cost measurement

Token-economy claims are sensitive to tokenizer, model, provider pricing, caching, retrieval setup, and prompt construction. The S4 study must report raw tokens, cache-adjusted tokens, context tokens versus task tokens, requests per task, and cost per resolved task.

### T8: Drift and security realism

Synthetic drift and security tasks may not reflect real projects. Security tasks must remain defensive and sandboxed, with no live targets or exploit development. RepoPact's own records must be evaluated as an attack surface. The claim is improved integrity, not immunity.

### T9: Provenance misuse

Provenance typing can be misunderstood. If users treat inferred or provisional records as equivalent to concrete proof, the type system loses its value. RepoPact must enforce completion restrictions and make non-concrete status visible in dashboards and review flows.

### T10: Standard versus implementation coupling

The reference validator defines RepoPact's current semantics. This is practical but risks coupling the standard too tightly to one implementation. The conformance suite mitigates this by giving alternative implementations a runnable target.

## 9. Conclusion and Future Work

The session-amnesia and silent-weakening problems are memory and authority problems. Coding agents do not merely need more context. They need a durable contract that records what matters, who has authority, what evidence proves completion, what invariants must not be weakened, and what uncertainty remains.

RepoPact turns the version-controlled repository into that contract. It models agentic work as a six-layer governance kernel over typed records, lifecycle state, invariant monitoring, enforcement tiers, derived artifacts, and adoption boundaries. Its central primitive is the binding invariant, the guarantee an agent must not silently weaken.

The formal model shows why enforcement cannot be treated as one mechanism. State invariants, transition invariants, temporal invariants, relational invariants, and meta-coverage claims require different enforcers. It also shows why brownfield adoption is difficult. In a concrete-only record language, total, faithful, and closed migration cannot all hold. RepoPact 2.0 resolves that by adding provenance-typed records. Reconstructed state can be valid without pretending to be proven, while completed work still requires concrete evidence.

The evaluation so far is reflexive, adversarial, and honest about defects. Some failures cracked the architecture and were fixed. Some adversarial cases held. Comparative value claims remain forthcoming, but the benchmark infrastructure has moved from plan to runnable public artifacts in RepoPact Proving Ground.

Future work includes:

1. running PactBench across multiple model families,
2. completing S5 drift and S6 security comparative results,
3. measuring S4 context-token economy against realistic baselines,
4. testing S2 recovery on long-horizon software evolution tasks,
5. testing S3 multi-agent coordination,
6. mechanizing temporal invariants over git history,
7. developing a refinement order for nested contracts,
8. expanding external ingestion across trackers and design documents,
9. hardening provenance review and ratcheting flows,
10. and inviting third-party conformance implementations.

The repository is already the substrate software engineering trusts. RepoPact's claim is that it can also be the operating system for durable agentic work.

## Ethics and Responsible Disclosure

The security components of the evaluation are defensive and benign by construction. Tasks are sandboxed and synthetic. They involve no live targets, no credential theft, no exploit deployment, and no instructions for real-world compromise. Security-invariant tasks focus on whether agents preserve or weaken defensive controls such as authorization checks, secret handling, input validation, and protected surfaces.

Injection tasks treat both convention files and RepoPact records as trusted text surfaces. RepoPact makes no immunity claim. Its defense is integrity structure: frozen surfaces, evidence validation, provenance typing, escalation paths, and reviewable records. Unsafe or disconfirming behavior is part of the evaluation and should be reported as such.

Defects found in RepoPact are recorded openly as findings rather than quietly patched. The evaluation involves no human subjects. Agent runs should respect model-provider terms and should preserve raw captures without exposing secrets.

## Data and Artifact Availability

RepoPact is open source under Apache-2.0. The formal model, experiment protocol, comparative benchmark protocol, threats register, findings register, current paper, and related research records live in RepoPact's public `research/` directory.

The runnable benchmark artifacts live in RepoPact Proving Ground. This includes PactBench task sets, fixtures, harnesses, deterministic mock runner support, subprocess real-runner support, drift harnesses, and future result bundles. The benchmark protocol remains in RepoPact. The runnable benchmark implementation belongs to Proving Ground.

Benchmark tasks are pre-registered. Corrections should be issued as new task ids or dated amendments rather than silent edits. Real model result bundles should include model id, harness version, task-set version, condition, prompts or command interface, raw captures where safe, scorer outputs, and evidence links.

The conformance suite is versioned with RepoPact and exists to make standard conformance independently testable.

## Appendices

### Appendix A: Typed invariant lattice

| Invariant type        | Example                                           | Decidable from                     | Enforcer                           |
| --------------------- | ------------------------------------------------- | ---------------------------------- | ---------------------------------- |
| state                 | completed work has no pending criteria            | one tree                           | validator                          |
| state                 | satisfied criterion links evidence                | one tree                           | validator                          |
| state with provenance | completed work is concrete                        | one tree                           | validator                          |
| state fixpoint        | dashboard equals generated projection             | one tree plus generator            | CI or dashboard check              |
| transition            | frozen-surface change requires acknowledgement    | base and head                      | diff-time checker                  |
| temporal              | completed work is not rewritten to look cleaner   | git trace                          | review, future trace semantics     |
| relational            | nested contract refines parent                    | contract pair and refinement order | human review, future formalization |
| meta                  | critical state does not live only in conversation | repository plus human judgment     | partial orphan checks and review   |

### Appendix B: Formal theorem sketch

Each claim is tagged with its discharge status: **[def]** true by definition for the reference implementation; **[ci]** machine-checked on every run; **[fix]** covered by the conformance fixture corpus; **[conj]** a conjecture whose falsification is a proving-ground target.

**T1: Recognizer definition. [def]/[fix]**
For the reference implementation, `validate(s)` accepts exactly the states in `R` by definition. For alternative implementations, this becomes a conformance theorem tested by fixtures: one valid baseline that must be accepted, plus one invalid overlay per rule that must be rejected with a declared diagnostic.

**T2: Constructor correctness. [ci]**
`init` creates a valid governed repository from an empty target. The CLI validates its own output and exits non-zero otherwise; every invocation is a proof instance.

**T3: Surface closure. [conj]**
Advertised commands should either succeed or fail cleanly on the initialized surface without corrupting state. The original counterexample was F-001. This is totality, weaker than `R`-preservation.

**T4: Completion safety. [fix]/[conj]**
A completed work item with pending criteria, missing evidence, non-concrete status, or non-concrete supporting proof is not conformant. Covered by fixtures; its negation — the validator accepting unproven completion — is a standing falsification target (H3).

**T5: Monitor non-bypass. [ci]/[conj]**
For arbitrary edit traces, the checkpoint admits the final state only if it is conformant. State invariants are enforced on the tree; the two-state frozen-surface invariant is enforced on the diff.

**T6a: Concrete-only adoption trilemma. [structural]**
In a concrete-only record language, no brownfield migration can be total, faithful, and closed. The argument is structural (Section 3.6), not empirical.

**T6b: Provenance-typed closure. [ci]**
In a provenance-aware language, adoption can be total, faithful, and closed by emitting inferred and provisional records honestly, while completion remains gated on concrete evidence. `adopt` now lands conformant output on real trees rather than reporting residue.

**T7: Ratchet monotonicity. [conj]**
`doctor` should ratchet records from non-concrete to concrete only when required concrete evidence exists, and should not silently replace differing source records. Partial evidence exists from a real drifted-adopter repair; the algebra remains a proof obligation.

**Open obligations.** The reference model carries a numbered backlog: `new`-correctness (a stamped template lands valid from any conformant state); lifecycle preservation (a guarded move preserves all state invariants, not only acceptance); the full `doctor` algebra (conservative, violation-monotone, identity on healthy repositories); a trace semantics over git history to mechanize the no-history-rewrite invariant; a refinement order on nested contracts to mechanize contract refinement; and promotion of the orphan-planning check into the numbered specification catalog. The last three are the path from human-gated to machine-checked enforcement tiers.

### Appendix C: Benchmark program

| Study | Name                                    | Hypothesis                                                                                   | Status                                                |
| ----- | --------------------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| S1    | PactBench guarantee-violation detection | RepoPact improves catch/escalate rate over baseline                                          | runnable infrastructure present; real results pending |
| S2    | Cross-session recovery                  | RepoPact improves recovery and reduces redo loops                                            | protocol defined                                      |
| S3    | Multi-agent coordination                | RepoPact reduces conflicts and duplicated work                                               | protocol defined                                      |
| S4    | Context-token economy                   | RepoPact improves cost-success frontier and bounded context scaling                          | protocol defined                                      |
| S5    | Drift detection                         | RepoPact lowers silent staleness and detection latency                                       | drift harness present; comparative results pending    |
| S6    | Security and injection resistance       | RepoPact improves defensive invariant preservation and lowers injected-context-followed rate | tasks scoped; real results pending                    |

### Appendix D: Figures and tables planned

**Figure 1.** Six-layer governance kernel, L0 through L5.
**Figure 2.** Work-item lifecycle automaton and checkpoint composition.
**Figure 3.** Concrete-record adoption trilemma and provenance-typed resolution.
**Table 1.** Typed invariant lattice.
**Table 2.** Study status table.
**Table 3.** Findings register summary.
**Figure 4.** S4 cost-success Pareto frontier.
**Figure 5.** S4 context-token scaling curve.
**Figure 6.** PactBench confusion matrix.
**Figure 7.** S5 drift detection latency and silent-staleness rate.
**Figure 8.** S6 injection-followed rate by condition.

### Appendix E: Informal references

This appendix is a positioning map, not yet a citation-complete related-work section. Before archival publication, these entries should be replaced with full bibliographic citations and a more systematic comparison.

* `AGENTS.md` (Linux Foundation stewardship) and repository agent-instruction files: `CLAUDE.md`, `.cursor/rules`, and editor-specific variants.
* ADRs and lightweight architecture decision records (Nygard-style).
* Policy-as-code systems such as OPA and Conftest.
* Architecture fitness functions (Ford, Parsons, and Kua's evolutionary-architecture line).
* Runtime agent governance and sandboxing architectures, including layered governance architectures (arXiv:2603.07191) and six-layer agentic-SDLC reference models (arXiv:2604.26275).
* Agent memory systems, including retrieval-augmented and persistent-memory approaches: MemGPT/Letta, Mem0, Zep, LangMem.
* Developer portals and service scorecards: Backstage, Cortex, OpsLevel.
* SWE-bench Verified and SWE-EVO for long-horizon software-evolution evaluation.
* RepoPact formal model, protocol, benchmark protocol, threats register, findings register, conformance suite, and RepoPact Proving Ground.
