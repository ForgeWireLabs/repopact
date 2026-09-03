# WI050 architecture - admission, authority, and leases

> Architecture phase only. This record chooses public contracts and records
> implementation obligations; it does not add a guard, schema, adapter, or
> platform backend.

## 1. Scope and authority

RepoPact remains a repository-native governance product. The work item is the
canonical project authority. An admission lease is a short-lived enforcement
capability derived from that authority; it is never a second work ledger and
never changes a work item's lifecycle.

The existing lifecycle remains:

    capture work -> proposed -> operator authorization -> active
      -> implementation -> evidence -> completion -> promotion/release

WI050 inserts a pre-execution question between an incoming task/session and the
first covered mutation:

    task/session -> orientation -> admission request -> protected approval
      -> active lease -> guarded action

The policy core reads the existing charter, invariants, workflow, AGENTS
contracts, owners, work-item state, dependencies, frozen-surface declaration,
preflight marker, provenance, repository validity, and the decision-0032
development identity. It must not create a parallel project-authority record.

The operator's explicit WI050 authorization is for this architecture phase.
Implementation of the chosen guard, schemas, API, adapters, and platform
backends requires a later implementation pass and separate approval for every
frozen-surface change.

## 2. Admission contract

An adapter asks the policy core to admit an action before executing it. The
request includes:

* canonical repository identity and RepoPact root;
* principal and adapter/session identity;
* work-item id and action kind;
* candidate path set and requested capabilities;
* current HEAD/tree and authority-policy digest;
* requested profile and delegation lineage;
* normal or repair mode;
* frozen-surface hits and approval reference, if any.

For ordinary mutation mode the core evaluates, in deterministic order:

1. repository registration is present, resolves to this root/worktree, and the
   protected guard is healthy;
2. the work item is concrete, active, preflight-valid, and not blocked,
   completed, deferred, or dependent on proposed work;
3. owner and applicable AGENTS contracts are discoverable and the action scope
   is allowed by the work item/profile;
4. mandatory repository validation and decision-0032 development identity are
   valid for the policy epoch;
5. requested paths/capabilities are within the lease and do not cross the
   protected operator/trust plane;
6. frozen-surface mutation has the stronger operator approval class, not merely
   a caller-supplied check-frozen acknowledgement;
7. the principal, adapter, lease, base state, and policy digests match.

The result is a stable code plus safe diagnostics. A denial names the
repository registration, work item, policy check, and remediation class, but
never prompts, source content, credentials, or private receipt material.
Equivalent requests over equivalent canonical state produce equivalent
allow/deny decisions.

The action taxonomy is intentionally narrower than a shell parser:

* read/orient - no target mutation;
* propose/amend-proposal/request-approval - bounded bootstrap records only;
* mutate - a filesystem, process, Git, or external side effect;
* authority/trust change - never an ordinary lease capability;
* frozen mutation - mutation plus the frozen approval class;
* repair/reconcile - an explicitly scoped exceptional mutation.

An adapter may report an intended path, but a path report alone does not prove
confinement. An adapter claiming path/process enforcement must use an actual
sandbox, proxy, OS policy, or equivalent boundary that constrains arbitrary
child processes.

## 3. Canonical admission state machine

The state machine has durable repository facts, protected runtime state, and
ephemeral session state. It does not make the lease a durable work authority.

| State | Meaning | Durable/protected facts | Agent actions | Protected transition |
| --- | --- | --- | --- | --- |
| UNREGISTERED | No trusted RepoPact registration binds this checkout to a root. | None or an untrusted declaration. | Read discovery only; no covered mutation. | Operator-protected adopt/register creates the binding. |
| REGISTERED / ADOPTED | A repository declaration and protected registration agree on canonical identity and root. | Versioned declaration; protected registration/pin. | Start orientation; request bounded bootstrap. | Guard verifies declaration and registration. |
| ORIENTATION | A session has read-only access and a narrow bootstrap capability. | Session identity and audit-safe start receipt may be ephemeral. | Read Git/RepoPact state; create/amend proposed work; request approval. | Guard issues no mutation lease. |
| PENDING OPERATOR AUTHORIZATION | A canonical request is waiting for a trusted operator act. | Request/nonce and digest are auditable; pending storage may be protected. | Present/refresh request; cannot approve it. | Protected user-presence signer approves, rejects, or expires. |
| AUTHORIZED | A valid operator receipt matches the exact request digest. | Signed receipt is durable/auditable; private key is external. | Ask guard to mint a lease. | Guard verifies signature, trust pin, policy, and freshness. |
| ACTIVE LEASE | A short-lived, scoped capability is usable by one principal/session. | Protected lease state or signed lease; revocation source. | Perform only admitted actions within scope. | Every action is checked; drift or expiry ends it. |
| EXPIRED | Lease lifetime ended. | Receipt/history remains; capability is unusable. | Re-orient and request again. | New approval may create a new lease. |
| REVOKED | Operator or policy explicitly withdrew authority. | Revocation record/epoch is protected and auditable. | No mutation; may inspect diagnostics. | Only a new trusted approval can replace it. |
| INVALIDATED | Binding, policy, authority, repository, adapter, or guard state changed. | Invalidation reason and state digest are retained. | No mutation; request reauthorization. | Fresh request after drift can proceed. |

Durable transitions are registration, operator approval/rejection, revocation,
and work-item lifecycle changes. Session orientation and active lease state are
ephemeral or protected runtime state, with a receipt sufficient for later audit.
An agent can request all transitions that do not grant authority. It cannot
activate its own item, approve a request, rotate the trust root, or mint a
lease.

The guard invalidates a lease when any of these change: repository registration
or root, common Git directory, work-item bytes/status/dependencies, applicable
AGENTS or owner policy, authority declaration, frozen-surface declaration or
approval, policy/profile version, base HEAD/tree, adapter/session binding,
delegation parent, revocation epoch, or guard health. A non-mutating policy
refresh may extend a lease only by issuing a new protected lease; an agent
cannot silently heal stale authority.

## 4. Bootstrap without self-authorization

Orientation is a real, low-privilege state, not a promise made by a prompt.
The reference bootstrap surface is bounded and deny-by-default:

* work propose writes one schema-valid record below work/proposed and refuses
  path traversal, symlink escape, status other than proposed, and unrelated
  files;
* work amend-proposal may change only an existing proposed item and only fields
  allowed by its schema; it cannot set active, completed, an approval field, or
  a protected registration;
* approval request creates a canonical pending request and returns its digest;
* status, doctor, validate, Git status/log/diff, and contract reads are
  orientation operations;
* no bootstrap command accepts an arbitrary command string, script path,
  destination, or shell escape.

The bootstrap implementation must run through the protected guard even when
the caller is a CLI. It may use a temporary file and an atomic guarded commit
of the bounded record, but it must reject source files, schemas, invariants,
authority declarations, guard configuration, hooks, and settings. A proposed
record remains non-authority until an operator creates the trusted activation
receipt.

The existing check-frozen --ack is retained as advisory/backward-compatible
procedural UX. In enforced mode it is only an input to a request; it is never
operator proof unless the protected approval receipt binds the same frozen
change. A non-interactive operator CLI is likewise a request/verification
surface, not proof of human presence.

## 5. Approval, receipt, and lease protocol

The public request is UI-neutral. It is serialized using a deterministic
canonical JSON form (UTF-8, RFC 8785-style canonicalization) and hashed with
SHA-256. The reference signature suite is Ed25519 with an explicit algorithm
identifier so future suites can be negotiated without changing semantics.

The digest binds at minimum:

    request_id / nonce
    canonical repository identity and RepoPact root identity
    work-item id and requested transition/action
    base HEAD/tree and authority-state digest
    authority-policy version and frozen-surface digest
    requested approval class and authorization profile
    exact scopes, paths, capabilities, and delegation ceiling
    normal versus repair mode
    adapter/session/principal identity
    issued-at, expiry, and revocation epoch

The nonce is unique within the protected authority scope and never reused.
Changing any bound value requires a new request and signature. The operator
front end can be CLI/TUI, desktop, browser, IDE, mobile, chat, or a future UI;
none changes canonicalization, signature, or policy semantics.

An approval receipt contains the canonical request digest, operator/key id,
signature algorithm and signature, approval class, issue/expiry times, and
optional revocation reference. It contains no private key, passphrase, prompt,
source content, or bearer secret. The guard verifies the receipt against a
protected trust pin and the versioned authority declaration before minting a
short-lived lease.

The lease is bound to the request digest plus canonical repository, root,
work-item, base state, policy/profile, scopes, capabilities, adapter/session,
principal, expiry, revocation epoch, and delegation lineage. It is not
accepted for a different repository, work item, session, profile, or base
state. Replay, altered receipt bytes, changed request fields, expiry, revocation,
and drift all fail closed. A receipt may be retained for audit, but it is not
itself an executable capability.

## 6. Operator trust model

The repository contains recoverable, non-secret intent in a proposed
authority declaration (implementation proposal: governance/operator-authority
record). It may list operator ids, public verification keys/fingerprints,
roles, approval classes, profile references, delegation ceilings, quorum
rules, policy version, and rotation/recovery rules. It never contains private
signing credentials or user-presence secrets.

The protected guard stores the pinned authority digest/key set and the
registration binding outside ordinary agent-writable state. A declaration edit
is visible and reviewable but does not become trusted merely because it is
committed. Trust-root rotation requires an already-trusted operator transition,
with an overlap/recovery rule that prevents an agent from replacing the old
key and immediately using the replacement.

The minimum approval classes are:

* activate/authorize a proposed work item;
* authorize a bounded repair/reconciliation lease;
* approve frozen-surface mutation;
* approve scope/profile expansion;
* rotate or recover operator authority;
* authorize a delegation ceiling;
* revoke an authorization.

Quorum is optional policy, but a receipt always identifies the approving
principal(s) and class. Plain chat text such as "approved" expresses intent
only. A chat adapter may present a protected Approve action whose authenticated
user-presence result is then signed into the RepoPact protocol.

## 7. Authorization profiles

Profiles are adopter-configurable policy bundles, not agent types:

| Profile | Baseline capabilities | Deliberate limits |
| --- | --- | --- |
| observe | Read contracts, Git state, status, doctor, validate, and propose/request operations. | No implementation mutation, process side effect, frozen or trust change. |
| bounded | Mutate declared work-item paths and run an allowlisted test/build set. | Exact path/scope binding; no frozen/trust changes; short lease; no or minimal delegation. |
| standard | Ordinary implementation within owner/work-item scopes plus declared tests/builds. | No operator/trust changes, no frozen changes without stronger class, bounded network. |
| elevated | Broader declared process, network, or scope capabilities where the adapter truly enforces them. | Extra approval, shorter duration, explicit adapter capability proof, bounded delegation. |
| unrestricted-within-boundary | Broadest operator-approved repository/session scope. | Still cannot cross repository/WI, forge approval, alter the trust root, escape the OS boundary, or claim unsupported confinement. |

The UI may call the last profile "YOLO", but that name is not a kernel
semantic. Every profile declares readable/writable scopes, path rules,
process/shell/network capabilities, test/build permissions, repair eligibility,
approval cadence, duration, delegation ceiling, and truthful enforcement
requirements. A profile cannot authorize what its adapter cannot enforce.

## 8. Generic principals and delegation

A principal is a stable identity the guard can authenticate: protected session,
process, adapter, tool client, or external orchestration principal. RepoPact
does not model control agents, subagents, models, prompts, routing, or topology.

An operator-authorized parent may mint a child authorization only when:

    child authority subset of parent delegable authority

The child has the same canonical repository and work item unless a separate
operator class permits otherwise; subsets of paths, scopes, capabilities, and
profile; expiry no later than the parent; bounded delegation depth; immutable
parent id and lineage; inherited revocation; and no operator/frozen/trust
capability unless explicitly delegable. A child cannot modify its own parent
record, erase lineage, or mint a broader child.

Thus a downstream system can map control agent to parent principal and worker
to child principal without RepoPact knowing the topology. A child that asks
for more authority receives a new operator request, not an implicit extension.

## 9. Repair/reconciliation mode

Repair is an explicit profile and approval class, never a fallback from denied
ordinary work. A repair request includes the diagnosed violations, exact files
or paths where practical, intended corrective operation, base state, expiry,
and an evidence/receipt destination. It is narrower than standard work:

* only listed records and deterministic repair commands;
* no arbitrary source implementation, dependency installation, trust-root
  rotation, or profile expansion;
* no frozen mutation unless separately approved;
* short lease, no delegation by default, and normal revocation/drift rules.

WI049 is the motivating pattern: an already-landed historical runtime change
can be documented and reconciled under a new work item, but the reconciliation
does not relabel the original commit as preflight-authorized. If repair reveals
new scope, the guard stops and requests a new approval.

## 10. WI044, WI047, and WI046 boundaries

| Work | Question | WI050 relationship |
| --- | --- | --- |
| WI050 | May covered mutation begin or continue? | Owns admission, leases, guard health, and pre-action denial. |
| WI044 | Does completion evidence prove the semantic claim? | Future canonical evidence requirements may be read by the policy core; WI050 does not define typed completion semantics. |
| WI047 | Was documentation impact explicitly resolved? | Future canonical documentation requirements may be checked when active; WI050 does not duplicate closure rules. |
| WI046 | Did verification run and prevent invalid promotion? | WI050 may expose a completed-work state to it; WI050 does not own post-work promotion admission. |

No proposed WI044/WI047 requirement becomes an implementation dependency
before that work is accepted and canonical. WI046 remains the later
verification/promotion checkpoint.

## 11. Compatibility and semantic versioning

The design is additive and opt-in at adoption time. Existing repositories
remain valid and continue to use instruction, validator, Git, and CI
backstops until they declare required admission coverage. A new repository or
an upgraded adopter can declare a policy such as observe, session-start, or
enforced pre-action/sandbox coverage. An adapter configured as enforced fails
closed when the protected guard is absent, unhealthy, stale, or tampered.

Unsupported host capabilities are reported as degraded or not-covered; they
are never promoted to enforced by label. A repository may still use RepoPact
without a vendor product, network, or permanent daemon. A semantic change to
existing record schemas or universal lifecycle rules would require a versioned
breaking decision; the architecture itself does not change VERSION or
RELEASE_LABEL.

## 12. Falsification gates for implementation

The implementation is inadequate if any controlled test shows:

* a proposed item can self-activate or --ack can create frozen authority;
* changing cwd, path casing, symlink/junction, linked worktree, session,
  subagent, adapter, or host escapes the same registration;
* arbitrary PowerShell, POSIX shell, Python, editor, or child-process writes
  bypass a claimed path-enforced adapter;
* copied approval works for another repository, work item, base state, profile,
  or session;
* a child exceeds any parent subset, expiry, depth, revocation, or lineage;
* editing/removing repo-local instructions, hooks, settings, or adapters
  disables an enforced guard;
* guard crash, tamper, missing protected state, or unsupported capability
  silently downgrades enforced to advisory;
* Windows, Linux, and macOS differ in allow, deny, expiry, revocation, scope,
  or self-authorization semantics;
* chat text or a non-interactive CLI is accepted as operator proof;
* RepoPact requires ForgeWire Fabric or another closed product to run;
* repair mode becomes general implementation authority.

These gates are implementation acceptance obligations, not evidence claimed by
this architecture phase.
