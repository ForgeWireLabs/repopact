# 046: Local-First Verification, CI/CD, and Optional Hosted Adapters

> **Status**: Proposed  
> **Owners**: governance-owner (lead); tooling-owner, evidence-owner, docs-owner, and work-coordinator affected.  
> **Depends on**: WI039 (completed enforcement-closure field study).  
> **Architecture decision**: Decision 0043, accepted 2026-09-12.

## Intent

Make RepoPact verification and release execution **local-first and locally complete**, while keeping hosted CI/CD systems as optional adapters that are disabled by default.

RepoPact must remain able to validate, test, check conformance, refresh/check derived artifacts, build release artifacts, perform release verification, and record evidence from an operator-controlled machine when GitHub Actions or another hosted provider is unavailable. Hosted execution may mirror or enforce the same contracts when explicitly enabled, but provider YAML is not the source of truth.

The target architecture is:

```text
                  repository-defined contracts
                            |
             +--------------+--------------+
             |                             |
             v                             v
      canonical local runner        optional adapters
             |                    /       |        \
             |              GitHub    self-hosted   Fabric/other
             |                    
             v
  validation / tests / conformance /
  packaging / release verification /
  evidence / optional publication
```

Local execution is the primary implementation. Hosted adapters must eventually reduce to thin invocations of the same local contracts.

## Why this changed

The previous wording treated local, hosted, self-hosted, and remote execution as peers under a runner-neutral model. That was directionally correct, but it left open the possibility that the product would still depend operationally on hosted CI for ordinary release readiness.

The operator has now fixed a stronger policy:

1. RepoPact CI/CD must be fully usable locally.
2. GitHub-hosted CI/CD is optional.
3. GitHub-hosted CI/CD is off by default.
4. Hosted CI and hosted CD are independently opt-in.
5. A provider outage or account restriction must not prevent local verification or release preparation.

Decision 0043 records this as architecture rather than a temporary workaround.

## Current repository state

RepoPact currently has two GitHub Actions workflows:

- `.github/workflows/governance.yml`
- `.github/workflows/release.yml`

Before Decision 0043 they were hard-disabled using `if: ${{ false }}` as a temporary local-only measure. They are now retained as optional hosted adapters guarded by repository variables:

```text
REPOPACT_GITHUB_CI=true
REPOPACT_GITHUB_CD=true
```

If those variables are absent or not exactly `true`, the hosted jobs remain disabled.

This is an immediate default-off switch, not the final local-CI architecture. The larger goal of WI046 is to remove semantic duplication between local execution and provider YAML.

## Architectural rules

### 1. Local is canonical

The complete required verification path must run locally from the repository without GitHub Actions.

At minimum the local contract must cover the checks that are part of release or governance readiness, including where applicable:

- canonical RepoPact validation;
- Python tests that remain part of the supported surface;
- Rust workspace tests and targeted crate/application tests;
- conformance suite;
- governance/admission regression suites;
- generated dashboard/spec freshness;
- frozen-surface reporting/checking when a base is available;
- package/build verification;
- wheel/sdist/native artifact inspection;
- installer or platform packaging checks where supported;
- artifact hashing and release manifests;
- evidence recording;
- release-readiness summary.

The exact profile decomposition is implementation work, but the local path may not be a weaker subset of the hosted path.

### 2. Verification, build, and publication are separate phases

A local operator must be able to:

```text
verify without building
build without publishing
verify built artifacts without publishing
publish only after explicit operator intent
```

Publication is never an accidental side effect of validation.

### 3. Hosted CI and hosted CD are separate switches

The repository uses two explicit opt-ins:

```text
REPOPACT_GITHUB_CI=true
REPOPACT_GITHUB_CD=true
```

They are separate because validation and publication have different authority and credential requirements.

An unset variable means off. A workflow file, release event, token, environment, or trusted-publisher configuration does not enable anything by itself.

### 4. Hosted YAML becomes an adapter

GitHub Actions should not permanently contain an independent hand-maintained copy of the semantic verification pipeline.

The desired end state is conceptually:

```yaml
- run: repopact verify release
```

rather than a long provider-specific list of commands that can drift from the local implementation.

The same applies to hosted release preparation and publication. GitHub YAML selects a venue and supplies venue-specific credentials/capabilities. It does not redefine what a RepoPact release means.

### 5. Local success is not remote enforcement closure

A local passing run is valid evidence for a local invocation. It is not evidence that GitHub branch protection, a hosted merge gate, or another remote boundary is effective.

RepoPact must keep these facts separate:

```text
contract declared
execution venue available
checkpoint invoked
checkpoint passed/failed
admission result bound to promotion
```

This preserves WI039's Cov/Inv/Eff model.

### 6. Release credentials stay outside the repository

A local publication path may use an operator-provided PyPI token, keyring, credential helper, environment injection, or another documented secure mechanism. The repository must not require committed secrets.

GitHub OIDC/Trusted Publishing can remain an optional hosted adapter, but it is not the canonical publication architecture.

### 7. Cross-platform local profiles

Local verification must account for platform-specific work without forcing one machine to impersonate every platform.

The architecture should support profiles/capabilities such as:

```text
core
windows
linux
macos
android
ios
release
full
```

or an equivalent model.

A profile records what actually ran and what could not run in the current environment. Missing platform capability is reported honestly, not silently treated as success.

### 8. Local evidence is repository-native

Checkpoint evidence must identify at least:

- verification/release profile;
- RepoPact/product version;
- repository identity and candidate tree/commit where available;
- executor class, with local as a first-class value;
- platform/capabilities;
- commands or semantic checks executed;
- result;
- relevant artifacts and hashes;
- timestamps/duration where useful;
- provenance.

The evidence model must not privilege GitHub run IDs as the primary identity.

### 9. No billing/provider API coupling

RepoPact does not need to know why GitHub Actions is unavailable. Billing lock, outage, quota exhaustion, organization policy, operator preference, or disabled workflow are all execution-venue facts.

The repository only needs to represent that the hosted venue is disabled/unavailable/not invoked while local execution remains available.

## Intended local command surface

Exact names remain subject to the architecture decision made during implementation, but the product should converge on a small local contract such as:

```text
repopact verify <profile>
repopact verify --changed <base>
repopact release verify
repopact release build
repopact release inspect
repopact release publish
```

The implementation may choose different names if the semantics are clearer, but the following properties are binding:

- commands are provider-neutral;
- commands are scriptable and return stable exit semantics;
- machine-readable output is available;
- human-readable output remains useful;
- evidence generation is integrated rather than reconstructed from terminal logs after the fact;
- publication always requires explicit intent.

## CI semantics

A CI profile should be able to answer:

```text
What must be checked?
What changed?
Which checks apply?
Which capabilities are required?
Which checks ran?
Which checks passed, failed, skipped, or were unavailable?
What evidence was produced?
Is this enough to claim the relevant admission boundary is closed?
```

The local runner should be able to use Git diff information for change-aware selection where safe, but a full profile must remain available and should be used for release readiness.

Change-aware optimization must not become an authority loophole. If applicability cannot be proven, the conservative profile runs.

## CD semantics

The local CD path should be able to prepare a complete release without hosted runners.

At minimum, where relevant to the release:

1. verify source state;
2. verify version/release labels;
3. build artifacts for the current supported platform/capability set;
4. inspect artifact contents and embedded engine versions;
5. run package/install smoke tests where available;
6. generate hashes/manifests;
7. record evidence;
8. produce a release-readiness summary;
9. stop before publication unless the operator explicitly requests publish.

Cross-platform artifacts that cannot be built on the current host remain separate capability evidence, not fabricated successes.

## Relationship to WI032

WI032 remains about a **remote/public cross-platform admission checkpoint**. It is not required for RepoPact to have a complete local CI/CD path.

After Decision 0043:

- WI046 owns local-first verification/release architecture and optional adapter semantics;
- WI032 may later prove remote enforcement closure when a suitable remote execution venue is available;
- failure or absence of WI032 must not make local RepoPact development/release verification impossible;
- a remote gate can strengthen admission assurance but is not the canonical definition of CI/CD.

## Relationship to WI050

WI050 admission/security authority remains separate. A local verification runner does not gain permission to bypass WI050 approvals or protected-operation rules.

Where a checkpoint intersects a protected surface, the local runner reports and enforces the applicable RepoPact rules through existing authority boundaries. CI orchestration is not operator approval.

## Adoption and doctor

Existing repositories may already have GitHub Actions, GitLab, Jenkins, or another CI system.

Adoption should treat these as candidate executor adapters. It may infer a verification contract only to the extent supported by observable configuration and must use provisional/inferred provenance when semantics are uncertain.

`doctor` should be able to migrate older RepoPact assumptions that treated `.github/workflows/**` as the conceptual gate. Migration must preserve historical evidence and must not imply that a disabled hosted adapter is effective enforcement.

## Derived views

Dashboard/spec/reporting should eventually show separate dimensions such as:

- local verification contract present;
- last local invocation/evidence;
- hosted CI adapter enabled/disabled;
- hosted CD adapter enabled/disabled;
- hosted executor availability where known from actual invocation evidence;
- admission coverage;
- invocation;
- effectiveness;
- enforcement closure;
- release readiness.

A green local run must not automatically render a hosted admission boundary green.

## Immediate GitHub adapter policy

The checked-in workflows remain because they are useful optional adapters and documentation of the hosted path.

They are frozen surfaces and require human review.

Their default behavior is intentionally inert:

```text
vars.REPOPACT_GITHUB_CI != "true" -> validation job skipped
vars.REPOPACT_GITHUB_CD != "true" -> build/publish jobs skipped
```

An operator can later opt in through repository variables without rewriting the workflow files.

## Acceptance and closeout

The machine-readable acceptance criteria in `work-item.json` are binding. Closeout must prove more than a local shell script exists.

Evidence must demonstrate:

- one canonical local verification contract;
- no required semantic checks existing only in GitHub YAML;
- a complete local release-preparation path;
- explicit publication separation;
- venue-neutral evidence;
- GitHub CI default-off and opt-in behavior;
- GitHub CD default-off and opt-in behavior;
- no accidental enablement from credentials/events;
- hosted adapters invoking the same local contract;
- truthful unavailable/degraded states;
- negative enforcement cases;
- cross-platform profile/capability behavior;
- adopter migration/conformance;
- documentation for operators and agents.

## Non-goals

- Building a hosted CI service.
- Building a generalized distributed job scheduler.
- Managing provider billing, quotas, or account health.
- Replacing Fabric or another remote execution substrate.
- Pretending one workstation can provide evidence for platforms it did not execute.
- Treating a local pass as proof of a remote branch-protection gate.
- Automatically publishing on every successful local verification.

## Closeout posture

WI046 remains proposed until implementation is explicitly prioritized. Decision 0043 is already accepted and fixes the architecture/default policy now. The current GitHub workflows are optional and default off immediately; the complete local CI/CD contract remains implementation work under this item.
