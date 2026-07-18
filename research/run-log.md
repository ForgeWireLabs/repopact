# Run log

Append-only, chronological. Each run links its raw capture and the findings it
produced. Times are local (machine timezone).

## Run 001 — package verification — 2026-06-15 20:22

**Goal.** Test H1 (adoptability) and H2 (closure) against the *packaged* product:
build the wheel, install it into an isolated venv with no access to the source
checkout, and run the full advertised CLI surface.

**Actions.**

1. `python -m venv .venv`; installed `build`; `python -m build` → produced
   `dist/repopact-0.1.0-py3-none-any.whl` and `repopact-0.1.0.tar.gz`. Seed data
   (`schemas/`, `templates/`) packaged under `share/repopact/`.
2. Fresh venv in `$TEMP/rp_pkgtest/venv`; `pip install` the wheel only.
3. `repopact --help`, then `init`, `validate`, `new work-item`, `validate`,
   `dashboard`, `spec`, `new decision` against a bootstrapped sandbox.

**Result.**

- **H1 holds.** Seed data resolved from `<venv>/share/repopact/`; `init` produced a
  repository that `validate` accepted with zero errors, from the wheel alone.
- **H2 partially fails.** `repopact spec` crashed with `FileNotFoundError` because
  `init` seeds no `SPEC.md`. Logged as **F-001** (major).
- `init`, `validate`, `new` (work-item and decision), `dashboard` all succeeded and
  re-validated clean.

**Capture.** [`captures/001-package-verification.md`](captures/001-package-verification.md)

**Findings.** F-001.

**Next.** Stand up the proving-ground CLI project and adopt RepoPact into it from
the installed wheel; begin the happy-path and adversarial exercises (H3–H6).

## Run 002 — proving-ground adoption and workflow — 2026-06-15 20:26

**Goal.** Test H3–H6 against a real adopter: a tiny working CLI (`unitconv`,
temperature + length conversion, 8 passing tests) governed by RepoPact installed
from the wheel.

**Actions and results.**

1. New repo `repopact-proving-ground`; venv; `pip install` the wheel; `repopact
   init` in place → valid (**H1 again, holds**). Authored `unitconv.py` + tests.
2. Added an `app` scope/role to `owners.json` and enabled
   `enforce_disjoint_active_scopes`. Created work item 001 via `repopact new`.
3. **F-003 (H3 holds):** set AC-1 `satisfied` with no evidence → `validate`
   rejected it. Produced evidence run `20260615-202635-unitconv-tests` from the
   real test run, linked it, set both criteria satisfied → `validate` passed.
4. **F-004 (H5 holds):** moved the item to `work/completed/` leaving
   `status: active` → `validate` rejected the status/directory mismatch. Set
   `status: completed` → passed; regenerated dashboard; committed baseline `f3be631`.
5. **F-005 (H4 holds) / F-002 (H4 footgun):** edited protected
   `governance/invariants.json`. `check-frozen` against an uncommitted working tree
   saw nothing (**F-002**); after committing, `check-frozen --base HEAD~1` flagged
   it (exit 1) and required `--ack` (exit 0). Restored the invariant; re-validated.
6. **F-006 (H6 holds):** the full story of work item 001 — intent, the pivot-unit
   decision, and its proof — was recoverable from the tree alone.

**Capture.** [`captures/002-proving-ground-workflow.md`](captures/002-proving-ground-workflow.md)

**Findings.** F-002 (minor, open); F-003/F-004/F-005/F-006 (holds).

**Net.** Of six hypotheses, five hold outright. H2 is the only real crack (F-001),
plus one footgun on H4 (F-002). Both are fixable before declaring 1.0.

**Next.** Resolve F-001 and F-002 in the RepoPact source as a hardening work item +
decision; rebuild and re-verify; then version bump and release prep.

## Run 003 — hardening, rebuild, and re-verification — 2026-06-15 20:37

**Goal.** Fix F-001 and F-002 in the RepoPact source using RepoPact's own workflow,
then prove the fixes hold *from a freshly built wheel* — not just in the checkout.

**Actions.**

1. Branch `007-proving-ground-hardening`. Work item `007` opened via `new.py`.
2. **F-001 fix:** `repopact spec` now prints one-line guidance and exits 1 when no
   `SPEC.md` is present (decision `0006`: `spec` is a maintainer command).
3. **F-002 fix:** `check_frozen_surface` unions `base...HEAD` with uncommitted
   changes vs `HEAD`.
4. Two regression tests added; full suite **30/30 green**.
5. Dogfooded: evidence run `20260615-203707-...`, criteria satisfied, item moved to
   `work/completed/`, dashboard regenerated, `validate` clean. `check-frozen --base
   main` on the branch: no protected paths touched.
6. `VERSION` → `1.0.0`; `SPEC.md` version block regenerated and alpha caveat dropped
   (decision `0007`, supersedes `0005`). Rebuilt `repopact-1.0.0-py3-none-any.whl`.
7. `--force-reinstall` into the proving-ground venv; re-ran both scenarios:
   - `repopact spec --root .` → guidance + exit 1 (F-001 fixed).
   - working-tree edit to protected `invariants.json` → `check-frozen` exit 1 (F-002 fixed).

**Result.** Both defects fixed and re-verified end-to-end from the packaged 1.0.0
product. All six hypotheses now hold. Evidence supports the 1.0 declaration.

**Capture.** [`captures/003-rebuild-reverify.md`](captures/003-rebuild-reverify.md)

**Findings.** F-001 → fixed; F-002 → fixed.

**Next.** Commit the branch; open PR; cut `v1.0.0` (GitHub release + PyPI). Release
mechanics need operator credentials — see the release checklist handed to the operator.

## Run 004 — brownfield adoption — 2026-06-15 21:15

**Goal.** Test H7 (brownfield adoptability), added after the operator identified that
greenfield-only proof was insufficient: an existing repo must be adoptable, turning
its existing workflows/ownership/contracts into a RepoPact.

**Actions.**

1. Built `scripts/adopt_repo.py` + `repopact adopt --target <repo> [--dry-run]`
   (work item 008, decision 0008). Non-destructive; maps CODEOWNERS → scopes,
   workflows → binding-gate policies + `INV-2` + frozen surface, nested `AGENTS.md`
   → registry (stubbing `_audit` triplets), git history → first evidence run.
2. 4 regression tests on a synthetic existing repo → green (full suite 34/34).
3. Read-only `--dry-run` against the **real forgewire** (4569 commits): would create
   27 records, skip 52 existing files.
4. Exported forgewire's working tree (`git archive`, no `.git`), ran real `adopt`:
   **validated as a conformant RepoPact** — 19 nested contracts registered, 7
   CODEOWNERS scopes, 4 CI gates mapped.

**Result.** **F-007 holds.** H7 confirmed on a real, mature, RepoPact-naive project.

**Capture.** [`captures/004-brownfield-forgewire.md`](captures/004-brownfield-forgewire.md)

**Findings.** F-007 (holds / shipped).

**Net.** All seven hypotheses now hold. Greenfield (proving ground) + brownfield
(forgewire) evidence together support declaring 1.0.0.

**Next.** Validate + dashboard; commit; then the operator-gated release (merge,
PyPI + GitHub, push proving ground).

## Run 005 — forgewire kernel re-integration (real repo) — 2026-06-15 21:30

**Goal.** Operator request: do the forgewire adoption *for real* on the actual repo,
on a branch, with a structured/explainable mapping doc for review. Also: operator
flagged that forgewire is RepoPact's **progenitor**, so this is confirmatory and a
real deliverable — not independent generality evidence (see
[`threats-to-validity.md`](threats-to-validity.md), T1).

**Actions.**

1. Confirmed real forgewire clean on `main`; branched `adopt-repopact`.
2. Ran `repopact adopt --target .` for real: 26 records created, **zero existing
   files modified**; `repopact validate` passed.
3. **F-008 found:** the generated `evidence/runs/*.json` was matched by forgewire's
   `.gitignore` `runs/` rule (`git check-ignore` confirmed) — would validate locally
   but break on clone/CI. Added a scoped negation so the records are tracked.
4. Wrote `REPOPACT-ADOPTION.md`: artifact→record mapping table, the F-008 fix, how to
   verify, owner next-steps (promote thesis to invariants, map todos to work items,
   wire `validate` into CI), and how to undo.
5. Committed on the branch (`72c21f4da`), unpushed/unmerged, for review.

**Result.** Kernel re-integrated into its source repo, structured and explainable.
**F-008 (major)** surfaced and mitigated. forgewire recorded as confirmatory (T1).

**Capture.** [`captures/005-forgewire-reintegration.md`](captures/005-forgewire-reintegration.md)

**Findings.** F-008 (major; mitigated in branch, adopt hardening open).

**Net.** Independent (clean-room) brownfield evidence remains the open gap. forgewire
proves the `adopt` engineering and the re-integration deliverable, not generality.

## Run 006 — adopt hardening + independent OSS adoption — 2026-06-15 21:51

**Goal.** Operator directives: harden `adopt` against F-008 now, and get *independent*
(non-progenitor) brownfield evidence now.

**Actions.**

1. **Hardening (WI 010).** `adopt` now runs `git check-ignore` on every record it
   writes and prints a warning naming any an existing `.gitignore` would swallow,
   with suggested negations. Regression test added (a `runs/` collision is flagged).
2. **Independent evidence (F-009).** Cloned **pallets/flask** (no RepoPact lineage),
   ran the hardened `adopt`: 5 workflows → policies, root contract synthesized,
   history → evidence; **validated as conformant**. The guardrail correctly stayed
   silent (Flask has no `runs/` collision). Full suite 35/35.

**Result.** F-008 **fixed** (guardrail); F-009 **holds** (independent datum). T1
partially retired — the workflows/sparse path now has non-circular evidence; the
CODEOWNERS/nested-contract generality still rests on forgewire.

**Capture.** [`captures/006-independent-oss-adoption-flask.md`](captures/006-independent-oss-adoption-flask.md)

**Findings.** F-008 (fixed), F-009 (holds).

**Next.** Complete work items 009 (PyPI Trusted Publishing workflow) and 010; land on
main; the one remaining manual step is the operator registering the PyPI trusted
publisher, after which a release auto-publishes.

## Run 007 — plan import (todos -> work/) — 2026-06-16 09:19

**Goal.** Operator-reported F-010: `adopt` leaves `work/` empty beside the team's real
`todos/`. Build import of existing plan items into the ledger, organized and honest.

**Actions.**

1. Built `repopact import-plan` (`plan_import.py`): detects plan directories
   (`todos/`/`tasks/`/… with `completed/`/`deferred/`/`blocked/` lifecycle folders) and
   markdown checklist files; standalone command for already-adopted repos. Decision 0010.
2. Honesty rule: completed source items → `status: completed` + criterion **waived**
   (no fabricated evidence); active/deferred/blocked → `pending`. Non-destructive
   (source preserved, `source` field records origin), idempotent (skip by slug),
   ID collisions remapped (00-→200, reserved 000 avoided).
3. 4 regression tests; full suite **39/39**.
4. Proven on real forgewire: dry-run detected 75 items; real import on the adopted
   export populated `work/` (21 active, 54 completed/waived, 1 deferred) and **validates**.

**Result.** **F-010 fixed.** The hollow-ledger problem is closed; an adopted repo's
backlog is reflected in `work/` without overstating proven completion.

**Capture.** [`captures/007-plan-import-forgewire.md`](captures/007-plan-import-forgewire.md)

**Findings.** F-010 (fixed).

**Next.** Land on main (1.1.0). Then full ForgeLink integration (upgrade its older
RepoPact, import any plan items) — the operator's stated next task.

## Run 008 — plan-import sources broadened — 2026-06-16 09:32

**Goal.** Operator: "add now" the remaining trackers — section roadmaps (no checkboxes)
and GitHub issues.

**Actions.** Extended `import-plan` (WI 012): `##` section headings classify the
lifecycle of their bullets (Now/Next→active, Later→deferred, Done/Shipped→completed,
Blocked→blocked); opt-in `--issues` imports GitHub issues via `gh` (open→active,
closed→completed/waived), a safe no-op without gh/remote. Top-level bullets only.
2 new tests (41/41). Verified end-to-end on repopact: one `--issues` pass drew from
ROADMAP.md sections and GitHub issues together.

**Result.** `import-plan` now covers four source kinds (dirs, checklists, section
roadmaps, issues). VERSION → 1.2.0.

**Capture.** [`captures/008-plan-import-sources.md`](captures/008-plan-import-sources.md)

**Next.** Apply `import-plan` to real forgewire (branch, review); then ForgeLink.

## Run 009 — real forgewire import + ForgeLink upgrade — 2026-06-16

**forgewire (operator: "yes").** Branched real forgewire; ran `import-plan`: `todos/`
→ `work/` (22 active, 53 completed/waived, 1 deferred); `todos/` preserved; validates.
Committed on branch `import-plan-todos` (`1a1ea75c7`), unmerged for review.

**ForgeLink upgrade (operator: commit WIP, reconcile + refresh to 1.2.0).** The real
ForgeLink ran an older RepoPact that **no longer validated** (4 errors: missing root
`AGENTS.md`; `registry.json` referencing a removed `docs/` scope and a renamed
`todos/001`; an unregistered `work/001` contract). Branched `adopt-repopact-1-2-0`;
added the root contract, repaired the registry, refreshed vendored tooling to 1.2.0,
generated the dashboard, included the WIP (`work/001-production-readiness`, requirements
edit); left `.local/` untracked. Validates as conformant 1.2.0. Committed `d56ea4b`,
unmerged for review.

**Finding F-011 (major).** Longitudinal adoption gap: an older adopter drifted invalid
and nothing in RepoPact detected or guided the upgrade. The architecture needs a
reconcile/upgrade story (`repopact upgrade`/`doctor`). Recorded for the paper as the
first evidence about adoption *over time* (vs. one-shot adopt).

**Capture.** [`captures/009-forgelink-upgrade.md`](captures/009-forgelink-upgrade.md)

**Net.** Both operator tasks delivered on review branches. The standout new datum is
F-011 — drift-over-time is real and currently unmanaged.

## Run 010 — repopact doctor (resolves F-011) — 2026-06-16 10:03

**Goal.** Operator: build `repopact doctor` now. Turn the F-011 manual reconciliation
into a tool.

**Actions.** Built `repopact doctor [--root] [--fix]` (WI 013, decision 0011): read-only
drift diagnosis (missing root contract, stale registry scopes, unregistered nested
contracts, incomplete `_audit` triplets, schema skew, gitignored records) + safe `--fix`.
3 regression tests (44/44). Verified on a synthetic drifted repo (diagnose → fix →
valid) and on the **real ForgeLink** and **forgewire**.

**Key safety finding (from the real ForgeLink run).** ForgeLink's `work-item.schema.json`
carries an intentional `preflight` extension. A naive schema refresh would have
clobbered it. So `--fix` only *adds missing* schemas and reports a differing schema as a
`schema-differs` warning for human review — never overwrites it. Only already-invalid
registry entries are removed; every other fix is additive.

**Result.** **F-011 fixed.** Adoption now has a longitudinal drift/repair surface.
VERSION → 1.3.0.

**Capture.** [`captures/010-repopact-doctor.md`](captures/010-repopact-doctor.md)

**Net.** The toolchain now spans the full adoption lifecycle: `adopt` (onboard),
`import-plan` (populate), `validate`/`check-frozen` (gate), `doctor` (keep healthy
over time).

## Run 011 — SkillForge real-world adoption — 2026-06-16 10:23

**Goal.** Operator: one more real-world test — adopt RepoPact into SkillForge Academy
(local dir `apex-a-plus-academy`), an independent Tauri cert-learning app.

**Actions.** Branch `adopt-repopact`. Ran the full lifecycle: `adopt` (kept their
`AGENTS.md`; release workflow → policy; their `audits/*.md` coexist with the added
registry) → `import-plan` (flat `todos/TODO-001..007` → `work/active/001..007`;
ROADMAP `Shipped`→8 completed/waived, `Next`→active; 14 active / 9 completed) →
`doctor` (healthy). `todos/`/`tracking/` preserved. Validates. Committed `f3ed89c`,
unmerged for review. The flat `TODO-NNN` files motivated a one-line importer fix
(`_split_num` strips `TODO-`/`TASK-`/`ISSUE-` prefixes; regression-tested, 45/45).

**Result.** **F-012 holds.** Third independent adopter, different domain, full
lifecycle to conformant. Threats T1 further narrowed (flat-todo + roadmap + homegrown-
audit coexistence now have an independent datum); CODEOWNERS/nested-contract generality
still rests on forgewire.

**Capture.** [`captures/011-skillforge-adoption.md`](captures/011-skillforge-adoption.md)

## Run 012 — tracking import + takeover — 2026-06-16 10:53

**Goal.** Operator (mid-takeover): SkillForge also plans via `tracking/`, which RepoPact
didn't adapt to — correct that — and provide the takeover that removes side-by-side
old/new methods.

**Actions.** (1) `track_import.py`, folded into `import-plan`: `tracking/` →
`decisions/` (DEC-NNN → free 4-digit id), `audits/findings/` (RISK-NNN → numeric id per
schema, original kept in `source`/observed; P-severity & status mapped), `work/`
(milestones). `status.md`/`work-log.md`/companions deliberately not fabricated.
(2) `takeover` command (CLI-wired, drafted before the redirect): retire a fully-migrated
plan dir, archive (default) or `--delete`, gated on validation + per-item provenance;
mixed/partial sources reported for review. 5 new tests (50/50).
(3) Applied to real SkillForge: tracking → 3 decisions + 5 findings + 6 milestones;
`takeover` archived `todos/` → `archive/todos/`; `tracking/` kept for review; validates,
doctor healthy. Branch `adopt-repopact` (commit 7fb28c7), unmerged.

**Result.** **F-013 holds.** Governance-folder planning is adapted; takeover yields one
ledger without losing un-captured content. VERSION → 1.4.0.

**Capture.** [`captures/012-tracking-and-takeover.md`](captures/012-tracking-and-takeover.md)

## Run 014 — canonical dashboard fixpoint + direct 2.2.0 release — 2026-07-18

**Goal.** Eliminate the state in which a valid ledger coexists with a stale checked-in
dashboard, release the new guarantee, and advance a real adopter from an interim commit
pin to the formal package.

**Actions.** Work item 026 added canonical dashboard rendering, missing/stale rejection
inside the one-tree validator, deterministic repair, and mutation-path regeneration.
ForgeLink proved the historical stale dashboard fails and regenerates cleanly. Work item
027 aligned version/spec/conformance surfaces at 2.2.0; 101 tests (2 documented skips),
8/8 conformance cases, governance validation, repeat-generation hashes, distribution
build, and Twine checks passed. GitHub Actions publishing was billing-blocked, so the
operator-authorized direct fallback uploaded the exact hash-pinned wheel and sdist.
Public PyPI metadata matched both hashes; a clean no-cache install bootstrapped and
validated a fresh repository. ForgeLink then pinned `repopact==2.2.0`, passed its full
commit/push gates, and pushed commit `86f8d67`.

**Result.** RepoPact 2.2.0 is public and externally installable. Dashboard equality is
now a validator-enforced state fixpoint. The direct upload restores distribution but
does not close the separate GitHub Actions/cross-platform CI limitation in GA-3.

**Capture.** [`captures/014-dashboard-integrity-and-direct-release.md`](captures/014-dashboard-integrity-and-direct-release.md)

## Run 015 — ecosystem-wide 2.2.0 adopter rollout — 2026-07-18

**Goal.** Move every known adopter to the formal dashboard-integrity release and prove
the public default branches, rather than only local checkouts, are current.

**Actions.** Local dependency discovery plus ForgeWireLabs code search identified five
adopter remotes. ForgeLink was already current. SkillForge, ForgeWire, and Proving
Ground moved from 2.1/interim commit pins to `repopact==2.2.0`. Moto reconciled its
vendored tooling while preserving ROM-lab safety extensions. Each adopter created and
closed its own preflight work item, regenerated its dashboard, ran governance and native
checks, committed only its owned files, and pushed its default branch. GitHub API and
remote-head checks verified all five public states.

**Result.** Every inventoried adopter is at RepoPact 2.2.0. The Proving Ground's stale
package pin gap is closed. Its drift harness now refreshes the dashboard after setup so
the 2.2.0 fixpoint cannot confound mutation attribution.

**Capture.** [`captures/015-repopact-2-2-0-adopter-rollout.md`](captures/015-repopact-2-2-0-adopter-rollout.md)
