# 045 — Release RepoPact 3.0.1 and Reconcile Stable Source/Artifact Identity

> **Status**: 🚧 Active.
> **Owners**: governance-owner (lead); tooling-owner, docs-owner, evidence-owner, and work-coordinator affected.
> **Depends on**: WI038 (3.0.0 release boundary), WI041 (release-lag field evidence).

## Intent

RepoPact's public package and its default branch currently share the same stable version string
while containing different runtime code.

The public `repopact==3.0.0` wheel is a valid 3.0.0 artifact, but stable changes landed on
`main` after the `v3.0.0` release. The reproduced example is contract discovery: the public
wheel's `repopact.repo_model.IGNORED_PARTS` does not include `worktrees`, while current `main`
does. RepoPact's own WI041/F-015 evidence established that the fix landed after the 3.0.0 tag
and was never packaged. A clean Windows reproduction on 2026-08-22 independently confirmed
the public-wheel/source difference from isolated imports outside the source checkout.

This creates two obligations:

1. publish the already-accepted stable corrective delta as **RepoPact 3.0.1**; and
2. close the release-identity gap that allowed materially newer runtime source and the already
   published artifact to remain indistinguishable at the governed version layer.

These obligations belong together because publishing 3.0.1 without preventing recurrence would
repair the current artifact while preserving the mechanism that caused the ambiguity.

## Release scope

3.0.1 is intended as a backwards-compatible corrective release, not a vehicle for proposed
feature work.

The release investigation must enumerate the complete `v3.0.0..release` package/runtime delta
before publication. The known runtime correction is the post-release worktree exclusion in
`repopact/repo_model.py` and its regression coverage. Research records, evidence, completed work,
and planning records may naturally be present because they are part of the repository history,
but their presence does not authorize their proposed implementations.

In particular:

- WI042 remains proposed structural worktree-awareness work beyond the already-landed conventional
  `worktrees` exclusion;
- WI043 remains proposed `source_of_truth` path-resolution work;
- WI044 remains proposed typed completion/cutover evidence-contract work.

None of those implementation scopes is implicitly accelerated into 3.0.1.

## The identity problem

RepoPact already has two relevant concepts:

- `VERSION` is the strict `MAJOR.MINOR.PATCH` equality and total-order anchor used by adopter
  compatibility logic;
- decision 0026 defines optional `RELEASE_LABEL` as a governed pre-release identity whose SemVer
  core must equal `VERSION`.

That existing design must be respected rather than bypassed. However, the package metadata in
`pyproject.toml` currently derives its distribution version from `VERSION`, so merely having a
pre-release record available does not by itself prove that an installed development artifact
would carry a distinct identity.

WI045 therefore does **not** preselect a fix. It must evaluate the existing `RELEASE_LABEL`
mechanism, package-metadata derivation, post-release version transitions, and tag/release-aware
validation (or justified equivalents) and select the narrowest rule that preserves adopter
semantics while preventing recurrence.

The required invariant is behavioral:

> Once a stable version has been published/tagged, a later package/runtime-affecting `main` state
> must not remain indistinguishable from that published artifact solely because `VERSION` has not
> changed.

An unchanged release commit is valid. The problem begins when later runtime/package source moves
while its governed artifact identity does not.

## Publication proof

The 3.0.1 release must reuse the hardened 3.0.0 release boundary rather than treating a normal
checkout wheel as trustworthy:

1. prepare and validate the exact committed release tree;
2. run `repopact release-build` so two independent clean exports produce byte-identical artifacts;
3. run `twine check` and record exact hashes;
4. install the exact local wheel in a clean Windows environment outside the checkout;
5. prove the packaged runtime contains the intended stable correction, specifically that
   `"worktrees" in repopact.repo_model.IGNORED_PARTS` is `True`;
6. tag and push the exact commit;
7. publish those exact artifacts;
8. no-cache download the public wheel, prove its hash equals the validated local artifact, and
   repeat the installed-package behavior check from `site-packages`.

The release evidence must distinguish repository/source success from public artifact success.

## Boundaries

This work does not close unrelated obligations:

- WI037 continues to own adopter-fleet migration/package-boundary reconciliation;
- WI032 continues to own remote cross-platform governance enforcement;
- WI042, WI043, and WI044 keep their existing statuses and scopes;
- a successful local or public package release does not count unavailable GitHub Actions as green.

## Closeout

All acceptance criteria in `work-item.json` require concrete linked evidence. The closeout must
show both sides of the repair: public 3.0.1 actually contains the stable code that 3.0.0 missed,
and the repository now has a tested rule preventing a future materially newer package/runtime
`main` from silently retaining the exact identity of an already-published stable artifact.

## 2026-09-02 local release state

Decision 0032 keeps strict `VERSION` adopter equality, accepts the exact matching stable tag
tree without a label, and requires later same-core package/runtime source to carry a
VERSION-pinned `RELEASE_LABEL`; package metadata maps that label deterministically to PEP 440.
Decision 0033 enumerates the 3.0.1 delta: the `worktrees/` exclusion, identity-aware validation and
packaging/release machinery, and their regression coverage. The exact tagged commit is
`181e35a84605a966487199a6ee22cb5e4dfb9176`; local evidence is recorded in
`20260902-045-release-3-0-1-local-validation`. AC-1 through AC-8, AC-10, and AC-11 are
satisfied. AC-9 remains pending because the non-interactive PyPI upload found no API token;
see `20260902-045-publication-blocked`. No hosted GitHub execution was invoked.

## 2026-09-02 public publication closeout

The exact validated wheel and sdist were published manually from operator-owned
Windows hardware. Public PyPI metadata and no-cache downloads match both local
SHA-256 values exactly. A fresh external environment installed the public wheel
from site-packages, reported distribution version `3.0.1`, resolved packaged
schemas/templates, and passed installed `init`, `adopt`, and `validate` smoke
tests with `worktrees=True`. AC-9 is now satisfied by
`20260902-045-public-publication-verification`; the earlier blocked attempt is
retained as historical evidence. WI032 remains blocked/local-only and WI037
remains a separate adopter-fleet obligation.
