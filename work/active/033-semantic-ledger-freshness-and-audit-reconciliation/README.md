# 033 — Semantic ledger freshness and audit reconciliation

> **Status**: Active
> **Owners**: work-coordinator (lead); governance-owner, tooling-owner, and evidence-owner affected.
> **Depends on**: `028`.

## Intent

Prevent a canonical dashboard from lending false confidence to stale source assertions.
The dashboard now exactly projects the manifests, but the July gap audit and active work
items demonstrate that semantically stale manifests and prose can still validate.

## Blind-spot coverage map

| Observation | Owning work |
| --- | --- |
| Ecosystem release/version drift | `029` |
| Incomplete conformance coverage | `030` |
| Research fact drift and missing F-014/capture 013 | `031` |
| Remote cross-platform checkpoint absent | `032` |
| Semantic ledger and audit freshness | `033` |
| Statistical plan and first RealRunner execution | existing `022`, AC-4/AC-5 |
| Independent public reproduction | `034` |

## Decisions

Freshness is not truth, but it makes review debt observable. Machine-derived projections
remain exact; external and semantic assertions need explicit verification provenance and
an expiry policy rather than pretending they can be regenerated.

## Scope

- Gap-audit and active-ledger reconciliation.
- A freshness contract for non-derived claims.
- Validator diagnostics, documentation, and dated reconciliation evidence.

## Acceptance criteria

- [ ] **AC-1** — re-verify and accurately restate every July gap.
- [ ] **AC-2** — reconcile work items 020–022 against current evidence.
- [ ] **AC-3** — enforce review freshness for non-derived claims.
- [ ] **AC-4** — document the dashboard's semantic boundary.
- [ ] **AC-5** — preserve the reconciliation in a dated evidence run.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
