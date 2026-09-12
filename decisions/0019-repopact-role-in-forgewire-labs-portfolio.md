---
id: 0019
title: RepoPact's role in the ForgeWire Labs portfolio
status: accepted
date: 2026-06-24
supersedes: []
---

# 0019: RepoPact's role in the ForgeWire Labs portfolio

> **Amendment — 2026-09-11:** ForgeWire Fabric has moved from the public/open
> portfolio to private proprietary development. Any older statement in this decision
> describing Fabric as Apache-2.0, public alpha, or open-core is superseded by this
> amendment. Historical Fabric versions or copies previously distributed under Apache
> License 2.0 remain under the terms that accompanied those versions. RepoPact's own
> Apache-2.0 license and open standard-defining role are unchanged.

## Context

RepoPact is not a standalone product. It is one pillar of the **ForgeWire Labs**
portfolio of inspectable agentic infrastructure, whose thesis is *agentic systems are
systems first and models second* and whose premise is *inspect the work, bound the
authority, preserve the evidence*. The current portfolio posture is:

```text
PUBLIC / OPEN
  RepoPact   → work governance         (this repo; Apache-2.0)
  ForgeLink  → communication governance (ForgeLink; human approval/decision boundary)

PRIVATE DEVELOPMENT
  Fabric     → execution governance    (proprietary; alpha)
  ForgeWire  → integrated agentic environment (proprietary)

PRODUCT / ADJACENT
  SkillForge → standalone learning product
```

Earlier revisions of this decision recorded Fabric as an Apache-2.0 public alpha and
open-core downstream product. That accurately described the portfolio at the time but
is no longer the current licensing or release posture.

Because the portfolio has a private/commercial dimension, RepoPact's own commercial
posture and license must be recorded, so the project's direction is not re-litigated
each session and so contributors and adopters understand what RepoPact will and will
not become.

The portfolio-level monetization, funding, and launch-sequencing strategy lives **above
this repo** (ForgeWire-Overview). This decision records only RepoPact's role within it.

## Decision

RepoPact is the **open, standard-defining, top-of-funnel pillar**. Concretely:

1. **License stays permissive (Apache-2.0).** RepoPact is meant to be adopted by anyone
   — including competitors and other tool vendors — without friction. No relicense to
   source-available/BSL, no open-core gating *of RepoPact itself*.
2. **RepoPact is not directly monetized.** Its job is adoption, developer trust, and
   academic legitimacy (the `research/` paper). Portfolio value can be captured
   downstream in private ForgeWire Labs systems such as **Fabric** and **ForgeWire**;
   that downstream posture does not change RepoPact's license.
3. **RepoPact is the funnel entry.** The intended conceptual path remains
   `RepoPact (govern the work) → Fabric (govern the execution) → ForgeLink (govern the
   human decision) → ForgeWire (integrated environment)`. RepoPact records and docs may
   reference the adjacent pillars, but RepoPact must remain **independently useful** to
   an adopter who never touches the rest of the stack. The conceptual path does not
   imply that every adjacent pillar is public or open source.
4. **Conformance becomes load-bearing.** Calling RepoPact a *standard* obliges a
   published, versioned conformance suite (work item `019`) so third-party
   implementations can claim conformance. This is a direct consequence of choosing the
   standard posture over the product posture.

## Alternatives considered

- **Monetize RepoPact directly** (paid tiers / hosted dashboard as the primary revenue).
  Rejected: it would throttle the adoption RepoPact exists to create, and it would put
  RepoPact in a losing fight against the free, foundation-backed `AGENTS.md` standard
  (see decision [`0020`](0020-launch-positioning-layer-above-agents-md.md)).
- **Relicense RepoPact to source-available / BSL.** Rejected: incompatible with
  "adoptable standard"; a standard nobody can freely implement is not a standard.
  This rejection applies to RepoPact and does not constrain the licensing of adjacent
  ForgeWire Labs systems.
- **Fold RepoPact into Fabric as one product.** Rejected: they are different layers
  (durable work memory vs. remote execution control). Each is independently useful;
  collapsing them raises the first-touch cost of RepoPact and couples an open governance
  standard to a separately licensed execution system.

## Consequences

- RepoPact's design decisions optimize for **adoptability and conformance**, not revenue
  capture. When the two conflict, adoptability wins.
- A versioned conformance suite (`019`) and the comparative benchmarks (`020`,
  `research/benchmark-protocol.md`) matter because credibility, not licensing, is
  RepoPact's moat.
- Cross-pillar references (to Fabric/ForgeLink) are allowed and encouraged where useful,
  but docs must accurately identify Fabric as private/proprietary rather than imply that
  the entire adjacent stack is open. The validator, the spec, and the quickstart must
  never *require* the rest of the stack.
- The portfolio's funding and launch strategy is owned at the ForgeWire-Overview level;
  this repo defers to it and does not duplicate it.
