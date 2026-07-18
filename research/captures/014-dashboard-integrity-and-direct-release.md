# Capture 014 — canonical dashboard integrity and direct 2.2.0 release

Evidence capture, 2026-07-18. Number 013 remains reserved for the proposed-state
episode identified by GA-2; this capture does not silently consume that open slot.

## Failure class and correction

ForgeLink's source work ledger and manifests were valid, but its checked-in generated
dashboard still reported ten active items from June 27. RepoPact validation accepted
that split state because it did not compare the dashboard to a fresh projection.

Work item 026 added canonical, date-stable rendering and a one-tree validator check:

```text
exists(audits/reports/dashboard.md)
and read(audits/reports/dashboard.md) = π_dashboard(repository state)
```

A deliberately stale ForgeLink dashboard then failed its authoritative validator;
packaged regeneration restored a pass. The upstream regression suite passed 101 tests
with two documented formal-model skips, and all eight conformance cases passed.

## Release and publication

RepoPact 2.2.0 release commit and annotated tag:

```text
2b3b549e915bbe69264ac59401cf533d11cc6532  main
2b3b549e915bbe69264ac59401cf533d11cc6532  v2.2.0^{}
```

GitHub Actions publishing was unavailable because the account was billing-locked.
The operator authorized the documented direct-PyPI fallback. Twine uploaded the exact
pre-validated artifacts; public PyPI metadata returned the same hashes:

```text
repopact-2.2.0-py3-none-any.whl
  bytes   70343
  sha256  a254a6b23af94d30118b53422082c7a1e52310e1e40c58e16c6cb3716fa8f3ba

repopact-2.2.0.tar.gz
  bytes   73193
  sha256  f137400f74b0fb743cffcf8b28dbe5d24b6a1fe13fc37b75af977f9745d437b8
```

A no-cache clean virtual environment installed `repopact==2.2.0` from PyPI,
bootstrapped a new repository with `repopact init --target`, and passed packaged
validation. The initial exploratory smoke used `init --root`, which is not a supported
flag and exited before creating state; the documented invocation is the recorded proof.

## Independent adopter closure

ForgeLink replaced the interim commit dependency with exact PyPI pin
`repopact==2.2.0` at commit `86f8d67c9ea8770f364b35d06a5c417a00f31815`.
The pushed commit passed its RepoPact audit, 221 renderer tests, and 202 of 203 Node
tests; the only skip was the explicitly opt-in live Twilio credential test.

## Interpretation for the paper

This episode strengthens the derive-layer mechanism claim: dashboard projection
equality is now a validator-decided state fixpoint, so it is enforceable anywhere the
validator runs and is not dependent on CI-specific diff plumbing. It does not repair
the separate CI-availability threat. Direct PyPI publication proves package delivery,
artifact identity, and installability; it does not prove the billing-blocked GitHub
workflow or cross-platform execution.
