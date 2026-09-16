# WI050 — Linux Landlock confinement tranche

Status: authorized implementation plan
Date: 2026-09-15
Authority: Decision 0060

## Why this tranche exists

The native Debian 13/WSL2 proof in `20260915-050-linux-native-proof` satisfied AC-15 and falsified any stronger claim for the existing reference adapter: direct Python, POSIX-shell, and child-process writes outside `NativeGuardClient` succeeded. The current Linux reference therefore remains truthfully `pre-action`.

Decision 0060 keeps `pre-action` as RepoPact's portable reference baseline and authorizes a separate optional `sandbox/process-enforced` reference path. The first backend is Linux Landlock.

This tranche is the implementation/proof path for WI050 AC-16. It does not alter AC-18's independent Windows/Linux/macOS native-reference requirement.

## Architectural boundary

The protected guard remains the authority boundary. The confinement backend is an enforcement consumer, not authority.

The backend MUST NOT:

- activate work or manufacture operator approval;
- grant frozen-surface authority;
- widen repository, work-item, principal/session, profile, scope, path, capability, expiry, delegation, or revocation state;
- accept caller-provided filesystem allowlists as authority;
- create a durable second work ledger; or
- claim network confinement merely because filesystem/process confinement exists.

The backend MAY accept a caller request that is narrower than the guard-derived ceiling.

## Implementation direction

The reference implementation should be a small Linux-specific confinement backend/launcher, preferably in the canonical Rust workspace after confirming the existing workspace boundaries and dependency policy.

The implementation should:

1. authenticate to the protected guard and verify a live authorization/lease for the canonical repository and session;
2. derive the effective filesystem ceiling from protected guard-authorized state, never from an untrusted caller allowlist;
3. canonicalize/validate applicable roots and reject ambiguous or unsafe confinement inputs;
4. determine Landlock support and ABI capabilities at runtime;
5. require a defined minimum Landlock filesystem contract before advertising `sandbox/process-enforced`;
6. sanitize unexpected inherited writable file descriptors or equivalent mutation capabilities;
7. establish `no_new_privs` or the supported Landlock equivalent bound to successful restriction;
8. create and apply the Landlock domain before executing the target process;
9. launch without shell-string reinterpretation in the confinement helper;
10. ensure descendants inherit the domain; and
11. fail closed before child execution when the requested sandbox class cannot be established.

Existing `pre-action` adapters and standalone RepoPact operation must remain compatible and retain their truthful lower capability class.

## Guard-to-confinement data boundary

Before coding a new public protocol, inspect the current opaque-lease/guard flow and determine whether the launcher can already obtain a guard-derived canonical path/scope ceiling without trusting caller data.

If a new provider/IPC operation is needed, it must remain adopter-neutral and be bound to the same repository/work item/principal/session/lease/expiry/revocation/drift authority already enforced by WI050.

Do not expose protected signing material or turn the opaque lease into caller-editable authority.

If satisfying this boundary requires changing a frozen schema or another frozen surface not already covered by an explicit WI050 approval, stop before that change and produce the exact INV-6 approval packet. Do not infer approval from this plan.

## Minimum native Linux bypass matrix

The backend cannot advertise `sandbox/process-enforced` until executable native proof shows:

### Allowed behavior

- authorized direct Python write inside an allowed path succeeds;
- authorized POSIX-shell write inside an allowed path succeeds;
- authorized child and descendant writes inside an allowed path succeed; and
- normal build/tool execution needed by a bounded development session remains usable within the granted boundary.

### Denied behavior

At minimum, attempts must be denied at the OS boundary for:

- direct Python filesystem write outside allowed paths;
- shell redirection outside allowed paths;
- `touch`, `printf`, `cp`, `mv`, append, truncate, create, remove, and rename/link operations outside the ceiling;
- child and descendant process writes outside the ceiling;
- nested-working-directory escapes;
- linked-worktree escapes;
- a second/unrelated repository;
- protected RepoPact guard/trust state;
- frozen paths without the exact required protected approval;
- symlink-mediated escape to a disallowed target;
- rename/hard-link/reparent escape cases supported by the running Landlock ABI; and
- inherited writable file-descriptor or equivalent pre-opened capability bypass.

Where a sentinel is practical, denied targets must be byte-identical before and after the attempt.

## Capability truthfulness

`path_confinement=true` and `process_confinement=true` may only be surfaced when:

- the running host supports the minimum backend contract;
- the process actually entered the Landlock domain;
- backend health/attestation supports the claim; and
- the native bypass matrix passes.

If policy requires `sandbox/process-enforced` and any of those conditions is absent, execution fails closed. There is no silent downgrade.

If policy only requires `pre-action`, the existing reference behavior remains valid and does not require Landlock.

## Acceptance impact

AC-16 is **satisfied** by the concrete native Linux evidence run
`20260916-050-linux-landlock-native-proof`. This closes only the Linux
reference-path criterion; it does not weaken or reinterpret AC-18.

AC-18 remains **pending** until its Windows, Linux, and macOS native reference-proof requirement is independently satisfied.

## Required evidence

Record a new immutable evidence run for the Landlock tranche, including at least:

- authoritative starting SHA/tree;
- Linux distribution/kernel/filesystem;
- detected Landlock ABI and handled rights;
- exact backend/launcher identity;
- guard/lease binding used to derive confinement;
- effective read/write roots in privacy-safe canonical form;
- `no_new_privs`/restriction result;
- positive mutation cases;
- negative bypass cases and sentinel hashes;
- child/descendant inheritance proof;
- inherited-FD proof;
- capability-class resolution;
- unsupported/disabled/failure-mode proof;
- full Python and Rust regression counts;
- conformance counts;
- frozen-surface check; and
- cleanup state.

Suggested evidence id:

`20260915-050-linux-landlock-confinement`

## Stop conditions

Stop rather than manufacture assurance if:

- the running kernel cannot provide the minimum Landlock contract;
- the confinement ceiling cannot be derived from protected guard authority without trusting caller-controlled data;
- a required frozen-surface change lacks exact approval;
- a native bypass remains that invalidates `path_confinement` or `process_confinement`; or
- the implementation would require making a general container/runtime dependency part of RepoPact's canonical semantics.
