---
id: 0031
title: Restore remote enforcement on GitHub Actions with an AppVeyor fallback
status: proposed
date: 2026-07-27
supersedes: []
---

# 0031: Restore remote enforcement on GitHub Actions with an AppVeyor fallback

> **Status is `proposed` on purpose.** Work item `032` AC-1 requires an *operator*
> to select the checkpoint. This record exists so that choice is a yes/no rather
> than a research project. Nothing here is provisioned; accepting this record and
> performing the steps in "Operator actions" is what unblocks `032`.

## Context

Work item `032` has been blocked since 2026-07-18 on the belief that restoring
remote CI requires an operator to "clear the billing lock or authorize and
provision an alternative CI service" — framing that treats buying CI as a live
possibility. Investigation on 2026-07-27 shows the situation is both cheaper and
narrower than that framing assumes.

**What is actually true.**

1. `ForgeWireLabs/repopact` is a **public** repository on a user account
   (`gh repo view` reports `"visibility":"PUBLIC"`).
2. GitHub Actions on standard runners is **free and unlimited for public
   repositories**. GitHub's billing documentation states: "GitHub Actions usage
   is free for self-hosted runners and for public repositories that use standard
   GitHub-hosted runners." There is therefore **no CI bill to pay** for this
   repository's gates.
3. Actions is **enabled at the repository level**:
   `gh api repos/ForgeWireLabs/repopact/actions/permissions` returns
   `{"enabled":true,"allowed_actions":"all"}`. Nothing is misconfigured here.
4. Workflows are **dispatched and immediately rejected**, not disabled. Every
   push to `main` since 2026-07-22 produced a run that failed in 2–6 seconds with
   the annotation: *"The job was not started because your account is locked due to
   a billing issue."* The most recent is run `30218026017`.
5. `main` has **no branch protection at all**
   (`gh api .../branches/main/protection` returns 404, "Branch not protected").

Point 5 matters independently of the billing lock: `032` AC-3 requires a *required*
gate that demonstrably rejects an invalid change. Even with CI green tomorrow,
there is currently no merge gate to make required. Protected branches are
available for public repositories on the Free plan, so this costs nothing but has
never been configured.

**What is uncertain.** This lock is a widely reported failure mode on free-tier
accounts whose repositories are public and whose usage is zero — GitHub Community
discussions [184077](https://github.com/orgs/community/discussions/184077),
[186091](https://github.com/orgs/community/discussions/186091), and
[188932](https://github.com/orgs/community/discussions/188932) all describe
accounts locked with nothing owed. Reported triggers include an unverified primary
email, a payment method whose authorization hold failed, and residue from a lapsed
Copilot trial. Reported outcomes are mixed: some clear in minutes by re-adding the
payment method, while others report waiting weeks with no support response. **The
cause for this specific account has not been established** — only the symptom is
confirmed — so a plan that assumes the unlock will succeed is not safe.

## Decision

Restore enforcement on **GitHub Actions**, and pre-commit to **AppVeyor** as the
fallback if the account lock does not clear within a fixed window.

1. **Primary — GitHub Actions.** The workflow already exists
   (`.github/workflows/governance.yml`), already runs the correct gate set, and
   costs nothing for a public repository. The only work is clearing the account
   lock. No migration, no new vendor, no new credentials.
2. **Time-box — 14 days from acceptance.** If Actions has not run a green job by
   then, the fallback triggers automatically rather than the item drifting back
   into "blocked" for another month. The community evidence above is the reason
   for a deadline: this class of lock is known to persist indefinitely for some
   accounts, and `032` has already sat idle for nine days.
3. **Fallback — AppVeyor.** Free for open source with unlimited public projects,
   hosted **Windows and Ubuntu Linux** images, one concurrent job, and a 60-minute
   per-job cap. The cap is comfortable: the suite now runs in under two minutes
   after work item `036` AC-5, and the full gate set well inside ten.
4. **Branch protection is provisioned either way**, and is not contingent on which
   provider wins. Without it there is no required gate and AC-3 cannot be
   satisfied by any provider.

## Alternatives considered

- **Cirrus CI.** Would have been the natural first choice — free for public
  repositories, first-class Windows and Linux. **It is dead.** Cirrus Labs
  announced the shutdown on 2026-04-07 and the service stopped running jobs on
  **2026-06-01**. Recorded explicitly because it is the option a reader would
  otherwise reach for, and because it is no longer discoverable as a failure by
  trying it.
- **Self-hosted GitHub Actions runners.** Superficially attractive, since
  self-hosted runner usage is free. It does not work: the account lock prevents
  jobs from *starting at all*, before any runner is selected, so a self-hosted
  runner inherits the same block. It also cannot satisfy AC-2's "no dependency on
  a developer workstation" if hosted on the maintainer's machine.
- **GitLab CI via repository mirror.** Free Linux minutes, but hosted Windows
  runners are not part of the free SaaS offering, and AC-2 requires Windows. It
  would also split the source of truth across two forges for a gate that exists to
  make the repository self-describing.
- **CircleCI / Buildkite / Travis.** All viable engineering, none free for the
  Linux-plus-Windows matrix this needs on a public repo without credit accounting.
  Rejected as strictly worse than AppVeyor for the same requirement.
- **Accept workstation-only enforcement and drop the claim.** The honest
  do-nothing option: stop asserting CI-backed enforcement and say gates run
  locally. Rejected because `release_build.py` already carries the weight of a
  missing checkpoint, and the paper's enforcement claim is load-bearing for the
  project's thesis. But note this *is* the current de facto state, and AC-4
  already forbids claiming otherwise until a green public run exists.

## Operator actions

The yes/no. Steps 1–3 are the whole of AC-1 if the unlock works.

1. **Verify the primary email.** GitHub → Settings → Emails; resend verification
   if the primary address is unverified. Commonly reported as the entire cause.
2. **Re-seat the payment method.** Settings → Billing and plans → Payment
   information; remove the existing card and re-add it, which forces a fresh
   authorization hold. Check for a "payment method authorization failed" notice.
3. **Push any commit and confirm a green run.** If it runs, AC-1 and AC-2 follow
   immediately from the existing workflow.
4. **Enable branch protection on `main`** regardless of outcome: require the
   governance status check to pass before merging. This is what makes AC-3
   provable.
5. **If no green run within 14 days**, sign up for AppVeyor with the GitHub
   account, enable the `repopact` project, and port the gate set to
   `appveyor.yml` across `Ubuntu` and `Visual Studio 2022` images.

## Consequences

- Expected ongoing cost is **zero** under either provider. The blocked item was
  never a spending decision; it was an account-hygiene decision wearing a
  spending decision's clothes.
- Accepting this record does not by itself satisfy any of `032`'s criteria. AC-1
  is satisfied when a checkpoint is provisioned and this record is moved to
  `accepted` with the residual trust assumptions recorded.
- Branch protection introduces friction the repository has never had: direct
  pushes to `main` stop working, and the maintainer's current workflow is
  direct-to-`main` commits. That is the point of AC-3, and it is a real change to
  how this repository is worked. It should be a deliberate yes, not a side effect.
- If the fallback triggers, `.github/workflows/governance.yml` remains in the tree
  as the canonical statement of the gate set, and `appveyor.yml` must be kept in
  step with it — a duplication cost that is itself an argument for spending real
  effort on the unlock first.
