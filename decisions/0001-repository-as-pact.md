---
id: 0001
title: Repository as Pact
status: accepted
date: 2026-06-15
supersedes: []
---

# 0001: Repository as Pact

## Context

Agent and human work increasingly originates in conversations whose state is lost
when the session ends. Conventions such as `AGENTS.md`, ADRs, and issue trackers
each capture a fragment, but none of them makes the *binding* part — the
guarantees that must not be silently weakened — explicit and enforceable.

## Decision

RepoPact's differentiating primitive is the **binding invariant**: a declared
guarantee that an agent cannot weaken without flagging it and obtaining explicit
human operator approval. Invariants live in `governance/invariants.json`, are
distinct from principles (human judgment), and carry an escalation path. Where an
invariant is machine-checkable it names its enforcer; where it is not, escalation
is the enforcement.

This is what makes the repository a *pact* rather than a folder convention.

## Alternatives considered

- **Principles only (status quo).** A charter of values with no binding enforcement. Rejected:
  values do not stop a confident agent from regressing a guarantee.
- **Enforce everything in code.** Rejected: many invariants (e.g. "do not launder
  history") are not statically checkable; pretending otherwise produces false
  confidence. The honest model pairs machine checks with named escalation.

## Consequences

- The charter splits into principles and invariants.
- A frozen surface and an escalation gate become first-class (`INV-6`).
- Adopters can read the pact in one file and see exactly what is non-negotiable.
