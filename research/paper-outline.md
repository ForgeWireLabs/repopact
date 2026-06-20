# Working outline — "The Repository as the Operating System for Agentic Work"

A paper draft assembled from this proving ground. The outline is a scaffold; the
evidence that fills it comes from [`findings.md`](findings.md) and the
proving-ground evidence runs, not from assertion.

## Thesis

Durable collaboration between humans and coding agents fails not for lack of
intelligence but for lack of a *substrate*: state that survives the end of a
conversation. We argue the version-controlled repository — not a chat log, ticket
tracker, or external memory store — is the right substrate, and that a thin layer
of machine-checked records (binding invariants, scoped authority, evidence-gated
work items, a filesystem state machine, drift audits) turns a folder convention
into an operating system for agentic work. We evaluate the claim adversarially.

## 1. Introduction

- The session-amnesia problem: agents lose load-bearing state at conversation boundaries.
- Existing fragments (`AGENTS.md`, ADRs, issue trackers) each capture a slice; none
  makes the *binding* part machine-checkable.
- Contribution: a repository-native model + a reflexive, adversarial evaluation of it.

## 2. Background and related work

- Memory/state approaches for agents (external stores, RAG, scratchpads) and why
  they are not recoverable by a third party from the repository alone.
- ADRs, conventional commits, policy-as-code, CI gates — prior art this composes.
- Where RepoPact's binding-invariant primitive differs from documentation.

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

## Appendices

- A: full findings register. B: representative evidence runs. C: the SPEC.
