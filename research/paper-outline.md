# Working outline: RepoPact: Repository-Native Governance for Durable Human-Agent Software Engineering

**Subtitle:** Portable intent, authority, evidence, and conformance across agents, humans,
machines, and sessions

This outline is a scaffold for the archival paper. Claims must be filled from the
versioned implementation, formal model, findings register, conformance corpus, case
studies, and pre-registered benchmark protocols rather than from assertion.

## Thesis

The central problem in long-running human-agent software work is not only context loss. It
is **governance discontinuity**: source code survives a worker or session boundary while
the intent, authority, invariants, decisions, provenance, evidence, and lifecycle needed
to change that code responsibly may not.

RepoPact makes the version-controlled repository the durable rendezvous point between
humans and agents. A fresh legitimate worker should be able to clone the repository and
recover both its represented governed state and its current known violations without
requiring predecessor-private context.

This property is called **governance continuity**.

## 1. Introduction

- Session amnesia as a symptom; governance discontinuity as the deeper problem.
- The repository as the artifact every legitimate software worker already has to obtain.
- Repository as rendezvous point: UI, agent, CLI, automation, and clean clone should not
  own separate truths.
- Six contributions: repository-native governance model, governance continuity, binding
  invariants, typed enforcement lattice, provenance-aware brownfield adoption, and a
  falsification-oriented evaluation program.

## 2. Background and related work

- **Agent context files.** `AGENTS.md`, `CLAUDE.md`, Cursor/editor rules. Useful
  instructions, but not a complete typed evidence and authority system.
- **Agent memory.** MemGPT/Letta and related persistent-memory approaches solve context
  extension beside the agent process; RepoPact focuses on load-bearing governance inside
  the artifact of record.
- **Decision records.** ADRs preserve rationale and history but do not normally bind work
  completion to evidence or unify authority and conformance.
- **Policy as code.** OPA/Conftest and CI controls provide powerful enforcement mechanisms;
  RepoPact differs in unit and substrate, and composes with them.
- **Architecture fitness functions.** Related in the use of executable constraints, but
  RepoPact attaches those constraints to durable work, authority, evidence, and provenance.
- **Developer catalogs.** Backstage provides discoverability and relationship metadata at
  organizational scale; RepoPact is repository-local and versioned with the governed work.
- **Software-agent benchmarks.** SWE-bench and SWE-EVO motivate evaluation on real and
  long-horizon engineering rather than toy generation tasks.
- Runtime governance and sandboxing remain complementary rather than replaced.

## 3. Model

- Six kernel layers L0 through L5.
- State tuple and provenance types.
- Work lifecycle as authority, not merely progress.
- Binding invariant as first-class guarantee.
- Typed enforcement lattice.
- Derive-over-declare.
- Brownfield adoption boundary and the **concrete-record** trilemma.
- Important qualification: provenance resolves epistemic reconstruction without
  fabricating certainty; it does not make arbitrary structural contradictions conformant.
- **Governance continuity:**

```text
G(s) = represented governance projection
recover(clean_clone(s)) = <G(s), Viol(s)>
```

for supported semantics and represented repository-authoritative state.

## 4. Reference implementation

- Current public release line and canonical Rust semantic engine for proven surfaces.
- Python compatibility tooling where retained.
- Tauri 2 Workbench as the human operator surface over the same Rust authority.
- Human operator control parity as part of inspectability.
- Cross-platform evidence stated conservatively: Windows, Linux, Android evidenced;
  macOS/iOS remain targets until native validation exists.
- Conformance is behavioral, not implementation-language identity.

## 5. Evaluation method

### Reflexive/adversarial

- H1-H7 and their falsification criteria.
- Packaged product rather than source-checkout-only testing.
- Findings fed back through RepoPact itself.

### Comparative/pre-registered

- H8-H13 / S1-S6: guarantee preservation, recovery, coordination, token economy, drift,
  defensive security.
- H14 / S7: enforcement closure, registered 2026-08-21.
- H15 / S8: governance continuity and clean-clone orientation, registered 2026-09-12
  before any S8 run.

The comparative evaluation (`benchmark-protocol.md`, H8–H15) now spans the complete
registered suite S1–S8.

## 6. Results to date

- Findings register with held/cracked outcomes.
- Design-changing defects: surface closure, working-tree protection, ignored governance
  records, hollow ledger, longitudinal drift.
- Brownfield evidence and provenance shift.
- Proposed lifecycle state as authority typing exposed by adoption pressure.
- Enforcement-closure naturalistic case reported as motivating evidence, not confirmation.
- Comparative model results remain pending.

## 7. Discussion

- Repository as common governed substrate across humans and agents.
- Governance continuity versus ordinary memory persistence.
- Human operator control as part of inspectability.
- Ceremony versus recoverability.
- Explicit enforcement boundaries instead of fictional total automation.
- Provenance and authority as type problems.
- L5 remains a real limit.
- Runtime controls and repository governance compose.
- A durable repository-orientation graph, if adopted, belongs here as a future derived
  mechanism only after its architecture and evaluation condition are registered. It must
  not be described as current functionality before implementation evidence exists.

## 8. Threats to validity

- Reflexivity and author-as-operator bias.
- Limited independent adoption.
- Scale and domain limits.
- Holds are not proofs.
- Benchmark curation and baseline fairness.
- Token/cost measurement sensitivity.
- Security-task realism.
- Provenance misuse.
- Standard/implementation coupling.
- Workbench maturity and operator-parity overstatement.

## 9. Conclusion and future work

- Governance continuity as the central framing.
- Real cross-model benchmark execution.
- Independent reproduction and adoption.
- Workbench operator parity and platform validation.
- External ingestion with preserved provenance.
- Stronger temporal/relational enforcement.
- Repository orientation and graph-derived context as a prospective extension, evaluated
  against the already-registered S8 orientation-cost measures rather than assumed useful.

## Planned appendices

- Typed invariant lattice.
- Governance continuity sketch.
- Formal theorem and proof-obligation summary.
- Benchmark program H8-H15 / S1-S8.
- Planned figures and result tables.
- Complete bibliographic references for related work.
