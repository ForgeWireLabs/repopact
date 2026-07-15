# Roadmap

A human-curated forward view. The authoritative status of in-flight work is the
derived [dashboard](audits/reports/dashboard.md) and the `work/` ledger; this file
adds editorial intent (what we mean to do next and why) that is not derivable.

## Now — shipped in v2.0.x

RepoPact 2.0 changed the record language itself; the 2.0.x line hardened it:

- **Mandatory preflight** (decision `0021`, v2.0.0; generalizing the opt-in
  marker of decision `0018`): a work item must exist before implementation
  begins. Existing adopters are grandfathered through a preflight epoch and a
  `doctor` migration path.
- **Provenance-typed records** (decision `0021`): every record carries
  `concrete | provisional | inferred` provenance. This resolves the brownfield
  adoption trilemma — `adopt` now emits provisional work items backed by inferred
  evidence, so migration is total, faithful, *and* closed, while completion stays
  gated on concrete evidence. `doctor` ratchets `provisional → concrete` when the
  proof exists.
- **Published conformance suite** (`CONFORMANCE.md`, `conformance/`, work item
  `019`): a versioned fixture corpus plus runner so third-party implementations
  can test acceptance/rejection against the standard, not against our Python.
- **PactBench relocated to RepoPact Proving Ground** (v2.0.1):
  RepoPact keeps the benchmark *protocol*; the runnable suite lives in
  [`repopact-proving-ground`](https://github.com/ForgeWireLabs/repopact-proving-ground)
  and consumes RepoPact from PyPI.
- **`proposed` lifecycle state** (decision `0023`; on `main`, unreleased —
  ships in the next release): candidate work can be captured durably without
  granting implementation authority; `active`/`completed` items may not depend
  on `proposed` ones. Driven by a real downstream adopter.

## Next — the active ledger

- **`020` PactBench ↔ Proving Ground integration**: finish the repo split so the
  benchmark suite runs entirely against the packaged product.
- **`021` Public launch**: arXiv preprint (cs.SE), launch release on PyPI, Show HN.
  Operator-gated — account, endorsement, and posting are human steps.
- **`022` Comparative benchmark suite (S2–S6 / H9–H13)**: one matched-arm harness
  for recovery, coordination, token economy, drift, and security studies.
  Real cross-model results are gated on compute/model access.

## Later

- **Alternative validator** in a compiled language, proving the SPEC is
  implementation-independent (the conformance suite is its runnable target).
- **GitHub Action** that runs the gates as a reusable workflow.
- **External ingestion at L5**: tracker exports and design documents as
  first-class, evidence-bearing records with provenance.
- **Mechanized temporal/relational invariants**: trace semantics for INV-4 over
  git history; a refinement order for nested contracts (INV-5).

## Earlier

- **v1.x**: 1.0 declared on proving-ground evidence (decision `0007`); brownfield
  `adopt` (`0008`); PyPI trusted publishing (`0009`); `import-plan` (`0010`);
  `doctor` upgrade/repair (`0011`); `takeover` for legacy planning trees (`0012`);
  inbound-reference drift fixes (`0015`, `0016`).
- **v0.1.0-alpha**: governance core, adoption surface, specification and docs
  (work items `001`–`004`).

## How to influence it

Open a [task issue](https://github.com/ForgeWireLabs/repopact/issues/new/choose) or
start a [discussion](https://github.com/ForgeWireLabs/repopact/discussions). Scoped
contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md).
