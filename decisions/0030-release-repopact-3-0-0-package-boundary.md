---
id: 0030
title: Release RepoPact 3.0.0 package boundary
status: accepted
date: 2026-07-26
supersedes: []
---

# 0030: Release RepoPact 3.0.0 package boundary

## Context

Decision `0029` records the operator-approved breaking boundary: replace the
seventeen flat modules and the seeded `scripts/*.py` tooling channel with one
installed `repopact` package and CLI, and ship that change as 3.0.0. WI-036
implemented the source conversion and later moved schemas/templates into package
resources with separate INV-6 approval.

WI-036 was nevertheless closed while `VERSION` and public PyPI remained 2.3.0.
The public 2.3.0 wheel still installs sixteen flat modules, including the
conflicting `frontmatter`, and fourteen deprecated data-files entries. Its local
evidence proves a candidate wheel, not publication. Verification also found that
a direct checkout wheel build can inherit obsolete flat modules from ignored
`build/lib` state even when `top_level.txt` says only `repopact`.

## Decision

Release RepoPact 3.0.0 as the delivery of decision `0029`.

- `VERSION`, README, roadmap, conformance identity, tag, and public artifact use
  3.0.0.
- Release artifacts are built twice from independent exports of the committed
  tree with a fixed source-date epoch. Publication is blocked unless hashes
  match and wheel inspection proves one import root, packaged seeds, and no
  data-files.
- A public package release and downstream migration are separate phases. WI-037
  owns upgrading the five adopter default branches and proving Moto's legacy
  vendored overlay transition. A stale fleet therefore blocks ecosystem
  closeout, not package publication or the upstream version record.
- GitHub Actions remains billing-locked. The operator-authorized direct PyPI
  fallback in `research/release-runbook.md` is used without claiming CI
  restoration.

## Alternatives considered

- **Leave the source fix unreleased at 2.3.0.** Rejected: users continue to
  receive the exact namespace pollution WI-036 claims closed.
- **Republish 2.3.0.** Impossible and dishonest: PyPI versions are immutable,
  and the interface break was explicitly approved as a major release.
- **Require the entire adopter fleet to migrate before publishing.** Rejected:
  WI-029 deliberately separates package publication from ecosystem rollout.
  The fleet verifier must make lag visible, not prevent the upstream release
  record from advancing.

## Consequences

- `pip install repopact==3.0.0` installs the supported package/CLI boundary and
  no generic root modules.
- Existing adopters stay on their declared older release until WI-037 migrates
  them. `repopact fleet-verify` is expected to fail closed during that interval.
- Release construction no longer trusts ignored checkout build state.
- The release is locally and publicly verifiable, but remote CI remains an open
  operator-gated obligation under WI-032.
