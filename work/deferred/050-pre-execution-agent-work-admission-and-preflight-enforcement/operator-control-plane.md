# WI050 Operator Control Plane — Design Target

This note records the current architecture target for operator authority inside WI050. It is a planning input, not an accepted decision; WI050 must still perform its architecture comparison and issue an accepted decision before implementation.

## Principle

Conversational approval remains useful UX, but plain chat text is not itself machine-verifiable operator authority in enforced mode.

RepoPact must separate:

1. **durable authority declaration** — recoverable, non-secret project state;
2. **protected operator identity/signing capability** — outside ordinary agent-writable state;
3. **approval request** — exact action the agent wants authorized;
4. **operator user-presence act** — non-agent-forgeable confirmation;
5. **signed/verifiable approval receipt** — durable or auditable proof;
6. **short-lived work authorization** — runtime capability consumed by the guard.

## Split authority model

### Repository-owned, durable, non-secret state

RepoPact should evaluate a versioned declaration describing the operator authority model for the adopted repository. Exact filename/schema is an implementation decision, but it may contain concepts such as:

- operator/key identifiers;
- public verification material or fingerprints;
- operator roles;
- approval classes each role may grant;
- authorization-profile definitions or references;
- delegation ceilings/policy;
- quorum/threshold rules where configured;
- authority-policy version;
- recovery/rotation rules.

This declaration must contain no private key, password, passphrase, token, passkey secret, or equivalent user-presence secret.

The repository copy is recoverable project authority under INV-1, but it is not sufficient by itself: an agent that can edit a JSON file must not thereby become an operator.

### Protected guard/operator state

The RepoPact protected guard pins/verifies the repository authority declaration and maintains whatever protected trust state is necessary to detect unauthorized authority changes.

Private signing credentials or equivalent operator-presence mechanisms live outside ordinary agent-writable repository state. Platform backends may use OS/hardware facilities appropriate to Windows, Linux, and macOS, but the RepoPact approval protocol must remain platform-neutral.

A change to the repository's operator/trust declaration must itself be approved using an already-trusted authority path; an agent cannot replace the trusted key set and then sign with its replacement.

## Approval request

An agent may request approval but cannot approve its own request.

The canonical request digest should bind at least:

- request id / nonce;
- canonical repository identity;
- canonical RepoPact root;
- work-item id;
- current base HEAD/tree and other authority-state digest as required;
- requested transition or authorization class;
- requested authorization profile;
- requested scopes/paths/capabilities;
- requested delegation ceiling if any;
- frozen-surface hits/implications;
- normal vs repair/reconciliation mode;
- adapter/session identity where relevant;
- issued-at and expiry;
- policy/authority version.

Changing any bound input after approval invalidates the approval or requires a new request.

## Operator surfaces

RepoPact should support multiple replaceable operator front ends over one protocol. Candidate surfaces include:

- protected CLI/TUI;
- local desktop/operator UI;
- browser/web UI;
- IDE integration;
- mobile/remote approval surface;
- chat/product integration.

No front end defines the authority semantics.

A CLI command that an agent can invoke non-interactively is not, by itself, proof of human approval. A protected front end must require a user-presence or signing act that the gated agent cannot reproduce.

## Chat approval

Chat remains a first-class convenience surface.

A conversation may work like:

```text
Agent: WI051 is proposed. I need standard implementation authorization for tooling/** and tests/**.

Operator: approved
```

The adapter may translate that conversational approval into a RepoPact approval request or open/present the pending request, but in enforced mode the text `approved` is not sufficient by itself unless the chat platform provides a trusted operator action whose authenticated result is bound into the RepoPact operator protocol.

Desired mature UX:

```text
chat approval intent
        |
        v
RepoPact pending request
        |
        v
trusted Approve control / user-presence prompt
        |
        v
signed/verifiable RepoPact approval
        |
        v
short-lived work authorization
```

This allows ChatGPT, Claude, Codex, Cursor, IDEs, MCP hosts, and other clients to provide pleasant UI without making any vendor the authority source.

## Approval classes

The architecture should consider distinct approval classes rather than one all-powerful boolean. Examples include:

- activate/authorize a proposed work item;
- authorize repair/reconciliation mode;
- approve frozen-surface mutation;
- approve scope expansion;
- approve operator-authority rotation/recovery;
- authorize a delegation ceiling;
- revoke a work authorization;
- approve another explicitly governed exceptional transition.

The accepted decision must determine the minimal generic set.

## Tiered authorization profiles

RepoPact should support adopter-configurable authorization profiles so operators do not have to approve every session with the same risk envelope.

The profiles are policy bundles, not agent types. A neutral reference progression could be conceptually similar to:

1. **observe** — read/orientation only;
2. **bounded** — exact WI plus explicit paths/scopes and tightly constrained commands;
3. **standard** — ordinary work-item implementation within declared scopes, tests/build tooling, no frozen/trust changes;
4. **elevated** — broader process/network/scope capabilities where the adapter can actually enforce them, with additional approval requirements;
5. **unrestricted-within-boundary** — the broadest adopter-approved repository/session capability.

Names and exact contents are policy decisions. A UI may call the broadest profile `YOLO`, but `YOLO` should not become a universal RepoPact semantic.

Every profile still has hard ceilings. No profile, including the broadest one, may:

- forge operator approval;
- modify the protected guard/trust root as an ordinary agent action;
- expand its own authorization profile;
- cross to another repository/work item unless separately authorized;
- bypass frozen-surface rules that require a stronger approval class;
- claim sandbox/process/network guarantees not provided by the active adapter;
- erase required audit/delegation lineage.

Thus `unrestricted` means broad authority *inside the approved RepoPact boundary*, not root/admin authority over the machine or authority to rewrite RepoPact's trust system.

Profiles should be assignable to operator roles, adapter/session principals, and approval requests. They should also define whether delegation is permitted and, if so, the maximum delegation ceiling.

## Generic principals and delegation

RepoPact should understand **principals and delegated authority**, but not agent orchestration.

A principal might be a protected session, adapter identity, process identity, tool client, or another stable identity supported by the guard. RepoPact does not need to know whether that principal is called a control agent, subagent, supervisor, worker, IDE, or chat session.

An operator may authorize a parent principal with delegation rights. That principal can derive child authorizations only under a strict non-escalation rule:

```text
child authority ⊆ parent delegable authority
```

A child authorization should be bounded by at least:

- same canonical repository unless explicitly allowed otherwise by operator policy;
- same work item or a specifically operator-authorized related work item;
- subset of scopes/paths/capabilities;
- expiry no later than the parent ceiling;
- delegation depth no greater than policy allows;
- explicit lineage to the parent authorization;
- revocation/invalidation inherited from the parent where appropriate;
- no operator/frozen/trust approval capability unless that exact delegation class was operator-authorized and is itself safe to delegate.

A downstream orchestration product can map:

```text
control agent -> parent RepoPact principal
subagent      -> delegated child principal
```

but RepoPact does not:

- choose models;
- spawn agents;
- route tasks;
- decide control-agent topology;
- manage prompts/context;
- implement vendor-specific agent hierarchies.

This keeps RepoPact useful to Codex/Claude/Cursor/ChatGPT/MCP/IDEs and future systems without becoming their agent framework.

## Receipts, leases, replay and revocation

A successful approval should produce a verifiable receipt or equivalent proof. The receipt must reveal no private credential material and must be bound to the canonical request digest.

The protected guard derives a short-lived runtime authorization from the approved state. It must support:

- expiry;
- revocation;
- state-drift invalidation;
- repository/work-item binding;
- scope/path/capability binding;
- authorization-profile binding;
- delegation ceiling and lineage where applicable;
- adapter/session binding where needed;
- replay resistance;
- operator/key rotation semantics.

A copied receipt from another repository, work item, base state, request, profile, or expired authorization must fail.

## Existing `--ack` behavior

`repopact check-frozen --ack` currently means the caller asserts that a human approved the change. That remains useful as backward-compatible procedural UX, but an agent can invoke the flag itself.

Therefore WI050 enforced mode must not treat an unbacked `--ack` flag as operator proof. If the CLI remains, enforced mode should require the protected operator protocol to supply the verifiable approval that authorizes the acknowledgement.

## Hidden-state boundary

RepoPact must avoid replacing recoverable repository governance with opaque machine-only authority.

The intended split is:

```text
versioned repository
  public authority declaration
  work items / scopes / frozen surface / policy
              |
              v
protected guard
  verifies project declaration
  holds/pins protected trust state
              |
              v
operator signer / user-presence mechanism
  secret material never stored in repository
```

A fresh operator can reconstruct what authority is intended from versioned records, while an agent cannot forge the private/user-presence side needed to exercise that authority.

## Cross-platform rule

The operator approval protocol and signed/verifiable request semantics are identical on Windows, Linux, and macOS. Protected storage/signing/user-presence backends may differ by platform.

No platform-specific credential facility becomes RepoPact's semantic source of truth.

## Required negative proof

WI050 closeout should prove at minimum that a gated agent cannot obtain authority by:

- writing `active` into its own work item;
- invoking a non-protected operator CLI path;
- invoking `check-frozen --ack` itself;
- editing the public operator-authority declaration;
- replacing a trusted key/fingerprint with an agent-controlled one;
- expanding its own authorization profile;
- delegating authority it does not possess;
- extending child expiry beyond the parent ceiling;
- stripping delegation lineage;
- replaying an approval from another repo/work item/base state;
- modifying a stored approval receipt;
- waiting for or exploiting an expired/stale authorization;
- using a different adapter/session to reuse an authorization where binding forbids it.

It must also prove the real operator can approve efficiently without exposing signing secrets to the agent.
