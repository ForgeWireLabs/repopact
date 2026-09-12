# 034 — Independent reproduction and third-party adoption evidence

> **Status**: Active
> **Owners**: governance-owner (lead); docs-owner and evidence-owner affected.
> **Depends on**: `010`.

## Intent

Reduce the paper's reflexivity and single-operator threats with reproducible public
evidence. All current public adopters are ForgeWireLabs repositories; the Flask datum is
independent in source but currently exists only as an author-run local capture.

The September 2026 competitive-state review sharpened the closeout standard: external
adoption alone is not enough to support a category-leading claim. RepoPact also needs
independent attempts to break its load-bearing guarantees, adoption in repositories whose
instruction/governance stack was not designed around ForgeWire, and comparative evidence
against an appropriate instruction-only baseline.

## Decisions

Publishing a reproduction recipe is agent-actionable. Claiming third-party reproduction
is not: completion requires evidence produced by an external person or organization, and
negative or partial results count.

Independent review is evidence, not endorsement. A hostile review that discovers a real
bypass, usability failure, or false positive is more useful than a friendly reproduction
whose only result is `validate` returning zero.

## Scope

- A pinned, reproducible Flask adoption path.
- Public external reproduction/adoption evidence with explicit provenance.
- Independent adversarial review of at least one load-bearing guarantee.
- Adoption against a non-ForgeWire-native repository/instruction stack.
- Comparative PactBench/benchmark evidence against an appropriate baseline.
- A public, private-state-free reproduction package.
- Findings and paper-threat reconciliation.

## Acceptance criteria

- [ ] **AC-1** — publish the Flask adoption as a reproducible public artifact.
- [ ] **AC-2** — obtain one independently executed public result.
- [ ] **AC-3** — preserve friction and disconfirming outcomes.
- [ ] **AC-4** — update T1/T2 without overstating mitigation.
- [ ] **AC-5** — preserve one independent adversarial attempt to break a load-bearing RepoPact guarantee, including negative findings and remediation decisions.
- [ ] **AC-6** — prove adoption against at least one repository whose existing agent/developer governance was not designed around ForgeWire conventions.
- [ ] **AC-7** — publish a protocol-faithful comparative PactBench/benchmark result against an appropriate instruction-only baseline, including null or negative results.
- [ ] **AC-8** — prove a third party can reproduce the selected adoption/benchmark path from a pinned public release without ForgeWireLabs private state and record resulting product/documentation friction.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
