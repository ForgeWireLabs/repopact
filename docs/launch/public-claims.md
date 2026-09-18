# RepoPact public-claim boundary

Status: current public-language guardrail for launch preparation.

*Diataxis mode: explanation (claim and maturity boundary).*

This document separates what RepoPact can accurately claim from implementation present in the repository, integration-dependent guarantees, active/deferred work, and research claims that still require evidence. It is a writing and launch guardrail, not a substitute for canonical work/evidence records.

## Claim hierarchy

| Boundary | Safe public claim | Do not turn it into |
| --- | --- | --- |
| Stable release 3.1.3 | RepoPact is a public Apache-2.0 repository-native governance system with typed work/lifecycle, evidence, decisions, provenance, brownfield adoption/repair, conformance, the canonical Rust semantic engine, and repository-defined local verification/release surfaces. | A claim that every application, integration, active work item, or platform-specific surface visible on `main` is part of the PyPI package contract. |
| Current `main` | The repository contains the stable product plus ongoing Workbench, graph, provider, enforcement, benchmark, launch, mobile, and research work governed by their own records. | A claim that code existing on `main` is automatically shipped, production-complete, or cross-platform proven. |
| Integration-dependent reference guarantees | RepoPact defines explicit assurance classes and provider/adaptor contracts. A guarantee can be claimed when the integration and platform actually provide and prove the corresponding boundary. | "RepoPact prevents all agent actions from violating policy" or any claim that a weaker adapter inherits a stronger assurance class. |
| Deferred/active enforcement work | The portable optional admission baseline is `pre-action`; stronger `sandbox/process-enforced` confinement requires a real OS boundary and native proof. Linux Landlock work demonstrates the higher class, while WI050 remains deferred rather than falsely complete. | "Universal agent containment", "cross-platform sandboxing complete", or treating pre-action interception as arbitrary-process filesystem confinement. |
| Repository Orientation Graph | A derived, versioned graph implementation exists with deterministic/incremental behavior, semantic and metadata extraction, bounded queries, and explicit freshness/coverage state. Source and governance records remain authoritative. | A complete call graph, a new source of truth, or a proven orientation-performance win before the corresponding evaluation closes. |
| Workbench | A Tauri 2 operator application exists and has concrete Windows/Linux/Android bring-up/operator evidence. | A claim that Workbench/mobile installers are part of the PyPI package, or that macOS/iOS have equivalent native validation without matching evidence. |
| Research infrastructure | RepoPact has a formal model, conformance surface, recorded findings, pre-registered benchmark protocols, deterministic benchmark infrastructure, and real-runner plumbing. | A claim that the benchmark program has already established comparative model performance. |
| Comparative empirical results | Cross-model comparative findings remain separate from benchmark infrastructure and are reported only from the corresponding runs/evidence. | Token-savings percentages, higher success rates, security-improvement percentages, or coordination gains before reportable runs exist. |
| Generality | RepoPact has been exercised on multiple real repositories, including unrelated projects, and keeps threats to generality explicit. | Universal enterprise, regulated-industry, monorepo, or many-agent-team applicability. |
| Positioning | "Repository-native governance", "durable engineering state", "governance continuity", and "the repository as a rendezvous point between humans and agents" accurately describe the project. | "AI operating system" as the primary category claim. |

## Preferred short claims

Use these when a compact description is needed:

- **RepoPact is repository-native governance for durable human-agent software engineering.**
- **RepoPact keeps load-bearing engineering state in the repository so humans and coding agents can work from the same governed context.**
- **The repository is the pact: intent, authority, decisions, work state, evidence, provenance, and history survive the session.**
- **RepoPact addresses governance discontinuity, not just context-window or agent-memory loss.**
- **RepoPact is not another agent-memory system; it is a governance-continuity system.**

## `AGENTS.md` comparison

Preferred wording:

> `AGENTS.md` and similar instruction files tell an agent how it should behave. RepoPact records the durable project state around that behavior and validates, and where the relevant boundary exists can enforce, whether the repository still respects its declared contract.

Avoid the stronger unqualified statement that RepoPact always "enforces whether the work respected the contract." Some guarantees are state-decidable, some require a diff/history/runtime boundary, some are integration-dependent, and some still require human judgment.

RepoPact is not `AGENTS.md++`. Instruction files can participate in the pact, but they are not the entire governance substrate.

## Assurance wording

When describing enforcement, name the class or the actual boundary:

- **`instruction-only`**: governed state exists; no pre-execution host boundary is claimed;
- **`session-start`**: a covered session or child start is gated;
- **`pre-action`**: a covered mutation is denied before its action/callback starts;
- **`sandbox/process-enforced`**: a real OS-backed boundary constrains the launched process tree for the claimed capability.

Do not use `enforced` as a floating adjective when the class matters. Do not imply that `pre-action` provides arbitrary-process path confinement.

## Stable-versus-repository wording

When installation instructions and current architecture appear in the same document, prefer an explicit boundary such as:

> The stable public package is RepoPact 3.1.3. The repository also contains application, integration, active/deferred work, and research surfaces whose presence on `main` does not automatically make them part of the PyPI package contract or prove cross-platform completion.

Do not invent a development release label from branch position. Use the repository's actual `VERSION`/`RELEASE_LABEL` state when a precise source identity matters.

## Claim vocabulary

Use these categories consistently:

- **stable package capability** — shipped in the current stable artifact contract;
- **implemented on `main`** — code exists in the repository, without implying packaging or product completion;
- **reference guarantee** — the reference integration demonstrates a bounded behavior under its stated conditions;
- **integration-dependent guarantee** — a provider/host can claim it only after satisfying the corresponding capability/evidence contract;
- **active / blocked / deferred / proposed** — use the work ledger's actual lifecycle term;
- **research infrastructure** — experiment machinery exists;
- **empirical result** — a measured result backed by the registered run/evidence.

These labels are intentionally not interchangeable.

## Evidence language

Prefer:

- "evidence exists for..."
- "implemented and tested on..."
- "the reference integration demonstrates..."
- "pre-registered..."
- "deterministic infrastructure is present..."
- "comparative results remain pending..."
- "the tested case held..."

Avoid:

- "proven safe"
- "prevents all..."
- "industry-leading"
- "SOTA" without an agreed comparison and measured evidence
- "benchmark-proven" before reportable comparative runs exist
- "cross-platform validated" when only a subset of target platforms has native evidence

## Release and launch implication

RepoPact 3.1.3 is the stable package line for public installation language. Launch material may also discuss Workbench, ROG, provider integrations, or enforcement research, but it must preserve each surface's actual package, lifecycle, platform, and evidence boundary.

A broad launch should not make the PyPI package appear equivalent to the entire development repository, and it should not make active or deferred work appear complete merely because substantial implementation exists.
