# 021 — RepoPact's slice of the ForgeWire Labs public launch

> **Status**: 🟢 Active
> **Owners**: governance-owner (lead); docs + tooling support.
> **Depends on**: [`019`](../019-conformance-suite/) (the "standard" claim needs the
> conformance suite green).

## Needs you (operator gates)

This work item cannot be completed by an agent alone. The human/operator actions, in
order:

1. **arXiv** (AC-2) — create/confirm an arXiv account; cs.SE may require **endorsement**
   for a first submission; approve the final paper text; submit. Drafting is mine; the
   account and submission are yours.
2. **PyPI** (AC-3) — cut the launch release (trusted publishing is already set up, see
   decision `0009`).
3. **Show HN + socials** (AC-4) — your accounts, ~2–3 weeks of HN karma warm-up, your
   approval of copy, and **you** post. No automated posting, no upvote solicitation.

I never mark these satisfied on your behalf; they stay `pending` until the real action is
done and an evidence run records it.

## Intent

RepoPact is one pillar of a **unified ForgeWire Labs launch** (RepoPact + ForgeLink lead;
Fabric rides along as public alpha; ForgeWire/SkillForge are teased as the larger
system). This work item scopes **only RepoPact's part** of that launch — the paper, the
package, the conformance claim, and the RepoPact-specific launch copy. The portfolio-level
strategy, funding, and cross-pillar sequencing live above this repo (ForgeWire-Overview)
and in the private `launch/` working area; they are deliberately **not** RepoPact records
(decision `0019`).

The launch leads with the wedge from decision `0020`: **agent-proof your guarantees**,
with durable-memory/recoverability as the secondary beat.

## Decisions

Driven by decisions
[`0019`](../../../decisions/0019-repopact-role-in-forgewire-labs-portfolio.md) (RepoPact's
funnel-top role) and
[`0020`](../../../decisions/0020-launch-positioning-layer-above-agents-md.md) (positioning
above AGENTS.md).

## Scope

- Public: the README hero, the expanded paper draft in `research/`, the PyPI release, and
  the conformance link.
- Private (`launch/`, gitignored): the Show HN / X / Reddit drafts, the essay, the
  competitive battle card, funding targets, and the portfolio strategy.
- Out of scope: the ForgeLink↔Fabric approval bridge (a ForgeLink work item — the "hero
  demo" of the unified launch — owned in that repo) and anything Fabric/SkillForge.

## Acceptance criteria

- **AC-1** — RepoPact launch assets drafted + operator-approved (private `launch/`).
- **AC-2** — paper on arXiv (cs.SE). *Operator-gated.*
- **AC-3** — PyPI launch release with positioning-aligned README; `019` green and linked.
  *Operator-gated.*
- **AC-4** — Show HN posted and launch day handled. *Operator-gated.*

(State tracked in [`work-item.json`](work-item.json); `pending` until evidence-backed.)
