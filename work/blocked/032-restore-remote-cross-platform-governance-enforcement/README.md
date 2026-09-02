# 032 — Restore remote cross-platform governance enforcement

> **Status**: Blocked
> **Owners**: tooling-owner (lead); governance-owner and evidence-owner affected.
> **Depends on**: `009`, `027`.

## Intent

Restore a public, cross-platform commit checkpoint. Direct PyPI upload recovered package
distribution, but it did not restore the paper's CI enforcement claim or exercise Windows
and Linux on every change.

**Cross-reference (2026-08-21, work item 039, narrow — no criterion here is
satisfied and no status changes as a result).** An independent, naturalistic
field observation in ForgeWire's own repository (RepoPact's progenitor
adopter) demonstrated the same higher-level failure class this item exists
to close, through a different mechanism: ForgeWire's hosted CI ran on every
push but never invoked the RepoPact CLI at all (a checkpoint-*coverage*
gap), whereas this repository's gap — documented above and in decision 0031
— is checkpoint-*invocation* (the billing lock) compounded by checkpoint-
*effectiveness* (no branch protection), with coverage itself present. The
case study (`research/case-studies/2026-08-forgewire-wi230-wi237-
enforcement-closure/`) and the resulting formal treatment
(`formal-model.md` §7: `Cov`/`Inv`/`Eff`/`EC`) independently motivate why
this item's AC-1–AC-3 need to be evaluated as three distinct properties
rather than one "CI is restored" outcome — they do not resolve this item's
own provider/operator decision, and the chosen remote-enforcement design
(GitHub Actions primary, AppVeyor fallback, decision 0031) is unchanged.

## Blocker

GitHub Actions is payment-locked. Progress requires an operator to clear that lock or
authorize and provision an alternative CI service and its repository credentials.

**Narrowed 2026-07-27** (decision [`0031`](../../../decisions/0031-restore-remote-enforcement-on-github-actions-with-an-appveyor-fallback.md)).
The blocker is smaller than this framing implied, and the operator choice is now a
yes/no rather than an open question:

- This repository is **public**, and Actions on standard runners is **free and
  unlimited** for public repositories. There is no CI bill to authorize.
- Actions is **enabled** at the repository level; workflows are dispatched and
  rejected in 2–6 seconds with *"your account is locked due to a billing issue"*
  (most recently run `30218026017`). The lock is account-level, not repository- or
  cost-related.
- The lock is a widely reported failure mode on free-tier accounts with nothing
  owed, often caused by an unverified primary email or a failed payment-method
  authorization hold. Its cause **on this account is not established** — only the
  symptom — and community reports show it can persist for weeks, so decision `0031`
  time-boxes the attempt at 14 days and pre-commits **AppVeyor** as the fallback
  (free for open source, hosted Windows and Linux images).
- Cirrus CI, the option a reader would otherwise reach for, **shut down on
  2026-06-01** and must not be proposed.

Independently of the lock: `main` has **no branch protection** (`404 Branch not
protected`). AC-3 requires a *required* gate, so there is currently nothing to make
required under any provider. Protected branches are free for public repositories.

### Temporary local-only operator directive — 2026-09-01

The operator has explicitly suspended **all GitHub-hosted task execution** while the
account lock is active. RepoPact's `governance.yml` and `release.yml` remain checked in
for history/provider-adapter reference, but their hosted jobs are deliberately guarded
with `if: ${{ false }}` so no GitHub-hosted runner is allocated.

Until the operator explicitly lifts this stop:

- do not retry or re-run GitHub Actions;
- run governance, tests, conformance, dashboard/spec generation, release-build checks,
  and work-item acceptance validation on operator-owned/local hardware;
- commit concrete local evidence under `evidence/runs/` in the normal RepoPact format;
- do not represent a successful local run as proof that the remote/public admission
  boundary is effective — WI032 remains blocked and AC-1 through AC-3 remain pending.

This temporary execution policy is also a motivating field case for proposed WI046,
which investigates a runner-neutral verification/admission-checkpoint architecture.
WI046 does not satisfy any WI032 criterion and does not replace this item's remote
checkpoint requirement.

## Decisions

Do not treat direct PyPI publication as CI restoration. The chosen remote service and
required-gate policy are durable operational choices and require an operator-approved
decision.

## Scope

- Remote Linux and Windows governance/test/build jobs.
- Required merge enforcement and a negative gate proof.
- Public run evidence and accurate release/paper language.

## Acceptance criteria

- [ ] **AC-1** — operator selects and provisions a usable remote checkpoint.
- [ ] **AC-2** — Linux and Windows run the complete release-relevant gate set.
- [ ] **AC-3** — required enforcement demonstrably rejects an invalid change.
- [ ] **AC-4** — publication and enforcement claims remain explicitly separated.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
