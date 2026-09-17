---
id: 0063
title: Release RepoPact 3.1.1 sdist License-File corrective
status: accepted
date: 2026-09-16
supersedes: []
---

# 0062: Release RepoPact 3.1.1 sdist License-File corrective

## Context

Publishing the `v3.1.0` wheel and sdist to PyPI surfaced a packaging defect:
`pyproject.toml`'s `[tool.maturin] include` list never added `LICENSE` to the
sdist format, while Maturin's autodetected `license = "Apache-2.0"` metadata
still emits `License-File: LICENSE` in `PKG-INFO`. PyPI's upload-time
License-File check rejected `repopact-3.1.0.tar.gz` for declaring a license
file it does not contain (HTTP 400, `License-File LICENSE does not exist in
distribution file`). The wheel does not have this defect: Maturin bundles the
license into the wheel's `.dist-info/licenses/` directory independently of the
`include` list, and `repopact-3.1.0-py3-none-win_amd64.whl` uploaded and is
live on PyPI.

The `repopact-3.1.0.tar.gz` upload attempt was rejected by PyPI and never
stored, so the filename `repopact-3.1.0.tar.gz` remains available; nothing
public needs to be superseded for the sdist specifically. But this repository's
decisions require the exact stable release tree to remain immutable once
tagged and published, and `v3.1.0`'s wheel is already public. Amending the
`v3.1.0` tag in place, after a public wheel built from it already exists on
PyPI, would break that immutability guarantee for a release third parties may
already have installed.

## Decision

Release RepoPact `3.1.1` as a backwards-compatible corrective patch, following
the precedent of decision 0033 (`3.0.1`). The only runtime/package delta is:

- `pyproject.toml`'s `[tool.maturin] include` list adds
  `{ path = "LICENSE", format = ["sdist"] }` so the sdist actually contains the
  file its own `PKG-INFO` declares.

No schema, protocol, CLI, lifecycle, or provenance behavior changes. The
`v3.1.0` tag and its already-published wheel are left untouched; `3.1.1`
supersedes `3.1.0` as the current stable identity and is the version new
installs should target. Both the wheel and sdist are rebuilt and published for
`3.1.1` from the corrected tree.

## Alternatives considered

- **Amend the `v3.1.0` tag/commit in place.** Rejected: a public wheel already
  exists for that exact tag; moving the tag afterward would make "the tag" and
  "what was actually published" diverge, which is the exact failure mode the
  source-artifact-identity rule (decision 0033) exists to prevent.
- **Hand-patch the rejected sdist tarball without a source fix.** Rejected: the
  published artifact would then no longer be reproducible from any committed
  tree, breaking the reproducible-build guarantee the release tooling exists
  to enforce.

## Consequences

`VERSION`, package metadata, `CONFORMANCE.md`, `SPEC.md`'s changelog line, and
`conformance/manifest.json`'s `suite_version` move to `3.1.1`. The release
candidate is rebuilt with `repopact release-build`, verified reproducible, and
`twine check`-clean before publication of both the wheel and sdist.
