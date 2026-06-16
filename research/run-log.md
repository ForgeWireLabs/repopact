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
