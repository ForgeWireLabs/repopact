---
id: 0038
title: Vendor-neutral pre-execution admission and protected guard architecture
status: accepted
date: 2026-09-03
supersedes: []
---

# 0038: Vendor-neutral pre-execution admission and protected guard architecture

## Context

RepoPact records a work item and mandatory preflight, but the repository
validator, Git hooks, and CI run after or around mutations. The post-3.0.2
f2c80b7/WI049 case demonstrated the gap: an autonomous agent could edit
runtime source, run green local checks, commit, and push before a later
review noticed that first-write workflow authority had not preceded the edit.

WI050 needs a pre-execution contract without replacing the work ledger,
depending on a vendor, or claiming that a prompt, hook, or post-change check
is a process boundary. The design must work for independent OSS adopters and
must report degraded coverage honestly.

## Decision

RepoPact adopts a hybrid admission architecture:

1. A deterministic, vendor-neutral policy core consumes canonical repository
   records and answers an action-admission question before mutation.
2. A protected local guard/service holds the protected registration, trust pin,
   revocation state, and lease verification boundary outside ordinary
   agent-writable repository state. A library embedded in the gated process
   is policy code, not a protected guard by itself.
3. Thin adapters implement a public capability SPI. They report identity,
   interception, confinement, operator handoff, delegation propagation, and
   guard-health facts. RepoPact computes the truthful enforcement class;
   adapters cannot assert enforced with a boolean or promote MCP, hooks, or
   chat context beyond their actual coverage.
4. Operator authority is split between recoverable, non-secret repository
   declarations and protected signing/user-presence capability. A canonical
   request is RFC-8785-style canonical JSON hashed with SHA-256 and signed by
   the reference Ed25519 suite. It binds repository/root, work item, base
   state, authority and frozen digests, class/profile, scopes/capabilities,
   principal/session, mode, nonce, expiry, and revocation epoch.
5. A signed approval receipt is verified by the protected guard, which derives
   a short-lived lease. The lease is not project authority and is invalidated
   by expiry, revocation, repository/work-item/policy/authority/frozen/base
   drift, adapter/session mismatch, delegation change, or guard-health loss.
6. The canonical state machine is UNREGISTERED → REGISTERED/ADOPTED →
   ORIENTATION → PENDING OPERATOR AUTHORIZATION → AUTHORIZED → ACTIVE LEASE,
   with EXPIRED, REVOKED, and INVALIDATED terminal runtime outcomes.
   Proposed work creation/amendment and approval requests are bounded bootstrap
   operations; activation and approval are protected operator transitions.
7. Profiles are adopter-configurable bundles: observe, bounded, standard,
   elevated, and unrestricted-within-boundary. “YOLO” may be a UI alias only.
   Every profile retains hard ceilings for trust, self-escalation, repository
   binding, frozen approval, truthful capability reporting, and delegation.
8. Principals are generic authenticated sessions/processes/adapters/tool
   clients. Delegation is strict subset authority with same repository/work
   item by default, bounded paths/capabilities/profile/expiry/depth, immutable
   lineage, inherited revocation, and no implicit operator/frozen authority.
9. Repair/reconciliation is a separate, narrower, operator-approved mode
   tied to diagnosed files/scope and short expiry. It cannot become ordinary
   implementation authority.
10. The semantics and conformance vectors are identical on Windows, Linux,
    and macOS. Protected service, key storage, IPC, and process/sandbox
    mechanisms are replaceable backends. A repository can remain observe or
    session-start only; an enforced configuration fails closed if its guard or
    claimed boundary is missing.

The existing check-frozen --ack remains procedural compatibility UX. In
enforced mode it is never operator proof without the protected approval
receipt bound to the exact frozen change. Plain chat text and a
non-interactive operator command express intent or create a request; they do
not prove user presence.

## Alternatives considered

* Repository instructions only: rejected because an agent can ignore or edit
  them and use another writer.
* Git hooks: rejected as the primary boundary because they run too late,
  can be removed/bypassed, and do not constrain an editor or child process.
* Launcher/session gate only: useful for admission at session start but
  insufficient when another process or adapter can write the repository.
* Embedded library only: rejected as a protection root because the gated
  process can replace the library or its policy inputs.
* Permanent platform daemon as the semantic contract: rejected because
  installation and OS APIs differ; a local protected service is an optional
  backend beneath the common contract.
* Vendor-specific hook as kernel authority: rejected because host coverage,
  names, and security guarantees change; useful hooks remain replaceable
  adapters.
* Durable lease as a second work ledger: rejected because it would split
  authority and violate the repository-as-pact model.
* Plain approval text or a caller acknowledgement flag: rejected because the
  gated agent can produce it without operator presence.

## Consequences

The next implementation pass must add the policy/guard contract, schemas,
protected registration, adapter SPI, platform backends, and executable
cross-platform bypass proof. Existing adopters are not broken until they opt
into or migrate to required coverage. Unsupported adapters are reported as
degraded/not-covered, not mislabeled.

The architecture does not change VERSION, RELEASE_LABEL, the v3.0.2 tag, or
any WI044, WI046, or WI047 semantics. It proposes frozen-surface changes in
the WI050 record but does not make them without WI050-specific INV-6 approval.

## Rejection and falsification

This decision must be revisited if implementation shows that a direct writer
can mutate before denial, a copied receipt works elsewhere, a child widens
authority, local configuration is the sole guard, guard failure downgrades
silently, OS outcomes differ, an adapter overclaims, or an independent OSS
installation cannot operate without ForgeWire Fabric or another closed
product.
