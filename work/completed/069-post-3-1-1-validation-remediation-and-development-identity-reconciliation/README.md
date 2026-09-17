# 069 — Post-3.1.1 Validation Remediation and Development Identity Reconciliation

> **Status**: 🚧 Active
> **Owners**: governance-owner (lead), tooling-owner, work-coordinator, evidence-owner.
> **Depends on**: none.

## Intent

An independent validation pass against public `repopact==3.1.1` and the repository
found two avoidable local failure modes and confirmed the published package itself
is sound. This work item:

- reconciles the local checkout and development environment with `origin/main`
  (the checkout had drifted 11 commits behind);
- removes the stale foreign editable `repopact` PATH registration
  (`C:\Projects\repopact-wi050-windows-proof`, reporting `3.1.0.dev1`) and replaces
  it with stable PyPI `3.1.1`;
- adds a top-level `repopact --version` observability surface;
- removes the Workbench type generator's implicit shared `%TEMP%\repopact-rust-target`
  default, which was implicated in a `tree-sitter` build corruption;
- reconciles current-state release governance (WI021 AC-3, WI066) with the real
  published state;
- and, per the authorized release amendment, promotes the validated corrective
  tranche to a published **RepoPact 3.1.2** patch release.

Out of scope: no schema, lifecycle, protocol, or provenance contract changes; no
rewriting of dated historical evidence describing the 3.1.0 milestone or the 3.1.1
sdist corrective (decision 0063, tag `v3.1.1` remain historical authority).

## Decisions

- Development identity during this remediation follows Decision 0032:
  `VERSION=3.1.1` + `RELEASE_LABEL=3.1.1-dev.1` while package/runtime-affecting
  fixes are implemented, removed again once the tree is promoted to stable
  `3.1.2`.
- A new decision record documents the 3.1.2 release rationale as a
  backward-compatible corrective patch (no new governance capability or
  compatibility line), preserving decision 0063 as 3.1.1's historical authority.
- The global PATH fix installs stable PyPI `3.1.1` into the affected environment
  rather than removing the global entry point outright, so a system-wide
  `repopact` command keeps working (operator choice).

## Scope

- `repopact/cli.py`, `repopact/package_version.py` (or equivalent) — `--version`.
- `rust/apps/repopact-desktop/scripts/generate-types.mjs` and a regression test.
- `VERSION`, `RELEASE_LABEL` (transient), release decision record.
- `evidence/runs/` — durable remediation + 3.1.1/3.1.2 clean-install evidence.
- `work/active/021-public-launch/` (AC-3), `work/completed/066-...` reconciliation
  wording only (no historical evidence rewritten).
- `audits/reports/dashboard.md` regeneration.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
