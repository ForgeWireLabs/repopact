---
id: 0043
title: Local-first CI/CD with optional hosted adapters disabled by default
status: accepted
date: 2026-09-12
supersedes: []
---

# 0043: Local-first CI/CD with optional hosted adapters disabled by default

## Context

RepoPact has historically carried GitHub Actions workflows for governance validation and PyPI publication. Those workflows were already suspended after GitHub-hosted execution became unavailable at the account level, while the underlying validation and build operations remained possible from an operator-controlled machine.

That failure exposed an architectural problem: provider availability should not determine whether RepoPact can verify, build, package, or prepare a release. GitHub Actions is useful when available, but it should not be the canonical implementation of RepoPact CI/CD and it should not be required for a healthy local development or release path.

WI039 separated checkpoint coverage, invocation, and effectiveness. WI046 already proposed runner-neutral verification contracts. This decision tightens that direction: the canonical execution path is local-first and locally complete, while hosted systems are optional adapters.

## Decision

1. **RepoPact CI/CD is local-first.** Required verification, test, conformance, derived-artifact freshness, packaging, release verification, evidence capture, and other release gates must have an operator-controlled local execution path that does not depend on GitHub Actions or another hosted CI provider.
2. **Hosted automation is optional.** GitHub Actions, GitLab CI, Jenkins, AppVeyor, self-hosted runners, ForgeWire Fabric, or another executor may invoke the same logical contracts, but none of them is the source of truth for what RepoPact considers a verification or release gate.
3. **GitHub Actions is disabled by default.** The checked-in GitHub validation adapter runs only when repository variable `REPOPACT_GITHUB_CI` is exactly `true`. The checked-in hosted publication adapter runs only when repository variable `REPOPACT_GITHUB_CD` is exactly `true`. Missing, empty, or any other value is off.
4. **CI and CD are independently switchable.** Enabling hosted validation must not implicitly authorize hosted publication. Enabling hosted publication must be an explicit operator action separate from CI.
5. **No secret or credential presence enables hosted automation.** A token, trusted-publisher configuration, environment, release event, or workflow file existing in the repository must not turn hosted CI/CD on by implication.
6. **Local commands become canonical.** WI046 must converge verification and release behavior onto local, provider-neutral entry points. Hosted workflow YAML should become a thin adapter that invokes those same entry points rather than maintaining a second copy of the command sequence.
7. **Local evidence is first-class but truthfully scoped.** A passing local run proves that the recorded local checkpoint executed and passed. It does not by itself prove that a remote branch-protection or merge boundary is closed.
8. **Hosted evidence remains venue-specific.** When hosted execution is enabled, evidence records identify the executor/venue and may support admission-closure claims only for boundaries where coverage, invocation, and effectiveness are actually demonstrated.
9. **Release publication must have a local path.** GitHub OIDC or GitHub Trusted Publishing may remain an optional hosted publication adapter, but RepoPact release architecture must not require GitHub OIDC. Local publication credentials, if used, stay outside the repository and are supplied through an operator-controlled credential mechanism.
10. **Local release preparation is separable from publication.** Building wheels, sdists, native bundles/installers, manifests, hashes, signatures where applicable, release notes, and release evidence must be possible without publishing anything.
11. **Cross-platform local execution is part of the contract.** Verification and release profiles may contain platform-specific steps, but their semantics and evidence contract remain repository-defined rather than encoded only in one provider's YAML.
12. **Hosted provider state is not a governance dependency.** RepoPact does not query billing state or provider account status to decide correctness. Hosted availability is an execution-venue fact, not part of the repository's semantic validity.
13. **The optional hosted adapters remain frozen surfaces.** Changes to `.github/workflows/**` continue to require human review because an enabled adapter can execute privileged validation or publication actions even though it is disabled by default.
14. **Adoption must not equate workflow presence with enforcement.** Discovering `.github/workflows/**` may indicate a candidate hosted adapter, but RepoPact must separately represent whether hosted execution is enabled, available, invoked, and effective.
15. **This decision does not claim local CI/CD is fully implemented yet.** It fixes the architecture and default policy. WI046 owns the remaining implementation, migration, conformance, documentation, and evidence work.

## Immediate hosted-switch contract

The repository keeps GitHub workflows as optional adapters. Their execution guards are:

```text
REPOPACT_GITHUB_CI == "true"  -> hosted governance validation may run
REPOPACT_GITHUB_CD == "true"  -> hosted publication may run
anything else                  -> hosted job remains disabled
```

The variables are intentionally repository/operator configuration rather than committed booleans. The committed default behavior is off because an unset variable evaluates false.

## Target local shape

Exact command names remain implementation work under WI046, but the intended direction is conceptually:

```text
repopact verify <profile>
repopact release verify
repopact release build
repopact release publish
```

or an equivalent provider-neutral surface with the same separation of verification, build, and publication.

Hosted adapters should eventually reduce to calls into that local contract rather than spelling out the semantic pipeline themselves.

## Consequences

RepoPact can remain fully usable and releasable when GitHub-hosted runners are unavailable. Operators gain an explicit opt-in path for hosted automation without making hosted services part of the correctness model. The cost is that RepoPact must maintain a strong local orchestration and evidence path across supported platforms, which WI046 now treats as the primary implementation rather than a fallback.
