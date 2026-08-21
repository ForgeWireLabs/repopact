# Capture 016 — ForgeWire WI 238 field evidence (RepoPact 3.0.0 migration)

Evidence capture, 2026-08-21. Adopter field evidence from ForgeWire's completed WI 238
("Upgrade ForgeWire to RepoPact 3.0.0"), migrating its pin from `repopact==2.2.0` to
`repopact==3.0.0`. Sourced directly from ForgeWire's own durable evidence records
(`evidence/runs/20260821-152352-238-ci-closeout.json` and
`evidence/runs/20260821-238-repopact-3-0-0-migration-verification.json`, commits
`ed6b28e9` and `91e59163`), not from a relayed summary. This is adopter field evidence,
not a designed proving-ground run or a benchmark result — no benchmark score is
manufactured from it.

## A. Public-version context

ForgeWire installed the package fresh via `python -m pip install --upgrade
--no-cache-dir -r requirements-repopact.txt` pinning the exact string
`repopact==3.0.0`. The installed version was confirmed three independent ways: `pip show
repopact` (`Version: 3.0.0`), `python -c "import repopact; print(repopact.__version__)"`
(`3.0.0`), and the package's own `repopact-3.0.0.dist-info/METADATA` (`Version: 3.0.0`).
This is the current public release at the time of migration — no newer version existed
to pin to instead — not a stale adopter pin.

## B. Worktree reproduction

With the fresh `3.0.0` install, ForgeWire ran a safe, self-cleaning regression check for
the worktree-scan false-positive class first identified in the earlier WI 237 incident
(F-015):

1. `git worktree add --detach .claude/worktrees/wi238-regression-test HEAD` — a real,
   throwaway worktree: detached `HEAD`, zero uncommitted files, an ancestor of `main`,
   the same shape the original WI 237 incident's stale worktrees had.
2. `repopact validate --root .` reported 20 errors: 19x `nested contract is not
   registered in audits/registry.json`, one for every `AGENTS.md` file under the
   worktree, plus 1x stale-dashboard error.
3. `git worktree remove --force .claude/worktrees/wi238-regression-test` followed by
   `git worktree prune -v`. Confirmed fully removed: absent from `git worktree list`,
   absent from disk, `git status --short` showed no residue.
4. `repopact validate --root .` re-run: clean.

Source inspection of the installed `3.0.0` package (`repopact/repo_model.py`) found
`IGNORED_PARTS = {".git", "__pycache__", "node_modules", ".venv", ".pytest_cache",
"build", "dist", "fixtures"}` — no `"worktrees"` entry, and no other worktree-awareness
anywhere in the installed package (a package-wide search for the substring `worktree`
returned zero matches outside this one constant's absence).

**Distinguishing the fix from the release.** Commit `0096d70` ("fix: exclude worktrees/
scratch checkouts from contract scanning", merged 2026-07-28) already adds `"worktrees"`
to `IGNORED_PARTS` on upstream `main`. But the `v3.0.0` tag (`f4039a6` VERSION bump,
`f1db6b4` publication-closing commit) was cut 2026-07-26 — **two days before** `0096d70`
merged. The fix exists on `main`; it was never packaged into any release an adopter
could `pip install`. Re-reading the exclusion check itself —
`any(part in IGNORED_PARTS for part in path.relative_to(root).parts)`, a per-path-
*component* match — confirms the fix, had it shipped, would have correctly excluded
ForgeWire's exact `.claude/worktrees/<name>/` layout (`"worktrees"` is literally one of
that path's components). This is release lag, not a fix that fails to generalize to
ForgeWire's convention.

## C. Doctor reproduction

`repopact doctor --root .` against the same, otherwise-clean `3.0.0` install reported 3
`[source-of-truth-stale]` warnings:

```
work/active/114-forgewire-fabric/_audit/alignment-report.md: source_of_truth points at missing path '../AGENTS.md'
work/active/114-forgewire-fabric/_audit/inventory.md: source_of_truth points at missing path '../AGENTS.md'
work/active/114-forgewire-fabric/_audit/README.md: source_of_truth points at missing path '../AGENTS.md'
```

Each of these 3 records genuinely declares `source_of_truth: ../AGENTS.md`, and
`work/active/114-forgewire-fabric/AGENTS.md` genuinely exists — confirmed directly on
disk. `repopact validate --root .` did not flag any of the three; only `doctor` did.

**Source-code cause:** `doctor.py`'s `_dead_source_of_truth` resolves every token with
`not (root / token).exists()` — unconditionally against the repository root, regardless
of the declaring record's own location or any `../` prefix on the token.

**Semantic verification performed before concluding this is a bug** (not assumed):
`source_of_truth:` is free-form frontmatter, absent from every JSON schema in the
package. The only existing specification is decision `0016`
("Takeover Repoints Inbound References Before Retiring a Plan Directory"), which treats
`source_of_truth:` identically to a Markdown link target and is implemented that way in
`takeover.py`'s `rewrite_inbound_references` — its matching regex
(`(?:\.\./)*(?:{retired}...)/...`) is written specifically to preserve a leading run of
`../` segments, which is only meaningful under record-relative resolution. The one
existing unit test covering `doctor`'s own resolution
(`test_doctor_flags_dead_source_of_truth_pointer`) exercises only a bare, `/`-free token
(`AGENTS.md`) from a record one level below root; it happens to pass under root-relative
resolution only because that bare token coincides with the well-known root-level
contract file (`init_repo.bootstrap` writes only a root `AGENTS.md`, never a nested
`decisions/AGENTS.md`). No existing test exercises a `../`-prefixed token against
`doctor` at all. Conclusion: the corpus establishes record-relative semantics
(decision 0016, implemented in `takeover.py`); `doctor.py`'s root-relative resolution is
an internal inconsistency with the codebase's own other consumer of the same field, not
a defensible alternative reading and not evidence that ForgeWire's records are invalid.

Recorded as [F-017](../findings.md#f-017--doctor-resolves-source_of_truth-against-the-repo-root-not-the-declaring-record).

## D. Downstream impact

Neither issue blocked ForgeWire's migration. WI 238 reached fully green on all three of
its canonical gates — `scripts/ci.py fast`, `scripts/ci.py full`, and
`scripts/ci.py closeout` — with `repopact validate` clean throughout. Both issues were
discovered, reproduced safely, cleaned up (worktree case) or left unmodified (doctor
case — no ForgeWire record was changed to chase a false positive), and reported upstream
here rather than worked around silently in the adopter repository.
