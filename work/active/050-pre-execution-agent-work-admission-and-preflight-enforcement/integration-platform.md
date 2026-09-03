# WI050 integration, guard, identity, and platform architecture

> Architecture phase only. The matrices select contracts and backend
> directions; no executable integration is implemented here.

## 1. Guard-placement alternatives

The guard is the enforcement root, not the place where a vendor happens to
offer a hook. Four required placements were compared:

| Placement | First-write protection | Tamper resistance | Portability/offline | Main failure |
| --- | --- | --- | --- | --- |
| Repository instructions only | Advisory; an agent can ignore them or write through another tool. | None when the agent can edit the file. | Excellent. | Cannot establish admission or truthful coverage. |
| Git hooks | Commit-time or selected command-time backstop; cannot stop an editor or arbitrary first write. no-verify bypass and hook removal are escape routes. | Low unless installed/protected outside the tree. | Good but inconsistent across clones/worktrees. | Detects too late and is not a process boundary. |
| RepoPact launcher/session gate | Prevents a covered session from starting without admission. | Good only while every mutation stays in the launcher and its children. | Good, including offline local use. | A second process/adapter can write unless it is also confined. |
| Pre-action proxy or sandbox interception | Can deny each intercepted action before execution; a real sandbox also constrains arbitrary children. | Strong when the proxy/guard and policy are outside the writable tree. | Depends on host/OS backend and installation. | Coverage must be declared; un-intercepted paths remain not covered. |

### Guard implementation alternatives

| Candidate | Strength | Cost/limits | Decision |
| --- | --- | --- | --- |
| A. Protected local process/service | Agent cannot rewrite the service or its trust state; clear fail-closed IPC boundary. | Installation, lifecycle, and multi-repo registration need platform work; local IPC latency. | Useful primitive. |
| B. Embedded library | Portable and easy to test/offline. | Not protected when the gated process owns the library and can edit/replace its inputs. | Policy core only, never the sole enforced root. |
| C. Executable verifier plus protected state | Small attack surface; signed state can be verified by many clients. | Stateless verifier cannot by itself intercept arbitrary writes; state lifecycle must be protected. | Use for verification and receipts. |
| D. Hybrid portable policy core + protected local guard/service + thin adapters | Combines deterministic semantics, a real trust boundary, offline local operation, and replaceable host integrations. | Requires a reference guard and per-OS service/process backend. | **Chosen.** |

The chosen architecture has a pure RepoPact policy core, a protected local
guard (service or equivalent host-managed process), a protected registration
and trust pin, and thin adapters. A library-only adapter may report
instruction, session-start, or pre-action coverage only to the extent the host
really intercepts actions. It cannot call itself sandbox-enforced.

The guard fails closed on absent, unhealthy, stale, unverifiable, or tampered
policy/receipt/registration state. Crash recovery leaves the last lease
invalid until the guard re-verifies state; it never silently downgrades to
advisory behavior.

## 2. Public adapter SPI and capability negotiation

The adapter is a replaceable transport and enforcement witness. It submits a
canonical action request and reports a signed/attested capability vector:

| Capability | Meaning |
| --- | --- |
| repo_discovery | Can locate and identify the adopted repository/root. |
| session_identity | Can provide a stable principal/session identity. |
| session_start_gate | Can prevent a mutating session from starting without admission. |
| pre_action_interception | Can send every claimed covered mutation to the guard before execution. |
| path_reporting | Can report exact target paths, worktree, and operation kind. |
| path_confinement | A real boundary constrains writes to admitted paths, including children. |
| process_confinement | A real boundary constrains arbitrary child processes. |
| network_confinement | A real boundary constrains network effects where claimed. |
| protected_host_config | Host-managed configuration cannot be replaced by the agent. |
| operator_handoff | Can present a request and return protected user-presence proof. |
| subprincipal_propagation | Child processes/sessions receive verifiable delegated lineage. |
| fail_closed_health | Missing/tampered guard causes denial, not downgrade. |
| audit_receipt | Emits a privacy-safe admission/denial receipt. |

The SPI includes adapter id/version, host and OS, capability version, guard
endpoint identity, root registration, principal/session id, action request,
and a health assertion. Capability claims are evidence-backed facts, not
configuration labels. RepoPact computes the truthful class:

* instruction-only: no protected gate;
* session-start: session_start_gate plus identity and healthy guard;
* pre-action: session-start plus pre_action_interception, path reporting, and
  fail-closed health for the claimed action family;
* sandbox/process-enforced: pre-action plus real path/process confinement and
  protected host configuration;
* not covered/degraded: missing capability, unknown coverage, guard failure, or
  an unsupported host.

Git/CI is reported separately as a backstop. It never upgrades an adapter to
pre-action or sandbox enforcement. An MCP adapter can carry the same request
and identity but does not claim native shell/filesystem interception merely
because a tool was routed through MCP.

Reference integration families for later implementation:

1. a true pre-action host hook adapter (for example a host with a pre-tool
   decision callback) that denies before invoking the tool;
2. an independent launcher/proxy/sandbox adapter that starts a process inside
   a protected boundary and intercepts child effects.

Both call the same policy/guard contract. Codex/ChatGPT, Claude/Anthropic,
Cursor, MCP hosts, IDEs, generic agent CLIs, and future hosts are replaceable
clients. A vendor-specific hook can improve coverage but cannot change the
RepoPact authority model.

## 3. Canonical repository identity

Path strings and prompt text are insufficient identity. The adopted repository
has a versioned public registration and a protected local binding:

* a random RepoPact adoption id generated at adoption/registration time;
* canonical Git common directory identity and repository root discovered from
  Git, when available;
* a digest of the root RepoPact contract and adopted policy declaration;
* optional normalized remote identities as corroborating metadata, never as
  the sole identity;
* a protected guard binding that maps this registration to the local Git
  repository and root.

The protected registration is not copied as authority when a repository is
cloned. A clone is re-adopted and receives a new local adoption binding while
its public records remain recoverable. Offline repositories work without a
remote URL.

Resolution rules:

* walk from the caller cwd to the RepoPact root and reject ambiguous nested
  repositories;
* use Git common-dir plus worktree metadata so linked worktrees share the
  registration while retaining their own root/index state;
* canonicalize absolute paths, resolve symlinks/junctions where the OS
  permits, and case-fold Windows comparisons; path spelling is a lookup aid,
  not the identity;
* bind every action to the resolved root, common-dir, registration id, and
  worktree identity; changing cwd, path casing, link, or worktree cannot
  select a different policy accidentally;
* remote/web adapters carry the registration id and a guard-issued challenge;
  they do not invent identity from a workspace name or prompt.

If Git is unavailable, RepoPact may use the protected adoption binding plus
root contract digest for a local non-Git repository. It must report the lower
identity assurance and must not pretend to have Git/worktree guarantees.

## 4. Common platform backend interface

All backends implement the same semantic interface:

    register(root, public_declaration) -> protected_registration
    discover(caller_context) -> canonical_identity
    request(action, principal, lease) -> allow | deny(diagnostic)
    issue_lease(verified_receipt) -> short_lease
    revoke(id_or_epoch) -> protected_revocation
    health() -> healthy | fail_closed(reason)
    audit(receipt) -> privacy_safe_record

The interface defines semantics, canonical encoding, error classes, expiry,
revocation, scope, and tamper behavior. OS APIs only implement storage,
identity, IPC, process, and sandbox mechanisms beneath it.

### Windows direction

Use a protected Windows service or per-user service with a named-pipe endpoint
whose ACL excludes the gated agent from changing the service/trust state.
Store operator key handles or protected trust material in Windows CNG/DPAPI or
an equivalent user-presence-backed facility; never place private keys in Git.
Use Job Objects, restricted tokens, AppContainer/WDAC or an equivalent host
boundary for process/path-enforced adapters where available. Normalize drive
letters, UNC paths, junctions, and case before root binding. Test ACL denial,
junction escape, PowerShell, direct Win32/Python writes, child processes, and
service restart/fail-closed behavior.

### Linux direction

Use a systemd user/system service or a separately installed daemon with a
0600 Unix-domain socket and peer credential checks. Store protected trust
state in an OS keyring or operator-controlled file outside the worktree.
Use namespaces, cgroups, seccomp, Landlock, or a container/sandbox backend
for process/path confinement according to the adapter's declared coverage;
do not infer confinement from shell parsing. Resolve real paths, bind mounts,
and linked worktrees. Test sh/bash redirection, Python/open/rename,
namespace boundaries, socket permissions, daemon loss, and fail-closed
restart behavior.

### macOS direction

Use a launchd-managed service/agent and XPC or a protected local IPC endpoint
with code-signing/identity checks appropriate to the deployment. Store key
handles and user-presence material in Keychain/Secure Enclave where available,
outside the repository. Use the App Sandbox, sandbox-exec successor mechanisms,
EndpointSecurity or a managed process wrapper when a real process/path
boundary is required; declare reduced coverage if entitlements or user
consent are unavailable. Resolve APFS case/symlink behavior and linked
worktrees. Test shell/Python writes, sandbox denials, launchd restart, key
rotation, and guard-health failure.

These are backend directions, not semantic dependencies. A minimal OSS
installation can provide observe/session-start locally; enforced coverage is
available only when the selected backend can prove its boundary.

## 5. Cross-platform conformance obligations

The reference conformance vectors are OS-neutral and must assert identical
results for:

* registered versus unregistered repository;
* allow within scope and deny outside scope;
* missing/proposed/blocked work item;
* missing or invalid preflight/development identity;
* frozen path without stronger approval;
* expiry, revocation, and authority/HEAD/policy drift;
* copied receipt, wrong repository/work item/session, and nonce replay;
* proposed self-activation and edited trust declaration;
* guard crash, missing protected state, and adapter tamper;
* child subset, expiry, depth, lineage, and revocation inheritance.

Each OS adds path/permission/process edge cases, but the expected semantic
outcome is the same. The implementation pass must include positive and
fail-closed negative runs on Windows, Linux, and macOS using the same
RepoPact-controlled reference integration. Vendor adapters publish their
actual host/OS capability matrix and cannot lower the RepoPact baseline.

## 6. Product boundary and OSS independence

RepoPact owns policy, records, identity, approval, leases, guard contract,
adapters, diagnostics, and conformance. External products supply adapters.
ForgeWire Fabric has no upstream dependency or privileged path in this design.
If Fabric later integrates, it consumes the public SPI downstream. Removing
Fabric or any vendor must leave the policy core, reference guard contract,
schemas, and conformance corpus usable by an independent OSS adopter.
