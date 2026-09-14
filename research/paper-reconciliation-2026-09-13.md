# Paper reconciliation — 2026-09-13

Status: editorial reconciliation plan for `research/paper.md`; no publication implied.

The paper's central argument remains current, but implementation and field evidence advanced materially after the 2026-09-12 revision. This record identifies the changes required before the manuscript is treated as submission-ready. The final paper should be reconciled against a settled, exact `main` commit rather than chasing a moving development branch during active implementation.

## 1. Stable release versus development implementation

The paper currently moves too quickly from "the current public release is 3.0.2" into capabilities that were implemented after the 3.0.2 corrective release.

Required boundary:

> The current stable public package is RepoPact 3.0.2. The development implementation described in later sections includes newer Rust-engine, Workbench, local-verification, and repository-orientation work on `main`. Unless stated otherwise, those newer development capabilities should not be assumed to exist in the stable 3.0.2 package.

The final manuscript should pin the development implementation to an exact commit.

## 2. Findings register

Section 6 currently says the findings register contains fourteen entries. The paper must be reconciled through the then-current register, including at minimum the later findings already present before this reconciliation:

- **F-015** — linked-worktree discovery produced false governance errors; corrected in the 3.0.2 line.
- **F-016** — human-readable work-item narrative can drift from canonical `work-item.json` identity while validation remains green; open at the time of this reconciliation.
- **F-017** — `doctor` resolved `source_of_truth` pointers against the wrong base; fixed through explicit record-relative semantics.
- **F-018** — evidence chronology could depend on wall-clock interpretation rather than deterministic recording chronology; fixed with a deterministic Git-recording basis for applicable records.
- **F-019** — mandatory preflight and post-change validation do not, by themselves, mechanically prevent an autonomous agent's first mutating action; this motivated the protected pre-execution admission boundary in WI050.

F-019 is important negative evidence. It should be included as an architecture-changing result rather than hidden until WI050 is complete.

Suggested discussion:

> A later field case exposed a stronger boundary than enforcement closure alone. RepoPact could durably require preflight and detect an invalid sequence after the fact while still lacking a mechanism that prevented an autonomous worker's first write before admission. This became F-019 and motivated WI050. The result narrows the claim: repository governance and post-change verification are not equivalent to pre-execution containment. Protected admission requires a distinct runtime or OS-backed enforcement boundary.

## 3. Local-first verification and release

WI046 is no longer future architecture. The development implementation now has a provider-neutral repository verification/release contract with locally complete execution and optional hosted adapters.

Add a reference-implementation subsection along these lines:

> RepoPact's development implementation treats repository-defined local verification as the canonical execution path. Named verification profiles define semantic checks independently of execution venue. Local verification and release preparation can run without GitHub Actions, while hosted workflows act as optional adapters and are disabled by default unless explicitly enabled. Verification success remains distinct from admission enforcement: a passing local run is evidence about the candidate state, not proof that every remote promotion path is closed.

Keep publication, verification, build, and admission conceptually separate.

## 4. Repository Orientation Graph

The current paper says a durable repository graph is future work. That statement is stale.

Replace the relevant §7.8 passage with wording equivalent to:

> A durable Repository Orientation Graph is now under active implementation. The current development implementation provides a derived, versioned graph substrate with deterministic rebuild, incremental convergence, language-aware semantic extraction, working-tree overlays, and explicit freshness and coverage state. It remains non-authoritative: source and governance records remain the source of truth. Higher-level orientation queries, adoption integration, broader topology, performance closeout, operator-facing graph workflows, and the pre-registered S8 graph-enabled evaluation remain incomplete.

Preserve the chronology argument: H15/S8 orientation-cost metrics were registered before ROG implementation began. This is a useful defense against mechanism-driven metric selection.

Do not describe the graph as a complete call graph, complete repository understanding, or a proven orientation-cost improvement.

## 5. Workbench maturity

The paper's future-work list still says to finish human operator control parity across the core governance lifecycle. Core create/edit/transition/inspect workflows are already implemented and evidenced in the completed Workbench milestones.

Replace that future-work item with:

> Maintain operator-control parity as new repository-orientation, verification, and admission capabilities are added, and validate equivalent native behavior on macOS and iOS.

Keep the platform boundary explicit: Windows, Linux, and Android have concrete evidence; macOS and iOS should remain targets rather than validated platforms until equivalent native evidence exists.

## 6. Benchmark maturity

The current paper is correct not to claim comparative results, but it understates the deterministic infrastructure now present.

Preferred wording:

> Deterministic task materialization, driver, scoring, and harness infrastructure now exists across the comparative program, but these artifacts establish experimental plumbing rather than model-performance findings. No comparative cross-model result is reported in this paper version.

Retain the distinction between mock/deterministic plumbing evidence and reportable real-model runs. Do not create token-economy, security, recovery, or coordination headline numbers until the live-run criteria are actually satisfied.

## 7. `AGENTS.md` comparison

The current sentence is too categorical:

> `AGENTS.md` tells an agent how to behave. RepoPact records and enforces whether the work respected the contract.

Use instead:

> `AGENTS.md` and similar instruction files tell an agent how it should behave. RepoPact records durable project state around that behavior and validates, and where enforceable can enforce, whether the repository still respects its declared contract.

This matches the typed enforcement lattice: some guarantees are state-decidable, some require a diff/history/runtime boundary, and some retain human judgment.

## 8. CLI surface

Section 4.2 should distinguish the stable public package from development-main operations.

Keep stable commands under an explicit stable heading. Add a development heading for newer capabilities such as repository-defined `verify`, grouped `release`, and graph operations only if those operations are present at the pinned implementation commit. Do not let the paper imply that a fresh stable 3.0.2 installation exposes every development command.

## 9. Results framing

Preserve the paper's existing restraint:

- a `holds` finding is a bounded tested case, not a universal proof;
- deterministic benchmark infrastructure is not empirical model evidence;
- F-019 is evidence against an overly broad interpretation of mandatory preflight;
- ROG implementation is not evidence that ROG improves orientation until S8 R1 is run;
- local verification success is not remote enforcement closure;
- Workbench visual maturity is not equivalent to every active governance capability being exposed.

## 10. Future-work rewrite

The current future-work list should be updated approximately as follows:

1. complete reportable PactBench and comparative benchmark runs across multiple model families;
2. complete comparative drift, recovery, coordination, security, and token-economy studies;
3. run enforcement-closure evaluation outside ForgeWire Labs-controlled repositories;
4. run the pre-registered S8 governance-continuity study across clean worker, machine, and client handoffs;
5. obtain genuine third-party reproduction and adoption evidence;
6. maintain Workbench operator-control parity as new governance/orientation/admission capabilities land;
7. validate macOS and iOS native behavior before claiming support;
8. complete protected pre-execution admission and cross-platform enforcement evidence without conflating it with repository validation;
9. finish ROG query/orientation/adoption/performance/operator surfaces and then evaluate the graph-enabled S8 condition against the frozen no-graph baseline;
10. expand external ingestion while preserving provenance and authority;
11. mechanize more temporal and relational invariants and harden repair semantics;
12. encourage independent conformance implementations.

## 11. Revision identity

For the next manuscript revision:

- update the revision date;
- record the exact repository commit used as the implementation/evidence snapshot;
- ensure findings count, WI states, release identity, benchmark status, and platform wording all match that commit;
- regenerate figures/tables that encode mutable counts or status labels;
- run the research claim-freshness checks before treating the manuscript as frozen.

## Submission-readiness rule

The paper is ready for final LaTeX/submission packaging when its claims can be read against one exact repository commit without requiring a reader to infer whether a sentence refers to stable 3.0.2, the development implementation, an active work item, or a planned experiment.
