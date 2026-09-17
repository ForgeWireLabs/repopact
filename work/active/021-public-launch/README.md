# 021 — RepoPact's slice of the ForgeWire Labs public launch

> **Status**: 🟢 Active  
> **Owners**: governance-owner (lead); docs + tooling support.  
> **Depends on**: [`019`](../019-conformance-suite/) (the public conformance claim needs the conformance suite green).

## Needs you (operator gates)

This work item cannot be completed by an agent alone. The remaining human/operator actions are:

1. **arXiv** (AC-2) — create/confirm an arXiv account; cs.SE may require endorsement for a first submission; approve the final paper text; submit. Drafting can be agent-assisted, but the account and submission are operator actions.
2. **PyPI / launch reconciliation** (AC-3) — RepoPact 3.1.1 has been released as the current stable package line, but AC-3 remains machine-pending until the launch criterion is reconciled against durable publication/conformance evidence in `work-item.json` rather than being closed from narrative alone.
3. **Show HN + socials** (AC-4) — the operator owns the accounts, final copy approval, and posting. No automated posting or vote solicitation.

These gates remain pending until the corresponding durable evidence and lifecycle records say otherwise.

## Intent

RepoPact is one pillar of the ForgeWire Labs public surface. This work item scopes RepoPact's launch-facing material: the landing README, documentation entry points, paper, package release, conformance claim, screenshots/visuals, and RepoPact-specific launch copy.

The positioning was sharpened on 2026-09-13 after the first LinkedIn traffic was directed at the repository. The README leads with the problem a new reader needs to understand first:

> AI-assisted implementation is getting faster while the durable engineering state around that work is still fragmented across chats, agent memory, tools, and people.

RepoPact's answer is governance continuity: the repository carries the load-bearing intent, authority, work state, decisions, invariants, provenance, evidence, and known violations needed for another legitimate human or agent to continue responsibly.

The earlier `AGENTS.md` wedge from decision `0020` remains useful, but it is a subordinate differentiator rather than the first sentence a new user sees. Instruction files tell an agent how it should behave; RepoPact provides durable governed project state plus validation and progressively stronger enforcement where the corresponding boundary actually exists.

## Decisions

Driven by decisions [`0019`](../../../decisions/0019-repopact-role-in-forgewire-labs-portfolio.md) and [`0020`](../../../decisions/0020-launch-positioning-layer-above-agents-md.md), plus the governance-continuity framing developed in [`research/paper.md`](../../../research/paper.md).

The launch/documentation overhauls are operator-directed refinements of public explanation, not changes to RepoPact's underlying governance semantics.

## Scope

- Public: root README, documentation map/tutorial/explanation, paper draft and eventual preprint, PyPI launch release, conformance link, screenshots/visuals, and public launch copy.
- Private launch planning may contain channel-specific drafts and portfolio strategy, but those are not RepoPact governance records.
- Out of scope: runtime orchestration in ForgeWire/Fabric, ForgeLink-specific communication work, and unrelated portfolio launch mechanics.

## README overhaul — 2026-09-13

The root README was rewritten as a landing page rather than an internal architecture note. The order was intentionally shifted toward the engineering problem, governance continuity, the repository-native answer, stable quick start, Workbench, interoperability, brownfield adoption, verification, research, and explicit non-goals.

That pass established the current public framing but later implementation/release work caused several documentation surfaces to drift again.

## Documentation architecture reconciliation — 2026-09-17

After RepoPact 3.1.1 became the current stable line, the launch-facing documentation was reconciled again so a new reader does not encounter several generations of the architecture at once.

The pass intentionally uses WI021 rather than WI047. WI047 remains proposed future product work for enforcing documentation-impact closure on governed code changes; it is not implementation authority for this documentation rewrite.

Changes in this tranche:

1. **Root README** — now leads with "repository-native governance for durable human-agent software engineering," links a real documentation map, distinguishes authoritative source records from derived views, states what RepoPact does and does not own, and documents the assurance ladder instead of using an ambiguous enforcement boolean.
2. **`docs/README.md`** — added as the audience-oriented entry point with paths for using, understanding, integrating, and developing RepoPact.
3. **`docs/concepts.md`** — expanded into the conceptual reference for governance discontinuity, authority, lifecycle, provenance, evidence-vs-authority, `AGENTS.md` boundaries, canonical Rust semantics, ROG derivation, and assurance classes.
4. **`docs/adopt-repopact.md`** — updated from the obsolete pre-3.0 clone/`requirements.txt` flow to the stable package workflow: install from PyPI, `repopact init`, mandatory preflight, repository-defined verification, evidence recording, evidence linkage, lifecycle closeout, dashboard, and validation.
5. **`ROADMAP.md`** — reconciled through v3.1.1. The canonical Rust engine is no longer described as future work; WI050 is represented as deferred; current active/proposed boundaries are explicit.
6. **Launch claim guardrails and public copy** — updated from stale 3.0.2 wording to stable 3.1.1 and now distinguish stable package capability, implementation on `main`, reference guarantees, integration-dependent guarantees, work lifecycle, research infrastructure, and empirical results.
7. **Documentation audit registry** — reviewed and refreshed for the new navigation, authority model, assurance ladder, tutorial, ROG limits, and launch language.

### Enforcement wording fixed by this pass

The public docs now distinguish:

- `instruction-only` — governed state, no claimed pre-execution host boundary;
- `session-start` — covered session/child creation is gated;
- `pre-action` — a covered mutation is checked before its action/callback starts;
- `sandbox/process-enforced` — an OS-backed boundary constrains the launched process tree for the capability actually proven.

`pre-action` remains the portable optional admission baseline and is not described as arbitrary-process filesystem confinement. Stronger Linux Landlock work is represented as a higher, evidence-dependent class rather than silently upgrading the portable baseline.

### Authority wording fixed by this pass

The dashboard, ROG, Workbench views, generated SPEC blocks, and indexes are documented as derived projections. `governance/`, `work/`, `decisions/`, `evidence/`, source, and explicitly governed configuration remain authoritative for the facts they own.

A better derived view does not become a second source of truth.

## Acceptance criteria

- **AC-1** — positioning-aligned README, visuals, documentation/public copy, and operator approval. The documentation architecture portion is materially advanced by the 2026-09-17 reconciliation; AC-1 remains pending until the complete launch asset set is operator-approved and evidence-reconciled.
- **AC-2** — paper on arXiv (cs.SE). *Operator-gated.*
- **AC-3** — PyPI launch release with positioning-aligned README and conformance evidence. RepoPact 3.1.1 publication is now part of the real release state, but the criterion remains pending until `work-item.json` is closed from durable evidence rather than this narrative.
- **AC-4** — Show HN posted and launch day handled. *Operator-gated.*

## Reconciliation — 2026-07-26

- [ ] **AC-1** — pending: no durable proof of operator approval for the complete launch asset set existed at that time.
- [ ] **AC-2** — pending: no arXiv submission record or public paper URL exists.
- [ ] **AC-3** — pending at that time: earlier package releases were infrastructure releases, not the operator-approved public launch event.
- [ ] **AC-4** — pending: no Show HN post or launch-day response record exists.

Evidence: [`20260726-semantic-ledger-freshness-reconciliation`](../../../evidence/runs/20260726-semantic-ledger-freshness-reconciliation.json).
