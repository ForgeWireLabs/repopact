---
id: 0058
title: Release RepoPact 3.1.0 as a compatible minor milestone
status: accepted
date: 2026-09-14
supersedes: []
---

# 0058: Release RepoPact 3.1.0 as a compatible minor milestone

## Context

Since the stable `v3.0.2` release, RepoPact's development line has accumulated
substantial additive product and standard capability: the canonical Rust
semantic engine and versioned transport, repository-defined local verification
and release operations, the Repository Orientation Graph (ROG) and bounded
query/orientation surface, provider-neutral assurance/control mapping with
sensitive-evidence guardrails, Workbench repository-map surfaces, benchmark
RealRunner smoke infrastructure, and the mobile app-private workspace and
Android SAF implementation checkpoint.

The release decision follows the compatibility audit recorded in
`20260914-066-compatibility-audit` for WI066. The audit compared the live
`origin/main` baseline `8a94cd748a419323975d07037d8a3d38c7a04b6b` with the
stable `v3.0.2` tag, whose commit is
`dfab8cb8ca010a865ebb3291c526179fb5cb4b2c`.

## Decision

Release RepoPact `3.1.0` as a MINOR release, subject to the canonical release
gates and publication evidence governed by WI066.

The compatibility audit supports a minor bump:

- A clean archive of the stable `v3.0.2` tree was accepted by the current
  validator, so an existing conformant 3.0.2 repository does not become
  invalid merely because it is evaluated by the new implementation.
- The only modified pre-existing schema adds an optional `local_extension`
  property to the adopter Pypi-consumption record. The other new schemas are
  new optional record types; absent assurance, admission, verification, or ROG
  records do not invalidate a repository.
- The conformance suite retained all 20 prior cases and 18 prior rules while
  adding 15 cases and 7 rules. No previously valid mandatory lifecycle or
  provenance behavior was removed.
- The Rust engine protocol did not exist in the 3.0.2 tree. The new protocol
  is explicitly major version 1, uses request identity and handshake
  capability negotiation, and the Python client checks protocol and product
  identity. There is therefore no prior 3.0.2 engine operation whose contract
  is broken by this release.
- Existing CLI command families remain present. `verify`, `release`, `graph`,
  `admission`, `guard`, `work`, `assurance`, and `approval` add capability;
  they do not remove the 3.0.2 command surface.
- Existing adopters do not need to fabricate new governance records to remain
  structurally valid. They do need an explicit post-publication migration of
  their exact RepoPact package/version pins to `3.1.0`; the fleet verifier and
  release closeout continue to expose that separate rollout obligation.

The release identity follows decisions 0026 and 0032: the exact stable release
commit has `VERSION=3.1.0` and no `RELEASE_LABEL`, while any materially later
development source at the same compatibility core must use the distinct
VERSION-pinned `RELEASE_LABEL=3.1.0-dev.1` identity. ArXiv preparation is
refreshed but publication remains unauthorized.

## Alternatives considered

- **Release 3.0.3.** Rejected: the audited delta is substantially additive and
  includes new product and standard capability rather than only a corrective
  patch.
- **Release 4.0.0.** Rejected: no mandatory incompatible schema, semantic,
  protocol, CLI, lifecycle, or provenance change was found.
- **Keep the development line at 3.0.2 indefinitely.** Rejected: the
  development implementation has materially outgrown the stable 3.0.2
  identity, and decision 0032 requires a new release decision for the next
  compatibility line.

## Consequences

The release candidate must synchronize VERSION, package metadata, conformance
identity, SPEC, README, CITATION, release records, the paper, and arXiv
preparation before publication. Release verification, reproducibility, package
inspection, and clean-install smoke are separate evidence obligations.

This decision does not close WI022's pending comparative program, WI063's open
ROG evaluation/closeout gates, or WI065's open production mobile runtime,
export/share-back, and end-to-end acceptance criteria. It also does not
authorize arXiv submission, HN publication, or any claim that active work is
production-complete.
