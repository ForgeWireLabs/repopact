---
id: 0025
title: Release 2.2.0 with canonical dashboard integrity
status: accepted
date: 2026-07-18
supersedes: []
---

# 0025: Release 2.2.0 with canonical dashboard integrity

## Context

RepoPact dashboards are derived, checked-in governance artifacts. Before work item
026, `repopact validate` checked the source ledger but did not compare the committed
dashboard with a fresh render. A repository could therefore pass validation while its
dashboard reported obsolete work counts and active items.

Work item 026 closed that integrity gap:

- dashboard output is canonical and omits run-date-only churn;
- validation rejects a missing or byte-stale dashboard;
- mutation and repair commands regenerate the projection before claiming success;
- tests cover rejection, repair, bootstrap, adoption, imports, takeover, record
  stamping, and conformance materialization;
- ForgeLink pinned the implementation and proved its previously stale dashboard fails.

## Decision

Release RepoPact 2.2.0 as a minor release.

This is not a patch-only change. It adds a new repository integrity guarantee and a
new validation failure class: a 2.1.0 repository with a stale or missing dashboard may
be rejected by 2.2.0 even when its source records are otherwise valid. The migration
is deterministic and non-destructive: run `repopact dashboard --root .` and commit the
canonical result. `repopact doctor --fix` can perform the same repair.

The conformance suite version moves to 2.2.0 so implementations and adopters can name
the dashboard-integrity semantics explicitly.

## Evidence

Work item 026 and evidence run `20260718-deterministic-dashboard-validation` record:

- 101 upstream regression tests passed, with 2 documented formal-model skips;
- governance validation and all 8 conformance cases passed;
- ForgeLink rejected its stale dashboard, regenerated it, and passed its complete
  governance and product pre-push gate.

The 2.2.0 release work item adds distribution-build, pushed-tag, direct-PyPI, public
index/install, and formal adopter-upgrade evidence. GitHub Actions publishing is
billing-blocked at release time, so the existing trusted-publishing workflow cannot
serve as evidence for this release. The operator-authorized fallback uploads the exact
locally validated tag artifacts with Twine. The evidence records artifact hashes and
public metadata, never credentials. A GitHub release is deliberately not used as a
proxy for a workflow that could not run.

## Consequences

- Adopters must commit canonical dashboard output; stale derived reports can no longer
  coexist with a green validator.
- Generators remain restricted to `audits/reports/dashboard.md`; validators remain
  read-only.
- Daily wall-clock churn is avoided, while audit-freshness output still changes when a
  scope actually crosses its review date.
- RepoPact 2.2.0 becomes the preferred formal dependency over interim commit pins.
- The direct upload preserves package identity and verifiability, but does not repair
  the separate CI-availability limitation recorded by the research gap audit.
