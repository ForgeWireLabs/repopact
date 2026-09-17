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

The v1 direction is a **GitHub App** with least-privilege user authorization rather than a classic broad OAuth App. Interactive authorization uses the browser-redirect authorization-code flow with PKCE (Decision 0062, superseding Decision 0061's original device-flow selection) -- never a reusable GitHub App private key, and the GitHub-named client secret is treated as packaged public application configuration, not confidential material. The native-client flow remains usable without requiring a ForgeWire Labs hosted authentication broker solely for repository import.

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

- **Checkpoint C — Bounded Snapshot Materialization — done.** See
  `evidence/runs/20260916-067-checkpoint-c-snapshot-materialization.json`.
  Landed: `WorkspaceManager::import_remote_snapshot`
  (`repopact-mobile-acquisition`), reusing WI065's *unmodified*
  staging-then-publish transaction and archive extractor, plus a purely
  filesystem-level post-extraction step that strips GitHub's synthetic
  zipball wrapper directory when requested; a real, bounded, streaming
  `StreamingDownloadTransport`/`ReqwestTransport::download` (a separate,
  longer-timeout HTTP client; manual redirect handling reusing Decision
  0061's Authorization-header allowlist; a 300MiB compressed-byte bound
  distinct from Checkpoint B's REST-JSON ceiling and from WI065's own
  expanded-byte bound); `GitHubProvider::describe_snapshot`/
  `open_snapshot` wired to the real `zipball` endpoint; the completed
  typed command surface (`remote_import_snapshot` + `remote_import_cancel`,
  accepting only typed repository/ref identifiers and re-resolving the
  exact commit SHA natively rather than trusting any value the frontend
  already displayed); and a real "Import Snapshot" UI action with
  cancellation and honest post-import offline/no-write-back messaging.
  **Live-proven** end-to-end against `octocat/Hello-World` (no
  authentication needed): ref resolution, real zipball download, the real
  `api.github.com` → `codeload.github.com` redirect, bounded streaming,
  real WI065 extraction, real publication with credential-free
  `RemoteSnapshot` provenance, the synthetic wrapper directory correctly
  stripped, and a fresh `WorkspaceManager` reopen reading the workspace via
  ordinary filesystem I/O with zero further network calls. GH-007, GH-008,
  and GH-009 become `satisfied`. GH-010 and GH-013 advanced substantially
  (redirect/auth/cancellation/adversarial-archive proof; download error
  mapping) but stay `pending` -- the full documented failure taxonomy
  (TLS failure, 5xx, redirect-loop-exceeded, truncated body) and a
  dedicated log-redaction sweep for the new download path were not
  exercised this checkpoint. GH-004/005/012/015 remain `pending`,
  unchanged -- still blocked on an operator registering a real GitHub App
  client ID and on Android protected credential storage.

- **Checkpoint D — Failure-Surface and Security/Privacy Closure — done.**
  See `evidence/runs/20260916-067-checkpoint-d-failure-security.json`.
  Closed the exact four gaps Checkpoint C left open: a real TLS
  certificate-validation failure through the production client (live,
  `self-signed.badssl.com`), real 5xx (500/503) handling, a real redirect-
  loop/`MAX_REDIRECTS`-exhaustion proof against a server that never stops
  redirecting, and confirmation that a truncated (short-Content-Length)
  transfer is already correctly rejected by the underlying HTTP stack.
  Fixed a real gap where a non-2xx download response body was embedded
  into an error message unredacted and only size-capped, not
  content-bounded -- now redacted and bounded, proven against a
  deliberately hostile response body containing a canary token and 10KB
  of padding. Added dedicated GH-013 evidence: token-canary redaction
  tests on `RemoteProviderError` construction itself (not only incidental
  coverage), and a structural regression test proving
  `repopact-mutation`/`repopact-graph`/`repopact-core` do not and cannot
  depend on any remote-provider crate, so GitHub metadata cannot reach
  governance/mutation authority even in principle. Hardened the typed
  command DTOs with `deny_unknown_fields` so an injected `url`/
  `destinationPath`/`headers` field is rejected at deserialization, not
  merely ignored. New `docs/guides/github-snapshot-import.md` documents
  the concretely-implemented behavior without speculating about
  private/organization/mobile behavior. GH-010 and GH-013 become
  `satisfied`; GH-015 stays `pending` (its private/organization/mobile
  documentation would still be speculative); GH-004/005/012 remain
  `pending`, unchanged.

- **Checkpoint E — Android Protected Credential Storage — done.** See
  `evidence/runs/20260916-067-checkpoint-e-android-protected-credentials.json`.
  New `repopact-mobile-credential` crate implements the existing
  `CredentialStore` trait against a real, non-exportable AES-256-GCM key
  generated inside Android Keystore (never the OAuth token itself stored
  in Keystore); only a versioned, ciphertext-only envelope reaches
  app-private SharedPreferences. Proven with real emulator evidence, not
  unit tests alone: put/get/overwrite/delete, distinct access/refresh and
  per-connection keys, persistence across a real process kill and
  restart, at-rest plaintext absence (`grep`, not visual inspection),
  zero logcat leakage of any synthetic canary across every operation, and
  three distinct real failure modes (missing key, corrupt/wrong-version
  envelope, authenticated-encryption tag failure) each surfacing a
  distinct typed error rather than plaintext, a crash, or a silently
  regenerated key. `AndroidManifest.xml` is byte-identical before and
  after -- no new permission. `GitHubProvider`/the GitHub connection
  command surface is deliberately *not* wired to run on Android in this
  checkpoint (that remains a separate integration step); this checkpoint
  proves the credential backend itself. GH-004 becomes `satisfied`;
  GH-012 records this as a satisfied prerequisite but stays `pending`
  (both the operator gate and the Android GitHubProvider wiring remain
  outstanding); GH-005/GH-015 unchanged.

- **Checkpoint F -- Browser-Redirect PKCE Authorization Revision -- done.**
  See `evidence/runs/20260916-067-browser-pkce-auth-revision.json` and
  Decision [`0062`](../../../decisions/0062-browser-redirect-pkce-authorization-replaces-device-flow.md),
  which supersedes only Decision 0061's interactive-authorization-flow
  section. Before registering the production GitHub App, the operator
  determined that the GitHub App device flow Checkpoints A-E built and
  proved -- copy a user code, open a browser, paste the code -- was not
  the intended installed-product Workbench UX. This checkpoint replaces
  it with a one-click "Connect GitHub" browser-redirect-with-PKCE flow on
  every platform, without touching repository listing, ref resolution, or
  snapshot download/materialization. Landed: RFC 7636 PKCE
  (`repopact-remote-provider::pkce`, CSPRNG verifier/state, `S256`
  challenge, verified against the RFC's own test vector); a revised
  `AuthState` (`StartingBrowserAuthorization`/`WaitingForCallback`/
  `ExchangingCode` replacing the device-flow-shaped `AwaitingUser`/
  `Revoked`); `repopact-provider-github::browser_flow` (authorization-URL
  construction, code-for-token exchange, refresh -- now correctly
  including GitHub's required `client_secret` field, since these tokens
  are no longer device-flow-issued); a one-shot native loopback callback
  listener (`repopact-provider-github::callback`, `127.0.0.1`-only,
  bounded request size, exact-path/single-terminal-callback semantics,
  proven against real local sockets) with 14 executable negative tests
  covering every attack Decision 0062 specifies (wrong/missing/replayed
  state, missing code, GitHub error callback, oversized request, wrong
  path, cancellation, retry-never-reuses-a-session, a second callback on
  an already-consumed session); `GitHubProvider::start_browser_authorization`
  orchestrating session generation, listener bind, URL construction, and
  a background worker that performs state validation, token exchange, and
  identity fetch, with a monotonic session generation counter so a
  superseded session's late-arriving result can never clobber a newer
  one; device flow itself (`device_flow.rs`) retained as a tested but
  unwired protocol library, per Decision 0062's explicit disposition; a
  typed `GitHubAppRegistration` (build-time `option_env!` values, with a
  `#[cfg(debug_assertions)]`-only developer environment-variable
  fallback) replacing the `REPOPACT_GITHUB_CLIENT_ID` production
  environment variable entirely; `remote_connect_start` now opens the
  system browser itself as one native operation (the separate
  `remote_open_verification_url` command is removed) and a new
  `remote_open_installation_page` command opens the GitHub App
  installation page from native `app_slug` configuration; and a rewritten
  `docs/guides/github-app-setup.md` describing the browser-redirect
  registration contract (Device Flow OFF, two callback URLs, no private
  key). GH-003 is re-satisfied with this checkpoint's evidence after being
  temporarily treated as pending refreshed evidence during implementation;
  no other acceptance criterion's state changed. GH-005/GH-012/GH-015
  remain pending, unchanged, on the operator registering the real GitHub
  App using this checkpoint's exact registration contract.

## Status

Active. Remaining work is entirely gated on an operator registering a real GitHub App using the registration contract in `docs/guides/github-app-setup.md` (GH-005, GH-012, GH-015's remaining sections) and on wiring GitHubProvider's browser-redirect connection command surface to actually run on Android (GH-012's remaining half -- the Android protected credential store it depends on is already implemented and proven, and Decision 0062 records the Android callback-transport disposition to use once that wiring happens). No further RepoPact-side implementation work is pending for the currently-scoped desktop/public-repository snapshot-import feature. WI067 is not closed by this checkpoint.
