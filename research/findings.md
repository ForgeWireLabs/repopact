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

**Status:** **shipped.** `scripts/adopt_repo.py` + `repopact adopt` (work item 008,
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
