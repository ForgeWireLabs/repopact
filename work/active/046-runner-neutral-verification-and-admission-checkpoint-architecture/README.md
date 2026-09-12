# 046: Local-First Verification, CI/CD, and Optional Hosted Adapters

> **Status**: Active  
> **Owners**: governance-owner (lead); tooling-owner, evidence-owner, docs-owner, and work-coordinator affected.  
> **Depends on**: WI039 (completed enforcement-closure field study).  
> **Architecture decision**: Decision 0043, accepted 2026-09-12.  
> **Architecture review**: [`architecture-review.md`](architecture-review.md).

## Intent

Make RepoPact verification and release execution local-first and locally complete. GitHub Actions and other hosted systems remain optional executor adapters, disabled by default, and must not define RepoPact's verification or release semantics.

The canonical direction is:

```text
                  repository-defined contracts
                            |
                            v
                   canonical local runner
                            |
             +--------------+--------------+
             |                             |
             v                             v
       local verification          local release path
             |                             |
             +--------------+--------------+
                            |
                            v
                 repository-native evidence

optional hosted adapters -> invoke the same contracts
```

## Binding rules

1. Required verification, test, conformance, generated-artifact freshness, packaging, release verification, and evidence paths must work without GitHub Actions.
2. GitHub validation runs only when `REPOPACT_GITHUB_CI == "true"`.
3. GitHub publication runs only when `REPOPACT_GITHUB_CD == "true"`.
4. CI and CD are independently opt-in. Secrets, environments, workflow events, or workflow-file presence do not enable either path implicitly.
5. Provider YAML is an adapter. The repository-owned verification contract is the semantic source of truth.
6. A local pass is concrete local evidence, not proof that a remote branch or merge boundary is closed.
7. Verification, artifact build, artifact verification, and publication remain separable operations. Publication always requires explicit operator intent.
8. Publication credentials stay outside governed repository records.
9. Cross-platform evidence states what actually ran. One machine may not fabricate evidence for a platform it did not execute.
10. WI050 approval and admission authority remain separate from CI/CD success.

## Implementation plan

### Phase A: contract and local runner

- add an optional typed `governance/verification.json` record;
- add the packaged structural schema;
- include the record in the canonical Rust repository snapshot and validator;
- retain Python validation only as the named comparator/retained workflow path;
- implement a provider-neutral local profile runner using argv arrays rather than shell strings;
- expose deterministic human and JSON result views with stable exit semantics;
- seed a minimal local-first contract from `init` and `adopt`;
- add positive and negative execution tests.

### Phase B: release surface

- expose `repopact verify <profile>`;
- introduce grouped local release verify/build/publish operations while retaining `release-build` compatibility;
- keep reproducible `release_build.py` as the proven artifact-construction primitive;
- add explicit evidence recording and release manifest/hash output;
- require explicit confirmation for publication and use only external operator credentials.

### Phase C: hosted adapters and migration

- reduce GitHub governance YAML to the same local `verify` contract;
- reduce hosted release preparation to the same local release contract;
- retain GitHub OIDC only as a venue-specific optional publication credential transport;
- reconcile `adopt` and `doctor` so workflow presence is not mistaken for effective enforcement;
- expose local/hosted state truthfully in derived views.

### Phase D: proof and closeout

Closeout must be proven on operator-owned hardware. GitHub-hosted execution is not a prerequisite. Evidence must include local CI, release preparation, negative publication cases, hosted-switch behavior, cross-platform capability truthfulness, and proof that no required semantic gate exists only in provider YAML.

## Frozen-surface approval

The operator explicitly directed WI046 implementation after directing RepoPact to become local-first with GitHub CI/CD default off. That direction authorizes the WI046-scoped changes required to:

- `repopact/schemas/**` for the verification contract;
- `.github/workflows/**` for thin optional hosted adapters.

This approval does not weaken INV-6 and does not extend to unrelated frozen changes.

## Relationship to WI032

WI032 remains an optional remote/public enforcement concern. It is no longer a prerequisite for RepoPact to be fully verifiable and releasable locally. A future remote gate can strengthen admission assurance, but remote provider availability does not define RepoPact correctness.

## Relationship to WI050

A successful verification profile does not grant authority. It cannot activate work, approve a frozen mutation, mint an operator receipt, bypass protected execution, or convert verification evidence into authorization.

## Acceptance

The machine-readable acceptance criteria in `work-item.json` are binding. Completion requires concrete local evidence and negative cases, not merely the presence of a runner or configuration file.
