# 050 — Pre-Execution Agent Work Admission and Preflight Enforcement

> **Status**: Active (protected enforcement substrate phase; AC-14, AC-15, AC-16, and AC-18 remain pending)
> **Owners**: governance-owner (lead); tooling-owner and docs-owner affected.
> **Depends on**: WI023 mandatory preflight and completed WI049 baseline reconciliation.

## Architecture-phase disposition

Decision [0038](../../../decisions/0038-pre-execution-admission-architecture.md)
accepts a vendor-neutral policy core, protected local guard/service, signed
operator approval and short-lived leases, truthful adapter capability
negotiation, canonical repository registration, tiered profiles, and
non-escalating generic delegation. The supporting records are:

* [architecture](architecture.md) - state machine, admission contract,
  bootstrap, approval, trust, profiles, delegation, repair, compatibility,
  boundaries, and falsification gates;
* [threat model](threat-model.md) - f2c80b7/WI049 field capture, bypass matrix,
  enforcement taxonomy, ecosystem observations, and implementation tests;
* [integration and platform architecture](integration-platform.md) - guard
  alternatives, adapter SPI, identity, and Windows/Linux/macOS backend
  directions;
* [frozen-surface proposal](frozen-surface-proposal.md) - exact required,
  optional, and not-needed INV-6 paths for the later implementation pass;
* [field capture 018](../../../research/captures/018-wi050-pre-execution-admission-field-evidence.md).

The implementation pass adds six approved schemas, a pure policy/crypto core,
protected registration and guard facade, CLI/API operations, truthful adapter
SPI, OS backend interfaces, and executable admission tests. WI044, WI046, and
WI047 remain outside this item.

The correction pass adds focused pre-remediation regressions and closes the
identified lease, receipt, bootstrap-output, revocation, linked-worktree, and
guard-health gaps without changing the approved frozen surface. Mutation,
process, repair, and frozen actions now require an operator-derived lease whose
repository, work item, principal, session, profile, mode, scopes, paths,
capabilities, lineage, base, expiry, and revocation state are rechecked before
execution. The reference filesystem backend deliberately reports lower
assurance until a host-managed process/path boundary exists.

The protected-substrate phase removes caller-asserted guard protection and adds
backend-owned attestation, operator-gated host installation flows, protected
Windows service/runtime and state paths, authenticated local IPC, and a
cross-platform platform-conformance harness. Until an elevated operator
installs and proves the native Windows service, the native backend remains
`not-covered`; testing-only attestations are never production evidence.

## Intent

RepoPact already requires a work item to exist before implementation begins, and an `active`
work item is the repository's authorization for design or implementation. Today those rules are
primarily recorded and validated as repository state. An autonomous coding agent can still make a
first source mutation before it has gone through the required workflow if the execution environment
does not mechanically consult RepoPact before allowing that action.

The post-3.0.2 `f2c80b7` / WI049 incident is direct field evidence: the patch itself could be
reviewed later, but RepoPact had no pre-execution boundary that forced the correct governance flow
before the first runtime edit.

WI050 therefore designs and builds a **RepoPact-native work-admission and enforcement plane**.
RepoPact owns the policy engine, authorization semantics, protected guard, canonical adopted-repo
registration, adapter/SPI contract, capability negotiation, diagnostics, conformance, and failure
semantics. External agent products, IDEs, chat products, tool protocols, and orchestrators are
integration surfaces only.

## Product boundary

RepoPact must not become a thin wrapper around any frontier-lab ecosystem.

Useful capabilities discovered in Codex, ChatGPT, Claude/Anthropic, Cursor, MCP hosts, IDEs, or
other runtimes may inspire or supply an enforcement substrate, but the generic capability is
internalized behind RepoPact-owned abstractions. If a vendor changes or disappears, RepoPact's
policy model remains valid and another adapter can satisfy the same public contract.

Likewise, RepoPact must remain cleanly separated from ForgeWire Fabric and any other closed/internal
product. RepoPact contains no Fabric-specific authority, dependency, or privileged integration.
If Fabric later uses RepoPact admission, Fabric implements that integration downstream against
RepoPact's public adapter/SPI contract. Any upstream change must be adopter-neutral and independently
justified for the OSS governance product.

## Lifecycle placement

```text
Task arrives
    |
    v
RepoPact work admission / preflight enforcement      <-- WI050
    |
    | authorized mutation only
    v
Agent/runtime performs work
    |
    v
Typed completion/evidence semantics                  <-- WI044 when canonical
    |
    v
Documentation closure                                <-- WI047 when canonical
    |
    v
Verification / promotion admission                   <-- WI046 / H14
    |
    v
merge / release / deployment
```

WI050 answers:

> May this agent/runtime begin or continue mutating this adopted repository for this work?

WI046 answers a later question:

> Did the required verification run and actually prevent an invalid promotion?

CI and Git hooks remain useful backstops but are not the primary enforcement boundary for WI050.

## RepoPact is the authority

The guard consumes canonical RepoPact state rather than inventing a second authority system.
Candidate inputs include:

- applicable `AGENTS.md` contracts and repository invariants;
- work-item lifecycle (`proposed` is not implementation authority; `active` is);
- mandatory preflight from decision 0021 / WI023;
- owner and affected scopes;
- dependency state;
- frozen-surface requirements and explicit operator approval;
- provenance rules;
- repository validity;
- post-release source/artifact development identity under decision 0032;
- future optional requirements from other RepoPact work only after those requirements become
  canonical.

A transient session authorization is only an enforcement capability derived from those records.
It never becomes durable project authority.

## Candidate RepoPact-native architecture

```text
                    canonical RepoPact records
                              |
                              v
                    RepoPact admission policy
                              |
                              v
                     RepoPact protected guard
                              |
                    public adapter / SPI contract
             +----------------+----------------+
             |                |                |
        host adapter     protocol adapter   generic launcher /
                                             sandbox adapter
```

A likely flow to evaluate is:

```text
repopact begin / authorize --work-item NNN
        |
        | validate canonical workflow prerequisites
        v
short-lived RepoPact authorization
  - canonical repository identity
  - RepoPact root identity
  - work-item id
  - base repository / authority state
  - allowed scopes / paths
  - frozen approval state
  - adapter / session identity
  - expiry / invalidation metadata
        |
        v
protected RepoPact guard
        |
        v
adapter asks before mutation
        |
        +-- allow
        +-- deny before execution
```

The exact token/lease format, protected storage, signing needs, lifetime, and invalidation rules are
architecture questions for this WI.

## Adapter SPI and capability negotiation

RepoPact should expose a public adapter contract rather than hard-code individual vendors into the
kernel. An adapter declares what it can actually enforce. Candidate capabilities include:

- canonical repository/session discovery;
- read-only orientation;
- session-start admission;
- pre-action interception;
- mutation-intent/path reporting;
- path/scope confinement;
- process/shell confinement;
- protected host configuration;
- operator-approval handoff;
- fail-closed guard-health semantics;
- subagent/session propagation.

RepoPact computes the truthful enforcement class from those capabilities and the adopted
repository's policy. An adapter cannot simply call itself `enforced` while omitting required
substrate.

Reference adapter families may include Codex/ChatGPT coding surfaces, Claude/Anthropic coding
surfaces, Cursor, MCP-capable hosts, generic local agents, IDE integrations, and future runtimes.
These are adapters and compatibility tests, not governance dependencies.

MCP is specifically treated as a protocol adapter, not as universal enforcement. Routing one tool
through MCP does not prove that native shell/filesystem paths are intercepted unless the host or a
sandbox also constrains them.

## Enforcement levels

RepoPact must report what is actually enforced:

1. **Instruction-only** — context/guidance; no mechanical first-write prevention.
2. **Session-start** — mutating session cannot start without RepoPact admission.
3. **Pre-action** — each mutating action passes through the RepoPact guard before execution.
4. **Sandbox/process boundary** — filesystem/process effects are constrained independently of agent
   cooperation.
5. **Git/CI backstop** — later detection/admission protection, not first-write enforcement.

A configured adapter advertised as enforced must fail closed if its protected guard is missing,
broken, stale, or tampered.

## Protected enforcement root

Repo-local `AGENTS.md`, `CLAUDE.md`, `.claude/`, `.codex/`, `.cursor/`, MCP configuration, helper
scripts, hooks, and links can aid discovery, but none may be the sole enforcement root.

For enforced admission, the guard must be anchored outside ordinary agent-writable repository state
or protected by a host-managed/OS boundary the gated process cannot modify. The guard binds adopted
repositories by canonical identity and RepoPact root, not by prompt text or mutable convenience
paths.

Changing cwd, entering a linked worktree, starting a subagent, using a shell/Python write, or
removing repo-local adapter configuration must not create an escape route.

## Operator-controlled authority transition

The same gated agent must not manufacture its own implementation authority.

Read-only orientation and a bounded bootstrap path may permit creating or updating a **proposed**
work record and requesting authorization. They must not permit the agent to make itself authorized
by:

- changing `proposed` to `active`;
- fabricating operator approval;
- fabricating frozen-surface approval;
- replaying an authorization from another repository/work item/session;
- rewriting protected guard registration or policy.

The authority transition must use an operator-controlled channel or an equivalent proof the gated
agent cannot forge.

## Admission checks

The final contract may evolve, but an ordinary mutation-capable session should be denied for cases
such as:

- no governed work item;
- proposed-only work;
- missing/invalid mandatory preflight;
- blocked/completed/deferred lifecycle state;
- unresolved dependency authority;
- attempted path outside authorized scope;
- frozen path without explicit approval;
- invalid repository baseline without an explicit repair mode;
- missing required post-release development identity;
- stale authorization after material authority drift;
- authorization for another repository/work item/session.

A controlled repair/reconciliation mode must exist so RepoPact does not deadlock an already-invalid
repository. It is narrower than ordinary implementation authority and auditable.

## Read-only orientation and bootstrap

Before authorization, an agent must still be able to inspect enough state to determine the correct
next governance action. Candidate allowed operations include reads, Git status/log/diff, RepoPact
status/doctor/validate, and creation/amendment of bounded proposed governance records.

Arbitrary mutation-capable shell/process execution is not part of pre-authorization orientation.
The bootstrap path needs executable negative tests proving it cannot be used to implement the real
task or self-authorize.

## Cross-platform baseline

RepoPact itself is not a Windows-only product. WI050 requires the same public admission semantics on:

- Windows;
- Linux;
- macOS.

The policy core, authorization format/semantics, protected-guard contract, canonical repository
identity rules, adapter SPI, diagnostics, and conformance behavior must be OS-neutral.

Platform implementations may differ underneath:

```text
RepoPact policy / guard contract
             |
    +--------+--------+
    |        |        |
 Windows   Linux    macOS
 backend   backend   backend
```

Platform-specific choices may include protected storage, service/daemon mechanisms, filesystem
permissions, process confinement, sandboxing, IPC, and installation layout. Those are replaceable
backends; no one OS defines the semantics.

Closeout must include positive and fail-closed pre-execution proof on all three operating systems
using a RepoPact-controlled reference integration. Individual external adapters declare their
actual host/OS support matrix and cannot reduce the RepoPact baseline merely because a particular
vendor product is unavailable on one platform.

## Required bypass coverage

Executable coverage must include at least:

- direct editor/write-tool mutation;
- PowerShell mutation;
- POSIX shell mutation;
- Python/direct filesystem mutation;
- nested working directory;
- linked Git worktree;
- subagent/new session;
- attempted guard/config modification;
- proposed-work self-activation;
- forged/replayed operator or frozen approval;
- stale authorization after authority drift;
- missing/tampered guard failure.

For any adapter advertised as path/scope-enforced, arbitrary process writes must be constrained by a
real sandbox/proxy/equivalent boundary rather than unreliable command-string parsing.

## Non-goals

- Do not make RepoPact a wrapper around any one AI vendor ecosystem.
- Do not put Fabric-specific or other proprietary product knowledge into RepoPact.
- Do not pretend repository instructions, Git hooks, or CI prevent the first write.
- Do not require RepoPact to become an LLM runtime, IDE, shell, or remote execution fleet.
- Do not claim universal protection against arbitrary out-of-band processes without an actual
  process/sandbox/OS boundary.
- Do not replace WI046 promotion admission.
- Do not collapse WI044 typed completion evidence or WI047 documentation closure into this item.
- Do not make transient runtime authorization durable project authority.

## Relationship to other work

- **WI023 / decision 0021** defines mandatory preflight. WI050 makes it enforceable before covered
  mutation.
- **WI049** supplies the direct historical bypass case and is now completed.
- **WI044** may later add richer completion semantics; WI050 can consume them only after canonical.
- **WI047** may later add documentation-impact requirements; likewise consumed only after canonical.
- **WI046** owns post-work verification/promotion admission.
- **Downstream closed/internal products** may consume RepoPact's public adapter/SPI contract but do
  not become RepoPact dependencies.

## Implementation ordering

Threat/bypass analysis and architecture decisions come first. Schema, guard, adoption, adapter, or
platform implementation starts only after the public contract and truthful enforcement classes are
chosen explicitly.

Any frozen-surface change requires WI050-specific operator approval under INV-6. Approval granted
for another WI does not carry over.

## Closeout standard

WI050 cannot close with a demo that merely generates configuration files. It must prove, on
Windows, Linux, and macOS, that a covered unauthorized mutation is refused **before target
filesystem state changes**, that authorized work can proceed within scope, that scope/frozen
authority expansion requires re-authorization, that the agent cannot self-authorize, and that
protected-guard failure cannot silently downgrade enforced mode.
