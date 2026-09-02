# 046 — Runner-Neutral Verification and Admission Checkpoint Architecture

> **Status**: Proposed
> **Owners**: governance-owner (lead); tooling-owner, evidence-owner, docs-owner, and work-coordinator affected.
> **Depends on**: WI039 (completed enforcement-closure field study).

## Intent

Make verification and admission checkpoints a first-class RepoPact architectural concept without turning RepoPact into a CI hosting or remote execution service.

RepoPact already models durable authority, work, evidence, drift, and enforcement closure. It also adopts existing `.github/workflows/*` files as binding-gate policies. What it does not yet own independently is the logical verification contract that those provider-specific workflows execute.

That omission becomes visible when execution venue and verification contract are conflated. A repository can have the right checks but lose GitHub-hosted execution because of an account/provider failure; it can also have healthy hosted CI that never invokes RepoPact at all. Those are different failures and should not require different definitions of what the repository considers a valid checkpoint.

The target architecture is therefore runner-neutral:

```text
RepoPact verification / admission contract
                |
        replaceable executors
     /          |          \
 local      hosted CI    operator-owned / Fabric-style
```

The same logical checkpoint should be executable on a maintainer workstation, GitHub Actions, another hosted provider, a self-hosted runner, or an operator-controlled ForgeWire/Fabric execution path. Provider YAML is an adapter, not the source of truth.

## Field trigger

This work is motivated by two already-recorded failure classes:

1. ForgeWire previously had hosted CI that ran but did not invoke RepoPact, a checkpoint-coverage failure promoted into RepoPact by WI039.
2. On 2026-09-01 ForgeWire moved to local-only validation after GitHub-hosted execution became unavailable at the account level. The verification commands remained executable locally, but the hosted venue could not be treated as authoritative or available.

WI039 already established the cross-cutting enforcement-closure vocabulary of checkpoint **coverage**, **invocation**, and **effectiveness**. WI046 must build on that model rather than invent a second overlapping theory.

## Architectural boundary

RepoPact should own:

- the durable definition of what must be verified;
- the admission boundary to which that verification applies;
- required capabilities/platform semantics;
- fail/pass/degraded/unavailable truthfulness;
- evidence requirements and provenance;
- representation of coverage, invocation, effectiveness, and closure;
- operator policy governing acceptable executor classes.

RepoPact should **not** own:

- hosted runner fleets;
- container scheduling;
- VM lifecycle;
- provider billing APIs;
- remote worker provisioning;
- cluster scheduling;
- generic job orchestration.

Those remain responsibilities of GitHub/GitLab/AppVeyor/Jenkins/self-hosted infrastructure/ForgeWire Fabric or equivalent execution substrates.

## Design questions to resolve before implementation

1. What is the canonical record placement for the verification/checkpoint definition?
2. Should evidence-run records gain additive checkpoint/executor fields, reference a companion record, or remain unchanged behind another record type?
3. How are admission boundaries named without forcing GitHub terminology into the standard?
4. What is the reference local CLI surface (`verify`, `checkpoint`, profiles, or another design)?
5. How does RepoPact represent a provider outage without implying that local proof restores remote merge enforcement?
6. How does operator-declared `local-only` or hosted-disabled policy interact with frozen surfaces and escalation?
7. How should `adopt` infer checkpoint contracts from existing CI without fabricating semantics the workflow does not actually prove?
8. How should `doctor` migrate current binding-gate policies non-destructively?
9. Which checkpoint state is source-authored and which dashboard/closure views are derived?
10. What compatibility/versioning treatment is required now that the RepoPact 3.x record surface is stable?

## Intended execution shape

The motivating shape is conceptually:

```text
repopact verify <named-contract>
```

where the named contract determines checks and evidence semantics, while the executor is replaceable.

A future GitHub workflow should therefore be able to reduce to a thin invocation of the same RepoPact verification contract used locally. A ForgeWire/Fabric runner should be able to consume the same contract without RepoPact importing or depending on Fabric.

The exact command names and record schema are deliberately **not** decided by this proposed work item.

## Relationship to WI032

WI032 remains a separate blocked operational work item about restoring a usable **remote, public, cross-platform enforcement checkpoint** for RepoPact's own repository.

WI046 does not declare WI032 obsolete and does not pretend local validation creates remote admission effectiveness. Instead, WI046 should make the distinction explicit:

- logical verification contract;
- available execution venues;
- evidence from an actual invocation;
- admission effectiveness at a specific boundary.

When both items are eventually complete, RepoPact should no longer need provider-specific workflow files to serve as the conceptual definition of CI, while WI032 can still prove that a real remote admission boundary is closed.

## Acceptance criteria

The machine-readable acceptance criteria live in `work-item.json`. In summary, this work must:

- capture and classify the real field failures that motivated it;
- reconcile with WI039's Cov/Inv/Eff model;
- decide the kernel/executor boundary before schema implementation;
- evaluate record-placement alternatives rather than jumping directly to a new file type;
- define runner-neutral verification and admission semantics;
- make local execution first-class without overstating remote enforcement;
- support explicit operator execution-venue policy such as local-only mode;
- produce venue-neutral repository evidence;
- distinguish passing verification from effective admission enforcement;
- migrate/adapt current workflow-based adoption semantics honestly;
- demonstrate thin local and hosted adapters plus an operator-owned/Fabric-shaped executor path;
- expose checkpoint state through derived views without collapsing distinct failure modes;
- add conformance and negative enforcement coverage;
- validate against ForgeWire and an adopter-neutral fixture;
- decide compatibility/versioning before implementation;
- update all required RepoPact surfaces only after the architecture is accepted.

## Non-goals

- Replacing GitHub Actions, Jenkins, AppVeyor, GitLab CI, or Fabric.
- Building a generalized DAG scheduler.
- Managing CI provider accounts, billing, or credentials.
- Making every verification continuous or runtime-resident; RepoPact remains admission/checkpoint oriented.
- Claiming a local passing run is equivalent to a protected remote merge gate.

## Closeout

This item begins as proposed architecture/reconciliation work. It should move to active only when implementation is explicitly accepted. Every satisfied criterion requires linked concrete evidence, and completion requires negative cases demonstrating that absent, unavailable, or ineffective checkpoints are not mislabeled as enforcement closure.
