---
id: 0060
title: Optional sandbox/process-enforced reference confinement
status: accepted
date: 2026-09-15
supersedes: []
---

# 0060: Optional sandbox/process-enforced reference confinement

## Context

Decision 0038 established RepoPact's vendor-neutral pre-execution admission model and Decision 0039 kept protected enforcement opt-in behind an adopter-neutral provider boundary. WI050's native Linux proof then established an important distinction with concrete evidence: the protected guard, repository registration, operator authority, leases, revocation, expiry, peer binding, restart invalidation, and fail-closed service loss all worked, while direct Python, POSIX-shell, and child-process writes outside the adapter still succeeded.

That result is not a failure of the protected guard. It demonstrates that **authority/admission** and **process/filesystem confinement** are separate security boundaries. A pre-action adapter can truthfully prevent a covered action from starting, but it cannot claim that an already-authorized arbitrary process is unable to write elsewhere unless an operating-system boundary actually constrains the process tree.

WI050 AC-16 deliberately requires that distinction. RepoPact therefore needs a path to the existing `sandbox/process-enforced` capability class without turning the project into a general container runtime or making one operating system the semantic source of truth.

## Decision

1. **`pre-action` remains RepoPact's portable reference enforcement baseline.** It is a valid and useful class, but it does not imply arbitrary-process path confinement.
2. **`sandbox/process-enforced` remains an optional higher-assurance class.** A repository or adapter may require it, but ordinary RepoPact adoption and the portable reference integration do not require every host to provide it.
3. **The policy core and protected guard remain the authority boundary.** A confinement backend consumes already-authorized, canonical RepoPact authority and may only enforce the same or a narrower boundary. It must never activate work, grant frozen approval, widen scopes or paths, extend expiry, alter delegation, or become a second work ledger.
4. **Linux Landlock is the first RepoPact-controlled reference confinement backend.** The backend is Linux-specific and replaceable. Landlock is selected because the Linux kernel can apply an unprivileged, stackable access-control domain to a process and its descendants; a successfully applied domain is inherited by subsequently created children and cannot be removed by the confined process, only further restricted.
5. **Confinement applies to the launched agent/session process tree, not to a hand-picked list of tools.** Wrapping `python`, a shell, Git, or an editor individually is insufficient because another child or direct filesystem API would remain an escape route.
6. **The effective confinement specification is derived from protected RepoPact authority, not caller-provided allowlists.** Canonical repository identity, work item, principal/session, lease validity, profile, scopes, paths, frozen authorization, expiry, revocation epoch, and relevant drift checks remain authoritative. A caller may request a narrower execution boundary but may not widen the guard-derived ceiling.
7. **The Linux backend must detect Landlock support and ABI capabilities at runtime.** It must handle only access rights the running kernel supports and must define a minimum filesystem-rights contract sufficient for the assurance it advertises. If that contract cannot be expressed or enforced, the backend is `not-covered` for `sandbox/process-enforced`.
8. **A required sandbox class fails closed.** If Landlock is unavailable, disabled, too old for the minimum contract, the confinement helper is missing or unhealthy, policy derivation fails, or the process cannot enter the domain, the child must not start. A request for `sandbox/process-enforced` must never silently downgrade to `pre-action`.
9. **`path_confinement` and `process_confinement` are evidence-backed capabilities.** They may be reported true only when backend-owned health/attestation and executable bypass proof establish that the launched process tree is actually constrained. Adapter declarations alone are not evidence.
10. **Network confinement is a separate capability.** Filesystem/process confinement must not imply or advertise network confinement unless a separate, proven mechanism enforces it.
11. **Inherited capabilities must be considered part of the launch boundary.** The reference launcher must avoid carrying unexpected writable file descriptors or equivalent pre-opened mutation capabilities into the confined process tree. Standard streams and any intentional descriptors require explicit treatment consistent with the claimed boundary.
12. **Windows and macOS may truthfully remain at lower enforcement classes until independent native confinement backends are implemented and proven.** Their absence does not weaken Linux's truthful capability class, and Linux Landlock does not become canonical cross-platform policy semantics. WI050 AC-18 remains the separate cross-platform native reference-proof gate.
13. **RepoPact does not become a general container runtime.** Namespace isolation, seccomp, bubblewrap, AppContainer, macOS sandboxing, containers, VMs, or downstream execution fabrics may become additional providers/backends where independently justified, but the public semantic contract remains the adopter-neutral enforcement/capability model.

## Linux reference execution contract

For the first backend, the expected launch sequence is conceptually:

1. authenticate to the protected guard and validate the repository/session/lease;
2. derive a canonical confinement specification from guard-authorized state;
3. resolve and validate filesystem roots without trusting mutable convenience links;
4. sanitize unexpected inherited file descriptors and privilege-gaining execution paths;
5. create a Landlock ruleset for the access rights supported by the running ABI and required by RepoPact's minimum confinement contract;
6. add read-only and writable path rules no broader than the authorized ceiling;
7. set `no_new_privs` or use the kernel-supported equivalent tied to successful restriction;
8. enter the Landlock domain;
9. execute the requested process without shell-string reinterpretation by the confinement launcher.

The exact Rust crate/application shape is implementation detail. The operating-system mechanism belongs below the RepoPact policy/authority layer.

## Required proof for `sandbox/process-enforced`

The reference backend must show both positive and negative behavior with real processes. At minimum, proof must cover direct filesystem APIs, POSIX shell mutation, child and descendant processes, nested working directories, linked worktrees, symlink and rename/link escape attempts, truncation/append/create/remove operations, unrelated repositories, protected guard/trust state, and frozen/out-of-scope paths. Authorized in-scope writes must still succeed. Denied targets must remain byte-identical where a sentinel can be used.

The proof must also demonstrate that an inherited writable descriptor or equivalent pre-opened capability cannot provide an unintended bypass of the advertised path boundary.

## Alternatives considered

* **Keep `pre-action` as RepoPact's highest possible class:** rejected because AC-16 and the Linux falsification show a meaningful, testable assurance level above interception, and the existing capability model already represents it.
* **Make sandboxing mandatory for all RepoPact adopters:** rejected because this would distort an OSS governance product into a host-runtime requirement and break Decision 0039's opt-in boundary.
* **Treat the protected guard as the sandbox:** rejected because authority verification and operating-system process confinement are different responsibilities and failure domains.
* **Wrap only known mutation tools:** rejected because arbitrary processes and direct APIs can bypass a tool list.
* **Use a container or VM as the canonical mechanism:** rejected as a semantic dependency. Such mechanisms may be valid replaceable providers, but they are not RepoPact authority.
* **Silently fall back to pre-action when the sandbox is unavailable:** rejected because it would overstate protection and violate the existing truthful capability model.

## Consequences

RepoPact gains a concrete path to close WI050 AC-16 without weakening its acceptance language. The first implementation tranche should add a small Linux confinement backend/launcher, preferably in the Rust workspace, bind it to protected guard-derived authority, retain existing `pre-action` behavior unchanged, and produce native Linux bypass evidence before advertising `sandbox/process-enforced`.

Windows and macOS confinement remain future replaceable backends. Their reference admission/protected-guard proofs remain part of AC-18, while their capability classes must continue to reflect what is actually enforced.

## Rejection and falsification

This decision must be revisited if any of the following is observed:

- a confined child or descendant can write outside the guard-authorized ceiling;
- a caller can widen confinement with command-line arguments, environment, mutable repo configuration, or a forged path list;
- symlink, rename, hard-link, truncation, inherited-file-descriptor, or equivalent filesystem behavior bypasses the claimed boundary;
- a required sandbox class silently degrades to pre-action or advisory execution;
- backend health or adapter booleans can claim sandbox enforcement without native proof;
- the Landlock backend becomes project authority rather than a consumer of protected RepoPact authority;
- Linux-specific details leak into canonical cross-platform policy semantics; or
- implementing the backend requires RepoPact to become dependent on a general container/orchestration runtime.

## References

- Decision 0038 — Vendor-neutral pre-execution admission and protected guard architecture.
- Decision 0039 — Opt-in enforcement provider boundary.
- WI050 evidence `20260915-050-linux-native-proof`.
- Linux kernel Landlock userspace API documentation; runtime ABI detection, `no_new_privs`, process-descendant inheritance, and monotonic restriction are treated as backend facts rather than RepoPact policy semantics.
