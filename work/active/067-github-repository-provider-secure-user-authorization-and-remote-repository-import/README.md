# WI067 — GitHub Repository Provider, Secure User Authorization, and Remote Repository Import

> **Status**: 🔨 Active
> **Coding agent**: Claude Code. **Architecture reviewer**: GPT-5.6 Sol High.
> **Depends on**: WI065 (completed).

## Purpose

RepoPact repository acquisition must not require the repository to already exist as a local filesystem tree or user-selected archive. GitHub is the first remote repository provider: a user should be able to connect GitHub, select an authorized repository and ref, and have RepoPact materialize an exact commit snapshot into the same ordinary local/app-private workspace model already proven by WI065.

This work item deliberately separates **remote snapshot acquisition** from **true Git synchronization**. V1 does not pretend a downloaded GitHub snapshot is a clone, does not silently push local mutations back to GitHub, and does not require remote write permissions. Embedded Git clone/fetch/pull/push remains the later Stage-2 path established by Decision 0056.

## Architectural invariant

```text
remote provider
    |
    v
authorized repository/ref selection
    |
    v
resolve immutable commit SHA
    |
    v
bounded snapshot materialization
    |
    v
ordinary local/app-private filesystem workspace
    |
    v
unchanged DesktopService / RepositorySession / mutation / graph / governance core
```

GitHub is a provider of acquisition material and provenance. It is not a second `Repository` implementation and is never repository governance authority.

## Authentication direction

The preferred v1 direction is a **GitHub App** with least-privilege user authorization rather than a classic broad OAuth App. Implementation must verify the current GitHub-native-client flow before landing, but it must not ship a reusable GitHub App private key or pretend an embedded client secret is confidential. The native-client flow must remain usable without requiring a ForgeWire Labs hosted authentication broker solely for repository import.

For v1 snapshot import, request only the permissions actually needed to enumerate authorized repositories/refs and read repository contents/archive material. Do not request remote write/push permission merely because future embedded Git may need it.

Tokens and refresh material belong only in OS-protected credential storage. They never belong in RepoPact repositories, the workspace registry, logs, evidence records, or ordinary frontend persistence.

## User experience

The Workbench repository-acquisition surface should evolve toward:

```text
Open / Import Repository

- Local folder
- ZIP/archive
- GitHub
- future providers: GitLab / Forgejo / generic Git
```

A GitHub-connected user should be able to:

1. connect/authorize GitHub;
2. see only repositories permitted by both the user and GitHub App installation;
3. browse/search personal and organization repositories where authorized;
4. select a repository;
5. choose a branch, tag, or exact commit;
6. resolve movable refs to an immutable commit SHA;
7. import that exact snapshot into a normal RepoPact workspace;
8. work offline after import;
9. disconnect GitHub without invalidating already-materialized workspaces.

The UI must label this honestly as **snapshot import** until the later embedded-Git synchronization work exists.

## Provider-neutral backend

Implementation should establish a provider-neutral remote source seam, conceptually similar to:

```text
RemoteRepositoryProvider
    authenticate/connect
    disconnect
    list accounts/installations
    list repositories
    list refs
    resolve ref -> immutable revision
    materialize snapshot
```

Exact names are deferred to architecture review. GitHub-specific REST/auth behavior remains behind that boundary. Workspace publication must reuse the existing RepoPact acquisition pipeline rather than implementing a second GitHub-specific extractor.

## Reuse of WI065

WI065 already establishes the important primitives this item should reuse:

- app-private/local workspace registry;
- staging-then-publish transaction;
- bounded resource accounting;
- path-containment rules;
- duplicate/case-conflict handling;
- safe ZIP/archive extraction;
- cleanup after cancellation/failure;
- normal `DesktopService` / `RepositorySession` opening;
- typed command/error/progress boundaries.

Remote acquisition should feed those primitives, not duplicate them.

## Snapshot provenance

A GitHub-imported workspace should retain bounded non-secret acquisition provenance such as:

- provider (`github`);
- repository stable identity and owner/name;
- GitHub installation/account identity where appropriate;
- selected branch/tag/ref text;
- resolved immutable commit SHA;
- acquisition timestamp;
- snapshot-import acquisition kind;
- source URL/identifier only in a non-credential-bearing form.

This is local product provenance, not RepoPact governance authority.

## Security boundary

The remote-provider layer must not expose:

- a generic authenticated HTTP proxy;
- arbitrary GitHub REST calls from the frontend;
- arbitrary URL download;
- token-returning commands;
- credential-bearing URLs;
- generic shell/process execution;
- caller-controlled filesystem destinations.

Authentication, repository enumeration, ref resolution, download, cancellation, and materialization remain typed operations owned by the native/backend layer.

Logs and evidence must redact tokens, authorization codes, refresh material, credential-bearing URLs, and sensitive response headers.

## Offline and failure semantics

Once materialized successfully, the repository is an ordinary workspace and must remain usable without GitHub connectivity. Authentication expiration or disconnect affects future remote operations, not the validity of an already-created local workspace.

Network/API/rate-limit/auth/revocation/download failures must leave no ready workspace and no credential-bearing temporary state. Partial downloads are staging artifacts and are cleaned/quarantined through the existing acquisition transaction semantics.

## Stage-2 boundary

This item does **not** implement real Git clone/fetch/pull/push synchronization. That future work should use the embedded `GitBackend` direction established by Decision 0056 and should make any required remote-write permission escalation explicit. GitHub API snapshot import must not evolve into an ad-hoc pseudo-Git synchronization protocol.

## Compatibility

GitHub integration is optional. Existing repositories and users who never connect GitHub must continue to work with no mandatory network, account, token, or provider dependency.

## Progress

- **Checkpoint A — Provider-Neutral Architecture, GitHub Auth Contract, and
  Remote Snapshot Foundation — done.** See
  `architecture-review.md`, Decision 0061, and
  `evidence/runs/20260915-067-checkpoint-a-provider-auth-architecture.json`.
  Landed: a provider-neutral core crate (`repopact-remote-provider`:
  `RemoteRepositoryProvider` trait, `AuthState` machine, `CredentialStore`
  trait + in-memory test double, typed `RemoteProviderError` taxonomy,
  `Secret`/`redact` credential hygiene, a `FakeProvider` proven end-to-end
  through WI065's *existing* archive materializer with zero GitHub-specific
  branching); a GitHub adapter crate (`repopact-provider-github`: the
  device-flow protocol against a mocked `HttpTransport`, the exact
  permission matrix, the API version constant, and the trusted-origin/
  header-authorization allowlist); a new, distinct
  `AcquisitionKind::RemoteSnapshot` and `RemoteSnapshotProvenance` on the
  workspace registry (never to be confused with WI068's `RemoteGit`); and
  a GitHub App registration spec (`docs/guides/github-app-setup.md`) with
  no secret values, so an operator can register a real app for a later
  checkpoint to consume. GH-001, GH-002, GH-003, GH-011, and GH-014 are
  `satisfied`; all other criteria remain `pending` -- no live GitHub
  authorization, no OS credential-store backend, no repository-browsing
  UI, and no production Tauri commands exist yet. Not started: Checkpoint
  B and beyond (real REST client, live device-flow UX, platform credential
  backends, repository/ref browsing, typed commands, runtime evidence).

- **Checkpoint B — Production GitHub Connection, Protected Credentials,
  Typed Native Commands, and Live Authorization — done.** See
  `evidence/runs/20260915-067-checkpoint-b-production-github-connection.json`.
  Landed: a real `reqwest`-based `ReqwestTransport` (rustls-tls, explicit
  timeouts, manual redirect handling enforcing Decision 0061's
  `Authorization`-header allowlist -- proven against real local sockets,
  not only unit-tested policy logic); a real, typed GitHub REST client
  (`rest.rs`: current user, installations, installation repositories with
  pagination, branches/tags, and immutable-revision resolution including
  annotated-tag peeling via the Git Data API) proven live against
  unauthenticated public GitHub endpoints (`octocat/Hello-World`,
  `torvalds/linux`'s real annotated release tags) -- no client ID needed
  for this proof; a real OS-protected `OsCredentialStore` (Windows
  Credential Manager via `keyring`) proven with real put/get/delete
  round-trips and a genuine cross-process write-then-read-then-delete
  sequence (three separate process invocations, cross-checked against the
  real OS store via `cmdkey`); native token refresh with atomic
  replacement and typed expired/denied/no-refresh-token failure paths,
  tested with an injectable clock; a `RemoteProviderService` Tauri-managed
  state owner wiring all of the above; the full typed command surface
  (`remote_provider_capabilities`, `remote_connections`,
  `remote_connect_start/status/cancel`, `remote_open_verification_url`,
  `remote_disconnect`, `remote_accounts`, `remote_repositories`,
  `remote_repository_refs`, `remote_resolve_ref` -- `remote_import_snapshot`
  deferred to Checkpoint C); a `remote-api.ts`/`RemoteRepositoryPanel.tsx`
  first-class "GitHub" entry point in the repository-acquisition UX,
  labeled "Snapshot"/"Resolved commit" throughout, never "Clone"/"Pull"/
  "Push"/"Sync"; and a real Windows Workbench launch proving the exact
  operator gate (`ProviderNotConfigured`, surfaced honestly in the UI)
  since no GitHub App has been registered yet. GH-006 additionally becomes
  `satisfied` (real, live-proven branch/tag/commit resolution and the
  provenance model). GH-004/005/007/008/009/010/012/013/015 remain
  `pending` -- live device-flow authorization, repository browsing, and
  snapshot import are all blocked on an operator registering a real GitHub
  App client ID, and Android credential storage remains an explicit,
  recorded gap rather than a plaintext fallback.

## Status

Active (Checkpoint B complete; Checkpoint C not started -- blocked on operator GitHub App registration for live end-to-end auth).
