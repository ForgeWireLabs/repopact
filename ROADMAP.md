# Roadmap

A human-curated forward view. The authoritative status of in-flight work is the derived [dashboard](audits/reports/dashboard.md) and the `work/` ledger; this file adds editorial intent that is not derivable.

## Now — shipped through v3.1.1

RepoPact's stable line has moved well beyond the original filesystem/validator prototype. The shipped product now includes the durable governance model plus a canonical Rust semantic engine and repository-defined verification/release surfaces.

Major stable milestones include:

- **Mandatory preflight and provenance-typed records** (decision `0021`, v2.0.0): work is recorded before implementation begins; reconstructed state can remain `concrete`, `provisional`, or `inferred` instead of pretending brownfield history is certain.
- **Published conformance suite** (`CONFORMANCE.md`, `conformance/`, WI019): third-party implementations can test acceptance/rejection against the standard rather than against one historical implementation.
- **`proposed` lifecycle state** (decision `0023`, v2.1.0): candidate work can be captured durably without granting implementation authority.
- **Canonical dashboard integrity** (decision `0025`, v2.2.0): committed derived state is checked against freshly generated output instead of being trusted because it exists in Git.
- **Release identity separation** (decision `0026`, v2.3.0 and later release decisions): stable version identity is kept distinct from development/pre-release maturity.
- **Single-package execution boundary** (decision `0029`, v3.0.0): adopters install RepoPact as a package rather than vendoring the implementation into every governed repository.
- **Semantic claim freshness** (WI033): human-authored audit/research claims have review horizons instead of hiding behind exact generated projections.
- **Canonical Rust semantic engine and versioned language-neutral protocol** (3.1.x line): migrated product semantics have one canonical engine; Python remains a compatibility, packaging, and comparator surface rather than a second product authority.
- **Local-first verification and release architecture** (WI046): repositories define verification profiles; local execution is canonical; hosted CI/CD is an optional adapter; verification, artifact construction, inspection, and publication remain separate operations.
- **Repository Orientation Graph foundations** (WI063 / 3.1.x line): deterministic, versioned, incremental semantic and metadata projection plus bounded graph queries are implemented while the graph remains explicitly derived rather than authoritative.
- **Provider-neutral assurance/admission model**: RepoPact represents progressively stronger enforcement classes instead of collapsing all integrations into a boolean "enforced" claim.
- **3.1.1 packaging corrective** (decision `0063`): the sdist now actually contains the `LICENSE` file declared by its package metadata; no schema, protocol, CLI, lifecycle, or provenance behavior changed from 3.1.0.

The stable package is not the same thing as every repository surface. Workbench, mobile application packaging, deferred/active enforcement work, and research/evaluation surfaces keep their own evidence and completion boundaries.

## Next — the active ledger

The dashboard remains authoritative for status. At this roadmap snapshot the principal active items are:

- **WI020 — PactBench / Proving Ground integration**: keep the runnable benchmark environment separate from RepoPact's product repository while exercising the packaged product.
- **WI021 — Public launch**: finish launch-facing assets and operator-gated publication steps such as arXiv and Show HN. Package publication alone does not complete the whole launch item.
- **WI022 — Comparative benchmark suite**: execute matched-arm recovery, coordination, efficiency, drift, and security studies. Benchmark infrastructure is not itself a comparative result.
- **WI034 — Independent reproduction**: obtain third-party evidence not produced solely by the maintainer.
- **WI063 — Repository Orientation Graph**: finish remaining evaluation, closeout, productization, and evidence obligations without promoting the graph into project authority.
- **WI066 — 3.1.x milestone/research reconciliation**: reconcile remaining release/research closeout evidence rather than rewriting history around the already-cut release line.
- **WI067 — GitHub repository provider and secure remote import**: continue provider-neutral remote repository acquisition/authorization work.

**WI032** remains blocked around remote cross-platform governance enforcement. The absence of that hosted/remote checkpoint must not be translated into a claim that local verification proves remote enforcement.

## Deferred and proposed boundaries

- **WI050 — Pre-execution agent work admission and preflight enforcement** is deferred after substantial admission/protected-guard and Linux confinement work. The portable `pre-action` class remains distinct from stronger `sandbox/process-enforced` confinement, and missing native proof is not papered over as completion.
- **WI044 — Typed capability completion/replacement evidence** remains proposed.
- **WI047 — Documentation impact and code/documentation closure** remains proposed. It is the future mechanism for making documentation impact an explicit closeout decision for governed code changes.
- **WI068 — Embedded mobile Git backend and remote synchronization** remains proposed.

Proposed work is durable planning, not implementation authority.

## Later

Likely later-stage work includes:

- complete remaining native/reference proof needed for stronger cross-platform enforcement classes when the required platforms and environments are available;
- finish documentation-impact closure design (WI047) without forcing meaningless documentation churn for internal refactors;
- expand provider-neutral remote/mobile repository workflows without making a single forge or transport authoritative;
- external ingestion at L5 for tracker exports and design documents as provenance-bearing records;
- mechanized temporal/relational invariants, including stronger history semantics and nested-contract refinement rules;
- continue Workbench and mobile productization while preserving the Rust engine as semantic authority and repository records as project authority.

## Earlier

- **v3.0.x**: package boundary consolidation, linked-worktree and record-relative correctness fixes, and release hardening.
- **v2.x**: mandatory preflight, provenance typing, proposed lifecycle, conformance publication, deterministic dashboard integrity, and release identity separation.
- **v1.x**: proving-ground evidence, brownfield adoption, PyPI publication, import-plan, doctor upgrade/repair, takeover, and inbound-reference drift fixes.
- **v0.1.0-alpha**: governance core, adoption surface, specification, and initial docs.

## How to influence it

Open a [task issue](https://github.com/ForgeWireLabs/repopact/issues/new/choose) or start a [discussion](https://github.com/ForgeWireLabs/repopact/discussions). Scoped contributions are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).
