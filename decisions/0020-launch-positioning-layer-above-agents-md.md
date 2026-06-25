---
id: 0020
title: Launch positioning — RepoPact is the layer above AGENTS.md
status: accepted
date: 2026-06-24
supersedes: []
---

# 0020: Launch positioning — RepoPact is the layer above AGENTS.md

## Context

By mid-2026 `AGENTS.md` has won the agent-context-file standardization: 60k+ public
repositories, governance under the Linux Foundation's Agentic AI Foundation, and native
support across Claude Code, Codex, Cursor, Copilot, Gemini CLI, Windsurf, Aider, Devin,
and Amazon Q, with backing from OpenAI, Anthropic, Google, and AWS. It is deliberately
**schema-free plain Markdown**: instructions for an agent, not an enforced contract.

RepoPact's public positioning must take an explicit stance toward this standard, because
the naive framing ("a better AGENTS.md") is both wrong and strategically fatal — it
picks a fight RepoPact cannot win against a free, foundation-backed incumbent, and it
misdescribes what RepoPact is.

This complements decision [`0019`](0019-repopact-role-in-forgewire-labs-portfolio.md):
RepoPact's job is adoption and trust, and that requires riding the AGENTS.md wave rather
than opposing it.

## Decision

RepoPact **complements, and never competes with, AGENTS.md.** It is positioned as the
*binding / enforcement / evidence layer above it*.

1. **Message.** "AGENTS.md tells the agent what to do. RepoPact proves it didn't quietly
   undo it." AGENTS.md is instructions; RepoPact adds the binding invariant (rationale +
   escalation + machine enforcer), evidence-gated work, and a filesystem state machine.
2. **Mechanical complement.** `adopt` ingests an existing `AGENTS.md` (and CODEOWNERS,
   CI workflows, nested contracts) rather than replacing it. RepoPact is a superset
   layer, not a substitute file.
3. **Launch wedge.** Lead with **"agent-proof your guarantees"** — binding invariants +
   frozen surface stopping an agent from silently weakening what matters — with
   `repopact adopt` as the 30-second first touch. The durable-memory / cross-session
   **recoverability** angle is the secondary beat. The full L0–L5 kernel vision is the
   *paper's* story (`research/`), not the launch headline.
4. **Never strawman AGENTS.md.** Per the project's framing rule, describe RepoPact's
   architecture positively, as the layer it is; do not argue against AGENTS.md or FSM
   models. AGENTS.md is a real, useful, composed-with input.

## Alternatives considered

- **Position as a better/replacement AGENTS.md.** Rejected: unwinnable against a
  foundation-backed incumbent, and false — RepoPact is a different (enforcement) layer.
- **Ignore AGENTS.md entirely.** Rejected: it is the category's center of gravity;
  silence forfeits the most natural adoption on-ramp (60k repos looking for the next
  layer).
- **Lead with the full kernel / agentic-OS vision.** Rejected for *launch*: too abstract
  for first contact and raises adoption cost. Kept as the paper's thesis.

## Consequences

- README, docs, and launch copy frame RepoPact as the enforcement layer over AGENTS.md,
  and `adopt` keeps consuming AGENTS.md as a first-class input.
- The launch playbook (the "AGENTS.md is not a contract" essay, the Show HN, the X
  thread) is built on this complement framing.
- The wedge ("agent-proof your guarantees") drives the README hero and the demo asset;
  recoverability is the supporting story.
