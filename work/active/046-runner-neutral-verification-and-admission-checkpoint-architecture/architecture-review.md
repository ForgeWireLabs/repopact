# WI046 Architecture Review: Local-First Verification and CI/CD

**Review date:** 2026-09-12  
**Reviewed base:** `a825b5184bfce9046f300e1347855281118ce6ee`  
**Architecture reviewer / coding agent:** GPT-5.6 Sol, High reasoning  
**Status:** pre-implementation architecture review

## Executive conclusion

WI046 should not build a second CI system beside the repository. It should make the repository carry a small, typed verification contract and make local execution the canonical runner for that contract. GitHub Actions, self-hosted runners, and future remote executors become adapters that invoke the same repository-defined profiles.

The implementation should converge on this shape:

```text
                    governance/verification.json
                              |
                              v
                  typed verification profiles
                              |
                    +---------+---------+
                    |                   |
                    v                   v
              local runner        hosted adapters
              (canonical)       (optional, off by default)
                    |
        +-----------+------------+
        |            |           |
        v            v           v
   validate/tests  release     evidence
   conformance     prepare     and reports
```

The most important architectural rule is that provider YAML never becomes the semantic definition of a RepoPact checkpoint. The repository record is the contract. The local runner is the reference implementation. Hosted systems choose an execution venue and provide venue-specific capabilities or credentials only.

## Current-state findings

RepoPact already has most of the local primitives required: canonical Rust-backed validation, Python regression tests, Rust workspace tests, conformance runners, dashboard and SPEC generation, frozen-surface checks, reproducible `release-build`, structural wheel/sdist inspection, fleet verification, release closeout, evidence records, and platform-specific validation developed in later work items.

The missing piece is orchestration plus one durable contract that declares which operations make up a named checkpoint.

The current GitHub workflows still duplicate that contract in provider YAML. Decision 0043 already makes those adapters default-off. WI046 should now remove the semantic duplication by reducing hosted YAML to invocations of the local contract.

`repopact/release_build.py` is already a proven local release primitive and should be wrapped and extended rather than replaced.

Decision 0042 keeps canonical RepoPact semantics in Rust. A Python orchestration layer is acceptable for external process coordination, but it must invoke canonical Rust-backed commands rather than recreate validation semantics.

## Selected architecture

### Repository-defined verification record

Add optional `governance/verification.json` with packaged `verification-profile.schema.json`. The record owns named profiles, ordered steps, platform applicability, required/optional behavior, timeouts, execution-policy metadata, and hosted-adapter defaults. It never owns secrets or provider credentials.

The record remains optional for backward compatibility. New `init` and `adopt` operations seed a minimal local-first contract. `repopact verify` fails with an actionable diagnostic when no usable verification contract exists.

### Argv arrays, not shell snippets

Command steps use argument arrays such as:

```json
{
  "id": "python-tests",
  "argv": ["{python}", "-m", "unittest", "discover", "-s", "tests", "-v"],
  "required": true,
  "timeout_seconds": 900
}
```

This avoids shell quoting differences and shell injection. `{python}` resolves to the interpreter running the orchestrator. Commands run from the repository root unless a contained relative `cwd` is declared.

### Explicit execution outcomes

Step states:

```text
passed
failed
unavailable
skipped
error
```

Profile states:

```text
pass
fail
incomplete
error
```

Exit semantics:

```text
0 = pass
1 = verification failure
2 = incomplete/unavailable/configuration/runner error
```

Human and deterministic JSON views represent the same state.

### Evidence is explicit

Ordinary verification must not mutate the repository just because a developer ran a check. Explicit evidence recording ties a concrete local run to a work item and records profile, executor class, platform/runtime, Git commit/tree when available, command results, artifacts/hashes, and overall result.

A local pass remains local evidence and does not claim remote merge enforcement.

### Verification, build, and publication are separate

Target public surface:

```text
repopact verify <profile>
repopact release verify
repopact release build --outdir <dir>
repopact release publish --dist <dir> --confirm-publish
```

The current `release-build` command remains a compatibility alias during migration. Publication requires explicit intent and uses credentials supplied outside repository records.

### Hosted systems are thin adapters

GitHub validation remains guarded by `REPOPACT_GITHUB_CI == "true"` and should reduce to setup/install plus `repopact verify ci`.

GitHub release execution remains guarded independently by `REPOPACT_GITHUB_CD == "true"` and should consume the same local release contract. GitHub OIDC can remain as venue-specific credential transport for the optional final upload step.

### Platform capability is truthful

Profiles may declare platform applicability, but a host records only what actually ran. A full or release view cannot fabricate evidence for another platform. Missing required capabilities produce an incomplete result rather than a false pass.

### Change-aware optimization is conservative

The first implementation runs the registered profile directly. Future Git-diff or graph-aware selection may narrow work only when the narrower set is proven sufficient. Otherwise the broader profile runs.

### Adoption semantics

Workflow discovery becomes a signal that a hosted adapter exists, not proof that it is enabled, available, invoked, effective, or a closed merge gate. Historical records remain history; new adoption must stop fabricating enforcement from workflow presence alone.

### Validator integration

When `governance/verification.json` is present, the canonical Rust validator validates its schema and semantic relationships. The retained Python validator validates the same structure as an explicit comparator/legacy workflow. The record is included in the canonical repository snapshot so clients do not recrawl independently.

## Initial RepoPact profiles

`quick` provides fast local confidence. `ci` is the local equivalent of the hosted governance adapter and includes canonical validation, Python tests, conformance, Rust workspace tests, and generated-artifact/frozen-surface checks where supported. `release` extends release readiness without publishing artifacts as a side effect.

## Implementation sequence

Phase A adds the contract, schema, canonical indexing/validation, local runner, init/adopt seed behavior, and execution tests.

Phase B exposes `verify`, grouped release commands, explicit evidence, local release manifests, and explicit publication.

Phase C reduces hosted workflows to thin adapters and reconciles adopt/doctor/reporting.

Phase D proves the path on operator-owned hardware, including negative cases and exact evidence records.

## Explicit non-goals

WI046 does not build a generalized DAG scheduler, daemon, runner fleet, provider billing client, secret store, or ForgeWire Fabric integration. It does not make local success equivalent to protected-branch enforcement and does not weaken WI050 authority.

## Architecture risks

| Risk | Mitigation |
| --- | --- |
| repository config becomes arbitrary shell execution | argv arrays only, no shell evaluation |
| GitHub YAML and local profiles drift | hosted adapters invoke named local profiles |
| profile claims unsupported platform coverage | explicit platform/capability state and incomplete result |
| verification silently mutates governance | no evidence write unless explicitly requested |
| release verification accidentally publishes | verify/build/publish remain separate and publish requires explicit intent |
| existing adopters break because the record is absent | record is optional for conformance; init/adopt seed it going forward |
| Python orchestration becomes a second semantic validator | canonical governance semantics continue through Rust-backed operations |
| provider availability becomes correctness state | provider state remains venue evidence, not repository semantic validity |
