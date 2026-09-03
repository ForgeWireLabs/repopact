# 048 — Release RepoPact 3.0.2 Corrective Path and Worktree Fixes

> **Status**: ✅ Completed.
> **Owners**: governance-owner (lead); tooling-owner, docs-owner, evidence-owner, and work-coordinator affected.
> **Depends on**: WI042, WI043, WI045.

## Intent

RepoPact 3.0.1 established a stable source/artifact identity rule and then intentionally moved `main` onto the governed development identity `3.0.1-dev.1` before accepting further runtime changes. Two confirmed corrective changes have now completed on that development line:

1. WI043 makes `source_of_truth:` path resolution uniformly record-relative, correcting `doctor.py` so it agrees with decision 0016 and `takeover.py` rather than resolving against repository root.
2. WI042 replaces directory-name-only worktree protection as the primary mechanism for contract discovery with structural same-repository linked-worktree detection, while preserving genuine independent nested repositories and retaining the literal `worktrees` fallback for stale/orphaned conventional scratch trees.

Both changes repair demonstrated false-positive/contract-discovery behavior without introducing a new product capability. They therefore form a coherent **3.0.2 corrective patch release**.

## Release scope

Before any version or tag change, the implementation session must enumerate the complete executable/package-affecting `v3.0.1..release` delta and verify that no unrelated runtime change has entered `main`.

The intended runtime scope is:

- WI043 record-relative `source_of_truth` diagnosis semantics;
- WI042 structural linked-worktree awareness in AGENTS contract discovery;
- tests, SPEC/audit updates, and release machinery/evidence necessary to ship those fixes.

The following remain outside 3.0.2:

- proposed WI044 typed capability-completion/cutover evidence contracts;
- proposed WI046 runner-neutral verification/admission checkpoint architecture;
- proposed WI047 documentation-impact/code-documentation closure;
- WI037 adopter-fleet migration/package-boundary reconciliation;
- WI032 remote cross-platform enforcement restoration.

No proposed feature scope is pulled into the release merely because its planning records are present on `main`.

## Stable identity transition

Decision 0032 remains binding. Development `main` currently carries:

- `VERSION = 3.0.1`
- `RELEASE_LABEL = 3.0.1-dev.1`

The exact stable 3.0.2 release tree must instead carry:

- `VERSION = 3.0.2`
- no `RELEASE_LABEL`
- package metadata exactly `3.0.2`

The `v3.0.1` tag must never move. The new annotated `v3.0.2` tag must point at the exact committed tree used for deterministic release building and isolated local-wheel validation.

## Required release proof

The established hardened release boundary is reused:

1. validate the exact committed release tree locally;
2. prove both WI043 and WI042 behaviors from that tree;
3. run `repopact release-build` to build twice from independent Git exports and require byte-identical wheel/sdist hashes;
4. run Twine metadata checks;
5. install the exact local wheel in a fresh Windows environment outside the checkout and exercise package resources, CLI behavior, record-relative doctor semantics, and structural worktree discovery from `site-packages`;
6. create and push annotated `v3.0.2` on that exact commit;
7. publish the exact validated artifacts manually/local from operator-owned hardware, without GitHub-hosted execution;
8. no-cache download the public wheel and sdist, prove exact SHA-256 equality with the validated local artifacts, and repeat installed public-wheel behavior proof;
9. only then satisfy publication criteria and move WI048 to completed.

## Execution boundary

RepoPact remains under the temporary local-only execution directive associated with WI032. No GitHub-hosted runner or workflow execution is needed or permitted for this release. A successful local/public package release does not satisfy WI032.

PyPI authentication is an operator-held secret. The implementation session may perform the local/manual Twine transaction, but credentials must never be printed, committed, included in evidence, or persisted in repository state.

## Closeout

All ten acceptance criteria require concrete evidence. Release success is source → exact commit → deterministic local artifacts → annotated tag → public artifacts with matching hashes → external installed behavior. Version strings or Twine upload success alone are insufficient.

## 2026-09-02 stable public publication and closeout

Decision [0036](../../../decisions/0036-release-repopact-3-0-2.md) defines
RepoPact 3.0.2 as the backwards-compatible corrective patch for WI043's
record-relative `source_of_truth:` diagnosis and WI042's structural linked-
worktree contract discovery. The exact stable release commit is
`dfab8cb8ca010a865ebb3291c526179fb5cb4b2c`; `VERSION` is `3.0.2`,
`RELEASE_LABEL` is absent, and annotated `v3.0.2` points there. `v3.0.1`
remains unchanged at `181e35a84605a966487199a6ee22cb5e4dfb9176`.

The deterministic artifacts built from that commit are
`repopact-3.0.2-py3-none-any.whl` (SHA-256
`d7781495977a686ab791d4ce97467a3714474c7682253d9d81f4edcfe4a2fa77`) and
`repopact-3.0.2.tar.gz` (SHA-256
`8815f8b335d20c6f27006bd29a6da48ef57885f13cb98df8bb7cddcd4776dd83`).
They passed Twine and package-boundary checks, were published manually/local
from operator-owned Windows hardware, and public no-cache downloads match both
hashes exactly. Fresh external local-wheel and public-wheel environments loaded
from `site-packages`, resolved schemas/templates, and passed `init`, `adopt`,
`validate`, WI043 doctor behavior (including the root-coincidence negative
control), and WI042 worktree/nested-repository behavior. Focused regressions
passed 11/11; the full suite passed 151 tests with two existing formal-model
skips; conformance passed 20/20; dashboard/SPEC, governance, and frozen-surface
checks passed. Evidence is recorded in
[20260902-048-release-delta](../../../evidence/runs/20260902-048-release-delta.json)
and [20260902-048-release-3-0-2-final](../../../evidence/runs/20260902-048-release-3-0-2-final.json).

WI032 remains blocked/local-only; WI037 remains separate adopter-fleet work;
WI044, WI046, and WI047 remain unimplemented. The complete work-item directory
is now under `work/completed/` and all ten acceptance criteria are satisfied.
