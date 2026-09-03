# Pre-execution admission

RepoPact's WI050 admission plane is opt-in. It is useful when a host must ask
whether a session or action may mutate a repository before the mutation occurs.
Installing or adopting RepoPact does not require this capability, a privileged
daemon, or a particular host product.

## Standalone and opted-in modes

With no `governance/admission-policy.json`, or with that policy's `enabled`
field set to `false`, the ordinary RepoPact core remains valid. `evaluate_action`
and `repopact admission status` report `enforcement_required=false` and the
truthful `instruction-only` class; no lease, provider, or guard prerequisite is
created. Existing adopters can therefore continue to use work items, preflight,
validation, doctor, dashboard, and other governance commands independently.

An adopter that sets `enabled=true` explicitly opts into the policy's
`minimum_enforcement`. A missing or weaker provider is then unsatisfied even
when `failure_mode` is `degraded`; degraded mode changes diagnostics, not the
required assurance. The canonical resolver reports adapter class, provider
class, and their effective intersection.

Run `repopact admission setup --root PATH --key-file OUTSIDE_REPO` once during
operator-controlled setup. The command writes three public, schema-validated
records under `governance/` and a trust-pinned registration under an external
protected directory. Private Ed25519 key material is encrypted and never placed
in the repository. Existing protected registration is not silently replaced.

`repopact admission begin --work-item NNN --session ID` creates a canonical
request. An operator signs that request with the approval CLI in an interactive
terminal. The guard verifies the receipt and issues a short-lived lease. Every
mutation, process, repair, or frozen-surface action must present that
operator-derived lease; only read/orientation and bounded proposed-work or
approval-request operations are lease-free. Lease paths, scopes, profile, mode,
capabilities, session, repository, work item, base, expiry, and revocation epoch
are checked again at action time. Every pre-action adapter calls the guard before
its callback; a denial means the callback is not invoked. `repopact admission status` and `repopact doctor`
expose safe health diagnostics only.

The public `EnforcementProvider` SPI is implemented by any adopter-owned
provider with health, discovery, authorization, check, delegation, and revoke
operations. The built-in `NativeGuardClient` is one reference implementation;
it does not define the SPI, and downstream providers do not become RepoPact
dependencies. The adapter SPI reports facts, not a marketing flag. The reference coding
adapter provides pre-action checks; the independent launcher adapter gates
child creation but reports its lower session-start class and does not claim
arbitrary child process or filesystem confinement. Host backends can report
`sandbox/process-enforced` only after a real OS boundary is installed. MCP,
hooks, repository settings, and shell wrappers are integration surfaces and
backstops, never the sole trust root.

Bootstrap commands `repopact work propose` and `repopact work amend-proposal`
cannot transition proposed work to active or fabricate an operator receipt.
Admission setup, init/adopt admission, and revocation require an external
encrypted key plus interactive operator presence; unattended setup never creates
an ephemeral trust root. Requests and receipts are written only to their
canonical evidence directories. Frozen-surface changes require a receipt bound
to the exact declared frozen-surface digest; `check-frozen --ack` remains
advisory. The reference filesystem guard reports `protected=false` because a
same-principal process can rewrite the adjacent HMAC key and state; a host must
provide the missing process/path boundary before claiming sandbox enforcement.

## Host-protected guard substrate

`repopact guard status` reports backend-owned attestation fields including the
installed runtime path, protected state path, service identity, IPC endpoint,
integrity check, and whether the gated principal is actually excluded from
maintenance. A caller cannot promote `not-covered` to `enforced` by setting a
boolean. On Windows, `repopact guard install --root <repo>` requires an elevated
operator token and installs the runtime outside the checkout as the
`RepoPactGuard` LocalSystem service using an authenticated named pipe. The
follow-up `repopact guard register --root <repo> --key-file <external-key>`
binds the repository to the service-owned state; private keys remain external.
`guard uninstall` is likewise elevation-gated. Missing, stopped, tampered, or
unattested native services fail closed; the session-start launcher does not
claim arbitrary child path/process confinement.

Linux uses a system service, restrictive state directory, and authenticated
Unix socket when installed; macOS uses a protected launch daemon and local IPC.
Neither same-user service class is reported as protected without verified host
ownership. Run `python -m repopact.run_admission_platform_conformance` for the
portable semantic corpus; `--require-installed` additionally requires the
native platform guard to be installed and healthy.
