# 021 — RepoPact's slice of the ForgeWire Labs public launch

> **Status**: 🟢 Active  
> **Owners**: governance-owner (lead); docs + tooling support.  
> **Depends on**: [`019`](../019-conformance-suite/) (the public conformance claim needs the conformance suite green).

## Needs you (operator gates)

This work item cannot be completed by an agent alone. The human/operator actions, in order:

1. **arXiv** (AC-2) — create/confirm an arXiv account; cs.SE may require endorsement for a first submission; approve the final paper text; submit. Drafting can be agent-assisted, but the account and submission are operator actions.
2. **PyPI** (AC-3) — cut the operator-approved launch release after the current development state is ready.
3. **Show HN + socials** (AC-4) — the operator owns the accounts, final copy approval, and posting. No automated posting or vote solicitation.

These gates remain pending until the real action is performed and durable evidence records it.

## Intent

RepoPact is one pillar of the ForgeWire Labs public surface. This work item scopes only RepoPact's launch-facing material: the landing README, paper, package release, conformance claim, and RepoPact-specific launch copy.

The positioning was sharpened on 2026-09-13 after the first LinkedIn traffic was directed at the repository. The README now leads with the problem a new reader needs to understand first:

> AI-assisted implementation is getting faster while the durable engineering state around that work is still fragmented across chats, agent memory, tools, and people.

RepoPact's answer is governance continuity: the repository carries the load-bearing intent, authority, work state, decisions, invariants, provenance, evidence, and known violations needed for another legitimate human or agent to continue responsibly.

The earlier `AGENTS.md` wedge from decision `0020` remains useful, but it is now a subordinate differentiator rather than the first sentence a new user sees. `AGENTS.md` tells an agent how it should behave; RepoPact provides durable governed project state and validation/enforcement where those guarantees are mechanically enforceable.

## Decisions

Driven by decisions [`0019`](../../../decisions/0019-repopact-role-in-forgewire-labs-portfolio.md) and [`0020`](../../../decisions/0020-launch-positioning-layer-above-agents-md.md), plus the governance-continuity framing developed in [`research/paper.md`](../../../research/paper.md).

The 2026-09-13 README overhaul is an operator-directed refinement of launch positioning, not a change to RepoPact's underlying governance model.

## Scope

- Public: root README, paper draft and eventual preprint, PyPI launch release, conformance link, screenshots/visuals, and public launch copy.
- Private launch planning may contain channel-specific drafts and portfolio strategy, but those are not RepoPact governance records.
- Out of scope: runtime orchestration in ForgeWire/Fabric, ForgeLink-specific communication work, and unrelated portfolio launch mechanics.

## README overhaul — 2026-09-13

The root README was rewritten as a landing page rather than an internal architecture note. The new order is intentional:

1. explain the engineering problem and governance discontinuity;
2. explain RepoPact's repository-native answer;
3. show the common failure modes it mitigates;
4. provide a stable-release quick start;
5. explain the pact and Workbench;
6. show agent/tool interoperability and brownfield adoption;
7. separate current-main local-first CI/CD work from stable 3.0.2 behavior;
8. surface the paper, formal model, conformance suite, and Proving Ground;
9. state clearly what RepoPact is not.

The README now distinguishes the stable PyPI release `3.1.0` from the
post-release `main` development identity (`3.1.0-dev.1`) so active work is not
represented as already shipped in the stable package.

Two invisible visual slots were left in the README for a follow-up pass: one hero visual and one real Workbench screenshot. Those should use actual RepoPact product/repository evidence, not generic AI art. The existing `docs/assets/repopact-governance.svg` is stale (`spec 1.0.0`) and the existing flow SVG omits the later `proposed` lifecycle state, so neither should be promoted as the new hero without correction.

## Acceptance criteria

- **AC-1** — positioning-aligned README, visuals, essay/social copy, and operator approval. Pending until the visual pass and remaining launch assets are reviewed.
- **AC-2** — paper on arXiv (cs.SE). *Operator-gated.*
- **AC-3** — PyPI launch release with positioning-aligned README and conformance evidence. *Operator-gated.*
- **AC-4** — Show HN posted and launch day handled. *Operator-gated.*

## Reconciliation — 2026-07-26

- [ ] **AC-1** — pending: no durable proof of operator approval for the complete launch asset set existed at that time.
- [ ] **AC-2** — pending: no arXiv submission record or public paper URL exists.
- [ ] **AC-3** — pending: earlier package releases were infrastructure releases, not the operator-approved public launch event.
- [ ] **AC-4** — pending: no Show HN post or launch-day response record exists.

Evidence: [`20260726-semantic-ledger-freshness-reconciliation`](../../../evidence/runs/20260726-semantic-ledger-freshness-reconciliation.json).
