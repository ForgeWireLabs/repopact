# WI050 — Opt-In Enforcement Capability Model

## Requirement

RepoPact is a standalone OSS governance product. Adoption of RepoPact must not require another application, hosted service, closed-source control plane, privileged daemon, or high-assurance execution environment.

WI050 adds governance and enforcement **capabilities**. It does not make every capability a universal adoption prerequisite.

The core distinction is:

> **RepoPact capability support and RepoPact capability requirement are separate concerns.**

RepoPact may define and ship advanced authorization, operator-control, delegation, protected-guard, adapter, and enforcement contracts while allowing each adopter to choose the assurance level it requires.

## Baseline standalone adoption

A repository may adopt RepoPact without enabling privileged pre-execution enforcement.

A baseline adopter can still use RepoPact for durable work authority, preflight, scope ownership, decisions, evidence, frozen-surface procedure, validation, doctor, audit, and other ordinary governance behavior.

Where no protected enforcement provider is configured, RepoPact must report the actual lower assurance level rather than declaring the repository invalid merely because advanced WI050 capabilities are unused.

This preserves existing adopter compatibility and keeps RepoPact useful as OSS without requiring elevated host control.

## Opt-in enforcement policy

An adopter may explicitly require stronger WI050 capabilities through RepoPact-owned policy.

For example, a repository may choose to require one of the following assurance classes for selected work, scopes, profiles, or environments:

1. instruction/advisory only;
2. session-start admission;
3. pre-action admission;
4. sandbox/process enforcement;
5. later Git/CI admission backstops in addition to, not as a substitute for, pre-execution control.

If an adopter's policy explicitly requires a capability, absence or degradation of that capability fails closed for the affected action/session.

If the adopter does not require the capability, its absence does not make RepoPact adoption itself nonconformant.

## Public capability contract

RepoPact owns the adopter-neutral public semantics required for downstream integration, including as applicable:

- canonical repository and work authority;
- operator approval request/receipt semantics;
- authorization profiles and ceilings;
- generic principal and delegation semantics;
- authorization/lease semantics;
- admission decisions and denial codes;
- protected-guard/provider protocol;
- adapter capability negotiation;
- truthful enforcement classes;
- diagnostics and conformance vectors.

These contracts are part of RepoPact because they define governance semantics. They are not tied to a particular execution product.

## Downstream enforcement providers

Other applications may opt into WI050 by implementing RepoPact's public provider/adapter contracts.

A downstream product may supply capabilities such as:

- protected service identity;
- process/session control;
- filesystem or workspace confinement;
- shell/process interception;
- network or egress enforcement;
- secret brokerage;
- distributed runner identity;
- remote execution;
- richer operator interfaces;
- fleet-wide audit or control-plane state.

Those mechanisms remain owned by the downstream application. RepoPact consumes their declared/proven capabilities through its public contract and computes or verifies the resulting assurance class.

RepoPact must not absorb downstream orchestration or host-control machinery merely because one adopter already provides it.

## No privileged-provider monoculture

The RepoPact implementation must not assume that the built-in standalone protected guard is the only or preferred high-assurance provider.

The standalone guard is a reference/provider implementation proving that RepoPact can enforce its governance independently. It may intentionally provide fewer host-control capabilities than a specialized downstream system.

Other providers can implement the same public semantics with stronger capabilities. Examples may include agent runtimes, IDE hosts, enterprise execution brokers, CI systems, local security tools, or private control planes. None becomes the semantic source of truth for RepoPact.

## Capability intersection

When RepoPact is integrated with another policy/enforcement system, effective authority must never be widened by the integration.

Conceptually:

```text
effective authority
    = RepoPact-authorized authority
    ∩ downstream-provider policy
    ∩ actually proven host capabilities
```

A downstream provider may further restrict RepoPact-authorized work. It may not use integration to manufacture RepoPact operator authority, exceed a RepoPact authorization profile, or claim a stronger enforcement class than it actually provides.

## Product boundary

RepoPact remains unaware of any specific downstream product at the governance-kernel level.

A downstream product may know how to consume RepoPact. RepoPact must not contain product-specific authority branches, required runtime dependencies, product-specific schemas, or product-specific orchestration semantics.

If a downstream integration reveals a missing generic capability that is broadly useful to RepoPact adopters, that capability may be upstreamed only as an adopter-neutral abstraction with its own justification and conformance behavior.

## Falsification tests

This requirement is violated if any of the following becomes true:

- installing or adopting RepoPact requires a separate control-plane product;
- repositories that do not opt into privileged enforcement become invalid solely for that reason;
- the standalone OSS path cannot function without a privileged daemon;
- a downstream product becomes a required RepoPact dependency;
- RepoPact duplicates scheduler, runner, secret-broker, remote-execution, or other downstream control-plane functions solely to match one integration;
- a downstream provider can widen authority beyond RepoPact's authorization;
- an adapter/provider can claim a stronger enforcement class than its actual capabilities prove;
- RepoPact's public semantics change merely because a particular downstream provider changes or disappears.
