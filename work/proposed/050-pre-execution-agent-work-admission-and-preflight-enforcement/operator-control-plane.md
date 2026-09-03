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
- requested scopes/paths;
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
Agent: WI051 is proposed. I need implementation authorization for tooling/** and tests/**.

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
- revoke a work authorization;
- approve another explicitly governed exceptional transition.

The accepted decision must determine the minimal generic set.

## Receipts, leases, replay and revocation

A successful approval should produce a verifiable receipt or equivalent proof. The receipt must reveal no private credential material and must be bound to the canonical request digest.

The protected guard derives a short-lived runtime authorization from the approved state. It must support:

- expiry;
- revocation;
- state-drift invalidation;
- repository/work-item binding;
- scope/path binding;
- adapter/session binding where needed;
- replay resistance;
- operator/key rotation semantics.

A copied receipt from another repository, work item, base state, request, or expired authorization must fail.

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
- replaying an approval from another repo/work item/base state;
- modifying a stored approval receipt;
- waiting for or exploiting an expired/stale authorization;
- using a different adapter/session to reuse an authorization where binding forbids it.

It must also prove the real operator can approve efficiently without exposing signing secrets to the agent.
