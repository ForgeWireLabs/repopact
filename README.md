# RepoPact

**Repository-native governance for durable human-agent software engineering.**

AI coding tools can change software faster than teams can preserve the governing context around those changes. Intent, decisions, authority, acceptance criteria, evidence, constraints, and unfinished work often live in chat histories, agent memory, issue trackers, local tooling, or one person's head.

RepoPact makes that load-bearing engineering state part of the repository itself.

A fresh human or agent can clone a governed repository and recover not only the source tree, but also the represented work state, authority boundaries, invariants, decisions, evidence, provenance, and known violations needed to continue responsibly.

> **The repository is the pact.**
>
> Humans, agents, and tools rendezvous through durable repository state rather than relying on one session or vendor to remember the project correctly.

`pip install repopact` · Apache-2.0 · stable release **3.1.1** ([changelog](decisions/0063-release-repopact-3-1-1-sdist-license-file-corrective.md))

> [!IMPORTANT]
> **GitHub Releases is not currently the authoritative source for the latest RepoPact package.** GitHub's **Latest Release** entry is out of date and may show an older version. The current stable package is published on **[PyPI](https://pypi.org/project/repopact/)**. Install or upgrade with `pip install --upgrade repopact`. The `main` branch may also contain work that is newer than the latest published package, so use PyPI when you need the current stable installable release.

[Documentation](docs/README.md) · [Paper draft](research/paper.md) · [Formal model](research/formal-model.md) · [Conformance](CONFORMANCE.md) · [Research protocol](research/protocol.md)

![RepoPact turns fragmented session context into durable repository state: humans, agents, and tools rendezvous through a repository that preserves intent, authority, decisions, work state, invariants, evidence, and provenance.](docs/assets/readme/repopact-hero.svg)

*RepoPact addresses governance discontinuity by making the repository the durable rendezvous point between workers and sessions.*

## Why RepoPact exists

Source code usually survives a handoff. The governing state around that code often does not.

A coding session ends. Another model takes over. A different developer opens the repository. Work moves to another machine. The new worker can see the files, but may not know what was authorized, why a constraint exists, which decisions are already settled, what evidence supports a claim, what is still uncertain, or what must happen before the work is actually complete.

RepoPact treats that as a software-engineering problem, not just a memory problem. The paper calls it **governance discontinuity**.

| Common failure mode | RepoPact's response |
| --- | --- |
| Agent or session memory resets | Durable governed state travels with the repository |
| Different tools reconstruct different project context | Humans and agents read the same typed, version-controlled records |
| "Done" means an agent said it was done | Acceptance criteria can require linked evidence before completion |
| Authority is implicit or scattered | Roles, scopes, frozen surfaces, invariants, and optional admission policy make boundaries explicit |
| Brownfield reconstruction is uncertain | Provenance distinguishes `concrete`, `provisional`, and `inferred` state |
| Generated views drift from source records | Dashboard and other derived projections are regenerated and validated |
| Verification is tied to one hosted provider | Repository-defined local verification is canonical; hosted CI/CD is optional |

The goal is not to slow AI-assisted development back down. The goal is to make higher implementation throughput **inspectable, recoverable, and sustainable across humans, agents, machines, providers, and sessions**.

## 30-second start

The stable 3.1.1 package creates or adopts governed repository state, runs the canonical validator, and exposes repository-defined local verification and release surfaces.

```powershell
pip install repopact

# Start a new governed repository
repopact init --target ../your-repo
cd ../your-repo

# Record work before implementation begins
repopact new work-item "Add retry policy"
repopact new work-item "Possible cache redesign" --status proposed

# Check the pact and refresh the derived view
repopact validate
repopact dashboard
```

For an existing repository:

```powershell
repopact adopt --target ../existing-repo --dry-run
repopact adopt --target ../existing-repo
repopact doctor
```

RepoPact does not require a particular model, coding agent, agent framework, editor, CI vendor, or cloud provider.

For the next step, use the [documentation map](docs/README.md): **Use RepoPact**, **Understand RepoPact**, **Integrate RepoPact**, or **Develop RepoPact**.

## How the pact works

```text
intent -> scoped authority -> work item -> implementation -> evidence -> audit -> history
```

RepoPact stores the parts of engineering state that need to survive a worker or session change:

- **Intent and decisions**: what the project is trying to accomplish and what choices are already settled.
- **Authority**: who or what may change which parts of the repository.
- **Work state**: `proposed`, `active`, `blocked`, `deferred`, and `completed` are durable lifecycle states rather than chat labels.
- **Acceptance criteria and evidence**: completion can be tied to concrete run records instead of narrative confidence.
- **Binding invariants**: guarantees with rationale, escalation, and machine enforcement where their logical type permits it.
- **Frozen surfaces**: paths or symbols that cannot be casually changed without explicit review.
- **Provenance**: reconstructed state can remain visibly provisional or inferred instead of being promoted to fact.
- **Reconciliation**: generated views and audits expose drift instead of hiding it.

A work item is a narrative `README.md` plus machine-readable `work-item.json`. Evidence lives under `evidence/runs/`. Decisions and policies remain durable after the implementation session is gone. The validator checks structural and cross-record semantics, and the dashboard is generated from source records rather than maintained by hand.

RepoPact's distinguishing primitive is the **binding invariant**: a declared guarantee coupled to rationale, escalation, and, where logically possible, an enforcer. The invariant is the thing a later worker must not silently weaken.

## Authority: source records vs derived views

RepoPact deliberately separates authority from convenience.

Authoritative project state comes from the governed source records and repository state that own a fact: `governance/`, `work/`, `decisions/`, `evidence/`, source code, and explicitly governed configuration. Different record types own different kinds of truth.

The dashboard, Repository Orientation Graph (ROG), Workbench views, generated specification blocks, indexes, and similar read models are **derived**. They can make the repository dramatically easier to inspect and operate, but they do not gain authority merely because they are easier to query.

If a derived view conflicts with the source records it projects, the derived view is stale or wrong and must be regenerated or reconciled.

## Enforcement is a ladder, not a boolean

RepoPact does not describe every integration as simply "enforced" or "not enforced." The assurance class must match the boundary that actually exists.

| Class | What it means |
| --- | --- |
| `instruction-only` | Durable rules and state exist, but no pre-execution host boundary is claimed. |
| `session-start` | A covered integration gates creation or start of a session/child process. |
| `pre-action` | A covered mutation is checked before its callback or action begins. |
| `sandbox/process-enforced` | A real OS-backed boundary constrains the launched process tree for the capability being claimed. |

The portable reference baseline for RepoPact's optional admission plane is `pre-action`. That is a real pre-execution guarantee for covered actions, but it does **not** imply arbitrary-process filesystem confinement.

A higher `sandbox/process-enforced` class requires independent native proof of the operating-system boundary. The Linux Landlock work demonstrates the intended distinction, while remaining platform/reference work is tracked separately. RepoPact never silently upgrades a weaker adapter into a stronger assurance claim.

See [Pre-execution admission](docs/guides/pre-execution-admission.md) and [decision 0060](decisions/0060-optional-sandbox-process-enforced-reference-confinement.md).

## The Workbench

RepoPact is not intended to be a JSON-editing exercise for human operators. The repository also contains a **Tauri 2 Workbench** that presents governed state through a human-facing interface while the Rust semantic engine remains the semantic authority.

The Workbench supports repository selection, inspection, work-item views, typed create/edit/lifecycle operations, validation, graph-backed repository views, plan/preview/apply mutation flow, refresh, and stale-plan/session rejection.

Native bring-up and operator evidence currently exist for **Windows, Linux, and Android**. macOS and iOS remain intended targets but do not yet have equivalent native validation evidence.

Workbench source: [`rust/apps/repopact-desktop/`](rust/apps/repopact-desktop/)

![RepoPact Workbench on Windows showing the validated Work lifecycle view with proposed, active, deferred, and complete counts, a blocked record, active work, and the selected repository session.](docs/assets/readme/workbench-governance.jpg)

*The screenshot is a capture-time view of the implementation. It is not a claim that Workbench installers or Android artifacts are part of the stable PyPI 3.1.1 package contract.*

## Use it with `AGENTS.md`, `CLAUDE.md`, and coding agents

RepoPact is not `AGENTS.md++`, and it does not replace instruction files.

`AGENTS.md`, `CLAUDE.md`, editor rules, skills, and system prompts tell an agent how it should behave. RepoPact records the durable project state around that behavior and validates, and where the relevant boundary exists can enforce, whether the repository still respects its declared contract.

Instruction files can participate in the pact. They are not the whole governance substrate.

That distinction matters when work moves between Claude, Codex, ChatGPT, local models, human developers, or future tools. The next worker should not need the previous worker's private conversation in order to recover the represented engineering state.

## Adopt a brownfield repository without pretending certainty

`repopact adopt` maps existing signals such as nested `AGENTS.md`, CODEOWNERS-style ownership, repository structure, and history into RepoPact records without treating reconstruction as omniscient truth.

This is why RepoPact has provenance types:

- `concrete`: directly established state or evidence;
- `provisional`: usable but not yet fully ratified;
- `inferred`: reconstructed from indirect evidence.

The point is not to manufacture a clean story for an old repository. It is to make uncertainty explicit and allow later evidence to ratchet it toward concrete state.

See [decision 0021](decisions/0021-preflight-mandatory-and-provenance.md) and the paper's brownfield adoption discussion for the formal treatment.

## Local-first verification and release

RepoPact keeps verification semantics in the repository instead of making a hosted provider the source of truth.

The stable 3.1.1 package includes repository-defined verification profiles and local release operations:

```powershell
repopact verify quick
repopact verify ci
repopact verify release

repopact release verify
repopact release build --outdir release-out
repopact release inspect --dist release-out
```

GitHub Actions is an **optional hosted adapter**, disabled by default. Hosted validation requires `REPOPACT_GITHUB_CI=true`; hosted publication requires the independent `REPOPACT_GITHUB_CD=true` switch. A local passing run is local evidence, not proof that a remote branch-protection or admission boundary is closed.

Verification, artifact construction, inspection, and publication remain separate operations. Publication requires explicit operator intent and credentials supplied outside the repository.

This local-first architecture was completed under [WI046](work/completed/046-runner-neutral-verification-and-admission-checkpoint-architecture/). Workbench, mobile applications, active research surfaces, and stronger platform-specific enforcement retain their own validation and packaging boundaries rather than being silently folded into the PyPI artifact contract.

See [`docs/guides/local-ci-cd.md`](docs/guides/local-ci-cd.md).

## Repository Orientation Graph

The optional **Repository Orientation Graph (ROG)** is a bounded, versioned, derived representation used for orientation, impact, dependency, test, and governance queries.

The graph can be durable and incrementally maintained without becoming a second source of truth. Repository and governance records remain authoritative; graph freshness, coverage, and excluded boundaries are explicit. If the graph and authoritative source disagree, rebuild or reconcile the graph.

See [`docs/repository-orientation-graph.md`](docs/repository-orientation-graph.md). WI063 remains the governing work record for unfinished graph evaluation and closeout obligations.

## Research, evidence, and falsifiability

RepoPact is both a working tool and an ongoing software-engineering research project. The research program is deliberately set up so the claims can fail.

The repository includes:

- the current [paper draft](research/paper.md), **RepoPact: Repository-Native Governance for Durable Human-Agent Software Engineering**;
- a [formal model](research/formal-model.md) covering the L0-L5 kernel, governance continuity, invariant classes, and brownfield adoption;
- a pre-registered [experiment protocol](research/protocol.md) and [benchmark protocol](research/benchmark-protocol.md);
- explicit [threats to validity](research/threats-to-validity.md) and a [findings register](research/findings.md);
- a machine-checkable [conformance suite](CONFORMANCE.md);
- the public [RepoPact Proving Ground](https://github.com/ForgeWireLabs/repopact-proving-ground), where runnable PactBench work is exercised against a real adopter.

Comparative cross-model results remain separate from the existence of benchmark infrastructure. They are reported only when the corresponding runs and evidence exist.

> RepoPact defines the pact. The Proving Ground tests whether the pact holds under agent pressure.

## What RepoPact is not

RepoPact is not an AI agent, model provider, agent runtime, distributed compute fabric, chat-memory store, general-purpose issue tracker, hosted CI service, IDE, or general-purpose sandbox.

It does not try to replace Git, compilers, test frameworks, agent orchestrators, operating-system security boundaries, or external authorization systems.

It is the **durable governance layer** those systems can share.

Optional admission and protected-execution integrations can consume RepoPact authority and enforce a covered boundary, but those integrations do not become a second work ledger or a new source of project truth.

## Deeper technical references

- [`docs/README.md`](docs/README.md): audience-oriented documentation map
- [`SPEC.md`](SPEC.md): normative repository model and machine-enforced rules
- [`CONFORMANCE.md`](CONFORMANCE.md): implementation-independent conformance contract
- [`governance/charter.md`](governance/charter.md): principles and non-goals
- [`governance/workflow.md`](governance/workflow.md): repository workflow
- [`docs/repository-orientation-graph.md`](docs/repository-orientation-graph.md): optional derived Repository Orientation Graph
- [`docs/guides/pre-execution-admission.md`](docs/guides/pre-execution-admission.md): optional enforcement/admission integration
- [`decisions/`](decisions/): durable architecture and policy decisions
- [`research/`](research/): formal model, protocols, findings, and paper
- [`work/`](work/): the project's own RepoPact-governed work ledger

The default conformance path exercises the canonical Rust semantic engine. The historical Python validator remains an explicit comparator/compatibility surface rather than a second product authority.

## ForgeWire Labs

RepoPact is the repository-governance layer of [ForgeWire Labs](https://github.com/ForgeWireLabs): inspect the work, bound the authority, preserve the evidence.

It is independently useful, but it also composes with the wider ForgeWire ecosystem where runtime execution, human communication, and broader agent orchestration are separate concerns.

## License

Apache-2.0. See [`LICENSE`](LICENSE) and [decision 0002](decisions/0002-license-apache-2.0.md).
