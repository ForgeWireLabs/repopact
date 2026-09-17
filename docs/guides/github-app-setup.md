# GitHub App setup for RepoPact remote repository import (WI067)

This document specifies the exact GitHub App configuration Decision 0062
requires. Registering the app is a **RepoPact/ForgeWire Labs release
concern**, not an end-user concern: an installed RepoPact user never sees,
enters, or is asked to configure a client ID, a client secret, an app
slug, or anything else on this page. An operator follows this once to
register the production app and hands the three resulting values (below)
to RepoPact's release/build configuration. No private key and no webhook
secret are ever required by RepoPact's own code.

## A note on "client secret"

GitHub's app-settings page calls one of the values you will generate a
"client secret." In a normal server-side OAuth application, that name is
accurate: the server keeps it confidential, and possessing it is part of
what proves a request came from that server. **RepoPact is not that kind
of application.** It is a distributed native client (a Windows/macOS/
Linux desktop binary and an Android APK) that GitHub's own documentation
does not, and cannot, treat as able to keep any embedded value
confidential -- anyone can extract it from the binary. GitHub's web/OAuth
token-exchange endpoint still requires this value as a request parameter
regardless of that fact, so RepoPact sends it, but it is treated
throughout RepoPact's code and documentation as **packaged application
metadata, not a trust boundary or an authentication secret**. Do not
architect around a belief that this value is confidential, and do not
build (or ask for) any mechanism that treats possessing it as proof of a
genuine RepoPact binary.

The values that are genuinely secret in this architecture are each user's
own GitHub access and refresh tokens, which RepoPact never bundles, never
compiles in, and stores only in that user's OS-protected credential
facility (Windows Credential Manager, Android Keystore-backed storage,
etc.) after they personally authorize the app.

## App identity

- **App name**: `RepoPact` (or an organization-scoped variant, e.g.
  `RepoPact (ForgeWire Labs)`), following GitHub's uniqueness rules.
- **Homepage URL**: the RepoPact project's public homepage/repository URL.

## Identifying and authorizing users (web application flow)

- **Callback URL(s)**: register **both** of the following. GitHub permits
  multiple callback URLs on one App; desktop and Android do not need to
  use the same transport.
  - `http://127.0.0.1/repopact/github/callback` -- the desktop native
    loopback callback base. RepoPact binds an ephemeral port on
    `127.0.0.1` for each authorization attempt and sends the full runtime
    redirect URI (`http://127.0.0.1:<ephemeral-port>/repopact/github/callback`)
    in each individual authorization request; register the base form
    above once, following GitHub's documented loopback-redirect
    convention for native/installed applications.
  - `repopact://oauth/github` -- the Android custom-URI-scheme deep-link
    callback. If GitHub's app-registration UI rejects a non-HTTP redirect
    URI at registration time, do not force it through -- record that
    result and use a desktop-style loopback callback on Android instead
    (Decision 0062 records this as the recorded fallback; do not invent a
    third option without a separate decision).
- **Request user authorization (OAuth) during installation**: leave this
  **unchecked/OFF** for v1. RepoPact's Workbench authorizes the user first
  (Connect GitHub) and separately offers "Configure repository access on
  GitHub" if no installation is visible yet -- it does not rely on
  GitHub's combined install+authorize shortcut.
- **Setup URL**: not required. RepoPact has no post-installation setup
  step of its own to redirect an operator to.

## Identifying and authorizing users (device flow)

- **Enable Device Flow**: **OFF.** Decision 0062 supersedes Decision
  0061's original device-flow selection; RepoPact's Workbench no longer
  uses device flow for interactive authorization, and GitHub's own
  guidance is not to enable it without a constrained/headless reason
  RepoPact does not currently have. Leaving it off is also a meaningful
  hardening step: it removes device-flow user-code phishing as an attack
  surface against this app registration entirely.

## Permissions (repository)

Request exactly:

| Permission | Level |
|---|---|
| Contents | Read-only |
| Metadata | Read-only |

Do not request Contents: Read & write, Administration, Actions, Workflows,
or Pull requests write. See
`rust/crates/repopact-provider-github/src/permissions.rs` for the exact
endpoint -> permission matrix these two permissions cover.

## Permissions (account)

None requested. Do not grant Email addresses, Followers, or any other
account-level permission unless a specific future endpoint requires it and
that requirement is recorded the same way `permissions.rs` records the
repository permission matrix.

## User authorization / token settings

- **Expire user authorization tokens**: **required, ON.** RepoPact relies
  on the documented ~8-hour access token / ~6-month refresh token
  expiration model; do not disable expiration for implementation
  convenience.

## Webhooks

- **Active**: **OFF.** RepoPact has no webhook receiver/server. Do not
  configure a webhook URL or subscribe to any webhook events; there is
  nothing running to receive them, and no webhook secret is ever needed.

## Where the app may be installed

- **Installation target**: "Any account" if RepoPact should be installable
  by any GitHub user/organization, or a single account if this is an
  internal/ForgeWire-Labs-only registration for early development. Either
  choice is compatible with this architecture; it does not change
  RepoPact's own code.

## Private key

- **Do not generate a private key** for this app. RepoPact never uses
  server-to-server installation-token minting (which is what a private key
  is for) and never will under this architecture without a separate,
  explicitly justified decision. If a private key is ever generated for
  some unrelated future reason, it must never be embedded in the RepoPact
  desktop/mobile client -- it is meaningful only to a server-side
  component RepoPact does not currently have.

## What the operator supplies back to RepoPact

Exactly three values from the app's settings page, all treated as
packaged application configuration (never end-user configuration, never
`CredentialStore` material):

1. **Client ID**
2. **Client secret** (see "A note on 'client secret'" above -- not
   confidential in this architecture, but still not something an
   installed user enters or sees)
3. **App slug** (the short name in the app's public URL,
   `https://github.com/apps/<slug>`), used to build the "Configure
   repository access on GitHub" installation-page link

Do not supply the private key, an App JWT, a webhook secret, or an
installation token -- none of these are used.

These three values are compiled into an official RepoPact release build
via build-time environment variables
(`REPOPACT_GITHUB_APP_CLIENT_ID`/`REPOPACT_GITHUB_APP_CLIENT_SECRET`/
`REPOPACT_GITHUB_APP_SLUG`, read with `option_env!` at compile time -- see
`rust/apps/repopact-desktop/src-tauri/src/github_app_registration.rs`) set
by the release pipeline before invoking the build, never committed to
source. An installed user never sets any of these; a developer building a
local debug binary against a personal test app may instead set
`REPOPACT_DEV_GITHUB_APP_CLIENT_ID`/`REPOPACT_DEV_GITHUB_APP_CLIENT_SECRET`/
`REPOPACT_DEV_GITHUB_APP_SLUG` as ordinary runtime environment variables --
this developer-only path is compiled out of every release build
(`#[cfg(debug_assertions)]`) and must never be documented or suggested as
normal product setup. A development build with neither source configured
truthfully reports "GitHub integration is not configured in this
development build."

## Status

As of this checkpoint, no GitHub App has been registered yet. This
document exists so an operator can register one using the exact contract
above; the next checkpoint after this one is the operator registering the
real app and handing back the three values, followed by a live-
authorization checkpoint that exercises real Connect GitHub / real browser
redirect / real repository import end to end.
