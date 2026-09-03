# WI050 field evidence and threat model

> Architecture phase record. The cases below are threat hypotheses and
> implementation test obligations, not claims that a runtime guard already
> exists.

## 1. Field capture: f2c80b7 and WI049

The primary field case is the post-v3.0.2 hotfix later reconciled by WI049:

| Point | Durable fact | What an autonomous agent could do |
| --- | --- | --- |
| Task received | A request existed in the session, but no preflight-authorized active work item preceded the runtime edit. | Treat the conversation as sufficient authority. |
| First mutation | Commit f2c80b7dcdc54ff9f4753bc996ef0b6dfba539bf changed repopact/validate_repo.py and tests to compare evidence timestamps to datetime.now. | Directly edit source through an editor/patch tool or shell. |
| Local validation | Tests and repository validation were run after the source mutation. | Produce green post-change evidence without having proved first-write admission. |
| Commit/push | The mutation was committed and pushed to main. | Git accepted the commit; no pre-action RepoPact boundary denied it. |
| Later detection | Manual review of the field defect and release-identity drift exposed that the workflow state had not preceded implementation. | A later work item, WI049, had to reconcile the history. |
| Remote checks | GitHub/CI and promotion checks can reject or report the resulting tree, but do not reverse the already-written filesystem state. | A backstop could detect the defect only after the first write. |

The incident is not a dispute about the later timestamp semantics. Its
governance lesson is first-write bypass: RepoPact required records and
preflight as durable state, but the editing environment did not mechanically
ask RepoPact before allowing the first runtime mutation. A conversation,
AGENTS.md, post-change validate, local hook, commit, or remote CI check is not
equivalent to a protected pre-execution boundary.

WI049 remains the honest historical record. Its reconciliation did not
retroactively authorize f2c80b7. It created decision 0037, restored the
development label, and documented the replacement without laundering the
original chronology.

## 2. Assets and trust boundaries

Assets:

* work-item lifecycle and preflight authority;
* owner/scope and frozen-surface declarations;
* operator trust root, public keys, approval receipts, and revocations;
* canonical repository registration and RepoPact root;
* active lease/profile/delegation constraints;
* source and governance filesystem state;
* privacy-safe receipts and diagnostics.

Trust boundaries:

1. durable repository records are reviewable but ordinary agent-writable;
2. the policy core is deterministic but is not protected merely by being
   imported into the gated process;
3. the protected guard and trust pin are outside ordinary agent-writable state;
4. the operator signer/user-presence capability is outside repository content;
5. adapters and vendors are untrusted integration surfaces until capabilities
   are proven;
6. Git and CI are later backstops, not first-write protection;
7. host/OS process boundaries are trusted only when the adapter can demonstrate
   their actual coverage.

Threat assumptions are explicit: an agent may be cooperative, confused,
prompt-injected, malicious, or simply able to invoke every tool exposed by its
host. It may run nested shells or Python, change cwd, start children, use
another session/adapter, edit repository configuration, or race a lease. The
design does not claim to stop an unrelated root/admin process that is outside
the protected OS boundary; it must say not covered instead of reporting
enforced protection.

## 3. Bypass and control matrix

| Threat/bypass | Why current records alone do not stop it | WI050 control and falsification test |
| --- | --- | --- |
| Direct editor/write tool | The editor can write before validate. | Pre-action guard denies before target bytes change; prove with a sentinel. |
| apply-patch | Patch tooling is just another writer. | Adapter must intercept the mutation or sandbox the process; test raw patch and denied path. |
| PowerShell write | Shell can bypass host UI and hooks. | Protected process/sandbox boundary; run Set-Content/Add-Content denial. |
| POSIX shell write | A different shell can use redirection/tools. | Same semantic guard on Linux/macOS; test sh, bash, and redirection. |
| Python filesystem write | A script can use open/os.replace directly. | Process/path confinement, not command-string matching; test direct APIs. |
| Arbitrary process execution | Child processes inherit broad account authority. | Fail closed unless process boundary is real; test a child not known to adapter. |
| Nested working directory | Root-relative goodwill can be bypassed by cwd. | Resolve canonical registration/root before admission; test every nested level. |
| Linked worktree | A second checkout may evade path checks. | Bind Git common-dir and registration; test linked worktree mutation. |
| New session | Session-start checks may not carry across sessions. | Protected lease binds session/principal; new session must reorient/re-authorize. |
| Subagent/child process | Child may lose parent restrictions or lineage. | Explicit delegated principal and subset check; test absent and overbroad lineage. |
| Alternate agent host/adapter | A weaker integration may claim the same repository. | Capability negotiation and protected registration; unsupported host is degraded/not-covered. |
| Repo-local hook removal | Hooks are ordinary files and may be deleted. | Hooks are discovery/backstop only; protected guard remains active when removed. |
| Repo-local settings removal | .codex/.claude/.cursor/MCP settings can be edited. | No sole enforcement root; guard detects missing/tampered adapter and fails closed. |
| Adapter tampering | A wrapper can report allow while bypassing policy. | Signed/attested adapter identity and guard-mediated action; tamper is fail closed. |
| Protected-guard tampering | If guard lives in the repository, the agent edits it. | Host-managed/protected guard outside writable tree; registration and health pin. |
| Proposed-WI self-activation | Agent can change proposed to active in JSON. | Bounded bootstrap disallows status transition; protected activation receipt required. |
| Forged frozen approval | A text field or local flag can look like approval. | Frozen class requires signed receipt bound to exact digest; --ack alone denied. |
| --ack self-assertion | Current check-frozen treats caller assertion as acknowledgement. | Keep backward-compatible UX but ignore it as proof in enforced mode. |
| Approval replay | A copied receipt may name a valid other action. | Nonce and digest bind repo, root, WI, base, profile, session, expiry, epoch. |
| Authorization replay | A lease copied between worktrees/sessions may appear valid. | Protected binding and principal/session checks; copied lease is rejected. |
| Scope escalation | Caller requests a broader path/profile than approved. | Exact subset comparison and reauthorization on any expansion. |
| Tier/profile escalation | “YOLO” or a CLI switch is treated as unlimited. | Profiles are policy bundles with immutable hard ceilings; adapter capability proof. |
| Child/delegation escalation | Delegate mints broader child or strips lineage. | child authority subset of parent, depth/expiry/lineage/revocation enforced. |
| Stale authority after HEAD/policy drift | A prior approval remains cryptographically valid but semantically stale. | Bind base tree, authority/policy/frozen digests and invalidate on drift. |
| OS differences | A Windows path check may not match symlinks or POSIX permissions. | OS-neutral decision vectors and three-platform conformance; no platform semantic source. |

## 4. Enforcement-level taxonomy

These are reporting levels, not interchangeable defenses:

1. Instruction/advisory: context tells an agent what to do; no mechanical
   admission and no first-write prevention.
2. Session-start admission: a mutating session cannot start until a valid
   request is admitted. A process launched later or an out-of-band writer may
   still bypass it.
3. Pre-action admission: each covered mutation is sent to the guard and denied
   before execution. Coverage is only as broad as the adapter interception.
4. Sandbox/process enforcement: an independent process/filesystem/OS boundary
   constrains effects even when a child process or command ignores instructions.
5. Git/CI backstop: a later commit/promotion check detects or rejects invalid
   state. It cannot prevent a first filesystem write and cannot claim to.

An adapter reports a capability vector and RepoPact computes the highest
truthful level. “enforced=true” is not a capability. A session-start-only
adapter must say session-start. An MCP route, repository hook, or chat context
is not automatically pre-action or sandbox enforcement.

## 5. Research observations

Current ecosystems supply useful substrate but not RepoPact authority:

* MCP's host/client/server architecture and transport authorization concepts
  are useful for identity and handoff, while the protocol itself does not
  guarantee that native shell or filesystem paths are intercepted:
  https://modelcontextprotocol.io/specification/2025-06-18/architecture
  and https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization
* Claude Code documents pre-tool hooks that can deny a tool call, which is a
  useful pre-action adapter shape. The hook remains a host capability and must
  be protected/attested before it can be advertised as enforced:
  https://code.claude.com/docs/en/hooks-guide
* Cursor documents session, pre-tool, shell, MCP, subagent, and file-edit hook
  points plus fail-closed hook hardening. This is useful adapter substrate, not
  a RepoPact kernel dependency:
  https://prod.cursor.com/docs/hooks
  and https://prod.cursor.com/docs/enterprise/security-hardening

Codex/ChatGPT, Claude/Anthropic, Cursor, IDEs, generic agent CLIs, MCP hosts,
and future products can implement the same public SPI. No product's hook,
permission mode, prompt, or settings file defines RepoPact semantics.

## 6. Falsification criteria

The architecture is disproved by an implementation that shows any of the
following:

* unauthorized direct, shell, Python, editor, child-process, or linked-worktree
  mutation changes target bytes before a denial;
* changing cwd, casing, symlink/junction, session, adapter, host, or settings
  avoids the canonical registration;
* a repository-local hook/config/script is the only thing preventing mutation;
* --ack, “approved”, a non-interactive operator CLI, or an edited receipt grants
  frozen/operator authority;
* a copied approval works for another repository, WI, base state, profile,
  principal, or adapter;
* a delegate broadens scope/capability, expiry, depth, or lineage;
* a revoked/expired/stale lease remains usable or guard failure downgrades to
  advisory;
* Windows, Linux, and macOS disagree on semantic allow/deny/revoke/expiry;
* a vendor adapter can label unsupported coverage as enforced;
* repair can be used as general development authority;
* the public OSS package requires ForgeWire Fabric or another closed product.

These falsification tests are intentionally stricter than the field capture:
architecture evidence establishes the contract, while implementation closeout
must establish the executable boundary.
