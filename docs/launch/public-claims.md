# RepoPact public-claim boundary

Status: current public-language guardrail for launch preparation.

This document separates what RepoPact can accurately claim today from development work, active research, and claims that still require evidence. It is a writing and launch guardrail, not a substitute for canonical work/evidence records.

## Claim hierarchy

| Boundary | Safe public claim | Do not turn it into |
| --- | --- | --- |
| Stable release 3.0.2 | RepoPact is a public Apache-2.0 repository-native governance system with typed work/lifecycle, evidence, decisions, provenance, brownfield adoption/repair, and conformance mechanisms. | A claim that every capability visible on current `main` ships in PyPI 3.0.2. |
| Current development `main` | Development work includes the canonical Rust semantic authority, the Tauri 2 Workbench, core human operator workflows, local-first verification/release architecture, concrete Windows/Linux/Android Workbench evidence, and substantial Repository Orientation Graph implementation. | A stable-release availability claim unless a new package/release has actually been cut. |
| Active/incomplete development | Protected pre-execution agent admission remains active work; the orientation graph still has higher-level query/orientation, adoption, performance, UI, and evaluation work remaining. | "Universal agent containment", "complete repository understanding", or "finished graph product". |
| Research infrastructure | RepoPact has a formal model, conformance surface, recorded adversarial/field findings, pre-registered benchmark protocols, deterministic benchmark infrastructure, and an S8 governance-continuity/orientation protocol registered before graph-enabled evaluation. | A claim that the benchmark program has already established comparative model performance. |
| Comparative empirical results | Cross-model comparative findings remain pending and disconfirming outcomes are explicitly in scope. | Token-savings percentages, higher success rates, security-improvement percentages, or coordination gains before reportable runs exist. |
| Platform evidence | Windows, Linux, and Android have concrete Workbench bring-up/operator evidence. | macOS or iOS native validation claims before equivalent evidence exists. |
| Generality | RepoPact has been exercised on multiple real repositories, including unrelated projects, and keeps threats to generality explicit. | Universal enterprise, regulated-industry, monorepo, or many-agent-team applicability. |
| Enforcement | RepoPact has explicit enforcement classes and a completed local verification/release architecture. Stronger protected pre-execution agent admission is an active, separately bounded effort. | A claim that all agent actions are mechanically prevented from violating policy. |
| Repository Orientation Graph | A derived, versioned graph implementation exists with deterministic and incremental behavior, semantic/metadata extraction, and explicit freshness/coverage state. Source and governance records remain authoritative. | A complete call graph, a new source of truth, or a proven orientation-performance win. |
| Positioning | "Repository-native governance", "durable engineering state", "governance continuity", and "the repository as a rendezvous point between humans and agents" accurately describe the project. | "AI operating system" as the primary category claim. |

## Preferred short claims

Use these when a compact description is needed:

- **RepoPact is repository-native governance for durable human-agent software engineering.**
- **RepoPact keeps load-bearing engineering state in the repository so humans and coding agents can work from the same governed context.**
- **The repository is the pact: intent, authority, decisions, work state, evidence, provenance, and history survive the session.**
- **RepoPact addresses governance discontinuity, not just context-window or agent-memory loss.**
- **RepoPact is not another agent-memory system; it is a governance-continuity system.**

## AGENTS.md comparison

Preferred wording:

> `AGENTS.md` and similar instruction files tell an agent how it should behave. RepoPact records durable project state around that behavior and validates, and where enforceable can enforce, whether the repository still respects its declared contract.

Avoid the stronger unqualified statement that RepoPact always "enforces whether the work respected the contract". Some guarantees are state-decidable, some require a diff/history/runtime boundary, and some still require human judgment.

## Stable-versus-development wording

When discussing current architecture in the same document as installation instructions, use an explicit boundary such as:

> The stable public package is RepoPact 3.0.2. Current development `main` contains newer Rust-engine, Workbench, local-verification, and repository-orientation work that should not be assumed to exist in the stable package until a later release is cut.

## Evidence language

Prefer:

- "evidence exists for..."
- "implemented and tested on current development `main`..."
- "pre-registered..."
- "deterministic plumbing/infrastructure is present..."
- "comparative results remain pending..."
- "the tested case held..."

Avoid:

- "proven safe"
- "prevents all..."
- "industry-leading"
- "SOTA" without an agreed comparison and measured evidence
- "benchmark-proven" before reportable real-model runs
- "cross-platform validated" when only a subset of target platforms has native evidence

## Release gate implication

The launch should not make the stable package appear equivalent to the development tree. Before a broad public launch, either:

1. cut a new stable release containing the publicized development surfaces with release evidence, or
2. keep every public page visibly explicit about which capabilities are stable 3.0.2 and which are development-only.

A new release is preferable if the README, screenshots, paper, and launch copy materially feature capabilities that a fresh `pip install repopact` user cannot exercise from the stable package.
