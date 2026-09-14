# arXiv submission preparation

**Status:** preparation only — **not submitted**.

This file stages the metadata and checklist for a future RepoPact arXiv submission. Publication, account/endorsement steps, final license selection, and operator approval are intentionally not performed in this preparation pass.

## Candidate metadata

**Title**  
RepoPact: Repository-Native Governance for Durable Human-Agent Software Engineering

**Author**  
Jeremy Shows

**Affiliation in manuscript**  
ForgeWire Labs

**Primary category**  
`cs.SE` — Software Engineering

**Cross-list**  
None selected for the initial preparation. A `cs.AI` cross-list can be reconsidered during final submission review if the final paper warrants it.

**Report number**  
Leave blank unless ForgeWire Labs intentionally establishes a technical-report numbering scheme.

**Journal reference / DOI**  
Leave blank for the initial preprint.

## Candidate arXiv abstract

Agentic software engineering increasingly distributes work across humans, coding agents, machines, model providers, and execution sessions. The source code usually survives these handoffs, but the governing state around it often does not: intent, authority, invariants, decisions, acceptance criteria, provenance, and evidence may remain in conversations, local memory, issue trackers, or one operator's head.

We present RepoPact, a repository-native governance kernel for durable human-agent software engineering. RepoPact stores governing project state as typed, version-controlled records alongside the artifact being governed, so a fresh human or agent can recover represented work state, authority boundaries, invariants, decisions, evidence, provenance, lifecycle, and known violations without requiring predecessor-private context. We call this property governance continuity.

RepoPact models the repository as a six-layer kernel: typed records, work-item lifecycle, invariant monitoring, a typed enforcement lattice, deterministic derived views, and a brownfield adoption boundary. Its central primitive is the binding invariant, a declared guarantee coupled to rationale, escalation, and, where logically possible, machine enforcement. Provenance types distinguish concrete, provisional, and inferred reconstructed state without weakening evidence-gated completion.

We evaluate the system through released packages, the current development implementation, its conformance suite, adversarial findings, naturalistic adoption cases, and a pre-registered comparative benchmark program. The implementation now uses a canonical Rust semantic engine with compatibility tooling and a Tauri 2 Workbench. The stable public boundary is 3.0.2; the reconciled development snapshot described in the manuscript is newer and is not attributed retroactively to that release. Comparative cross-model results remain pending and will be reported whether they support or challenge the claims.

## Candidate comments field

The final locally compiled manuscript reports 8 pages and 2 figures:

> Preprint. 8 pages, 2 figures. Artifact repository: https://github.com/ForgeWireLabs/repopact . Frozen implementation/evidence snapshot: `542cd5c01a79942ecb78040b19deb4e82f4f9068`. Comparative cross-model benchmark results are not included in this version.

## Local build record

- Source package: `research/arxiv/`
- Manuscript: `main.tex`
- Bibliography: `references.bib`
- Figures: `figures/repopact-hero-social.png`, `figures/workbench-governance.jpg`
- Compiler: Tectonic 0.17.0 Windows MSVC binary
- Clean command from the package directory: `tectonic --outdir build --keep-logs main.tex`
- Output: `main.pdf`, 8 pages, 330,559 bytes at final inspection
- PDF metadata: title matches the candidate title; author is Jeremy Shows
- QA: rendered all 8 pages with Poppler `pdftoppm` at 120 dpi and visually inspected the complete output; no clipped figures, missing glyphs, absolute-path dependency, or PII was found
- Build warnings: only non-fatal underfull boxes in the findings table; no overfull boxes remain

## License decision

**Not selected.** Treat this as an explicit operator gate. The final arXiv license choice should be made only after considering likely workshop/journal requirements because the arXiv license selection is a publication decision, not a repository default inherited automatically from Apache-2.0 software licensing.

## Required manuscript reconciliation before submission

The September 12 manuscript must not be submitted unchanged. Before final export:

- separate stable public release `3.0.2` from the newer development implementation;
- update the findings discussion from the original fourteen-entry summary through the current findings register, including the F-019 pre-execution-admission boundary;
- describe WI046 local-first verification/release as completed development architecture rather than future work;
- replace the statement that a Repository Orientation Graph is merely future work with the current active-implementation boundary;
- update Workbench future-work wording to reflect completed core operator workflows while keeping newer graph/orientation and platform work explicit;
- update benchmark maturity to acknowledge deterministic task/harness/materialization infrastructure without turning plumbing into empirical findings;
- soften the `AGENTS.md` comparison to "records, validates, and where enforceable, enforces";
- distinguish the stable CLI surface from development-only `verify`, grouped `release`, and graph operations;
- update the revision date;
- pin the paper's implementation snapshot to an exact settled `main` commit so claims remain reproducible while development continues;
- regenerate any paper figures/tables whose counts or implementation-state labels changed.

See [`paper-reconciliation-2026-09-13.md`](paper-reconciliation-2026-09-13.md).

## Source-package checklist

Before submission:

- [x] Freeze the final paper text against a specific RepoPact commit.
- [x] Convert/maintain the manuscript as arXiv-compatible LaTeX source rather than relying on PDF-only submission.
- [x] Include all locally required `.tex`, bibliography, and figure assets with repository-relative paths.
- [x] Remove absolute machine paths and local-only assets.
- [x] Confirm every figure is legible at paper width; grayscale suitability remains an operator review item.
- [x] Confirm references compile without missing bibliography entries.
- [x] Confirm title, author, affiliation, abstract, and keywords agree between manuscript and arXiv metadata.
- [x] Confirm no development-only capability is attributed to stable 3.0.2.
- [x] Confirm findings/results counts match the pinned repository snapshot.
- [x] Confirm no deterministic/mock benchmark result is described as a real-model empirical result.
- [x] Confirm Windows/Linux/Android evidence is distinguished from unvalidated macOS/iOS targets.
- [x] Run a clean LaTeX build from only the submission directory.
- [x] Inspect the generated PDF end to end.
- [x] Fill final page/figure counts in the Comments field.
- [ ] Decide arXiv license deliberately.
- [ ] Start the arXiv submission and handle endorsement/account requirements if arXiv requests them.
- [ ] Operator approves final text and metadata before submission.

## Post-publication follow-up

After an arXiv identifier exists:

- add the identifier and preferred citation to `CITATION.cff`;
- link the paper prominently from the README and launch copy;
- add the arXiv URL to GitHub repository metadata/homepage if appropriate;
- update the launch post from "paper preparing" to the exact paper link;
- record publication evidence against WI021 without rewriting earlier preparation history.
