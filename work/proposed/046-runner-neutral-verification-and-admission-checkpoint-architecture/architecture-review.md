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

### Existing local primitives are already strong

RepoPact already has most of the operations a local pipeline needs:

- canonical Rust-backed `repopact validate`;
- Python regression tests;
- Rust workspace tests;
- published conformance runners;
- dashboard and SPEC generation;
- frozen-surface checking;
- reproducible `release-build` from two independent Git exports;
- structural wheel/sdist inspection;
- fleet verification and release closeout;
- evidence-run records;
- platform-specific installer and Workbench validation developed in later WIs.

The missing piece is orchestration and one durable contract that says which of those operations make up a named checkpoint.

### GitHub workflow YAML currently duplicates the contract

`governance.yml` spells out validation, unit tests, conformance, dashboard generation, SPEC generation, a Git diff, and frozen-surface reporting. `release.yml` independently spells out build and publication behavior.

Decision 0043 already makes both hosted workflows default-off. WI046 should now remove the remaining semantic duplication by reducing hosted YAML to calls into local profile/release operations.

### `release-build` is already a useful local release primitive

`repopact/release_build.py` already enforces a clean worktree, resolves a commit, creates two independent `git archive` exports, builds through Maturin, normalizes nondeterministic metadata, structurally inspects wheel/sdist contents, requires byte-identical hashes, and copies only the proven artifacts into the requested output directory.

WI046 should wrap and extend this primitive, not replace it.

### Canonical semantic authority remains Rust

Decision 0042 moved proven validation and governance semantics into the Rust engine. The local CI orchestrator may remain a Python retained workflow because it coordinates external tools and platform capabilities, but it must call canonical Rust-backed commands for migrated RepoPact semantics rather than recreating validation in Python.

## Decision 1: repository-defined verification record

Add an optional durable record:

```text
governance/verification.json
```

with canonical packaged schema:

```text
verification-profile.schema.json
```

The record is optional for compatibility with existing repositories. New `init` and `adopt` operations should seed a minimal local-first contract. `repopact verify` requires a usable record and fails with an actionable diagnostic if one is absent.

The record owns:

- named profiles;
- ordered steps;
- platform applicability;
- required/optional capability semantics;
- timeouts;
- execution policy metadata;
- hosted-adapter defaults.

It does not own secrets or provider credentials.

## Decision 2: argv arrays, never shell snippets

Command steps use explicit argument arrays rather than shell strings.

Example:

```json
{
  "id": "python-tests",
  "argv": ["{python}", "-m", "unittest", "discover", "-s", "tests", "-v"],
  "required": true,
  "timeout_seconds": 900
}
```

This avoids platform-specific quoting, accidental shell interpretation, and shell injection through configuration. `{python}` resolves to the interpreter running the orchestrator. Other executables are resolved normally through the local environment.

Commands run with the repository root as the default working directory. A relative `cwd` may narrow the working directory but may not escape the repository.

## Decision 3: explicit execution outcomes

A step result should distinguish:

```text
passed       command executed and returned zero
failed       command executed and returned nonzero
unavailable  required platform/tool capability was not present
skipped      step did not apply to this platform/profile
error        runner/configuration failure prevented a valid execution
```

A profile result should distinguish at least:

```text
pass        every required applicable step passed
fail        at least one required applicable step failed
incomplete  no required step failed, but required capability was unavailable
error       profile/configuration/runner failure invalidated the run
```

Exit semantics:

```text
0 = pass
1 = verification failure
2 = incomplete/unavailable/configuration/runner error
```

Human text and deterministic JSON views must represent the same state.

## Decision 4: local evidence is explicit, not automatic repository churn

Ordinary local verification should not mutate the repository merely because a developer ran a check.

The runner should support explicit evidence recording, tied to a work item, that writes a normal `evidence/runs/*.json` record. The record must identify:

- profile;
- executor class (`local` for the reference runner);
- platform and Python/runtime information;
- Git commit/tree when available;
- ordered command results;
- artifacts/hashes where applicable;
- overall result;
- concrete provenance for an actually executed local run.

A local pass remains local evidence. It does not claim remote merge enforcement.

## Decision 5: verification, release build, artifact verification, and publication stay separate

The local product surface should converge toward:

```text
repopact verify <profile>
repopact release verify
repopact release build --outdir <dir>
repopact release publish --dist <dir> --confirm-publish
```

The existing `release-build` command remains as a compatibility alias while the new grouped release surface is introduced.

`release build` may invoke the release verification profile first or require an explicit option to skip it only for development/debugging. The default release path should be conservative.

Publication is a separate command and must require explicit operator intent. It may use Twine or an equivalent local client, but credentials are supplied by the operator environment/keyring and are never written into RepoPact records or config.

## Decision 6: GitHub Actions becomes a thin optional adapter

The hosted validation adapter remains guarded by:

```text
REPOPACT_GITHUB_CI == "true"
```

and should reduce to setup/install plus:

```text
repopact verify ci
```

The hosted release adapter remains guarded independently by:

```text
REPOPACT_GITHUB_CD == "true"
```

and should use the same local release build/verification contract. GitHub OIDC may remain venue-specific for the final upload step because credential transport is an executor concern, not release semantics.

No workflow event, secret, environment, or trusted-publisher configuration implicitly enables either hosted adapter.

## Decision 7: profile model is capability-aware, not fake cross-platform proof

Steps may declare platform applicability using normalized values such as:

```text
windows
linux
macos
android
ios
```

The reference workstation runner can directly execute desktop-host checks. Mobile/device checks may require a separately declared capability or external harness. A Windows run cannot claim Linux/macOS/mobile evidence simply because the same profile names those steps.

A `release` or `full` profile with missing required capability returns `incomplete`, not `pass`.

## Decision 8: change-aware optimization is advisory

Future profile execution may select a proven-safe subset based on Git diff or repository graph information, but the default implementation should run the declared profile exactly.

Any optimization must be conservative. If the runner cannot prove that a narrower subset is sufficient, it runs the broader profile. Optimization may reduce work, never authority.

## Decision 9: adoption semantics change

RepoPact adoption must stop treating discovery of `.github/workflows/**` as proof of an effective universal CI gate.

For a brownfield repository:

1. discovered hosted workflows are recorded as existing adapter signals;
2. a local verification record is seeded independently;
3. workflow presence does not imply enabled, available, invoked, or effective enforcement;
4. historical policy records are preserved rather than rewritten to fabricate a local-first past;
5. doctor may recommend migration from older workflow-centric language without destroying history.

## Decision 10: validator integration is additive

`governance/verification.json` is optional for compatibility. When present:

- the canonical Rust validator validates its schema and cross-field semantics;
- the legacy Python comparator validates the same schema;
- source/snapshot accounting includes the record;
- invalid profile definitions fail repository validation.

This preserves Decision 0003 and Decision 0042 rather than leaving CI configuration outside RepoPact's own conformance model.

## Initial profile set for RepoPact itself

The self-hosting repository should initially define:

### `quick`

Fast governance confidence for ordinary edits:

- canonical validation;
- focused Python tests selected only if a stable subset already exists, otherwise the full Python suite is preferred over an unproven selector.

### `ci`

The local equivalent of the hosted governance adapter:

- canonical validation;
- Python unit/regression suite;
- published conformance suite;
- Rust workspace tests;
- generated dashboard/SPEC freshness proof without accepting uncommitted drift;
- frozen-surface report when a usable base is available.

### `release`

Release-readiness profile:

- everything required by `ci`;
- packaging/release-specific regression checks;
- release surface/version checks already owned by validation;
- release-build prerequisites/toolchain checks.

Artifact construction remains a separate `release build` phase so running the release profile does not create or publish artifacts.

## Implementation sequence

### Phase A: contract and runner

1. activate WI046;
2. add the verification schema and self-hosting record;
3. add a local runner with deterministic JSON output and exit semantics;
4. add canonical Rust and legacy Python validation for the record;
5. seed a minimal record from init/adopt;
6. add focused tests for parsing, platform gating, missing tools, failure, incomplete capability, and no-shell execution.

### Phase B: product commands and release path

1. expose `verify` through the public CLI;
2. introduce grouped `release verify/build/publish` commands while preserving `release-build` compatibility;
3. create deterministic release manifest/hash output;
4. add explicit evidence recording;
5. add local publication with explicit confirmation and external credentials.

### Phase C: hosted adapters and migration

1. reduce `governance.yml` to `repopact verify ci` after environment setup;
2. reduce hosted release preparation to local release commands;
3. keep OIDC upload as a venue-specific final adapter step;
4. reconcile adopt/doctor wording and generated views;
5. add conformance cases for hosted switches and local/remote truth separation.

### Phase D: closeout proof

Run from operator-owned hardware because GitHub-hosted CI is not part of the proof requirement:

- Windows local CI profile;
- Linux local CI profile;
- release preparation from a clean commit;
- deterministic artifact comparison;
- publication dry/negative cases;
- hosted switches unset/off/on parsing tests without requiring paid hosted execution;
- negative tests proving local pass does not claim remote closure;
- exact evidence manifests linked to WI046.

## Explicit non-goals

WI046 does not:

- build a generalized DAG scheduler;
- add a daemon or runner fleet;
- import ForgeWire Fabric;
- manage provider billing or account state;
- store publication passwords/tokens;
- make every adopter run RepoPact's own Python/Rust test commands;
- make local success equivalent to protected-branch enforcement;
- weaken WI050 approval/admission boundaries.

## Architecture risks

| Risk | Mitigation |
| --- | --- |
| repository config becomes arbitrary shell execution | argv arrays only, no shell evaluation |
| GitHub YAML and local profiles drift | hosted adapter invokes named local profile |
| profile claims unsupported platform coverage | explicit platform/capability state and `incomplete` result |
| verification command silently mutates governance | no evidence write unless explicitly requested |
| release verification accidentally publishes | build/verify/publish are separate commands; publish requires explicit confirmation |
| existing adopters break because new record is absent | record is optional for conformance; init/adopt seed it going forward |
| Python orchestrator becomes second semantic validator | canonical RepoPact semantics continue through Rust-backed commands |
| provider availability becomes correctness state | provider state remains venue evidence, not repository semantic validity |

## Activation recommendation

Activate WI046 now. Phase A and Phase B are appropriate before public release because they remove a real external dependency from RepoPact's own release path and make the product's governance story consistent with its repository-native thesis. Remote enforcement restoration under WI032 remains optional and separate.