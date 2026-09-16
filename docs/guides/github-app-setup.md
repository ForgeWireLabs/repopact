# GitHub App setup for RepoPact remote repository import (WI067)

This document specifies the exact GitHub App configuration Decision 0061
requires. It contains no secret values -- an operator follows it once to
register the app and supplies only the resulting **client ID** (not
confidential) to a later WI067 checkpoint's configuration. No client
secret and no private key are ever required by RepoPact's own code.

## App identity

- **App name**: `RepoPact` (or an organization-scoped variant, e.g.
  `RepoPact (ForgeWire Labs)`), following GitHub's uniqueness rules.
- **Homepage URL**: the RepoPact project's public homepage/repository URL.
- **Callback URL**: not required for v1. WI067 v1 uses the GitHub App
  device flow (Decision 0061), which has no browser redirect/callback
  step. Leave this unset unless a later checkpoint adds the web+PKCE flow
  for a platform where device flow is unsuitable -- that would be a
  separate, explicitly justified decision, not an assumed extension.

## Device flow

- **Enable Device Flow**: **required, ON.** This is the flow Decision 0061
  selects for v1; the app will not authorize without it.

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

None requested for v1. Do not grant Email addresses, Followers, or any
other account-level permission unless a specific future endpoint requires
it and that requirement is recorded the same way `permissions.rs` records
the repository permission matrix.

## User authorization / token settings

- **Expire user authorization tokens**: **required, ON.** Decision 0061
  relies on the documented ~8-hour access token / ~6-month refresh token
  expiration model; do not disable expiration for implementation
  convenience.
- **Request user authorization (OAuth) during installation**: not required
  for v1's device-flow-only approach.

## Webhooks

- **Active**: **OFF.** WI067 v1 has no webhook receiver/server. Do not
  configure a webhook URL or subscribe to any webhook events; there is
  nothing running to receive them.

## Where the app may be installed

- **Installation target**: "Any account" if RepoPact should be installable
  by any GitHub user/organization, or a single account if this is an
  internal/ForgeWire-Labs-only registration for early development. Either
  choice is compatible with Decision 0061's architecture; it does not
  change RepoPact's own code.

## Private key

- **Do not generate a private key** for this app unless a future,
  separately justified decision requires server-to-server installation-
  token minting. WI067 v1 never uses one. If a private key is generated
  for some unrelated future reason, it must never be embedded in the
  RepoPact desktop/mobile client -- it is meaningful only to a
  server-side component RepoPact does not currently have.

## What the operator supplies back to RepoPact

Only the **Client ID** shown on the app's settings page. It is not
confidential (Decision 0061) and may be compiled into the native client
configuration. Do not supply the client secret, private key, or any
generated token to RepoPact's configuration, environment, source tree, or
issue tracker.

## Status

No GitHub App has been registered yet as of WI067 Checkpoint A. This
document exists so an operator can register one; a later checkpoint
consumes the resulting client ID once it exists. Nothing in Checkpoint A's
code assumes a specific client ID value.
