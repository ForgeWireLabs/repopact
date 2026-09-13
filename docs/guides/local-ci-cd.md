# Local-first verification and release workflow

> **Diataxis mode:** How-to guide.

RepoPact does not require GitHub Actions to verify or prepare a release. The repository owns the verification contract in `governance/verification.json`; the local runner is the reference execution path. Hosted CI/CD systems are optional adapters.

## Run local verification

Use the profile that matches the work you are doing:

```console
repopact verify quick
repopact verify ci
repopact verify release
```

For machine-readable output:

```console
repopact verify ci --json
```

The local runner uses argument arrays and does not evaluate profile commands through a shell. Required steps can report `passed`, `failed`, `unavailable`, `skipped`, or `error`. Overall profile states are `pass`, `fail`, `incomplete`, or `error`.

Exit codes are:

```text
0  profile passed
1  a required verification step failed
2  required coverage/capability unavailable, invalid configuration, or runner error
```

A local pass proves that the named local profile ran successfully on the reported host. It does not prove that a remote merge gate, branch protection rule, or hosted checkpoint is enabled or effective.

## Repository verification contract

A repository may define `governance/verification.json`. The record is schema-backed and contains named ordered profiles. Command steps use argv arrays such as:

```json
{
  "id": "tests",
  "argv": ["{python}", "-m", "unittest", "discover", "-s", "tests", "-v"],
  "required": true,
  "timeout_seconds": 900
}
```

Supported exact placeholders are:

- `{python}`: the Python interpreter running RepoPact;
- `{repopact}`: the installed RepoPact compatibility command through that Python environment;
- `{root}`: the canonical repository root supplied to the profile runner.

A step may declare platform applicability and an executable capability. A repository-relative `cwd` is allowed, but it may not escape the repository.

`repopact init` seeds a minimal local-first verification contract. Existing repositories without this optional record remain valid, but `repopact verify` requires a contract to run.

## Choose truthful coverage semantics

A profile may set one of two coverage modes:

```json
{
  "coverage": "host"
}
```

`host` means the profile proves the required checks applicable to the executing host. A required Windows-only step may be reported as `skipped` on Linux without fabricating Windows evidence. The report still records that not every declared step executed.

```json
{
  "coverage": "complete",
  "required_platforms": ["windows", "linux", "macos"]
}
```

`complete` is stricter and must declare a non-empty `required_platforms` list naming exactly the platforms that constitute a complete run — RepoPact rejects a `complete` profile with no `required_platforms` rather than letting it silently default to whatever happened to run on one host. A single local invocation only ever proves the host it actually ran on: `satisfied` is only true when this run's platform covers the *entire* declared `required_platforms` set (in practice, when the set names exactly this host). Any other declared platform is reported as a `missing_platforms` entry rather than assumed to have run elsewhere, so `coverage.satisfied` cannot become true merely because a step happened not to declare any per-step `platforms` at all. A required platform-specific step that is not applicable on the current host, or a required capability that is unavailable, also makes the profile `incomplete` rather than `pass`.

RepoPact's upstream `ci` profile uses host coverage. Its `release` profile uses complete coverage with `required_platforms: ["windows", "linux", "macos"]` (RepoPact's current desktop wheel targets; Android/iOS are not required merely because they are known platform names) so release readiness cannot silently treat missing required capability/platform evidence as success. In practice this means a single-host `release` run is expected to report `incomplete`, not `pass`, until each required platform's own run contributes its evidence — that is the intended, honest result, not a defect.

Machine-readable output includes a `coverage` object with required-step counts, declared platforms, the profile's `required_platforms`, any `missing_platforms`, capability states, whether coverage is satisfied, and whether every declared required step actually executed.

## Record an actual local invocation as evidence

Verification can write a normal immutable RepoPact evidence run for an existing work item:

```console
repopact verify ci --evidence-work-item 046
```

For deterministic scripting you may supply the record id explicitly:

```console
repopact verify ci \
  --evidence-work-item 046 \
  --evidence-id 20260912-wi046-local-ci \
  --json
```

The evidence record captures the profile, executor (`local`), platform, aggregate coverage, per-step states, executed command exit codes, and best-effort candidate Git commit/tree/dirty identity. It uses concrete provenance because it describes an invocation that actually happened.

Evidence recording is fail-closed:

- the referenced work item must already exist;
- an existing evidence id is never overwritten;
- the evidence path is repository-native under `evidence/runs/`;
- dashboard refresh is part of the write, and a refresh failure rolls the new evidence record back;
- a recorded local pass still does not claim remote admission closure.

Use evidence recording only for a run you actually intend to preserve. Ordinary exploratory `repopact verify` calls remain read-only.

## Prepare a release locally

Release verification, artifact construction, artifact verification, and publication are intentionally separate operations.

Verify release readiness without building or publishing:

```console
repopact release verify
```

Build reproducible wheel and sdist artifacts from a clean committed tree:

```console
repopact release build --outdir ./release-out
```

The build runs the `release` verification profile first by default, then uses RepoPact's existing double-build reproducibility check. The output directory includes package artifacts plus:

```text
release-manifest.json
release-build-report.json
```

Verify the manifest and artifact hashes later without rebuilding:

```console
repopact release inspect --dist ./release-out
```

`release build` does not publish anything.

## Publish explicitly from a local machine

Publication is an explicit operator action:

```console
repopact release publish --dist ./release-out --confirm-publish
```

RepoPact verifies the release manifest and artifact hashes before starting Twine. Publication credentials are supplied through the operator's external Twine/environment/keyring configuration. RepoPact does not write publication credentials into the repository or into `governance/verification.json`.

To exercise the publication boundary without contacting a provider:

```console
repopact release publish --dist ./release-out --confirm-publish --dry-run --json
```

## Optional GitHub validation

GitHub-hosted validation is disabled by default. The checked-in workflow runs only when the repository variable below is exactly `true`:

```text
REPOPACT_GITHUB_CI=true
```

When enabled, the GitHub workflow installs RepoPact and invokes the same `ci` profile used locally. It does not maintain a second semantic list of checks.

Unset, empty, `false`, or any other value leaves hosted CI off.

## Optional GitHub publication

GitHub-hosted release build/publication is controlled independently:

```text
REPOPACT_GITHUB_CD=true
```

Enabling CI does not enable CD. A GitHub release event, configured environment, secret, or trusted publisher does not turn hosted CD on by itself.

When hosted CD is enabled, release verification and artifact construction use the same local release path. GitHub OIDC is only a venue-specific credential mechanism for the final optional upload.

## Cross-platform evidence

One host only proves what actually ran there. A Windows verification run is not Linux or macOS evidence, and a desktop run is not Android or iOS evidence. Profile output records the executing platform, declared platform applicability, capability availability, and aggregate coverage.

Release claims that require multiple platforms should aggregate concrete evidence from those actual platforms rather than treating one green profile as universal proof.

## Relationship to authorization

Verification is evidence, not authority. A successful profile cannot:

- activate or complete a work item by itself;
- waive acceptance criteria;
- approve a frozen-surface change;
- mint an operator approval receipt;
- bypass WI050 admission or protected execution;
- prove that a remote admission boundary is closed.

Those remain governed by their existing RepoPact authority and evidence contracts.
