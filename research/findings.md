# Findings register

Each finding tests a hypothesis from [`protocol.md`](protocol.md) and cites the raw
capture behind it. Severity reflects impact on an adopter, not effort to fix.

- **blocker** — defeats a core claim or stops adoption cold.
- **major** — a documented path crashes or a gate fails to fire.
- **minor** — rough edge; the architecture holds but the experience or wording is wrong.
- **holds** — an adversarial case the architecture correctly caught (recorded as evidence *for*).

| ID | Hypothesis | Severity | One-line | Capture | Resolution |
| --- | --- | --- | --- | --- | --- |
| F-001 | H2 | major | `repopact spec` crashes on an `init`-fresh repo (no `SPEC.md` seeded) | [001](captures/001-package-verification.md) | **fixed** (WI 007, dec 0006) — re-verified [003](captures/003-rebuild-reverify.md) |
| F-002 | H4 | minor | `check-frozen` diffs `base...HEAD` only; a working-tree change to a protected file reports a false "all clear" pre-commit | [002](captures/002-proving-ground-workflow.md) | **fixed** (WI 007) — re-verified [003](captures/003-rebuild-reverify.md) |
| F-003 | H3 | holds | Validator rejects a criterion marked `satisfied` with no evidence | [002](captures/002-proving-ground-workflow.md) | n/a |
| F-004 | H5 | holds | Validator rejects a work item whose `status` disagrees with its directory | [002](captures/002-proving-ground-workflow.md) | n/a |
| F-005 | H4 | holds | `check-frozen` flags a committed change to a protected path; `--ack` is required to pass | [002](captures/002-proving-ground-workflow.md) | n/a |
| F-006 | H1,H6 | holds | A reader reconstructed the entire 001 work item — intent, decision, proof — from the tree alone | [002](captures/002-proving-ground-workflow.md) | n/a |
| F-007 | H7 | holds* | `repopact adopt` converted a real 4569-commit repo (forgewire; 19 contracts, 7 CODEOWNERS scopes, 4 CI gates) into a conformant RepoPact, non-destructively. *Confirmatory only — forgewire is RepoPact's progenitor (see validity caveat). | [004](captures/004-brownfield-forgewire.md) | shipped (WI 008, dec 0008) |
| F-008 | H7 | major | An existing repo's `.gitignore` (`runs/`) silently un-tracks RepoPact's `evidence/runs/*.json`: validates locally, breaks on clone/CI | [005](captures/005-forgewire-reintegration.md) | **fixed** — `adopt` now warns (WI 010); mitigated in forgewire branch |
| F-009 | H7 | holds | `repopact adopt` brought a clean-room OSS repo (pallets/flask; no RepoPact lineage, 5 workflows) to a conformant RepoPact — independent generality evidence | [006](captures/006-independent-oss-adoption-flask.md) | n/a (WI 010) |
| F-010 | H7 | major | `adopt` left `work/` empty beside the team's real `todos/` (operator-reported); the ledger didn't reflect actual planning | [007](captures/007-plan-import-forgewire.md) | **fixed** — `repopact import-plan` (WI 011, dec 0010) |
| F-011 | H7 | major | An older adopter (ForgeLink) drifted *invalid* as the standard evolved (stale registry paths, missing root contract); nothing detected or guided the upgrade | [009](captures/009-forgelink-upgrade.md) | **fixed** — `repopact doctor [--fix]` (WI 013, dec 0011), proven on real ForgeLink [010](captures/010-repopact-doctor.md) |
| F-012 | H7 | holds | Full lifecycle (adopt+import-plan+doctor) on an independent, different-domain real app (SkillForge, a Tauri cert-learning app) reached conformant RepoPact, non-destructively | [011](captures/011-skillforge-adoption.md) | shipped (WI 014); motivated TODO-prefix import fix |
| F-013 | H7 | holds | RepoPact adapts to governance-folder planning (tracking/ → decisions/findings/milestones) and `takeover` retires the migrated old method, leaving one ledger without losing un-captured data | [012](captures/012-tracking-and-takeover.md) | shipped (WI 015, dec 0012) |
| F-014 | H6,H7 | holds | A downstream adopter exposed the missing `proposed` authority state; the resolution is recoverable through decision, implementation, conformance, release, and adopter records | [013](captures/013-proposed-lifecycle-adoption-pressure.md) | shipped (WI 025, dec 0023/0024, release 2.1.0) |
| F-015 | H7 | major | 2.2.0's raw filesystem contract walk (`IGNORED_PARTS`) had no awareness of local `git worktree` checkouts, so scratch worktrees under an adopter's tree were scanned as governed nested content, producing false validation errors (171 of 269 reported errors in one longitudinal adopter incident); re-reproduced live on the current **public 3.0.0 release** during a second adopter migration (WI 238) | [case study](case-studies/2026-08-forgewire-wi230-wi237-enforcement-closure/), [016](captures/016-forgewire-wi238-3-0-0-field-evidence.md) | **fixed on upstream `main`** for the conventional `worktrees/` path (`0096d70`, PR #7) — **not present in the packaged `3.0.0` release** (the fix landed two days after the `3.0.0` tag); general worktree-discovery class (any other checkout convention) remains structurally open |
| F-016 | H6,H12 | minor | A work item's human-readable README heading can disagree with its own canonical `work-item.json` `id`/`title` while `repopact validate` stays green; reproduced in an isolated fixture, present in 2.2.0 and current `main`, decision 0014/0028's parity checks do not cover it | [case study](case-studies/2026-08-forgewire-wi230-wi237-enforcement-closure/06-representation-drift.md) | open — no fix implemented or prescribed |
| F-017 | H12 | minor | `repopact doctor`'s `source-of-truth-stale` check resolves a record's relative `source_of_truth:` pointer against the repository root instead of the declaring record's own directory, false-positiving legitimate record-relative pointers (3 of 3 observed on a real adopter tree); inconsistent with `takeover.py`'s own established record-relative treatment of the same field (decision 0016) | [016](captures/016-forgewire-wi238-3-0-0-field-evidence.md), [WI043](../work/completed/043-define-and-enforce-source-of-truth-path-resolution-semantics/README.md) | **fixed** — decision 0034 and WI043 make `doctor` uniformly resolve from the declaring record; historical capture retained |

## F-001 — `repopact spec` is not closed over `init` output

**Hypothesis tested:** H2 (closure — every advertised command works on an
`init`-fresh repo).

**Observed.** Installing the wheel into a clean venv and running the documented
command sequence, `repopact spec --root <fresh>` raised:

```
FileNotFoundError: [Errno 2] No such file or directory: '.../sandbox/SPEC.md'
```

`init_repo.bootstrap` does not write a `SPEC.md`, but `repopact spec` calls
`spec.read_text()` unconditionally. Every other advertised subcommand (`init`,
`validate`, `new`, `dashboard`, `check-frozen`) round-tripped cleanly.

**Why it matters.** The CLI help advertises `spec` as a top-level command, so an
adopter following the surface hits an unhandled traceback on a repository RepoPact
itself just created. The bootstrap output is not closed under the tool's own
command set (¬H2, partial).

**Design question.** `SPEC.md` is *RepoPact's own* specification, derived by
`generate_spec.py` from the schemas and invariants. An adopter repository does not
necessarily need to vendor the RepoPact spec. The defect is therefore one of two
things, to be settled by a decision record:

1. `spec` is a **maintainer** command that should not be advertised in the adopter
   CLI (or should refuse cleanly when no `SPEC.md` is present); or
2. `init` should seed a `SPEC.md` so the command is meaningful for adopters.

**Status:** **fixed.** Work item `007` makes `spec` print one-line guidance and exit
1 when no `SPEC.md` is present; `init` still seeds none. Decision `0006` records the
maintainer-vs-adopter rationale. Regression test
`test_cli_spec_fails_cleanly_without_spec_file`. Re-verified from the rebuilt 1.0.0
wheel in capture [003](captures/003-rebuild-reverify.md).

## F-002 — `check-frozen` is blind to working-tree changes

**Hypothesis tested:** H4 (authority is binding — frozen-surface changes are caught).

**Observed.** With `governance/invariants.json` (a protected path) modified but
**not yet committed**, `repopact check-frozen --base HEAD` reported *"No
frozen-surface changes detected"* and exited 0. After committing the change,
`check-frozen --base HEAD~1` correctly flagged it and required `--ack` (F-005).

**Cause.** `violations()` uses `git diff --name-only base...HEAD` — committed
changes between the merge-base and HEAD only. Working-tree and staged changes are
invisible to it.

**Why it matters.** The gate is sound *as a CI check* (where the branch's changes
are committed), and that is its documented use. But the natural developer reflex —
"let me check before I commit whether I touched the pact" — gets a false all-clear.
An adopter could weaken an invariant locally, see green, and only be caught after
push. The authority is binding at the CI boundary, not at the moment of editing.

**Proposed resolution.** Either (a) also diff the working tree / index (`git diff`
and `git diff --cached`) and union the results, or (b) document explicitly that
`check-frozen` is a committed-diff CI gate and add a `--staged`/`--worktree` mode.
Severity minor because the CI guarantee (the one that blocks merges) holds.

**Status:** **fixed.** Work item `007` unions the committed range (`base...HEAD`)
with uncommitted changes vs `HEAD`, so staged and working-tree edits to protected
paths are now caught locally while CI (clean tree) still sees exactly the branch's
commits. Regression test `test_check_frozen_detects_working_tree_change`.
Re-verified from the rebuilt 1.0.0 wheel in capture
[003](captures/003-rebuild-reverify.md).

## F-003 / F-004 / F-005 / F-006 — adversarial cases the architecture caught

Recorded as **holds** (evidence *for* the design), captured in
[`captures/002`](captures/002-proving-ground-workflow.md):

- **F-003 (H3).** AC-1 set to `satisfied` with `evidence: []` → validator:
  *"criterion AC-1 is satisfied without evidence."* Completion is genuinely gated.
- **F-004 (H5).** Work item moved to `work/completed/` with `status: active` →
  validator: *"status 'active' does not match directory 'completed'."* Status is a
  filesystem fact, not a self-asserted field.
- **F-005 (H4).** A committed edit to `governance/invariants.json` →
  `check-frozen` exit 1 with the protect reason; `--ack` required to reach exit 0.
- **F-006 (H1, H6).** Starting from only the repository, work item 001's intent,
  the pivot-unit decision, and the passing evidence run were all recoverable from
  `work/`, `decisions/`-style narrative, and `evidence/runs/` — no chat history.

## F-007 — brownfield adoption of a real repository holds

**Hypothesis tested:** H7 (brownfield adoptability).

**Observed.** `repopact adopt` run against an export of the real **forgewire**
repository (4569 commits, GTK/HTTP app, no prior RepoPact) created 27 records and
skipped 52 existing files, then the tree **validated as a conformant RepoPact**. It
registered **19 nested `AGENTS.md` contracts**, derived **7 scopes** from CODEOWNERS
teams, and mapped **4 CI workflows** to binding-gate policies plus invariant `INV-2`
and a frozen-surface entry. No existing file was modified; `--dry-run` is read-only.

**Why it matters.** This is the capability the operator flagged as the real
readiness bar: an existing project's ownership, enforcement, and contracts become
first-class RepoPact records without a rewrite. Greenfield proof (the proving ground)
plus brownfield proof (forgewire) together support the 1.0 declaration.

**Status:** **shipped.** `repopact/adopt_repo.py` + `repopact adopt` (work item 008,
decision 0008), 4 regression tests, re-verifiable via capture
[004](captures/004-brownfield-forgewire.md).

> **Validity caveat (recorded 2026-06-15, operator note).** RepoPact was *distilled
> from* forgewire's own practices (tiered `AGENTS.md`, the `_audit` system, todos,
> logs, history, trackers). forgewire therefore validates as **confirmatory**
> evidence — the architecture meeting its progenitor — and demonstrates `adopt` as an
> engineering capability, but it is **not independent** evidence of generality, and
> must not be cited as such. The reintegration of the RepoPact kernel back into
> forgewire is a real deliverable; the *generality* claim still needs a repository
> that did **not** inspire RepoPact. See [`threats-to-validity.md`](threats-to-validity.md).

## F-008 — an existing `.gitignore` can silently swallow RepoPact records

**Hypothesis tested:** H7 (brownfield adoptability), real-repo reintegration.

**Observed.** Adopting the real forgewire repo on a branch, `repopact adopt` wrote
`evidence/runs/<ts>-adopt.json` and validation passed — but the file was matched by
forgewire's pre-existing `.gitignore` rule `runs/` (intended for runtime/ML audit
data). `git check-ignore` confirmed it. The work item references that evidence id, so
the repo **validates on the author's disk but would fail on a fresh clone or in CI**,
where the ignored evidence is absent.

**Why it matters.** This is silent and serious: the most dangerous failure is one that
passes locally and breaks for everyone else. It is also a genuinely *structural* (not
lineage-dependent) collision — the kind only a real brownfield adoption surfaces.

**Resolution.**
- *Immediate (forgewire branch):* a scoped negation (`!evidence/runs/`,
  `!evidence/runs/*.json`) tracks the records while preserving the original `runs/`
  intent. Documented in `REPOPACT-ADOPTION.md`.
- *Upstream (open):* `repopact adopt` should run `git check-ignore` on each record it
  writes and warn (or offer to add the negation) when an adopter's `.gitignore` would
  swallow a governance record. **Done** in work item 010: `adopt` now runs
  `git check-ignore` on every record it writes and prints a warning with suggested
  `.gitignore` negations; regression-tested (`test_adopt_warns_on_gitignored_records`).

## F-010 — adoption left the work ledger hollow

**Hypothesis tested:** H7 (brownfield adoptability), operator-reported.

**Observed.** After adopting forgewire, `work/` held only the `000-adopt` item while the
team's ~75 real plan items stayed in `todos/`. The governed ledger did not reflect the
actual backlog, so it understated the project and split planning across two trees.

**Resolution (fixed).** New command `repopact import-plan` (work item 011, decision
0010) detects plan directories (`todos/`, `tasks/`, … with `completed/`/`deferred/`/
`blocked/` lifecycle folders) and markdown checklist files, and imports them into
`work/` by lifecycle. Completed items become `waived` (no fabricated evidence);
imports are non-destructive (source preserved, origin recorded in a `source` field) and
idempotent. Proven on forgewire (75 items → populated, conformant `work/`); 4 regression
tests.

## Independent-adoption gap (partially closed)

H7's **generality** now has one independent datum: **F-009** — `adopt` brought
pallets/flask (no RepoPact lineage) to a conformant RepoPact. This exercises the
sparse + workflows path. Still open: an *independent* repo that also has CODEOWNERS
and nested contracts, to show those mappings generalize beyond the progenitor
(forgewire). Tracked in [`threats-to-validity.md`](threats-to-validity.md) T1.

## F-014 — downstream adoption exposed a missing authority state

**Hypotheses tested:** H6 (recoverability from repository records) and H7 (brownfield
adoption under real project pressure).

**Observed.** The Moto One Hyper ROM Lab needed to retain candidate work without
authorizing implementation. RepoPact's four-state lifecycle had no truthful encoding:
every available state either granted authority, implied a blocker, or implied prior
acceptance. The downstream need is recorded in decision 0023 and the adopter's public
commit `0adb522`; capture [013](captures/013-proposed-lifecycle-adoption-pressure.md)
preserves the exact chain.

**Resolution (shipped).** Work item 025 added `proposed` to the schema, shared model,
bootstrap, CLI, semantic validator, and conformance corpus. Evidence run
`20260629-proposed-lifecycle-state` proves the implementation gates. Decision 0024 and
tag `v2.1.0` record the release, and the later five-adopter 2.2.0 rollout verifies the
originating vendored consumer and the rest of the public fleet after the change.

**Why it matters.** The standard evolved by adding an honest authority type instead of
tolerating a false assertion. More importantly for the paper's meta-claim, a reader can
recover the motivation, accepted decision, enforcement behavior, release, and downstream
use from linked records without the initiating conversation. This is one positive case,
not proof that the evolution process is universally complete.

## F-015 — worktree-walk validator false positives, and a version-currency lag

**Hypothesis tested:** H7 (brownfield adoptability — a real, longitudinally
governed adopter's actual disk state, not a synthetic fixture, surfaced this).

**Observed.** A longitudinal ForgeWire adoption incident (see the accepted
[case study](case-studies/2026-08-forgewire-wi230-wi237-enforcement-closure/)),
running `repopact validate` under pinned `2.2.0`, reported 171 of 269 total
errors as `AGENTS.md` files "not registered" inside `.claude/worktrees/**` —
local `git worktree` checkouts used for parallel agent sessions. `2.2.0`'s
`repo_model.IGNORED_PARTS` (the raw filesystem contract walk's exclusion
set: `.git, __pycache__, node_modules, .venv, .pytest_cache, build, dist,
fixtures`) had no concept of `git`-tracked status or worktree checkouts, so
every file inside a scratch worktree was scanned as if it were governed
nested content. None of these 171 were a real discrepancy in the adopter's
governed records; each disappeared once the stale worktree checkouts were
removed from local disk, with no change to any governed record.

**Why it matters — and why it is two findings in one, not one.** This is
not solely a validator defect. Upstream `main` had already fixed the
specific, conventionally-named `worktrees/` case (`0096d70`, PR #7, merged
2026-07-28) **three weeks before** the incident window this case study
examines — but the adopter's `requirements-repopact.txt` remained pinned to
`2.2.0`, released before that fix, and no upgrade occurred in the interim.
The incident therefore demonstrates two distinct things at once: (1) a real,
historical validator false-positive defect in `2.2.0`'s filesystem walk, and
(2) an adopter version-currency lag — the fix existed and was not received.
Severity is recorded against the false-positive defect specifically (the
validator producing incorrect output on a real, non-synthetic adopter tree,
obscuring genuine signal at scale), not against the version-currency
mechanism, which is a separate, already-recognized class (see GA-1/GA-8's
"stale adopter pin" concern and `fleet_verify.py`, work item `034`/`037`).

**Do not read this as closed.** The upstream fix adds the literal string
`"worktrees"` to `IGNORED_PARTS` — it remains a convention-name allowlist,
not genuine `git`-tracked-status awareness. A worktree checked out under any
other directory name reproduces the identical false-positive class on both
`2.2.0` and current `main`. `fleet_verify.py` checks an adopter's declared
version *pin* against RepoPact's current release; it does not check whether
a specific historical defect's fix has been received, so it would not by
itself have surfaced this particular lag.

**Second reproduction — release lag, not just adopter lag (WI 238).** A
later, independent ForgeWire migration ([capture 016](captures/016-forgewire-wi238-3-0-0-field-evidence.md))
upgraded the adopter's pin to `repopact==3.0.0` — the current public
release, installed fresh from PyPI, not a stale pin — and re-ran the exact
class of check. A real, throwaway `git worktree` was added under the
`.claude/worktrees/` convention (detached `HEAD`, zero uncommitted files),
`repopact validate` was run, and it reported the identical false-positive
shape: 19 "nested contract is not registered in audits/registry.json"
errors, one per `AGENTS.md` under the worktree, plus a stale-dashboard
error. The worktree was then fully removed and `validate` returned clean.
Source inspection of the installed `3.0.0` package confirmed why: its
`repo_model.IGNORED_PARTS` is `{".git", "__pycache__", "node_modules",
".venv", ".pytest_cache", "build", "dist", "fixtures"}` — no `"worktrees"`
entry. Cross-referencing commit dates against the `v3.0.0` tag
(`f4039a6`/`f1db6b4`, 2026-07-26) shows the worktree fix (`0096d70`,
2026-07-28) landed on `main` **two days after** the `3.0.0` package was
tagged and published — the fix exists upstream but was never packaged into
any released version an adopter could install.

**This distinguishes two version-drift classes that this finding previously
conflated as one "version-currency lag":**

- **Adopter version lag** (the original incident) — a fix is packaged and
  released, but the adopter's own pin has not moved past an older release
  that predates it.
- **Release lag** (WI 238's reproduction) — the adopter is pinned to the
  *current* public release, correctly, but a real fix exists only on
  upstream `main` and has not yet been packaged into any release the
  adopter could have installed. No adopter action — upgrading further —
  would have avoided this; only a new RepoPact release containing the fix
  would.

The one silver lining, confirmed by re-reading `repo_model.py`'s exclusion
check (`any(part in IGNORED_PARTS for part in path.relative_to(root).parts)`,
a per-path-*component* match): had `0096d70` been included in the `3.0.0`
package, it would have correctly excluded ForgeWire's exact
`.claude/worktrees/<name>/` layout — `"worktrees"` is literally one of that
path's components. This is not a case of the fix failing to generalize to
ForgeWire's convention; it is a case of the fix simply not having shipped
yet. That does not change the finding's open status — the fix is still a
directory-name allowlist, not a structural, git-aware solution to a worktree
checked out under an arbitrary name — but it does mean this specific
adopter's specific layout is not, in principle, an unsolved case, only an
unreleased one.

**Status:** open (general class — arbitrary-named worktree discovery has no
structural solution). The conventionally-named `worktrees/` instance is
fixed on upstream `main` but was not present in the `3.0.0` public release
and, as of this finding's last update, has not shipped in any released
package version an adopter can install.

## F-016 — a work-item README can silently disagree with its own manifest

**Hypotheses tested:** H6 (recoverability — a reader relying on a work
item's own self-identifying heading gets a wrong answer) and H12 (drift
visibility — documented state diverging from canonical state, undetected).

**Observed.** `repopact new work-item` writes the same id/title into both
`work-item.json` and the generated `README.md` heading (`# {id} — {title}`)
at creation time, so the two start in sync by construction, not by any
ongoing cross-check. Reproduced in an isolated, throwaway fixture (not the
governed adopter repository itself): after hand-editing only the README
heading to a different number, leaving the manifest's `id` field unchanged,
`repopact validate` reported the tree conformant — unchanged, and passing —
for both the pinned `2.2.0` and current `main`. The only existing
README-content check, `validate_readme_checkbox_parity` (decision 0014),
is narrowly scoped to the `- [ ] **ID** ...` checklist convention's
checked/unchecked state and does not activate at all for a README that
states its criteria as prose (as in the live adopter instance below). A
structurally similar, later, independently-added check (decision 0028) pins
only the *repository-root* `README.md`'s release-version line to `VERSION`
— narrower still, and does not cover work-item READMEs either.

**Why it matters.** The work-item README's identity line (`# {id} —
{title}`) is exactly the kind of fact the derive-over-declare principle
(`formal-model.md` §1/§4; charter principle 8, policy 001) says should be
generated rather than hand-maintained, since it is nothing but the
manifest's own `id`/`title` restated. The dashboard and `SPEC.md` receive
that fixpoint treatment (`I_derive_dash` since 2.2.0); the work-item
README's own identity line does not. This is a narrow instance of a general
representation-coverage gap: RepoPact's narrative-consistency checks so far
grow by one narrowly-scoped fix per discovered instance (decision 0014,
then decision 0028) rather than by a general "any prose restating typed
state must match it" invariant.

**What this is not.** The canonical typed record (`work-item.json`) is
unaffected and remains internally valid throughout — this is a projection/
narrative-consistency defect, not corruption or loss of the canonical
state. Severity is recorded as minor for exactly that reason, matching the
precedent set by F-002 (a real but narrow-impact gap where the core
guarantee — here, `I_ID`'s status-directory agreement and every schema/
referential-integrity check on the canonical record — still holds).

**Status:** open. No fix is implemented or prescribed by this finding; two
directions were identified and left as options for a future work item
(a narrow fixpoint check at `validate`/`dashboard` time analogous to
`I_derive_dash`, or a further decision following the 0014/0028 precedent),
deliberately not chosen here.

## F-017 — `doctor` resolves `source_of_truth` against the repo root, not the declaring record

**Hypothesis tested:** H12 (drift visibility — the mechanism that is
supposed to surface documented state diverging from actual state instead
manufactured a divergence signal where none existed). Discovered on a real,
longitudinally governed adopter tree ([capture 016](captures/016-forgewire-wi238-3-0-0-field-evidence.md)), the same H7 context F-015 shares, though the defect itself is in the
drift-detection mechanism, not adoptability.

**Observed.** During ForgeWire's WI 238 migration to `repopact==3.0.0`,
`repopact doctor --root .` reported 3 `[source-of-truth-stale]` warnings
against records that were not, in fact, stale: three `_audit/` companion
files under `work/active/114-forgewire-fabric/_audit/` each declare
`source_of_truth: ../AGENTS.md`, correctly pointing at
`work/active/114-forgewire-fabric/AGENTS.md`, which exists. `repopact
validate` never flagged these records; only the separate, advisory `doctor`
diagnostic did.

**Root cause.** `doctor.py`'s `_dead_source_of_truth` resolves each token
with `not (root / token).exists()` — against the repository root,
unconditionally, regardless of where the declaring record lives or whether
the token carries a `../` prefix.

**Semantics were verified before recording this, not assumed.** `source_of_truth:`
is free-form frontmatter — it appears in no JSON schema — so the only
existing specification is decision
[0016](../decisions/0016-takeover-repoints-inbound-references.md) ("Takeover
Repoints Inbound References Before Retiring a Plan Directory"), which
explicitly treats `source_of_truth:` frontmatter identically to a Markdown
link target: both are rewritten by `takeover.py`'s
`rewrite_inbound_references`, whose matching pattern
(`(?:\.\./)*(?:{retired}...)/...`) is written specifically to preserve a
leading run of `../` segments — behavior that only makes sense under a
record-relative (relative to the file declaring the token) resolution rule,
the same rule Markdown link targets follow. `takeover.py` already implements
and relies on this rule for exactly the `../`-prefixed shape ForgeWire's
records use. The one existing unit test covering `doctor`'s resolution,
`test_doctor_flags_dead_source_of_truth_pointer`, exercises only a *bare*
token with no `/` or `../` (`AGENTS.md`) from a record one level below root
(`decisions/9999-probe.md`); root-relative resolution happens to succeed
there only because that bare token coincides with the well-known top-level
contract file every bootstrapped repo has at its root
(`init_repo.bootstrap` writes only `AGENTS.md` at the repo root, never a
nested `decisions/AGENTS.md`) — the test does not exercise, and therefore
does not establish authoritative semantics for, a `../`-prefixed token, the
exact shape both ForgeWire's records and `takeover.py`'s own rewrite logic
use.

**Conclusion: this is a definite implementation bug, not a semantic
ambiguity.** The only documented treatment of `source_of_truth:` in the
corpus (decision 0016) is record-relative, and `takeover.py` already
implements that rule elsewhere in the same codebase. `doctor.py`'s
`_dead_source_of_truth` resolves the identical field root-relative, contrary
to the only specification that exists and inconsistent with the codebase's
own other consumer of the same field — an internal inconsistency between two
modules, not a case where ForgeWire's records are wrong.

**Impact, honestly bounded.** `repopact validate` — the actual enforcement
gate — was and remained clean throughout; this defect lives entirely in
`doctor`, an advisory diagnostic. The finding's `Finding(..., False)`
fixability flag confirms `source-of-truth-stale` is not auto-repaired by
`doctor --fix` even today — the docstring already notes "the correct target
needs judgment" — so this defect cannot silently corrupt a record by itself.
The real risk is indirect: an operator or agent trusting a false `doctor`
warning at face value could manually "repair" a genuinely correct
`source_of_truth:` pointer, introducing the very drift the check exists to
prevent. Recorded as **minor**: the architecture's real gate (`validate`)
holds, and the defect requires a human or agent acting on bad advice to
cause any actual harm — consistent with the severity precedent set by F-002
(a diagnostic-correctness gap in a non-blocking check, not a gate failure).

**Historical status at recording:** open. No fix was implemented or prescribed
by this finding at capture time; proposed work item 043 was the candidate
direction deliberately left for a later decision.

**Resolution (WI043, 2026-09-02).** Decision [0034](../decisions/0034-source-of-truth-record-relative-resolution.md)
adopts the record-relative rule and `doctor._dead_source_of_truth` now resolves
`path.parent / token` for every path-like token. Regression tests cover valid
`../` and bare pointers, stale bare pointers, a root-level filename coincidence,
and non-destructive `doctor --fix` behavior. The original field observation and
capture 016 remain unchanged; this note records implementation and disposition.

## Field-study synthesis: enforcement closure

**This section is explicitly not a normal `F-0XX` finding.** It does not
test a single preregistered hypothesis against a single adversarial case;
it synthesizes a pattern observed across a naturalistic, post-hoc field
case (the accepted
[ForgeWire WI230/WI237 case study](case-studies/2026-08-forgewire-wi230-wi237-enforcement-closure/)),
not a designed proving-ground run. Recording it here, clearly labeled, is a
deliberate choice: burying it inside F-015/F-016 would misstate its
evidentiary weight in either direction — folding it into a normal finding
would overstate it as a single adversarial-case result; omitting it from
the register entirely would understate that it is derived from *accepted*,
reviewed evidence, not idle speculation.

**What was observed.** A fully RepoPact-governed adopter (ForgeWire, the
progenitor repository) accumulated a `repopact validate` reported-error
count in the hundreds over an extended period while its own test suite
stayed green, because no checkpoint in its ordinary commit/CI loop ever
invoked the validator — not because the validator, when eventually run,
decided incorrectly. RepoPact's own repository independently exhibits the
same higher-level failure through a different mechanism: `main` has no
branch protection and its governance-validation workflow has been failing
on every push due to an account-level billing lock (work item `032`,
decision `0031`, both still open as of this synthesis).

**What this motivates, and what it does not establish.** This pattern
motivates a new, prospective, falsifiable hypothesis — **H14, enforcement
closure** — preregistered in a dated amendment to
[`protocol.md`](protocol.md) (2026-08-21) and a corresponding preregistered
comparative study, **S7**, in
[`benchmark-protocol.md`](benchmark-protocol.md). **The naturalistic case
does not itself confirm H14.** It is the motivating field observation for a
hypothesis that has not yet been tested under S7's controlled, pre-registered
conditions; treating the naturalistic case as if it already confirmed H14
would be exactly the kind of retroactive-confirmation dishonesty
`threats-to-validity.md` (T1, T5) exists to guard against. See
`formal-model.md`'s new cross-cutting admission-boundary treatment (§7) for
the formal statement, and `protocol.md`/`benchmark-protocol.md` for H14/S7.
