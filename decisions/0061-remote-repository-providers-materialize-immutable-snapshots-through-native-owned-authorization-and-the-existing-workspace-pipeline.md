---
id: 0061
title: Remote repository providers materialize immutable snapshots through native-owned authorization and the existing workspace pipeline
status: accepted
date: 2026-09-15
supersedes: []
---

# 0061: Remote repository providers materialize immutable snapshots through native-owned authorization and the existing workspace pipeline

## Context

WI067 (`work/active/067-github-repository-provider-secure-user-authorization-and-remote-repository-import/`)
asks RepoPact to acquire a repository from GitHub, so that repository
acquisition is not limited to local filesystem/SAF/archive sources
(Decision 0056/0057, WI065). This is architecturally adjacent to but
distinct from WI068 (Embedded Mobile Git Backend and Remote Repository
Synchronization, `work/proposed/068-.../`), which will implement real
`clone`/`fetch`/`pull`/`push` against a real `.git` working tree. This
decision governs WI067 only.

`DesktopService`, `RepositorySession`, and `RepositoryTopology`
(`rust/crates/repopact-desktop-api/src/lib.rs`,
`rust/crates/repopact-repository/src/lib.rs`) are `PathBuf`-based and
already provider-agnostic: they do not know or care whether a directory
under `workspaces/<id>/repository/` arrived via a local folder pick, SAF
import, archive import (WI065), or a downloaded GitHub archive. WI065
already proved this pattern by adding an entirely new acquisition adapter
with zero change to any of the three. This decision extends that same
pattern to a remote provider rather than inventing a new repository model.

## Decision

### Provider-neutral core, GitHub as first adapter

A new crate, `repopact-remote-provider`, defines a provider-neutral seam:
`RemoteRepositoryProvider` (connection lifecycle, account/repository/ref
enumeration, ref resolution, snapshot description/download),
provider-neutral DTOs (`RemoteAccount`, `RemoteRepository`, `RemoteRef`,
`ResolvedRevision`, `SnapshotDescriptor`, `SnapshotArtifact`), an explicit
`AuthState` state machine, a `CredentialStore` trait, and a typed
`RemoteProviderError` taxonomy. GitHub-specific behavior lives entirely in
a second crate, `repopact-provider-github`, which is the only place the
string `"github"` or a GitHub endpoint URL may appear in production
non-test code. Neither `DesktopService`, `RepositorySession`,
`mobile_acquisition.rs`, nor any React component may reference GitHub
directly (GH-001, GH-011).

Executable proof, not naming alone: `repopact-remote-provider` ships a
`FakeProvider` implementing the same trait, and
`repopact-remote-provider/tests/provider_neutrality.rs` proves its output
(a resolved immutable revision and a staged snapshot artifact) flows
through WI065's *existing*, unmodified
`repopact_mobile_acquisition::archive::import_archive` with zero
GitHub-specific branching anywhere on that path.

### GitHub App, not a classic OAuth App

GitHub App is chosen over a classic OAuth App because App installations
let repository access be scoped by app permissions, installation
repository selection, and user permissions jointly -- a materially
narrower blast radius than a classic OAuth App's all-authorized-repos
default. v1 requests exactly two permissions:

| Permission | Level | Why |
|---|---|---|
| `contents` | `read` | List branches/tags, resolve a ref to a commit SHA, download a repository archive. |
| `metadata` | `read` | GitHub's baseline permission for any repository the installation can see at all; recorded explicitly rather than left implicit. |

v1 explicitly does **not** request `contents:write`, `administration`,
`actions` (any level), `workflows`, or `pull_requests:write` (GH-002,
GH-014). The exact endpoint -> permission matrix is recorded in
`rust/crates/repopact-provider-github/src/permissions.rs` and is a
compile-checked table, not prose alone (see its own tests).

### v1 native authorization flow: GitHub App device flow

Three flows were compared against GitHub's own current documentation
(consulted live during this checkpoint, 2026-09-15: GitHub's OAuth-app
authorization docs and its GitHub-App user-access-token generation docs).

**Option A -- web application flow + PKCE.** Normal browser-redirect UX,
and PKCE is genuinely supported (`code_challenge`/`code_verifier`).
However, GitHub's own documented token-exchange step for this flow still
lists `client_secret` as a required parameter regardless of PKCE. A
`client_secret` embedded in a distributed native binary (Windows desktop
or an Android APK) cannot be kept confidential -- extracting it is a
matter of unpacking the binary, not compromising an account. Labeling such
an embedded value "confidential" would be false. **Rejected** for v1: it
requires a secret this class of client cannot protect, contradicting
GH-003's requirement that the selected flow be "appropriate for a
public/native client."

**Option B -- GitHub App device flow.** GitHub's documentation is
explicit that "unless your app uses the device flow," a client secret is
required to generate access tokens -- device flow is the one documented
exception. Verified request/response shape:

- `POST https://github.com/login/device/code` with only `client_id` (no
  secret) returns `device_code`, `user_code`, `verification_uri`
  (`https://github.com/login/device`), `expires_in` (900s default), and
  `interval`.
- `POST https://github.com/login/oauth/access_token` with `client_id`,
  `device_code`, and `grant_type=urn:ietf:params:oauth:grant-type:device_code`
  -- again, no `client_secret` field, confirmed against GitHub's own
  device-flow documentation and reproduced in
  `repopact-provider-github/src/device_flow.rs`'s tests, which assert no
  request ever includes a `client_secret` field.
- Polling errors are exactly `authorization_pending`, `slow_down` (with a
  server-supplied new `interval`), `expired_token`, `access_denied`, and
  `device_flow_disabled` -- all five modeled as distinct
  `DevicePollOutcome` variants, not collapsed into one generic failure.
- GitHub's own guidance names this flow for exactly RepoPact's situation:
  "CLI tools ... and desktop applications should use the device flow."

No GitHub App private key is ever needed for this flow (installation-token
minting via the private key is a server-to-server capability WI067 v1
does not use -- see "Private key disposition" below), and no hosted broker
is needed either. **Selected for v1.**

**Option C -- hosted ForgeWire Labs auth broker.** Would let a client
secret stay server-side, but introduces a hosted service, a network
dependency for every authorization, and an ongoing operations/privacy
burden. WI067's own GH-003 explicitly states a hosted service is not
required merely to import a repository. **Rejected for v1** -- nothing
about device flow's phishing/UX tradeoffs below rises to a level that
justifies taking on hosted infrastructure instead.

Device flow's known risk is user-code phishing (a malicious actor tricking
a user into entering *their* device code at github.com/login/device,
authorizing the attacker's session). This is mitigated by: opening only
the system browser (never an embedded, credential-capturing WebView) at a
compile-time-fixed, allowlisted origin (`redirect_policy::
TRUSTED_DEVICE_VERIFICATION_ORIGIN = "https://github.com/login/device"`,
never an arbitrary `verification_uri` taken from response data without
validation -- `device_flow::start_device_flow` rejects an untrusted one
outright); a short-lived authorization session bounded by GitHub's own
`expires_in`; strict adherence to the server-provided polling `interval`
(honoring `slow_down` rather than hot-looping); an explicit, always-
available Cancel; and no background/silent re-authorization.

### Private key disposition

WI067 v1 never ships, requests, or requires a GitHub App private key.
Installation-token minting (which requires the private key, held only by
the App's own backend in GitHub's model) is not used; the "GitHub App user
access token" produced by the device flow is sufficient for every v1
operation (repository listing, ref resolution, archive download) at the
intersection of user authorization and app installation permissions.

### Client secret disposition

WI067 v1 never ships or requires a GitHub App client secret. The device
flow's token exchange does not accept one; a `client_id` alone (not
confidential, safe to embed) is sufficient.

### Token expiration and refresh

GitHub App user access tokens, per GitHub's own current documentation,
expire after 8 hours (`expires_in` = 28800 seconds) when the app's
"Expire user authorization tokens" setting is enabled, with a refresh
token valid for 6 months without use (`refresh_token_expires_in` =
15897600 seconds); using the refresh token issues a new access token and a
new refresh token. RepoPact's GitHub App registration (see the setup
document below) enables this optional expiration -- it is not disabled for
implementation convenience (WI067 item 13). `AuthState` models
`Refreshing` and `Expired` as explicit states rather than treating a
token's death as an undifferentiated failure.

### Credential storage boundary

`repopact-remote-provider::credential::CredentialStore` is the native-
owned abstraction (`put`/`get`/`delete`, keyed by a typed
`CredentialKey{provider, connection_id, kind}`). The frontend never
receives a raw access/refresh token; only `AuthState` (which by
construction carries no credential field) crosses the Tauri command
boundary once commands are wired in a later checkpoint. Checkpoint A ships
the trait plus a process-memory-only `InMemoryCredentialStore` test
double; real backends target Windows Credential Manager, macOS/iOS
Keychain, Android Keystore-backed storage, and Linux Secret
Service/libsecret respectively, per platform, in a later checkpoint. None
of `registry.json`, plain JSON, frontend `localStorage`/`sessionStorage`/
IndexedDB, repository files, or environment variables may ever hold a
token.

### Snapshot vs. Stage-2 Git

A WI067 acquisition is `AcquisitionKind::RemoteSnapshot` (new in this
checkpoint) -- an immutable archive materialized once, with no `.git`
directory and no live remote relationship afterward. It reuses WI065's
existing safe archive materializer unmodified. `AcquisitionKind::RemoteGit`
(reserved by Decision 0056/0057) remains WI068's alone: a real `.git`
working tree with real `clone`/`fetch`/`pull`/`push`. The two must never be
conflated, even though both originate from a Git hosting provider.

### Redirect and header policy

`repopact-provider-github::redirect_policy` fixes the trusted device-flow
verification origin and records (for the archive-download implementation a
later checkpoint lands) that an `Authorization` header may only follow a
redirect to `api.github.com` or `codeload.github.com` -- GitHub's own REST
and archive-download hosts -- never an arbitrary redirect target. No live
download exists yet to exercise this against real traffic; it is recorded
here as the binding design constraint for when one does.

## Consequences

- No provider-specific `Repository`, `DesktopService`, `RepositorySession`,
  mutation, graph, or governance authority is introduced (GH-001).
- A later checkpoint selects a concrete HTTP client library behind
  `repopact-provider-github::transport::HttpTransport` to perform real
  network I/O (repository listing, ref resolution, archive download);
  Checkpoint A deliberately does not make that selection, since nothing
  yet needs to perform a live call.
- A live, operator-registered GitHub App (client ID) is required before
  any real authorization can occur; see the accompanying GitHub App setup
  document. No such registration exists yet, and none is invented by this
  decision.
- WI068's `GitBackend` seam is unaffected by this decision; the two items
  may later share provider/account UX, but that sharing is not assumed
  here.
