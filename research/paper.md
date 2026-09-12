# RepoPact: Repository-Native Governance for Durable Human-Agent Software Engineering

**Portable intent, authority, evidence, and conformance across agents, humans, machines, and sessions**

Jeremy Shows  
ForgeWire Labs  
Draft, 2026-06-25; revised 2026-09-12  
Target: arXiv cs.SE preprint, then software engineering or agentic systems workshop submission

## Abstract

Agentic software engineering increasingly distributes work across humans, coding agents, machines, model providers, and execution sessions. Yet the state that governs that work, including intent, authority, invariants, decisions, acceptance criteria, provenance, and evidence, often remains outside the software artifact itself. It lives in conversations, local agent memory, issue trackers, private planning state, runtime-specific context, or one operator's head. When a session ends or a different worker takes over, the repository may preserve the code while losing the governing state required to change that code safely.

We present **RepoPact**, a repository-native governance kernel for durable human-agent software engineering. RepoPact stores governing project state as typed, version-controlled records alongside the artifact being governed. A fresh human or agent can therefore clone a repository and recover not only its source tree, but also its declared work state, authority boundaries, binding invariants, decisions, evidence, provenance, and lifecycle without requiring the previous worker's local context or agent runtime. Humans and agents operate over the same governed substrate rather than maintaining separate control planes.

RepoPact organizes this substrate into six layers, L0 through L5: a typed record store, work-item lifecycle automata, an invariant monitor, a typed enforcement lattice, deterministic derived views, and a brownfield adoption boundary. Its distinguishing primitive is the **binding invariant**, a declared guarantee coupled to rationale, escalation, and, where logically possible, machine enforcement. We formalize repository conformance, introduce **governance continuity** as the recoverability of governed state across worker and session transitions, characterize invariant classes by the information required to enforce them, and show that concrete-only brownfield migration faces a totality-faithfulness-closure trilemma. Provenance-typed records, `concrete`, `provisional`, and `inferred`, resolve that trilemma without representing reconstructed state as established fact.

We evaluate RepoPact reflexively and adversarially using its packaged implementation, conformance suite, recorded findings, naturalistic case studies, and a pre-registered comparative benchmark program. The implementation has evolved from a Python reference system into a canonical Rust semantic engine consumed by compatibility tooling and a Tauri 2 Workbench, while preserving repository-level interoperability. Windows, Linux, and Android bring-up evidence exists for the Workbench; macOS and iOS remain intended targets that still require platform-specific validation. Comparative cross-model results remain forthcoming and will be reported whether they confirm or challenge the claims.

**Keywords:** agentic software engineering; human-agent collaboration; repository governance; governance continuity; software invariants; durable project state; evidence-gated workflows; brownfield adoption; provenance typing; conformance; agent evaluation.

## 1. Introduction

A coding agent is given a task, loads context, changes code, and finishes a session. The next session may use a different model, a different machine, a different developer, or the same agent after its context has been reset. The source code survives, but the reason the change was safe may not.

The missing state is often more important than a forgotten fact. It includes what the work was trying to accomplish, what was authorized, what must not be weakened, which decisions were already made, what counts as completion, what evidence exists, what remains uncertain, and what should happen next. These facts are commonly scattered across chat logs, prompt transcripts, local scratchpads, issue trackers, private planning documents, model memory, and operator knowledge.

We originally described this primarily as **session amnesia**. That remains a useful symptom, but it is not the whole problem. The deeper problem is **governance discontinuity**. A project may preserve its files while losing the durable state that tells the next worker how those files may legitimately change.

This distinction matters because software work is no longer performed by one stable actor. A repository may move from a human developer to Codex, from Codex to Claude, from one workstation to another, from a CLI to a graphical operator, or from a local session to a clean clone. If the governing state only exists inside one worker's context, every handoff is a partial reset.

RepoPact starts from a simple observation: the repository is the one artifact every legitimate participant already has to obtain. It is persistent, diffable, reviewable, replicated, branchable, and already tied to the code, tests, history, and change-control mechanisms software engineering trusts. RepoPact makes that same repository carry the durable state required to govern the work.

The central idea is:

> A fresh human or agent should be able to clone the repository and recover not only the code, but the governed state needed to continue the work responsibly.

That makes the repository a **rendezvous point** between humans and agents. The UI does not own a separate truth. The agent runtime does not own a separate truth. A chat transcript does not own the project state. A local database does not own it. The repository owns the durable contract.

This does not mean every useful fact must live in Git, or that runtime memory is unnecessary. RepoPact is deliberately narrower. It governs the load-bearing state that must survive worker, session, machine, and provider changes if later work is expected to remain inspectable and accountable.

The ecosystem's existing mechanisms each cover part of this problem. `AGENTS.md`, `CLAUDE.md`, editor rules, and similar files tell an agent how to behave. Agent-memory systems improve recall across calls. ADRs preserve decisions. CI and policy-as-code enforce checks. Issue trackers preserve tasks. Sandboxes and authorization systems govern live execution. These are useful and often necessary, but they usually do not combine project intent, authority, evidence, lifecycle, provenance, conformance, and drift into one repository-native contract.

RepoPact does not replace those systems. It gives them a durable governance layer to meet in.

The practical distinction is still useful:

> `AGENTS.md` tells an agent how to behave. RepoPact records and enforces whether the work respected the contract.

This paper makes six contributions.

1. **A repository-native governance model.** RepoPact models durable project governance as typed records in the version-controlled tree, organized as a six-layer kernel, L0 through L5.

2. **Governance continuity.** We define a worker handoff as successful when a fresh worker can recover the repository's governed state and current violations from the repository itself, without depending on the previous worker's private context.

3. **The binding invariant as a first-class primitive.** A binding invariant is a declared guarantee with rationale, escalation, and, where possible, machine enforcement. It is the unit a worker must not silently weaken.

4. **A typed enforcement lattice.** Invariants are classified by logical kind: state, state fixpoint, transition, temporal, relational, and meta. The kind predicts the required enforcer.

5. **A brownfield adoption trilemma and provenance-typed resolution.** In a concrete-only record language, no migration from a RepoPact-naive project can be total, faithful, and closed at once. Provenance-typed records allow reconstructed state to remain valid without pretending it is proven.

6. **A falsification-oriented evaluation program.** RepoPact is evaluated using the packaged product, a public Proving Ground, adversarial findings, a formal model, a machine-checkable conformance suite, naturalistic field observations, and pre-registered comparative benchmarks. Disconfirming results are in scope by design.

RepoPact is open source under Apache-2.0. The current public release is 3.0.2. It is part of the ForgeWire Labs inspectable-infrastructure stack, but the system and model described here are independently useful.

## 2. Background and Related Work

### 2.1 Agent context files

Agent context files such as `AGENTS.md`, `CLAUDE.md`, `.cursor/rules`, and editor-specific rule files have become a practical way to orient coding agents. They are easy to read, easy to commit, and easy to adopt. They can explain coding style, tests, project structure, preferred tools, and local conventions.

Their strength is also their boundary. They are instructions. By themselves they do not provide typed work records, lifecycle semantics, evidence-gated completion, provenance, conformance fixtures, or machine-enforced frozen surfaces. A context file can say, "Do not weaken authentication." It cannot by itself prove that a work item preserved the authentication invariant, reject completion without evidence, or distinguish a reconstructed claim from a proven one.

RepoPact treats context files as input, not competition. During adoption, nested context contracts can be registered and mapped into the repository's governance structure. Their content remains available to agents. RepoPact adds typed authority, evidence, lifecycle, and validation around them.

### 2.2 Agent memory and external state

External memory systems, retrieval-augmented generation, persistent scratchpads, vector stores, agent databases, and runtime memory frameworks give agents access to information beyond one context window. These systems are valuable, particularly for personalization, long-running assistants, and task continuity.

For software engineering, however, external memory has a portability problem. It may not be versioned with the code. It may not be reviewed in pull requests. It may not be available to a different model or provider. It may not survive a machine change. It may preserve facts while losing authority, acceptance criteria, provenance, or evidence.

RepoPact does not try to replace external memory. It asks a narrower question: which project facts are important enough that the next legitimate worker should be able to recover them from the artifact being changed? Those facts belong in the repository-native governance layer.

### 2.3 Issue trackers and planning systems

Issue trackers are effective at managing human-visible work queues, collaboration, comments, and organizational planning. They are often the right tool for broad product management. Their limitation for agent portability is that the tracker is usually external to the repository and may be provider-specific.

RepoPact does not attempt to replace every tracker feature. Its work ledger exists because implementation authority, acceptance criteria, dependencies, provenance, and evidence are part of the state a cloned repository may need to carry with it. External trackers can still exist, but RepoPact treats repository-local state as the durable contract that cannot depend on one SaaS account being present.

### 2.4 Runtime governance and execution controls

Agent-governance systems often focus on live execution: sandboxing, tool authorization, policy gates, intent verification, audit logging, approval, network control, and secret handling. These controls protect actions while an agent is running.

RepoPact guards a different boundary. Runtime controls ask whether a worker may perform an action now. RepoPact asks whether the project records enough intent, authority, evidence, provenance, and history that the resulting work remains inspectable and recoverable later.

The approaches compose naturally. A sandbox can prevent dangerous execution. A repository-native invariant can prevent silent weakening of a declared guarantee. Runtime audit logs can explain what an agent did. RepoPact can record whether the delivered work satisfied the repository contract.

### 2.5 Decision records, policy-as-code, and architecture fitness functions

ADRs preserve decisions. Conventional commits structure history. OPA, Conftest, CI gates, and architecture fitness functions enforce specific policies. Developer portals and scorecards evaluate services at organizational scale. These systems solve real problems, but usually at one slice of the governance surface.

RepoPact combines:

* a declared guarantee,
* its rationale,
* the authority boundary,
* the escalation path,
* the work item,
* acceptance criteria,
* linked evidence,
* lifecycle state,
* provenance type,
* derived views,
* and drift detection.

The point is not that every project needs all of these. The point is that when they matter, they should form one recoverable system rather than a collection of disconnected conventions.

## 3. Model

RepoPact is modeled as a layered governance kernel over a repository.

| Layer | Name | Object | Role |
| --- | --- | --- | --- |
| L0 | Record store | typed repository state `s` | stores governing source records |
| L1 | Lifecycle FSM | per-work-item automaton `M_w` | models work state and authority transitions |
| L2 | Invariant monitor | predicate `I`; language `R` | decides conformance |
| L3 | Enforcement lattice | typed invariants | maps invariant kind to an appropriate enforcer |
| L4 | Derive layer | projections `pi` | generates dashboards and specification views |
| L5 | Adoption boundary | migration and external state | brings previously ungoverned state into the pact |

L0 through L3 operate on state already represented in the repository. L4 derives materialized views. L5 is the boundary where RepoPact encounters state it does not yet contain, such as trackers, private documents, conversation history, and legacy planning systems.

### 3.1 State, records, and provenance

A repository state is a finite typed record store:

```text
s = <ver, Inv, Frz, Own, Reg, C, W, E, D, P, A, Prov>
```

where:

| Symbol | Component | Typical source location |
| --- | --- | --- |
| `ver` | semantic version | `VERSION` |
| `Inv` | declared invariants | `governance/invariants.json` |
| `Frz` | frozen surfaces | `governance/frozen-surface.json` |
| `Own` | scopes, roles, concurrency rules | `governance/owners.json` |
| `Reg` | audit registry | `audits/registry.json` |
| `C` | contract files | `**/AGENTS.md` and registered contracts |
| `W` | work items | `work/<status>/<id-slug>/work-item.json` |
| `E` | evidence runs | `evidence/runs/<id>.json` |
| `D` | decisions | `decisions/<id-slug>.md` |
| `P` | policies | `governance/policies/<id-slug>.md` |
| `A` | audit findings | `audits/findings/<id-slug>.json` |
| `Prov` | provenance typing | record fields and validator semantics |

Each relevant record `r` has a provenance type:

```text
prov(r) in {concrete, provisional, inferred}
```

A **concrete** record asserts that its claim is directly authored or backed by concrete evidence. A **provisional** record is valid but not complete. An **inferred** record is reconstructed from available signals rather than directly proven.

The distinction is epistemic, not cosmetic. Validity does not require pretending every known thing is fully proven. It requires that uncertainty be represented honestly. Completion remains stricter: completed work must rest on concrete evidence.

A work item is modeled as:

```text
w = (id, title, status, owner, affected_scopes, dependencies, AC, created, updated, prov)
```

with:

```text
status(w) in {proposed, active, blocked, deferred, completed}
```

These are not merely progress labels. They encode authority.

* `proposed` captures candidate work without authorizing implementation.
* `active` records accepted work that is authorized to proceed.
* `blocked` records accepted work that cannot currently proceed.
* `deferred` records accepted work that has intentionally been postponed.
* `completed` records delivered work whose acceptance criteria are evidence-closed.

A criterion is:

```text
c = (criterion_id, status, evidence_links)
status(c) in {pending, satisfied, waived}
```

The work item's declared status and its lifecycle directory must agree. That agreement is an invariant, not an assumption.

### 3.2 Governance continuity

RepoPact's newer implementation experience suggests a useful concept that is broader than session memory: **governance continuity**.

Let the recoverable governance projection of repository state `s` be:

```text
G(s) = <Inv, Frz, Own, Reg, C, W, E, D, P, A, Prov>
```

Let `Viol(s)` be the current set of conformance violations. A worker handoff is governance-continuous when a fresh legitimate worker, starting from a clone and the versioned RepoPact semantics, can recover both the governed state and its known violations without access to the predecessor's private context:

```text
recover(clone(s)) = <G(s), Viol(s)>
```

This is intentionally stronger than "the next agent can read the code" and weaker than "the repository contains everything anyone knows." RepoPact does not claim omniscience. State that never crosses L5 cannot be recovered from the repository.

The practical requirement is that load-bearing governance does not depend on worker identity.

A transition such as:

```text
human -> agent
agent A -> agent B
machine A -> machine B
CLI -> Workbench
local checkout -> clean clone
```

should not require the project to invent a new source of truth. The same repository should expose the same governed state to each participant.

This gives RepoPact a useful design test: if a capability only works because one client keeps hidden local state that another legitimate client cannot recover, that hidden state must either be made non-authoritative or moved into a governed repository representation.

Governance continuity is also allowed to recover an invalid state. If a repository has drifted, the next worker should see the drift rather than receive a falsely clean reconstruction. This is why the recovery target includes `Viol(s)` rather than only `G(s)`.

### 3.3 Lifecycle automaton, L1

Per work item, RepoPact defines a lifecycle automaton:

```text
M_w = (Q, Lambda, delta_w, Q0)

Q = {proposed, active, blocked, deferred, completed}
Q0 = {proposed, active}
```

A work item can be born as accepted work (`active`) or as a candidate (`proposed`). The `proposed -> active` transition is therefore an authority event. It is the point at which recorded intent becomes accepted implementation work.

The transition relation is deliberately permissive. Work can be blocked, deferred, reopened, or moved backward when reality changes. RepoPact prefers explicit degradation over pretending that progress is monotonic.

The completion edge is semantically guarded by:

```text
g_done(w, s) =
  every acceptance criterion is not pending
  and every satisfied criterion links evidence that exists
  and completed work is concrete
  and concrete completed work does not rest on non-concrete evidence
```

RepoPact does not require every filesystem edit to pass through a runtime gate. Humans and agents can still edit files directly. The checkpoint decides whether the resulting repository state is admissible.

That separation is important. RepoPact is repository-native, not runtime-exclusive.

### 3.4 Invariant monitor, L2

The canonical validator computes a finite set of violations:

```text
Viol(s)
```

Define:

```text
I(s) iff Viol(s) = empty
R = {s | I(s)}
```

`R` is the conformant repository language. A conformant implementation accepts repositories in `R` and rejects the rest for the rules represented in the specification and conformance corpus.

The predicate decomposes into checks including:

| Predicate | Meaning |
| --- | --- |
| `I_ver` | version is well formed |
| `I_struct` | records satisfy their schemas |
| `I_contract` | required contracts are present and registered |
| `I_id` | identifiers, paths, and lifecycle directories agree |
| `I_ref` | dependencies, evidence, owners, scopes, and decisions reference known records |
| `I_accept` | completion and satisfied criteria are evidence-closed |
| `I_acyclic` | work dependencies are acyclic |
| `I_conc` | disjoint-scope concurrency rules hold when enabled |
| `I_orphan` | planning content does not exist invisibly outside the ledger |
| `I_prov` | provenance is valid and completion does not rely on non-concrete proof |
| `I_derive` | enforced generated artifacts match their canonical projection |
| `I_frozen` | protected changes receive the required acknowledgement at diff time |

The monitor property is:

```text
for any edit trace s0 -> s1 -> ... -> sk,
the checkpoint admits sk iff sk is conformant for the enforced surface.
```

The edit trace may come from the CLI, Workbench, an agent, a human, or arbitrary filesystem operations. RepoPact's claim is not that invalid edits cannot be made. Its claim is that invalid governed states can be detected, rejected, and explained at the checkpoint.

### 3.5 Typed enforcement lattice, L3

Not all invariants are the same kind of proposition.

| Type | Example | Needed information | Typical enforcer |
| --- | --- | --- | --- |
| state | completed implies no pending criterion | one tree | validator |
| state | satisfied implies linked evidence | one tree | validator |
| state with provenance | completed implies concrete proof | one tree | validator |
| state fixpoint | dashboard equals canonical projection | one tree plus generator | validator and generator |
| transition | frozen-surface change requires acknowledgement | base and head | diff-time checker |
| temporal | completed history is not rewritten to look cleaner | git trace | history analysis and review |
| relational | nested contract refines parent contract | contract pair and semantic order | review, future formalization |
| meta | critical state does not live only in conversation | repository plus judgment | partial checks and human review |

This lattice prevents RepoPact from pretending one validator can prove everything. A schema cannot enforce a historical property. A one-tree validator cannot know whether a protected path changed relative to a base. Human judgment is still required for some semantic relationships.

The goal is explicit enforcement boundaries, not fictional total automation.

### 3.6 Derive layer, L4

RepoPact separates source records from derived views. Source records include work items, evidence, decisions, invariants, owner maps, policies, findings, and contract registrations. Derived artifacts include dashboards and specification projections.

The principle is:

```text
derive over declare
```

Anything computable from source records should be generated rather than hand-maintained.

For example:

```text
pi_dashboard(s) -> audits/reports/dashboard.md
pi_spec(s) -> SPEC.md derived blocks
```

Where enforced, a generated artifact is valid only when it matches its canonical projection. This prevents a status page from quietly becoming a second source of truth.

A generated dashboard still does not prove that every human-authored record describes external reality. It proves that the materialized view matches its source records. Semantic truth remains a separate responsibility.

### 3.7 Adoption boundary, L5, and the concrete-record trilemma

Brownfield projects already have history, issue trackers, planning docs, nested context files, CI workflows, CODEOWNERS, roadmaps, and implicit knowledge. Some state can be reconstructed from the repository. Some cannot.

A migration faces three goals:

1. **Totality.** It should operate on arbitrary input trees.
2. **Faithfulness.** It should preserve observed signals without inventing proof or discarding load-bearing information.
3. **Closure.** Its output should lie in the conformant language.

In a concrete-only record language, those goals cannot always hold together.

A legacy roadmap may say a task is complete but contain no evidence. A nested contract may name a scope with no owner record. A planning import may contain cyclic dependencies. A historical decision may have missing dates or unknown authority. If migration forces every reconstructed fact to be concrete, it must either invent missing proof or throw away the signal. Both break faithfulness.

This is the **concrete-record adoption trilemma**:

```text
total + faithful + closed cannot always hold when every emitted record is concrete
```

RepoPact resolves the problem by changing the record language. Reconstructed facts can be valid as `inferred` or `provisional`. The migration can preserve what it observed while being honest about what it did not prove.

Under the provenance-aware language, adoption can remain total, faithful, and closed while completion still requires concrete proof.

The type system therefore gives the repository a legitimate way to say, "we know this because we reconstructed it," rather than forcing a choice between silence and fabrication.

### 3.8 Action taxonomy

RepoPact actions differ in how they interact with conformance.

| Class | Examples | Intended property |
| --- | --- | --- |
| constructor | `init` | creates a valid governed repository from a clean target |
| record creation | `new` | creates a valid preflight work item |
| derive/read | `validate`, `dashboard`, `spec`, `check-frozen` | reads or regenerates projections without silently changing source intent |
| lifecycle mutation | create, edit, transition | valid when the post-state satisfies the governing invariants |
| repair | `doctor --fix` | conservative repair toward conformance |
| migration | `adopt`, `import-plan` | crosses L5 and may emit concrete, inferred, or provisional records |
| diff-time enforcement | `check-frozen --base` | evaluates two-state protected-surface rules |

`doctor` is intended as a conservative repair and ratchet operator. It should reduce known violations, leave a healthy repository unchanged, and avoid overwriting differing source records as if local intent were disposable.

## 4. Reference Implementation

The current RepoPact release line is no longer accurately described as only a Python CLI. The architecture has evolved while preserving one important rule: there should be **one semantic authority** for the governance model.

The current public release is 3.0.2. The implementation includes a canonical Rust semantic engine, a compatibility-oriented Python command surface, a Tauri 2 Workbench, schemas, templates, a conformance suite, migration tools, evidence records, and the research corpus.

### 4.1 Canonical Rust semantic engine

RepoPact's recent implementation work moved proven semantic surfaces into a reusable Rust core rather than creating separate implementations for every client.

The canonical Rust surfaces include repository discovery and modeling, schema-backed validation, structured diagnostics, dashboard rendering, relationship graphs, deterministic analysis, typed work-item creation, typed work-item editing for proven fields, lifecycle transition, and content-addressed mutation planning and recovery.

The design goal is not "all code must be Rust." It is "one semantic engine." Historical or migration-oriented operations may still live in Python when they have not been moved or proven in Rust, but final repository-validity decisions should converge on the canonical semantic authority rather than allow two implementations to drift silently.

For compatibility, Python can act as a local client to the Rust engine using a versioned process protocol. Native clients such as the Tauri Workbench can call the same Rust crates directly.

Conceptually:

```text
human / agent / automation
          |
          +-------------------------+
          |                         |
          v                         v
Python compatibility CLI       Tauri Workbench
          |                         |
          v                         |
versioned local protocol            |
          |                         |
          +------------+------------+
                       v
              canonical Rust engine
                       |
                       v
repository model, validation, graph,
analysis, mutation, projection
```

This architecture matters for governance continuity. The UI and the agent-facing tooling are different interaction surfaces over the same governed state, not separate products with different semantics.

### 4.2 CLI surface

The public command surface includes operations such as:

| Command | Purpose |
| --- | --- |
| `init` | seed a valid RepoPact into a new repository |
| `adopt` | map existing repository signals into RepoPact records |
| `import-plan` | import legacy planning material |
| `new` | create preflight work items |
| `validate` | check structural and semantic conformance |
| `dashboard` | generate dashboard views from source records |
| `spec` | generate or check specification projections |
| `check-frozen` | enforce frozen-surface changes against a base |
| `doctor` | diagnose, repair, and migrate drift |

Mandatory preflight makes intent visible before implementation begins. Existing repositories use migration and grandfathering semantics rather than pretending historical work was preflighted when it was not.

### 4.3 Human operator surface

RepoPact Workbench is a Tauri 2 application that exposes repository governance to a human operator without creating a second source of truth.

The Workbench is not a generic Markdown editor. It is a typed client of the canonical engine. It presents repository state, validation, work lifecycle, decisions, evidence, relationships, analysis, session state, and governed mutations through user-facing views.

This is important because inspectability is not complete if only agents have practical access to the governance controls. Human operators must be able to inspect the same state and exercise the same core authority without needing to ask an agent to mutate repository records on their behalf.

The release standard for the Workbench is therefore **operator control parity for core governance**, not pixel-perfect identity with the CLI. A human should be able to inspect, create, edit, and transition governed work using the same semantic engine that an agent or automation path uses. Remaining UI gaps are treated as release hardening, not as permission for a hidden second implementation.

The current information architecture is adaptive and task-oriented. Work is separated into Proposed, Active, Deferred, and Complete views, with blocked work surfaced explicitly rather than relabeled. Decisions, evidence, validation, graph relationships, analysis, and settings use bounded views rather than long mixed scroll surfaces.

### 4.4 Cross-platform posture

The Workbench shares one Tauri 2 application architecture across desktop and mobile targets.

Current evidence includes:

* Windows installer and application work,
* Linux `.deb` build, install, launcher integration, and launch validation,
* Android project initialization, build, installation, and launch on a real target.

macOS and iOS remain intended targets and already share application and asset foundations, but they should not be described as validated until native platform evidence exists.

The cross-platform point is not a marketing checkbox. Governance continuity is more credible when the durable project state can be inspected from different environments without moving authority into one machine-specific local database.

### 4.5 Validation and conformance

Validation has two broad layers.

First, JSON Schema validates individual record structure.

Second, semantic validation checks cross-record rules such as referential integrity, status-directory agreement, dependency cycles, evidence links, scope validity, concurrency rules, provenance, orphan planning content, and derived-artifact consistency.

A standard without conformance is only a convention. RepoPact therefore publishes a versioned conformance surface including `CONFORMANCE.md`, fixture repositories under `conformance/`, the suite manifest, and the conformance runner.

A third-party implementation does not need to copy RepoPact's internal code to claim compatibility. It needs to reproduce the specified accept/reject behavior for the versioned conformance surface.

This separates four claims that are easy to blur:

1. the paper's formal model,
2. the repository contract,
3. the machine-checkable conformance corpus,
4. the evidence produced by concrete runs.

### 4.6 Proving Ground split

RepoPact defines the governance model, validator semantics, conformance expectations, and research protocols. Runnable adversarial experiments live separately in **RepoPact Proving Ground**.

The split is deliberate:

```text
RepoPact defines the pact.
RepoPact Proving Ground tests whether the pact holds under pressure.
```

The Proving Ground consumes released RepoPact artifacts rather than depending only on the author's source checkout. That distinction matters because a user receives a packaged product, not the development tree.

## 5. Evaluation Method

The evaluation has two parts.

The first is reflexive and adversarial. It asks whether RepoPact catches the failure classes it claims to catch and whether real adoption exposes contradictions in the model.

The second is comparative. It asks whether RepoPact changes outcomes relative to a fair baseline when model, task, source, and harness are held constant.

### 5.1 Reflexive adversarial falsification

The reflexive protocol defines hypotheses H1 through H7 and falsification criteria before the relevant runs. The subject under test is the packaged product, not merely the local source tree.

| Hypothesis | Claim |
| --- | --- |
| H1 | an adopter can install the package and reach a valid governed repository |
| H2 | advertised commands are closed over the initialized repository surface |
| H3 | completion is evidence-gated |
| H4 | frozen-surface and invariant authority are binding |
| H5 | state-integrity violations are rejected |
| H6 | a reader can recover project state from the tree alone |
| H7 | brownfield adoption is non-destructive, sound, and honestly typed |

The falsification criteria are intentionally direct. A documented command crashing, a validator accepting unproven completion, a protected surface changing without the required acknowledgement, status-directory mismatch passing validation, or project recovery requiring missing chat history are not dismissed as cosmetic implementation bugs. They are evidence against the relevant claim.

Every defect is fed back through RepoPact's own machinery: finding, work item, decision, evidence, and validation. This is itself a test of recoverability.

### 5.2 Comparative benchmark program

The comparative program uses:

```text
condition in {baseline, repopact}
```

Source, task, model, and harness are held constant. The governance layer changes.

| Study | Hypothesis | Construct | Primary outputs | Status |
| --- | --- | --- | --- | --- |
| S1 | H8 | guarantee-violation detection, PactBench | catch/escalate rate, false-stop rate, confusion matrix | runnable infrastructure present; real results pending |
| S2 | H9 | cross-session recovery and efficiency | resolution rate, recovery score, tokens, interventions | protocol defined |
| S3 | H10 | multi-agent coordination | conflict rate, duplicated work, joint success | protocol defined |
| S4 | H11 | context-token economy | cost-success frontier, scaling curve | protocol defined |
| S5 | H12 | drift detection and staleness | detection rate, latency, silent-staleness rate | drift harness present; comparative results pending |
| S6 | H13 | defensive security and injection resistance | defensive catch rate, injected-context-followed rate | tasks scoped; real results pending |
| S7 | H14 | enforcement closure and longitudinal drift | checkpoint coverage, invocation, effectiveness, nonconformant admission | pre-registered; not yet completed |

The current benchmark maturity should be described precisely. The program is pre-registered and partially implemented. Deterministic mock runs validate plumbing, not agent behavior. Cross-model findings should not be claimed until real runs are complete.

### 5.3 PactBench

PactBench measures whether agents silently weaken declared guarantees less often when RepoPact governance is present.

The core task shape is:

1. A repository contains a declared guarantee.
2. A task creates pressure to take a shortcut.
3. The shortcut would violate the guarantee.
4. Correct behavior is to preserve, refuse, escalate, or provide valid evidence.
5. The scorer records silent weakening, correct preservation, escalation, or false stopping.

The suite includes correctness and security classes, evidence-fabrication pressure, context-injection cases, real fixtures, a model-agnostic harness, deterministic pipeline selftests, a subprocess interface for real agents, and drift experiments.

## 6. Results to Date

The reflexive findings register currently contains fourteen entries. Severity reflects impact on an adopter rather than implementation effort. "Holds" means one defined adversarial case behaved as intended; it does not mean the system is proven globally safe.

| ID | Hypothesis | Severity | Finding | Resolution |
| --- | --- | --- | --- | --- |
| F-001 | H2 | major | `spec` crashed on an `init`-fresh repository | fixed and re-verified |
| F-002 | H4 | minor | `check-frozen` was blind to working-tree edits | fixed and re-verified |
| F-003 | H3 | holds | satisfied criterion without evidence rejected | n/a |
| F-004 | H5 | holds | status-directory mismatch rejected | n/a |
| F-005 | H4 | holds | protected committed change required acknowledgement | n/a |
| F-006 | H1, H6 | holds | full work item recovered from tree alone | n/a |
| F-007 | H7 | holds* | real progenitor brownfield adoption was non-destructive | confirmatory only |
| F-008 | H7 | major | adopter `.gitignore` swallowed governance evidence | fixed with warning and repair guidance |
| F-009 | H7 | holds | clean-room adoption of an unrelated OSS repository reached conformance | n/a |
| F-010 | H7 | major | adoption left the real planning ledger outside RepoPact | fixed with `import-plan` |
| F-011 | H7 | major | an older adopter drifted invalid as the standard evolved | fixed with `doctor` path |
| F-012 | H7 | holds | full lifecycle worked on a different-domain application | shipped |
| F-013 | H7 | holds | governance-folder planning migrated without data loss | shipped |
| F-014 | H6, H7 | holds | downstream adoption exposed a missing authority state and the full resolution trace remained recoverable | shipped |

### 6.1 Failures that changed the design

Several defects were useful because they broke assumptions rather than merely breaking code.

**F-001, surface closure.** A documented command crashed on an `init`-fresh repository because it expected a specification file bootstrap had not created. This contradicted H2. The command was corrected and the behavior re-verified from a rebuilt package.

**F-002, working-tree protection gap.** The frozen-surface checker originally evaluated committed ranges but could miss an uncommitted local edit. That weakened the practical meaning of local preflight. The checker was changed so protected working-tree edits are visible before commit.

**F-008, the swallowed record.** A brownfield repository's existing `.gitignore` rule matched `evidence/runs/`. The repository could validate on the author's disk while a clean clone lost the ignored evidence. This is exactly the kind of failure repository-native governance is supposed to make hard to hide. Adoption now checks whether generated records are ignored and surfaces actionable warnings.

**F-010, the hollow ledger.** Adoption could produce a structurally valid RepoPact tree while the team's real planning still lived in a legacy plan directory. The result was formally governed but practically misleading. This motivated `import-plan`, which brings legacy planning into the governed work ledger without inventing evidence.

**F-011, longitudinal drift.** An older adopter fell out of conformance as the standard evolved. Validation could detect the problem when invoked, but the adopter had no guided repair path. This motivated `doctor` as an upgrade and repair mechanism.

These failures matter because they show that the evaluation can change the architecture. They were not edited out of the story once fixed.

### 6.2 What held

Several adversarial cases behaved as intended.

A satisfied criterion without evidence was rejected. Status-directory mismatch was rejected. Protected changes required acknowledgement. A reader could reconstruct work intent, decision context, and proof from the tree without relying on chat history.

These are bounded results. They support the tested mechanisms for the tested cases. They do not prove there is no bypass.

### 6.3 Brownfield adoption and provenance

Brownfield adoption has been exercised against projects with different levels of independence and complexity. The progenitor repository is useful but confirmatory because RepoPact was distilled from the practices it already used. An unrelated open-source repository provided a cleaner sparse-signal adoption case. Different-domain applications exercised adoption, planning import, repair, and lifecycle behavior.

The main theoretical shift came from realizing that brownfield reconstruction should not force uncertain state to masquerade as fact. Provenance typing allows the repository to preserve useful reconstructed state while keeping completion strict.

This distinction is also useful for agents. A system that accepts only "known" or "missing" invites fabrication under pressure. A system with explicit `inferred` and `provisional` types gives a worker an honest third option.

### 6.4 Authority typing under adoption pressure

The `proposed` lifecycle state was introduced because real usage exposed a missing authority type. Candidate work deserved durable capture but was not yet accepted for implementation.

Without `proposed`, every available mapping was dishonest in a different way. `active` granted authority too early. `blocked` implied an external impediment. `deferred` implied prior acceptance.

The system grew a type rather than tolerating ambiguity.

This mirrors the provenance lesson. Provenance types prevent a record from claiming more certainty than it has. The `proposed` state prevents a work item from claiming more authority than it has.

### 6.5 Enforcement closure field case

A naturalistic field observation in ForgeWire exposed another important boundary. RepoPact validation had accumulated a substantial reported error count while the project's ordinary tests stayed green because the governance validator was not consistently invoked in the workflow path that mattered.

The raw count was not itself a clean measure of governance drift. Investigation showed that a significant portion came from a version-specific validator defect involving local worktree scanning, while the remainder were confirmed governance discrepancies. The two classes are preserved separately in the findings rather than combined into one dramatic number.

The case supports several narrower conclusions.

First, when invoked, RepoPact detected governance discrepancies that ordinary project tests did not reveal.

Second, adoption plus a correct validator does not guarantee that the validator is exercised at the boundaries where admission happens.

Third, deployment needs three distinct properties:

* **checkpoint coverage**, whether an admission path routes through the checker;
* **checkpoint invocation**, whether the checker actually runs for a candidate;
* **checkpoint effectiveness**, whether a rejecting result prevents promotion.

These properties motivate **enforcement closure** and H14. The field observation motivated the hypothesis. It does not confirm it. S7 remains the prospective test.

## 7. Discussion

### 7.1 The repository as the common governed substrate

RepoPact's strongest practical property is not that it stores more files. It is that the same durable state is available to different legitimate workers.

A human can inspect a work item in the Workbench. An agent can inspect the same work item from the tree. Automation can validate the same acceptance criteria. A clean clone can reconstruct the same decision and evidence links. None of those clients should need a private hidden database to understand what the project believes is true.

That is the core of governance continuity.

The repository becomes the meeting point between humans and agents because it survives both sides. Humans change jobs, machines fail, models change, subscriptions change, sessions expire, and context windows reset. The repository remains.

This does not make Git magical. It simply uses a substrate software engineering already knows how to replicate, review, diff, and audit.

### 7.2 Human operator control is part of inspectability

A governance system is incomplete if humans can technically inspect the files but must ask an agent to perform ordinary governance actions for them.

RepoPact Workbench is therefore not just presentation polish. It is the human operator surface of the same semantic system. Creating, editing, transitioning, validating, and understanding governed work must be available to humans without weakening the model or creating a separate UI-owned truth.

This is why UI parity is a release concern. A missing button is not always cosmetic if the missing action forces the human operator to leave the governed interaction model or delegate authority merely to compensate for an incomplete client.

At the same time, parity should be defined semantically. The Workbench does not need to reproduce every CLI command literally. It needs to expose the core governance capabilities a human operator requires while preserving the same underlying authority and validation rules.

### 7.3 Why repository-native governance

Repository-native governance creates ceremony. The claim is not that ceremony is free or appropriate everywhere.

It is most justified when:

* multiple humans or agents will touch the same work,
* project guarantees must survive context loss,
* authority boundaries matter,
* completion must be evidence-backed,
* uncertainty must be represented honestly,
* and future workers must be able to recover state from the repository alone.

For a throwaway script or low-risk experiment, this may be unnecessary. For long-lived agentic engineering, the cost-benefit calculation changes because high-speed workers can also create high-speed drift.

### 7.4 Enforcement without pretending total automation

RepoPact does not claim every invariant can be fully automated. The typed enforcement lattice is partly a restraint mechanism.

Some rules are decidable from one tree. Some require a diff. Some require history. Some still require human judgment. The system should say which category a guarantee belongs to rather than imply that a JSON schema somehow proves semantic intent.

The benefit is not perfect automation. It is explicit enforcement boundaries and durable evidence about what was actually checked.

### 7.5 Provenance and authority are both type problems

Two of RepoPact's most useful changes came from refusing to force messy reality into dishonest binary states.

Provenance typing says a reconstructed fact can be useful without being concrete proof.

The proposed lifecycle state says captured work can be useful without being implementation authority.

Both changes follow the same rule: when the system repeatedly has to lie to fit reality into the schema, the schema is missing a type.

### 7.6 The L5 boundary remains real

RepoPact only governs what crosses into the repository. It cannot recover a decision that was never written down, a private conversation it never received, or intent that exists only in someone's head.

This limitation should remain explicit. Governance continuity is a property of represented state, not omniscience.

Future integrations with trackers, design systems, and external planning sources should therefore preserve provenance and authority rather than pretending ingestion turns every external statement into fact.

### 7.7 Relationship to runtime governance

RepoPact is not a sandbox, authorization server, agent firewall, or distributed execution system.

A secure agentic engineering stack likely needs both runtime and repository governance. Runtime controls limit what can happen now. Repository governance limits what can be accepted as durable project state later.

The distinction is useful because the two failure modes are different. A runtime may safely execute a command that still produces a governance-invalid change. A repository may remain perfectly conformant while a live tool call violates a runtime security boundary. One system should not pretend to replace the other.

## 8. Threats to Validity

### T1: Reflexivity

RepoPact was distilled from the author's own agentic engineering practices. That gives it practical relevance but threatens generality. ForgeWire, the progenitor adopter, is confirmatory rather than independent. RepoPact's own repository is also ForgeWire Labs controlled.

Unrelated repositories, different domains, outside operators, and third-party reproduction are necessary before broad generality claims are warranted.

### T2: Single evaluator and operator effects

Much of the early evidence comes from one primary human operator working with several AI systems. The process may reflect that operator's habits, technical judgment, model choices, and tolerance for governance ceremony.

Exact commands, public artifacts, evidence records, and falsification criteria improve inspectability but do not remove this threat.

### T3: Scale and domain narrowness

Current evidence is not representative of every enterprise, regulated environment, monorepo, or many-agent team. Some adoption subjects are substantial, but organization-scale claims require broader evidence.

### T4: Holds are not proofs

Catching one adversarial case proves only that the tested case was caught under the tested conditions. It does not establish universal safety.

### T5: Benchmark curation

PactBench could overfit to RepoPact's strengths. Pre-registration, fair baselines, frozen task identities, and publication of disconfirming results are necessary mitigations.

### T6: Baseline fairness

A deliberately weak baseline would exaggerate RepoPact's value. Comparative studies should include realistic convention files and, where appropriate, external-memory or retrieval support. Governance ceremony must count as cost.

### T7: Token and cost measurement

Token-economy claims depend on model, tokenizer, provider pricing, caching, prompt construction, and retrieval behavior. Raw and adjusted measurements should be reported rather than reduced to one headline number.

### T8: Drift and security realism

Synthetic drift and defensive-security tasks may not reflect real projects. Security work must remain benign and sandboxed. RepoPact's own records must also be treated as an attack surface.

### T9: Provenance misuse

If users treat `inferred` or `provisional` records as equivalent to concrete proof, provenance typing loses its value. Completion restrictions and UI visibility are therefore part of the model, not merely presentation.

### T10: Standard and implementation coupling

The canonical implementation currently defines a large part of the operational semantics. The conformance suite mitigates this by giving alternate implementations an observable target, but the standard must continue to resist accidental dependence on private implementation details.

### T11: Workbench maturity

The existence of a polished-looking UI can create a false impression that every governance capability has reached operator parity. UI claims must distinguish between implemented, wired, validated, and merely planned controls. Missing operator controls should remain visible as release work rather than being hidden by overall visual quality.

## 9. Conclusion and Future Work

Agentic software engineering does not only have a context problem. It has a continuity problem.

Code can survive a session while the governing state around that code disappears. The next worker may receive the files but not the intent, authority, evidence, decisions, provenance, or invariants that made the previous work legitimate.

RepoPact addresses that problem by making the version-controlled repository carry a typed governance layer. A fresh human or agent can clone the project and recover not only source code, but the governed state needed to understand what work exists, what is authorized, what must not be weakened, what has been proven, and what remains uncertain.

This paper calls that property **governance continuity**.

RepoPact models the repository as a six-layer governance kernel with typed records, lifecycle authority, invariant monitoring, a typed enforcement lattice, derived views, and an adoption boundary. Its central primitive is the binding invariant. Its provenance types let reconstructed state remain useful without pretending to be proven. Its lifecycle types let candidate work remain visible without pretending to be authorized.

The implementation has also moved toward the architectural implication of the model: one semantic authority, multiple legitimate interaction surfaces. The canonical Rust engine supports agent, CLI, automation, and Tauri Workbench clients without requiring each client to invent its own governance semantics. Human operator control and agent interoperability therefore become two views of the same repository state rather than separate systems.

The evaluation so far is intentionally incomplete. Real failures have changed the design. Real cases have held. Comparative model results are still pending. The enforcement-closure field observation motivated a new hypothesis rather than being retroactively treated as proof.

Future work includes:

1. completing real PactBench runs across multiple model families;
2. completing comparative drift, recovery, coordination, security, and token-economy studies;
3. running the pre-registered enforcement-closure study outside ForgeWire Labs controlled repositories;
4. obtaining genuine third-party reproduction and adoption evidence;
5. finishing human operator control parity across the Workbench's core governance lifecycle;
6. validating macOS and iOS platform behavior before claiming support;
7. expanding external ingestion while preserving provenance and authority;
8. mechanizing more temporal and relational invariants;
9. hardening repair and provenance-ratcheting semantics;
10. encouraging independent conformance implementations.

The repository is already the artifact software engineering expects every worker to share. RepoPact's claim is that the same artifact can also carry the durable governance needed for humans and agents to continue each other's work without starting over.

## Ethics and Responsible Disclosure

The security portions of the evaluation are defensive and benign by design. Tasks are synthetic or sandboxed. They involve no live targets, credential theft, exploit deployment, or instructions for real-world compromise.

Security-invariant tasks focus on whether agents preserve defensive controls such as authorization checks, secret handling, input validation, evidence integrity, and protected surfaces.

Injection tasks treat both convention files and RepoPact records as potentially hostile text surfaces. RepoPact makes no immunity claim. Its defenses are structural: provenance, frozen surfaces, evidence validation, explicit escalation, and reviewable records.

Defects should be recorded openly rather than quietly patched out of the evaluation history. Agent runs should respect model-provider terms and preserve raw captures without exposing secrets.

## Data and Artifact Availability

RepoPact is open source under Apache-2.0. The formal model, experiment protocol, benchmark protocol, threats register, findings register, current paper, conformance suite, and related research records live in the public RepoPact repository.

RepoPact Proving Ground hosts runnable benchmark artifacts including PactBench tasks, fixtures, harnesses, deterministic pipeline selftests, subprocess real-runner support, drift experiments, and future result bundles.

Benchmark corrections should be issued as new task ids or dated amendments rather than silent edits. Real model result bundles should identify model, harness version, task-set version, condition, prompts or command interface, raw captures where safe, scorer outputs, and evidence links.

The conformance suite is versioned with RepoPact so the standard's observable semantics remain independently testable.

## Appendices

### Appendix A: Typed invariant lattice

| Invariant type | Example | Decidable from | Enforcer |
| --- | --- | --- | --- |
| state | completed work has no pending criteria | one tree | validator |
| state | satisfied criterion links evidence | one tree | validator |
| state with provenance | completed work is concrete | one tree | validator |
| state fixpoint | dashboard equals generated projection | one tree plus generator | validator and generator |
| transition | frozen-surface change requires acknowledgement | base and head | diff-time checker |
| temporal | completed work is not rewritten to look cleaner | git trace | review, future trace semantics |
| relational | nested contract refines parent | contract pair and refinement order | human review, future formalization |
| meta | critical state does not live only in conversation | repository plus human judgment | partial checks and review |

### Appendix B: Governance continuity sketch

Let `s` be a repository state, `G(s)` its governance projection, and `Viol(s)` its current violations.

A worker transition `h` is governance-continuous for represented state when a fresh legitimate worker can obtain a clone and recover:

```text
recover_h(clone(s)) = <G(s), Viol(s)>
```

without requiring private predecessor context.

This is a recoverability claim, not an omniscience claim. Facts that never entered the repository remain outside the model. A nonconformant repository may still satisfy governance continuity if the next worker faithfully recovers both its governed records and the violations that make the state nonconformant.

The practical falsification target is any load-bearing state that is required to continue legitimate work but exists only in a specific worker, machine, provider, UI cache, or conversation when the system claims the repository is authoritative for that state.

### Appendix C: Formal theorem sketch

Each claim is tagged by discharge status: **[def]** true by definition for the canonical implementation, **[ci]** machine-checked on ordinary validation paths, **[fix]** covered by the conformance corpus, **[conj]** a conjecture or standing falsification target.

**T1: Recognizer definition. [def]/[fix]**  
For the canonical implementation, validation accepts exactly the states in the currently specified recognized language. Alternate implementations are tested through conformance fixtures rather than by code identity.

**T2: Constructor correctness. [ci]**  
`init` should create a valid governed repository from a supported clean target.

**T3: Surface closure. [conj]**  
Advertised commands should either succeed or fail cleanly on the initialized surface without corrupting state. F-001 was an original counterexample.

**T4: Completion safety. [fix]/[conj]**  
A completed item with pending criteria, missing evidence, non-concrete status, or non-concrete supporting proof is not conformant.

**T5: Monitor non-bypass at covered checkpoints. [ci]/[conj]**  
Where an admission path is covered and the validator is invoked effectively, a nonconformant final state is rejected. H14 exists because actual deployment must separately establish coverage, invocation, and effectiveness.

**T6a: Concrete-only adoption trilemma. [structural]**  
A concrete-only brownfield migration cannot always be total, faithful, and closed simultaneously.

**T6b: Provenance-typed closure. [ci]**  
A provenance-aware migration can preserve reconstructed state as inferred or provisional while keeping completion gated on concrete evidence.

**T7: Repair monotonicity. [conj]**  
`doctor` should reduce known violations without silently replacing differing source intent and should be identity-like on already healthy repositories.

**T8: Governance continuity for represented state. [conj]/[empirical]**  
A clean worker transition should preserve recoverability of the repository's represented governance projection and current violations without predecessor-private context. F-006 provides bounded evidence for one recovery case. Broader worker, client, and machine transitions remain evaluation targets.

### Appendix D: Benchmark program

| Study | Name | Hypothesis | Status |
| --- | --- | --- | --- |
| S1 | PactBench guarantee-violation detection | RepoPact improves preservation and escalation over baseline | runnable infrastructure present; real results pending |
| S2 | Cross-session recovery | RepoPact improves recovery and reduces redo loops | protocol defined |
| S3 | Multi-agent coordination | RepoPact reduces conflicts and duplicated work | protocol defined |
| S4 | Context-token economy | RepoPact improves the cost-success frontier under realistic conditions | protocol defined |
| S5 | Drift detection | RepoPact reduces silent staleness and detection latency | drift harness present; comparative results pending |
| S6 | Defensive security and injection resistance | RepoPact improves defensive invariant preservation | tasks scoped; real results pending |
| S7 | Enforcement closure and longitudinal drift | covered, invoked, effective checkpoints admit less known-nonconformant state | pre-registered; not yet completed |
| S8 | Governance continuity | fresh human, agent, machine, and client handoffs recover equivalent governed state and known violations | proposed extension; requires dated protocol amendment before execution |

### Appendix E: Figures and tables planned

**Figure 1.** Six-layer governance kernel, L0 through L5.  
**Figure 2.** Governance continuity across human, agent, machine, and client transitions.  
**Figure 3.** Work-item lifecycle and authority transitions.  
**Figure 4.** Concrete-record adoption trilemma and provenance-typed resolution.  
**Table 1.** Typed invariant lattice.  
**Table 2.** Study status table.  
**Table 3.** Findings register summary.  
**Figure 5.** PactBench confusion matrix.  
**Figure 6.** S5 drift detection latency and silent-staleness rate.  
**Figure 7.** S4 cost-success Pareto frontier.  
**Figure 8.** S8 governance-continuity handoff matrix, if pre-registered and run.

### Appendix F: Informal references

This remains a positioning map rather than the final citation-complete related-work section. Before archival publication, each entry should be replaced with a complete bibliographic citation and a more systematic comparison.

* `AGENTS.md` and repository agent-instruction files such as `CLAUDE.md`, `.cursor/rules`, and editor-specific variants.
* ADRs and lightweight architecture decision records.
* Policy-as-code systems such as OPA and Conftest.
* Architecture fitness functions from the evolutionary-architecture literature.
* Runtime agent governance, sandboxing, and agentic-SDLC reference models.
* Agent memory systems including retrieval-augmented and persistent-memory approaches such as MemGPT/Letta, Mem0, Zep, and LangMem.
* Developer portals and service scorecards such as Backstage, Cortex, and OpsLevel.
* SWE-bench Verified and SWE-EVO for software-engineering evaluation.
* RepoPact formal model, protocol, benchmark protocol, findings register, conformance suite, and RepoPact Proving Ground.
