---
id: 0062
title: Browser-Redirect PKCE Authorization Replaces Device Flow as RepoPact's Interactive GitHub Authentication Path
status: accepted
date: 2026-09-16
supersedes: []
---

# 0062: Browser-Redirect PKCE Authorization Replaces Device Flow as RepoPact's Interactive GitHub Authentication Path

## Context

Decision [`0061`](0061-remote-repository-providers-materialize-immutable-snapshots-through-native-owned-authorization-and-the-existing-workspace-pipeline.md)
selected the GitHub App **device flow** for WI067 v1's interactive
authorization, on the grounds that it was, at the time, the one GitHub App
flow whose documented token-exchange step never required a `client_secret`
-- a genuine advantage for a distributed native client that cannot keep
one confidential. That decision was made before WI067's product UX was
reviewed end-to-end against what installing and using RepoPact actually
looks like for a normal user, and before the production RepoPact GitHub
App was registered.

Re-reading the current, live WI067 state before this revision: GH-001,
GH-002, GH-004, GH-006, GH-007, GH-008, GH-009, GH-010, GH-011, GH-013, and
GH-014 are satisfied; GH-003 was satisfied under the device-flow decision
and is treated as requiring refreshed evidence for the remainder of this
checkpoint; GH-005, GH-012, and GH-015 remain pending, gated on an operator
registering the real GitHub App. WI067 is not closed by this decision.

The device-flow UX that Decision 0061 built and Checkpoint A-E proved out
is real, tested, and correct as far as it goes: open a code, an eight-hour
token, an explicit refresh model, no client secret, no private key. But
its actual end-user shape is:

```text
Connect GitHub
    |
    v
copy a user code
    |
    v
open github.com/login/device in a browser
    |
    v
paste the code
    |
    v
RepoPact polls until authorized
```

That is not the installed-product experience RepoPact's Workbench should
ship before registering the production GitHub App. A normal user should
never copy a code, open PowerShell, set an environment variable, or paste
anything back into RepoPact. This decision revises the authorization
mechanism -- and only the authorization mechanism -- to a one-click
browser-redirect flow with PKCE, matching how essentially every other
installed desktop/mobile application that integrates with GitHub, Google,
or a similar identity provider actually behaves.

## What this decision does and does not change

This decision governs **only the interactive user-authorization
mechanism** WI067 uses. It does not revisit and does not change:

- the provider-neutral core architecture (`repopact-remote-provider`,
  `repopact-provider-github`, `RemoteRepositoryProvider`);
- the choice of a GitHub App over a classic OAuth App;
- the least-privilege `contents:read`/`metadata:read` permission model;
- credential-storage ownership (OS-protected `CredentialStore` backends);
- immutable-snapshot acquisition semantics
  (`AcquisitionKind::RemoteSnapshot`) or its separation from
  `RemoteGit`/Stage-2 real Git synchronization (Decision 0056);
- the absolute rule that RepoPact never ships or requires a GitHub App
  private key.

Decision 0061 remains authoritative for all of the above (see the partial
supersession note appended to its own text). This decision supersedes only
its "v1 native authorization flow: GitHub App device flow" section and the
client-secret/private-key disposition text that followed directly from
that choice.

## Why the flow is changing: current GitHub documentation, re-verified

Re-verified against GitHub's own current (2026-09-16) documentation for
GitHub App user authorization, consistent with what Decision 0061 already
found and what this decision now acts on differently:

- A public/native client (a distributed desktop binary or Android APK)
  **cannot** keep an embedded value confidential. Anyone can extract it
  from the binary; this is a property of the distribution model, not an
  implementation defect RepoPact could fix.
- GitHub's own web/OAuth authorization-code token-exchange endpoint
  (`POST https://github.com/login/oauth/access_token`) still documents
  `client_secret` as a required parameter for this flow, PKCE or not.
  GitHub does not offer a first-party "public client, no secret, browser
  redirect" combination for a GitHub App's user-to-server token exchange.
- GitHub's authorization-code flow **does** genuinely support PKCE
  (`code_challenge`/`code_challenge_method=S256`/`code_verifier`), and
  GitHub's own guidance recommends authorization-code-with-PKCE for public
  clients where a redirect-capable browser is available.
- GitHub's device-flow documentation cautions that device flow exists for
  constrained/headless/input-limited devices, and separately, GitHub's
  broader OAuth guidance warns that unconstrained use of device flow opens
  a phishing/remote-impersonation risk (a malicious actor tricking a user
  into entering *their own* device code at a legitimate-looking prompt,
  authorizing the attacker's session instead of the user's). RepoPact's
  Workbench is not a constrained/headless device -- it is a normal desktop
  or mobile application with a normal browser available -- so device flow
  is not the flow GitHub's own guidance recommends for it.

The conclusion this decision draws from that evidence:

```text
client_id
+
GitHub-named client_secret
=
public application protocol configuration

NOT
=
security authority
```

RepoPact's GitHub App `client_id` and the value GitHub calls its
`client_secret` are both **packaged, non-confidential application
metadata** in this native-client architecture. Neither authenticates "a
genuine RepoPact binary" to anything -- anyone can extract and reuse
either value from a public build, exactly as Decision 0061 already
established for `client_id` alone. This decision extends that same
honest treatment to the value GitHub happens to call a secret, rather than
avoiding it by picking a flow (device flow) whose main advantage was
narrowly "does not require sending this particular non-confidential
value," at the cost of a materially worse product UX and a flow GitHub
itself does not recommend for this class of client.

The actual, meaningful authorization protections in this architecture are,
and remain:

- PKCE (`S256`): binds the authorization code to the exact process that
  initiated the request, so an intercepted authorization code alone is
  useless without the matching verifier, which never leaves the native
  process or crosses any boundary the code itself crosses.
- A cryptographically random, single-use `state` value: prevents
  cross-session request forgery and callback confusion.
- Strict callback ownership/validation: only a request at the exact
  expected loopback path/deep-link, bound to `127.0.0.1` only, is ever
  treated as a terminal callback.
- A short-lived authorization session: the native callback listener times
  out and the session expires rather than lingering indefinitely.
- OS-protected storage for the values that are actually secret: the user's
  own GitHub access and refresh tokens, exactly as Decision 0061 already
  required and Checkpoint E already proved on Android.
- GitHub's own installation/repository permission model, unchanged by this
  decision.

RepoPact must never claim, imply, or build UI/documentation suggesting
that possession of the bundled `client_id`/`client_secret` pair
authenticates a genuine RepoPact binary or gates any ForgeWire Labs
service. This decision does not introduce any such use, and any future
proposal to do so would need its own separate, explicit decision.

## Decision

### The Workbench's default interactive flow becomes browser-redirect PKCE, on every platform

```text
Connect GitHub
    |
    v
native auth session created
    |
    +-- cryptographically random state
    +-- PKCE verifier (CSPRNG)
    +-- S256 challenge
    +-- callback receiver (loopback on desktop; deep link/loopback on Android)
    |
    v
open trusted GitHub authorization URL (system browser only)
    |
    v
user authorizes
    |
    v
GitHub callback
    |
    v
native callback validation (state, single-use, fail-closed)
    |
    v
native token exchange (code + PKCE verifier + client_id + public client_secret)
    |
    v
OS-protected credential store
    |
    v
identity + installations
    |
    v
Connected
```

Both desktop and Android Workbench builds use browser redirect + PKCE.
There is no separate "Android needs device flow because it differs from
desktop" exception: Android gets its own callback *transport* (a deep
link, or loopback if the deep link registration does not hold up -- see
"Android callback disposition" below) behind the same
provider-neutral authorization semantics, not a different authorization
mechanism.

### PKCE and state generation

Implemented in `repopact-remote-provider::pkce`
(provider-neutral, since any future OAuth2+PKCE provider can reuse it):

- `code_verifier`: 32 CSPRNG-generated bytes (`rand`'s cryptographically
  secure generator), base64url-encoded without padding -- 43 characters,
  within RFC 7636's 43-128 character bound.
- `code_challenge`: `BASE64URL(SHA256(code_verifier))`
  (`code_challenge_method=S256`), verified against RFC 7636 Appendix B's
  own worked test vector.
- `state`: 32 CSPRNG-generated bytes, base64url-encoded, compared in
  constant time against a callback's returned value.

Neither type implements `Serialize`/`Deserialize`. Both are held only in
an in-memory `BrowserAuthSession` for the lifetime of one authorization
attempt (`repopact-provider-github::provider::GitHubProvider`) and are
never written to the workspace registry, frontend storage
(`localStorage`/`sessionStorage`/IndexedDB), repository files, logs, or
evidence. The authorization code, once exchanged, and the PKCE verifier
are not referenced again after the exchange call and are dropped when the
session's owning closure returns.

### State validation is fail-closed and single-use

The native callback path (`repopact-provider-github::callback`,
`GitHubProvider::spawn_callback_worker`) rejects, without attempting a
token exchange or storing any credential:

- a callback with no `state`;
- a callback whose `state` does not match the current session's (wrong or
  replayed state, or a state from a superseded/prior session);
- a callback with no `code`;
- a GitHub-reported `error` callback (mapped to `AuthorizationDenied`);
- any callback arriving after the session was cancelled or superseded by
  a retry (tracked via a monotonically increasing session `generation`
  that a background worker checks before committing any result);
- a second physical connection to the same (already-consumed, one-shot)
  loopback listener.

Every one of these is covered by an executable test in
`repopact-provider-github::provider::tests` and
`repopact-provider-github::callback::tests`, driving the real loopback
socket and the real `GitHubProvider` state machine end-to-end, not a mock
of the validation logic alone.

### Desktop callback: native loopback listener

`repopact-provider-github::callback::CallbackListener` binds an ephemeral
port on `127.0.0.1` only (never `0.0.0.0`) per authorization attempt, and
the redirect URI is `http://127.0.0.1:<ephemeral-port>/repopact/github/callback`.
The listener:

- accepts exactly `GET /repopact/github/callback` as the one terminal
  callback; any other path or method is answered but does not terminate
  the wait (a browser's incidental `favicon.ico` request, for example,
  must not be mistaken for the real callback);
- bounds request-line and header size (8 KiB) and rejects/ignores an
  oversized request without ever buffering it unbounded;
- times out (10 minutes) if no callback arrives, moving the session to
  `Expired`;
- stops immediately on cancellation, moving the session to `Cancelled`;
- returns a minimal, secret-free HTML success or failure page (never
  echoing `code`, `state`, a token, or any GitHub response data back into
  the page the browser renders) and shuts down right after resolving.

This is not a general HTTP server: it has no routing, no persistent
listener across sessions, and cannot be reached by anything other than a
process on the same machine connecting to that one ephemeral loopback
port for that one session's lifetime.

### Android callback disposition

RepoPact's Android build must use the same browser-redirect product UX,
not a reintroduced device-code entry screen. This decision records the
callback-transport choice as follows, to be exercised when GitHub App
user-authorization wiring is actually integrated into the Android build
(GH-012's remaining Android-integration gap, tracked separately from this
checkpoint -- Checkpoint E already proved the Android protected-credential
backend this flow will store tokens into):

- A **custom URI scheme deep link**
  (`repopact://oauth/github`) is GitHub's documented, currently-supported
  non-HTTP redirect URI shape for an App's user-authorization step. PKCE
  still protects the authorization code even if a hostile app registered
  the same custom scheme on the device, because that hostile app cannot
  produce the matching `code_verifier` -- it never had it.
- The redirect-URI *registration* on the GitHub App side is a single
  fixed string; the *runtime* handling on Android is a Tauri/native
  deep-link receiver that validates the callback's exact scheme, host,
  and path before treating it as a terminal callback, exactly mirroring
  the desktop loopback listener's own exact-path validation.
- If GitHub's App registration UI or live behavior does not accept the
  desired custom scheme when an operator actually registers the app (this
  decision's registration contract below specifies what to try first),
  the fallback is a loopback callback on Android (Android can bind a local
  socket the same way desktop does) rather than introducing a hosted
  ForgeWire Labs token-exchange/redirect server. Standing up such a
  hosted redirect service is explicitly out of scope for this decision;
  it would require its own separate, explicitly justified architecture
  decision if a future checkpoint concludes it is necessary.
- A provider-neutral callback-receiver seam
  (`CallbackListener`'s trait-shaped role, currently one concrete loopback
  implementation) is the extension point a future Android-specific
  receiver implements, so authorization *semantics* (state validation,
  single-use, fail-closed) are shared code, not reimplemented per
  platform.

### Device flow disposition

Device flow's own protocol implementation
(`repopact-provider-github::device_flow`) is **retained as a tested
protocol library**, unused by any production Workbench command:
`GitHubProvider` no longer calls it, no Tauri command exposes it, and it
is not offered as a second ordinary login choice anywhere in the UI. Per
GitHub's own guidance not to enable device flow without a constrained/
headless reason, RepoPact's production GitHub App registration keeps
Device Flow **OFF** (see the registration contract below). If a genuine
future headless/CLI RepoPact use case emerges, it would need its own
explicit decision to re-enable device flow in the GitHub App and wire this
existing, already-tested module into that specific capability -- it is
not resurrected implicitly.

### Token exchange

`repopact-provider-github::browser_flow::exchange_code_for_token` sends
`client_id`, the public `client_secret`, `code`, `redirect_uri`, and
`code_verifier` as a form body (never a URL query string) to
`https://github.com/login/oauth/access_token`, per GitHub's documented
web-flow contract. Refreshing an expiring access token
(`refresh_access_token`) now also includes `client_secret`, since GitHub's
own documentation states it is "required unless the user access token was
generated using the device flow" -- which, after this decision, RepoPact's
Workbench tokens never are. Neither call is logged; `RemoteProviderError`
redaction (Decision 0061, `repopact_remote_provider::redact`) continues to
scrub any credential-shaped string that might otherwise reach an error
message.

### The public client secret is packaged application configuration, not a credential

`GitHubProviderConfig::public_client_secret` is a plain configuration
field, not a `Secret`/`CredentialStore` value: labeling it as securely
stored, or building any encryption/obfuscation around it, would be
security theater given it ships inside every copy of the binary. It is
also not sprayed carelessly: `GitHubProviderConfig`'s `Debug` impl prints
only whether it is configured, never the value; it never appears in a
frontend DTO, a Tauri command return value, a log line, or evidence.
Access and refresh tokens -- the values that are genuinely secret to a
specific user -- are completely unaffected by this decision and continue
to live only in OS-protected `CredentialStore` backends (Windows
Credential Manager, Android Keystore-backed storage per Checkpoint E,
future Keychain/Secret Service backends), never crossing into JavaScript.

### Product configuration, not user configuration

`REPOPACT_GITHUB_CLIENT_ID` (the Checkpoint B environment-variable
production configuration path) is retired. Registering a GitHub App is a
RepoPact/ForgeWire Labs product/release concern; an installed RepoPact
user is never asked to open a terminal, set an environment variable, or
supply a client ID/secret. `GitHubAppRegistration`
(`rust/apps/repopact-desktop/src-tauri/src/github_app_registration.rs`)
loads `client_id`/`public_client_secret`/`app_slug` from **build-time**
values (`option_env!`, resolved when the official binary is compiled by
RepoPact's release pipeline, not read from the running user's
environment) with a `#[cfg(debug_assertions)]`-gated developer-only
environment-variable fallback that is compiled out of every release build
and is never documented as normal product setup. A development build with
neither source configured truthfully reports "GitHub integration is not
configured in this development build" (`ProviderNotConfigured`) --
distinct wording from, and never suggesting, manual end-user setup.

### Typed command surface

`remote_connect_start` is now the single native operation that generates
the authorization session, builds the trusted authorization URL, and opens
the system browser -- all inside one Tauri command, owned entirely by the
backend. The separate `remote_open_verification_url` device-flow command
is removed; there is no `open_url`-shaped command of any kind. A new
`remote_open_installation_page` command opens GitHub's own installation/
configuration page for the RepoPact GitHub App, built from the native
`app_slug` configuration only. Frontend code continues to supply nothing
more than typed repository/ref identifiers for the already-existing
browsing/import commands (Decision 0061's boundary is unchanged); it never
supplies an authorization host, redirect host, client ID, client secret,
state, or PKCE verifier.

### Authorization state model

`repopact_remote_provider::auth::AuthState` is revised to:

```text
Disconnected
StartingBrowserAuthorization
WaitingForCallback { expires_at }
ExchangingCode
Authorized { account_label }
Refreshing
Cancelled
Expired
Failed { code }
```

There is no `RequestingAuthorization`/`AwaitingUser{user_code,
verification_uri, expires_at}`/`Revoked` variant anymore; the Workbench
never displays or accepts a device/user code. `GitHubProvider`'s
`poll_authorization` no longer performs an active network poll -- a
background thread (spawned by `start_browser_authorization`) progresses
the state as the callback and token exchange resolve, and polling merely
reads whatever state that thread has reached.

### Installation vs. authorization remain distinct

Unchanged in substance from Decision 0061's intent, now made concrete:
after authorization, the Workbench lists GitHub App installations visible
to the connected identity (`remote_accounts`, unchanged). A connected user
with no visible installation is offered "Configure repository access on
GitHub," which opens the trusted installation URL
(`https://github.com/apps/<app_slug>/installations/new`, built from native
configuration only). RepoPact does not trust an `installation_id`
supplied by any browser/callback data without it coming from an
authenticated `remote_accounts`/`remote_repositories` call against the
connected identity's own GitHub API responses.

## Consequences

- WI067's GH-003 acceptance criterion, satisfied under the device-flow
  decision, is treated as requiring refreshed evidence for the duration of
  this checkpoint and is re-satisfied only once the browser-PKCE
  architecture, its executable negative-callback-attack tests, and this
  decision are complete -- the original Checkpoint A evidence record
  documenting the device-flow decision remains historical and unedited.
- `docs/guides/github-app-setup.md` is rewritten to describe the
  browser-redirect-PKCE registration contract (Device Flow OFF, web/OAuth
  authorization enabled, the exact callback URL(s)); it no longer
  instructs an operator to enable Device Flow, and it explicitly explains
  why the GitHub-named "client secret" is packaged application metadata,
  not a trust boundary, in this architecture.
- A live, operator-registered GitHub App (client ID, the public client
  secret value, and the app slug) is still required before any real
  authorization can occur; none of the three GitHub App-dependent
  acceptance criteria this decision cannot itself satisfy (GH-005, GH-012,
  GH-015) become satisfied by this decision alone.
- Decision 0056's `GitBackend`/Stage-2 seam, and every part of Decision
  0061 not named above as superseded, are unaffected.
