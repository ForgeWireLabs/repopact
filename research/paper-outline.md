# Working outline — "The Repository as the Operating System for Agentic Work"

This draft is assembled from the repopact proving ground. The outline is a scaffold; the
evidence that fills it comes from [`findings.md`](findings.md) and the
proving-ground evidence runs, not from assertion.

## Thesis

Durable collaboration between humans and coding agents fails not for lack of
intelligence but for lack of a *substrate*: state that survives the end of a
conversation. We argue the version-controlled repository — not a chat log, ticket
tracker, or external memory store — is the right substrate, and that a thin layer
of machine-checked records (binding invariants, scoped authority, evidence-gated
work items, a filesystem state machine, drift audits) turns a folder convention
into a repository-native contract and evidence layer for agentic work. We evaluate
the claim adversarially.

## 1. Introduction

- The session-amnesia problem: agents lose load-bearing state at conversation boundaries.
- Existing fragments (`AGENTS.md`, ADRs, issue trackers) each capture a slice; none
  makes the *binding* part machine-checkable.
- Contribution: a repository-native model + a reflexive, adversarial evaluation of it.

## 2. Background and related work

- **Agent context files.** `AGENTS.md` (Linux Foundation Agentic AI Foundation;
  60k+ repos by mid-2026), `CLAUDE.md`, Cursor rules. Schema-free instructions, not
  enforced contracts: none makes the *binding* part machine-checkable. RepoPact is the
  enforcement layer above them and `adopt` ingests them.
- **Agent memory/state.** External stores, RAG, scratchpads, and runtime memory
  frameworks (Letta/MemGPT, Mem0, Zep, LangMem) — memory beside the agent process, not
  recoverable by a third party from the repository alone.
- **Agent governance architectures.** Recent layered/security frameworks govern at
  *runtime*: e.g. the Layered Governance Architecture (arXiv:2603.07191 — sandboxing,
  LLM-judge intent checks, zero-trust inter-agent auth, immutable audit) and six-layer
  agentic-SDLC reference architectures (arXiv:2604.26275). These are runtime controls or
  reference taxonomies; none is repository-native, and none makes the binding guarantee
  an evidence-gated, git-versioned primitive. RepoPact differs on substrate (the repo)
  and on the unit (the binding invariant), not merely on layering.
- **Prior art this composes.** ADRs, conventional commits, policy-as-code (OPA/Conftest),
  CI gates, architecture fitness functions, and developer-portal scorecards
  (Backstage/Cortex/OpsLevel) — each enforces one slice at service or CI scale; RepoPact
  unifies enforcer + rationale + escalation in the repository itself.


## 3. The model

- Primitives: charter & invariants; frozen surface; scopes & roles; work items;
  evidence; decisions & policies; reconciliation/audits. (Map to RepoPact SPEC §3–§7.)
- The core loop: intent → scoped authority → work item → implementation → evidence
  → audit → history.
- Status as a filesystem transition; derive-over-declare.

## 4. Reference implementation

- The validator as reference semantics; schemas as structure; the two-layer split.
- Packaging and adoption surface (`repopact init/new/validate/...`).

## 5. Evaluation method (reflexive, adversarial)

- Hypotheses H1–H6 and falsification criteria (from `protocol.md`).
- The proving ground: a throwaway real CLI adopting the *packaged* product.
- RepoPact used to govern its own evaluation — a test of recoverability (H6).

## 6. Results

- Per-hypothesis: held / cracked, with the citing capture.
- Defect taxonomy from `findings.md` (e.g. F-001: command-set closure).
- What the architecture caught that a naive convention would not.

## 7. Discussion

- Cost/benefit of ceremony vs. recoverability.
- Limits: trivial project size, single evaluator, no longitudinal multi-agent study.
- Threats to validity, including author-as-evaluator bias and the reflexivity risk.

## 8. Conclusion and future work

- When repository-native governance is worth it; the road to multi-adopter evidence.
- **Comparative value.** Beyond the reflexive falsification of §5–§6, a controlled
  comparative evaluation (`benchmark-protocol.md`, H8–H10): guarantee-violation
  detection (PactBench), cross-session recovery + efficiency on SWE-bench Verified /
  SWE-EVO, and multi-agent coordination. This is the path from "the architecture catches
  what it claims" to "governing with it measurably changes agent behaviour."
- Provenance-typed records (`inferred`/`provisional`/`concrete`) and external ingestion
  as the route past the L5 adoption boundary.

## Appendices

- A: full findings register. B: representative evidence runs. C: the SPEC.
