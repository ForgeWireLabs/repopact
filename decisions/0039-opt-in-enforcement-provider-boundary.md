---
id: 0039
title: Opt-in enforcement capability and provider boundary
status: accepted
date: 2026-09-03
supersedes: []
---

# 0039: Opt-in enforcement capability and provider boundary

## Context

WI050 Decision 0038 establishes a vendor-neutral admission policy core,
protected guard, and replaceable adapters. Its implementation must also remain
usable as a standalone OSS governance product. Capability support therefore
cannot be confused with a requirement that every adopter install a provider or
privileged host boundary.

## Decision

The semantic boundary is layered:

```text
RepoPact core policy/state/authority
        |
        v
optional adopter-owned EnforcementProvider contract
        |
        +--> RepoPact NativeGuardClient/reference provider
        +--> arbitrary downstream provider
```

1. No admission policy, and an adopter-owned policy with `enabled=false`, are
   valid standalone RepoPact modes. They report `instruction-only` and do not
   require a lease, guard, privileged daemon, or downstream application.
2. `enabled=true` opts a repository into the policy's
   `minimum_enforcement`. The requirement is unsatisfied, and affected actions
   fail closed, when no provider and adapter together prove that class. A
   `degraded` failure mode changes diagnostics only; it never upgrades a weak
   provider into the required class.
3. `EnforcementProvider` is an adopter-neutral runtime SPI with health,
   discovery, authorization, check, delegation, and revoke operations.
   `NativeGuardClient` implements that SPI; it does not define or own it.
4. Adapters and providers advertise separate capability classes. RepoPact
   computes effective assurance as their intersection, so a provider may narrow
   authority or capability but may not widen a RepoPact lease, profile, path,
   scope, expiry, repository/work-item binding, or operator authority.
5. The built-in guard is a reference provider. A downstream implementation may
   satisfy the public contract without becoming a RepoPact dependency, semantic
   kernel special case, scheduler, runner, or control plane.

This record clarifies implementation boundaries without superseding or
silently rewriting the historical meaning of Decision 0038.

## Alternatives considered

* Require the built-in guard for every adoption: rejected because it would turn
  an optional enforcement capability into a universal product prerequisite.
* Treat `degraded` as permission to run below an explicit minimum: rejected
  because it would make the declared assurance requirement untruthful.
* Make a downstream product the canonical provider: rejected because provider
  mechanisms and orchestration belong to adopters/downstream systems, not the
  RepoPact semantic kernel.
* Combine adapter and provider claims into one boolean: rejected because it
  hides which layer is weaker and permits accidental assurance widening.

## Consequences

Standalone adopters remain independently conformant and existing pre-WI050
repositories remain compatible. Opted-in adopters receive deterministic,
fail-closed diagnostics when provider coverage is absent or lost. The base
package depends on `jsonschema` only; cryptographic enforcement support is an
optional `enforcement` extra. Platform-specific installation and privileged
service work remain separate backend concerns and are not performed by this
clarification.

