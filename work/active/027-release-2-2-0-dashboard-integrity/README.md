# 027 — Release RepoPact 2.2.0 with dashboard integrity

> **Status**: Active.
> **Owners**: governance-owner (lead); tooling, docs, work, and evidence support.
> **Depends on**: completed work item 026.

## Intent

Publish canonical generated-dashboard enforcement as RepoPact 2.2.0 and move the
first adopter from an interim commit pin to the formal release.

This is a minor release: the validator gains a new deterministic integrity rule and
the CLI gains canonical regeneration/repair behavior. Repositories with missing or
stale dashboards that passed 2.1.0 are intentionally rejected by 2.2.0.

## Acceptance criteria

- [x] **REL220-001** Align all release and conformance version surfaces at 2.2.0.
- [x] **REL220-002** Pass governance, regression, conformance, derived-output, and
  distribution-build checks.
- [ ] **REL220-003** Merge, tag, upload the exact validated artifacts directly to
  PyPI, and externally verify v2.2.0.
- [ ] **REL220-004** Advance ForgeLink to the formal release without reopening drift.

## Release boundary

- The release contains the already-reviewed dashboard-integrity implementation from
  work item 026; it does not widen schemas or alter unrelated lifecycle semantics.
- GitHub Actions release publication is unavailable because the repository's Actions
  account is billing-locked. This release uses the documented token-authenticated
  direct-PyPI fallback from the operator machine.
- The pushed `v2.2.0` tag, local build hashes, Twine checks, upload result, public PyPI
  metadata, and clean-environment install are separate evidence points. No credential
  material may enter the evidence record.
- Do not claim PyPI completion until the public index and install metadata show 2.2.0.

## Evidence

- `20260718-repopact-2-2-0-release-candidate` records the passing 101-test suite,
  8-case conformance suite, governance validation, deterministic generated-output
  hashes, checked distribution metadata, and exact pre-publication artifact hashes.
- Publication and adopter evidence remain pending for REL220-003 and REL220-004.
