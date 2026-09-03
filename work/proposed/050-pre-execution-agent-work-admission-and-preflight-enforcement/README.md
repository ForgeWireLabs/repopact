# 050 — Pre-Execution Agent Work Admission and Preflight Enforcement

> **Status**: 📋 Planning (proposed — not started)
> **Owners**: governance-owner (lead); tooling-owner and docs-owner affected.
> **Depends on**: WI023 mandatory preflight and WI049 baseline reconciliation.

## Intent

RepoPact already requires a work item to exist before implementation begins, and an `active`
work item is the repository's authorization for design or implementation. Today, however, those
rules are primarily **recorded and validated after the fact**. An autonomous coding agent can
still start editing an adopted repository, run mutating commands, or change runtime source before
it has created/activated the correct work item, resolved dependencies, checked the frozen surface,
or established the required development identity. CI, `repopact validate`, Git hooks, and review
can catch the violation later, but they do not stop the first unauthorized mutation.

The immediate field case is the post-3.0.2 `f2c80b7` evidence-timestamp hotfix now being
reconciled by WI049: the implementation itself may be sound, but it landed before RepoPact's own
required workflow records and without the post-release development identity required by decision
0032. WI049 must preserve that ordering rather than retroactively laundering it. WI050 asks the
next question: **how can RepoPact make this class of violation mechanically difficult or
impossible for a covered coding-agent runtime before the first write occurs?**

The target is a RepoPact-owned **work-admission handshake** plus replaceable agent/runtime
adapters. RepoPact decides whether a proposed work session is authorized; an adapter at the
agent execution boundary refuses mutating tool calls when that authorization is absent, stale,
out of scope, or insufficient.

## Core distinction

This is not CI and it is not the same as WI046.

```text
User / agent receives a task
        |
        v
RepoPact pre-execution work admission   <-- WI050
        |
        | authorized session only
        v
Agent reads / edits / runs commands
        |
        v
Verification + promotion/admission      <-- WI046 / H14
        |
        v
merge / release / deployment
```

WI050 answers **"may this agent begin mutating this adopted repository for this work?"**

WI046 answers **"did the required verification run and actually block a bad promotion?"**

Both are useful. Neither should be collapsed into the other.

## Current RepoPact authority already available to the gate

The gate should consume existing canonical RepoPact state rather than invent a parallel authority
system. Candidate inputs include:

- applicable `AGENTS.md` contracts and repository invariants;
- work-item lifecycle (`proposed` is not implementation authority; `active` is);
- mandatory `preflight` marker from decision 0021 / WI023;
- owner and affected scopes;
- dependency state;
- frozen-surface requirements and explicit operator approval where applicable;
- provenance rules;
- repository validity;
- post-release source/artifact identity (decision 0032);
- later optional requirements introduced by other work items when those requirements become
  canonical (for example documentation-impact declarations or typed completion claims).

The transient authorization must never become the source of truth for those facts. It should be a
runtime capability derived from canonical repository records.

## Candidate architecture — not yet a decision

A likely shape to evaluate is:

```text
repopact begin / authorize --work-item NNN
        |
        | validate canonical workflow prerequisites
        v
short-lived local work authorization
  - repository identity
  - work-item id
  - base HEAD/tree
  - allowed scopes / paths
  - frozen-surface authorization state
  - adapter/session identity
  - expiry / invalidation metadata
        |
        v
agent-runtime adapter
        |
        +-- read/orientation actions -> allow
        |
        +-- write / mutating command -> repopact gate check
                                      |
                                      +-- allow
                                      +-- deny before execution
```

The exact record/token shape, storage location, cryptographic needs, lifetime, and invalidation
rules are research questions for this work item. Any local lease/token is an enforcement artifact,
not durable project authority and should normally be gitignored.

## Enforcement levels must be honest

A repository file alone cannot universally stop every arbitrary process running as the same OS
user. RepoPact must not claim universal pre-execution enforcement where the host gives it no
interception point. The work therefore needs an explicit **coverage/capability model** for agent
adapters.

Candidate enforcement classes include:

1. **Instruction-only** — AGENTS/prompt guidance. Useful context, not mechanical enforcement.
2. **Session-start gate** — a launcher or host `before_run` hook refuses to start a mutating agent
   session until RepoPact authorizes it.
3. **Pre-tool gate** — the agent host calls RepoPact before each mutating tool action and can deny
   execution. This is the preferred covered-host behavior.
4. **Sandbox/OS boundary** — optional stronger enforcement where the host/platform can constrain
   filesystem writes independently of agent cooperation.
5. **Git/CI backstop** — catches bypass later; useful but explicitly not sufficient for WI050's
   pre-execution claim.

For a host registered as `enforced`, inability to load or execute its RepoPact adapter must fail
closed rather than silently degrade to instruction-only behavior.

## Agent/runtime adapters

The core contract must be agent-neutral, with thin adapters for concrete execution hosts.
Architecture work should evaluate at least:

- Claude Code `PreToolUse` / session hooks, which can run before tool execution and are suitable
  for refusing a mutating tool call;
- Codex-family/local orchestrator boundaries, including fatal `before_run` hooks where supported,
  managed requirements/sandbox controls, and whether a RepoPact launcher or MCP/tool-execution
  proxy is required for per-action enforcement;
- a generic `repopact agent-run --work-item NNN -- <agent command>` launcher/reference adapter;
- ForgeWire/Fabric-style execution as a future adapter, without coupling RepoPact's kernel to
  ForgeWire.

The work must distinguish what each host can actually enforce. A session-start-only integration
must not be reported as equivalent to a pre-tool gate.

## Expected admission checks

The design should determine the exact policy surface, but a mutation-capable agent session should
be able to fail before work for conditions such as:

- no work item exists;
- only a `proposed` item exists;
- required preflight is absent or invalid;
- work item is blocked/completed/deferred rather than active;
- dependency state does not authorize execution;
- requested write is outside owner/affected scopes;
- requested write touches the frozen surface without explicit operator approval;
- repository baseline is already invalid and policy does not explicitly permit repair mode;
- post-release package/runtime source lacks required development identity;
- authorization was issued for a different repository, work item, base commit, or scope;
- the session authorization has expired or been invalidated by material state drift.

A controlled **repair/reconciliation mode** may be needed for cases like WI049 where the baseline
is already nonconformant. That path must be explicit, narrower than ordinary implementation
authority, and auditable; the gate must not make recovery from an invalid repository impossible.

## Read-only orientation

The system should not prevent an agent from reading enough of the repository to discover what work
item is needed. A likely policy is:

- reads, status inspection, `repopact doctor`, `repopact validate`, and planning/orientation are
  allowed before authorization;
- creation/activation of the governance records required to obtain authorization is allowed via a
  narrowly defined bootstrap path;
- ordinary source/configuration mutations are denied until work admission succeeds.

This bootstrap exception is itself part of the security model and must be kept narrow enough that
an agent cannot implement the task while claiming to be "creating preflight records."

## Non-goals

- Do not pretend Git hooks or CI are pre-execution enforcement.
- Do not require RepoPact to become a coding-agent runtime, shell, IDE, or remote execution fleet.
- Do not hard-code one agent vendor into the governance kernel.
- Do not claim to block arbitrary out-of-band human/process writes that bypass every registered
  adapter unless an OS-level mechanism actually provides that guarantee.
- Do not replace WI046's promotion/admission-verification architecture.
- Do not collapse WI044 typed completion evidence or WI047 documentation closure into this item.
- Do not make transient session leases the durable source of project authority.

## Relationship to other work

- **WI023 / decision 0021** defines mandatory preflight as a repository contract. WI050 makes that
  contract enforceable before covered-agent mutation rather than only detectable later.
- **WI049** is direct field evidence for the bypass class and must complete before WI050 becomes
  active.
- **WI046** owns verification/promotion admission after work; WI050 owns admission to begin work.
- **WI044** may later define richer completion claims, but it is not required to decide whether an
  agent may start mutating code.
- **WI047** may later add documentation-impact workflow requirements that WI050 can consume once
  they are canonical.

## Implementation ordering

Architecture and threat/bypass analysis come first. Before changing schemas, adoption policy,
agent configuration, or frozen surfaces, the work must record which enforcement boundary is
actually achievable for each reference host and what RepoPact can truthfully guarantee.

Any frozen-surface change requires its own explicit operator approval under INV-6. Approval granted
for another work item does not automatically authorize WI050 changes.

## Closeout standard

Closeout must include executable negative proof where a covered agent/runtime attempts a mutating
action without valid work authorization and the action is refused **before the target filesystem
state changes**. It must also prove a valid authorized session can work normally, scope expansion
is denied or re-authorized, adapter failure fails closed in enforced mode, and an unsupported host
is reported honestly rather than counted as covered.
