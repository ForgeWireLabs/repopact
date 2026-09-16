# GitHub repository snapshot import (WI067)

This documents what is concretely implemented as of WI067 Checkpoint D --
not a roadmap or aspiration. See `decisions/0061-*.md` for the
architecture decision this behavior implements, and
`docs/guides/github-app-setup.md` for the GitHub App registration this
feature depends on.

## What this is

RepoPact can materialize a GitHub repository at an exact commit as an
ordinary local, app-private workspace -- the same kind of workspace a
local folder pick or a mobile SAF/archive import produces. This is a
**snapshot**, not a clone:

- there is no `.git` directory;
- there is no live relationship to GitHub after import;
- local edits never write back to GitHub, automatically or otherwise;
- the workspace works fully offline once imported.

Real Git synchronization (`clone`/`fetch`/`pull`/`push` against a real
`.git` working tree) is a separate, later work item (WI068) and shares no
acquisition code with this feature.

## How it works

1. The user selects a branch, tag, or commit.
2. RepoPact resolves that selection to an exact, immutable 40-character
   commit SHA -- server-side, natively, never trusting a value the
   frontend already displayed. Annotated tags are peeled to their
   underlying commit via GitHub's Git Data API, with a bounded depth
   against a malformed or cyclic tag chain.
3. RepoPact downloads GitHub's zipball archive for that exact commit,
   streamed to app-private staging with a bounded byte count enforced
   while streaming (never buffered fully in memory, never trusting a
   declared `Content-Length` alone).
4. The archive is extracted through RepoPact's existing, unmodified,
   security-hardened archive importer (the same one used for local ZIP
   imports and Android SAF archive imports) -- zip-slip/`..`-traversal
   rejection, symlink-escape protection, entry-count/expanded-byte/depth
   bounds, and duplicate/case-conflict handling all apply identically.
   GitHub's synthetic `owner-repo-shortsha/` wrapper directory is
   promoted away as a purely filesystem-level step, after extraction has
   already validated every path.
5. The result publishes as an ordinary workspace through the same
   staging-then-publish transaction every other acquisition path uses. A
   failed or cancelled import leaves no ready workspace, no completed
   registry entry, no orphaned staging tree, and no partial downloaded
   archive.
6. Workspace provenance records the provider, repository identity,
   selected ref, resolved commit SHA, acquisition timestamp, and snapshot
   semantics -- never a credential, never a temporary signed URL, never an
   `Authorization` header value.

## Network and resource bounds

- Snapshot downloads are capped at 300 MiB compressed, enforced while
  streaming.
- REST API responses are capped at 8 MiB.
- Connect timeout: 10 seconds. Download timeout: 10 minutes (a
  repository archive is much larger and slower than an API call).
- Redirects are followed manually, up to 5 hops. An `Authorization`
  header is only ever forwarded to `api.github.com` or
  `codeload.github.com`; a redirect to any other host, or a redirect that
  would downgrade an https request to http, drops the header or fails
  outright.

## Cancellation

Import can be cancelled from the UI at any point before publication;
cancellation is native (a shared cancellation token checked between
bounded units of work), never a matter of the frontend simply abandoning
the request. Cancellation leaves no ready workspace and no partial
downloaded file.

## Error behavior

Failures are typed, not raw GitHub prose: expired/invalid credentials,
forbidden/not-found repositories, rate limiting (including GitHub's
secondary rate limit, detected via a `Retry-After` response), 5xx server
errors, network/TLS failures, redirect problems, and malformed or
truncated responses are all represented as a small set of stable error
codes the frontend can branch on. A hostile or oversized error response
body is bounded and redacted before it ever reaches an error message,
evidence, or a log.

## Credential storage

Access and refresh tokens are stored only through the real OS-protected
credential facility for the running platform:

- **Windows** (this feature's currently supported desktop platform):
  Windows Credential Manager, via the `keyring` crate. Proven with real
  round-trip and cross-process persistence tests.
- **macOS/iOS, Linux**: the same `CredentialStore` abstraction targets
  Keychain and Secret Service/libsecret respectively via the same
  `keyring` crate, but this has not been exercised at runtime on those
  platforms as of this checkpoint.
- **Android**: no protected credential-store backend exists yet. This is
  an explicit, tracked gap (GH-004), not a plaintext fallback -- the
  GitHub connection feature is desktop-only until it is resolved.

## Operator gate: GitHub App registration

No GitHub App has been registered as of this checkpoint. Live device-flow
authorization, private-repository browsing, and organization-installation
browsing are all blocked on an operator completing the registration steps
in `docs/guides/github-app-setup.md` and setting the
`REPOPACT_GITHUB_CLIENT_ID` environment variable. Public-repository
snapshot import (ref resolution and archive download) requires no
authentication at all and has been proven live against
`octocat/Hello-World`.

## What is still pending

- Live authorized browsing of private repositories and organization
  installations (GH-005) -- requires the operator gate above.
- Full desktop *and* Android end-to-end authenticated runtime proof
  (GH-012) -- requires both the operator gate and the Android credential
  store below.
- Android protected credential storage (GH-004) and an Android-side
  authenticated GitHub runtime.
- iOS has not been evaluated at all for this feature.

## Provider extension

The architecture is provider-neutral in executable structure, not naming
only. A future non-GitHub provider (GitLab, Forgejo, a generic Git host)
implements the same seam:

```text
resolve ref -> immutable revision
describe snapshot -> provider-neutral descriptor
open snapshot -> bounded byte stream to app-private staging
                    -> the existing secure archive importer
                    -> ordinary workspace publication
```

and requires no change to `DesktopService`, `RepositorySession`,
`RepositoryTopology`'s core repository model, the mutation pipeline, or
governance/ROG authority. `repopact-remote-provider`'s `FakeProvider` is a
non-GitHub implementation of this exact seam, and its own test suite
proves a fake provider's snapshot flows through the real, unmodified
workspace-publication pipeline with zero GitHub-specific branching
anywhere on the path.
