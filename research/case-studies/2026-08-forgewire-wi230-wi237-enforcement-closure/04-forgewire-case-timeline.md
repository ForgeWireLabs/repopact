# 04 — ForgeWire Case Timeline (Phase 4)

Each entry is marked **[HARD]** (directly quoted from a committed record) or
**[INFER]** (a reasonable inference not directly evidenced in the sources
inspected for this case study). No entry states a fact this case study could
not locate a source for.

## 1. Original governance lineage (pre-RepoPact)

**[HARD]** ForgeWire ran a homegrown governance system before RepoPact
existed: tiered `AGENTS.md` contracts (19 nested files at adoption time), an
`_audit/` inventory/alignment system, `todos/` planning trees, decisions,
logs, and history, per `REPOPACT-ADOPTION.md`'s own "What maps to what"
table and `findings.md` F-007 ("forgewire; 19 contracts, 7 CODEOWNERS scopes,
4 CI gates").

## 2. Extraction and generalization into RepoPact

**[HARD]** `paper.md` §1 and `threats-to-validity.md` T1 state RepoPact "was
distilled from real practices in the author's agentic development workflow"
— i.e., from ForgeWire's own practices, named explicitly.
`REPOPACT-ADOPTION.md`'s opening line: "RepoPact was itself distilled from
how ForgeWire is managed... so this is the kernel re-integrated into its
source repository in a structured, machine-checkable form, not a foreign
convention bolted on."

## 3. RepoPact re-adoption into ForgeWire

**[HARD]** `findings.md` F-007: `repopact adopt` run against a 4569-commit
export of ForgeWire created 27 records, skipped 52 existing files, and the
tree validated conformant — registering 19 nested `AGENTS.md` contracts, 7
CODEOWNERS-derived scopes, 4 CI workflows mapped to binding-gate policies.
Recorded with an explicit validity caveat in the same finding: "confirmatory
only... not independent evidence of generality." F-008 (same adoption event)
found ForgeWire's pre-existing `.gitignore` `runs/` rule silently swallowing
`evidence/runs/*.json`; fixed with a scoped negation, documented in
`REPOPACT-ADOPTION.md`'s "One required fix."

## 4. RepoPact 2.2.0 installed/wired

**[HARD]** `requirements-repopact.txt` pins `repopact==2.2.0`. Per this
session's own investigation (recorded in the live conversation, not
re-derived here): none of ForgeWire's four pre-WI236 GitHub Actions workflows
(`python-ci.yml`, `windows-ci.yml`, `forgewire-core-ci.yml`,
`audit-validation.yml`) called `repopact validate` or `repopact check-frozen`
at any point before the WI236/237 intervention, despite
`REPOPACT-ADOPTION.md`'s own "Next steps" section naming "Wire `repopact
validate` and `check-frozen` into CI" as an open item since adoption.

## 5. Why normal agent workflows could proceed without invoking it

**[INFER]** No pre-commit hook, CI step, or documented agent-facing rule
required `repopact validate` to be run before a commit or PR in ForgeWire
prior to WI236. `.pre-commit-config.yaml` (pre-WI236) had exactly two local
hooks (`core-import-boundaries`, `gtk-async-guard`), neither RepoPact-related.
Combined with hard evidence item 4, this is a direct, low-inference
conclusion rather than a speculative one.

## 6. First documented drift episode: GA-1, 2026-07-15

**[HARD]** RepoPact's own pre-publication gap audit
(`research/gap-audit-2026-07.md`, GA-1), run from RepoPact's side (not
ForgeWire's CI), validated ForgeWire against the then-current (2.1.0)
validator on 2026-07-15 and found **39 errors** ("16 × unknown
`affected_scope`, preflight-marker drift, others"), explicitly naming this
"the F-011 longitudinal-drift class recurring in the wild." This predates
WI230 by 34 days.

**[INFER]** Whether `doctor --fix` (the move GA-1 itself recommended: "`doctor
--fix` both repos from the 2.1.0 package; capture; new findings register
entries; re-validate") was actually run against ForgeWire between
2026-07-15 and WI230's start is **not established** by any source this case
study located. GA-1's 2026-07-26 update discusses a *fleet* rollout to 2.2.0
and version-pin currency, not a specific ForgeWire error-count re-check
between those dates. This is a genuine gap in the evidence trail, flagged
rather than filled.

## 7. WI230 work period (2026-08-18 onward)

**[HARD]** WI230 ("FCB Canonical Messaging / AgentBus Retirement")'s own
evidence chain spans 2026-08-18 through 2026-08-20 (`evidence/runs/20260818-
230-m0-agentbus-inventory.json` through `20260820-230-closeout.json`).
Substantial architecture work proceeded: AgentBus retirement across five
milestones (M0–M5), a leaf-boundary reduction (15→14 substrate-to-application
imports, per `governance/invariants.json` INV-3's escalation clause), and
`scripts/wi230_closeout_scan.py`'s 11 durable invariant checks, all reported
clean at closeout.

## 8. Green tests vs. governance drift, concurrently

**[HARD]** WI230's own closeout line: "**11059 passed, 0 failed.** RepoPact:
**0 WI230 errors** (26 at the start of closeout)." — the full test suite was
green (11,059 passing tests) at a point where RepoPact's validator, if run,
reported 26 errors attributable to WI230's own record and (per item 9 below)
297 repository-wide. Test greenness and governance conformance were
independent axes during this period; nothing in the test suite's pass/fail
status reflected the RepoPact drift.

## 9. Discovery of the repository-wide 297 → 269 state

**[HARD]** `evidence/runs/20260820-230-closeout.json`, field
`repopact_repository_errors_before: 297`, measured at commit `b75af681`
"before any closeout change," and its accompanying note: "269 after WI230
moved to work/completed/; 279 while it was still active. Both are below the
297 baseline because closeout also repaired pre-existing free-text evidence
entries on FMR-001..FMR-009." The same evidence run's summary field states
the residual 269 were "predominantly unknown `affected_scope` 'security-team'
and missing preflight markers across `work/proposed/213, 214, 217-223`" —
i.e., WI230's own closeout *partially* reconciled repo-wide drift as a
side effect of repairing its own evidence, without that being WI230's
stated purpose, and left the residual 269 for a later, dedicated effort.

## 10. WI236/237 intervention (2026-08-20)

**[HARD]** The user's directive opening the WI236 session named this residual
269-error state as the starting condition, cross-confirmed independently by
this session's own first `repopact validate --root .` run at the start of the
WI236/237 work, which reported exactly 269 errors before any change — a
second, independent confirmation of the figure recorded in item 9.

## 11. 269 → 0 reconciliation

**[HARD]** Documented in this session's own commits
(`e02aec1e`, `0d3d7608`) and evidence
(`evidence/runs/20260820-236-repopact-zero.json`, later renamed to
`20260820-237-repopact-zero.json`). 171 of the 269 (about 64%) were the
`.claude/worktrees` false-positive class (see `01-paper-claims.md` C6); the
remaining ~98 were genuine record-level drift (unregistered work
directories, missing preflight markers, invalid owner-scope references,
prose-instead-of-evidence-run citations, one provenance-state error, one
stale-dashboard error) reconciled with real, re-verified repairs (see the
commit message of `e02aec1e` for the itemization) — never a fabricated
baseline exemption.

## 12. Declared-but-ineffective gates discovered by actually exercising them

**[HARD]** Building `scripts/ci.py`'s `full` profile and actually running it
(not merely writing it) surfaced, in order: 7 forbidden `core/`→`modules.*`
imports unenforced since 2026-05-05 despite a declared guard
(`scripts/check_core_import_boundaries.py`) wired into CI since March; an
orphaned test (`tests/shell/test_cluster_cli.py`) for code WI230 already
retired from `main`; widespread `cargo fmt`/`clippy --all-features` drift
across both Rust workspaces (`forgewire-runtime`, `forgewire-fabric`) that
had apparently never been exercised with the full feature matrix locally,
including a Windows-only native-toolchain gap (`rdkafka-sys`/`cmake`); a
miscalibrated `git diff --check` misreading this CRLF-heavy repository's line
endings as whitespace errors; and a bug in the new canonical runner itself
(Windows `npm`/`npx` shim resolution). Each was a real, previously-silent
gap, independently discovered by *running* a declared or newly-built gate for
the first time in this session, not by reading documentation.

## 13. Canonical local CI implementation

**[HARD]** `scripts/ci.py fast|full|closeout`, `scripts/ci.ps1`,
`scripts/ci.sh` — one implementation, invoked identically by developers,
agents, and (after commit `5f05b3e2`) by `python-ci.yml`. Decision 0009
("Repository-local CI is canonical") committed at
`decisions/0009-repository-local-ci-is-canonical.md`.

## 14. Concurrent WI236 collision and renumbering

**[HARD]** `origin/main` gained commits `cb91fea6`/`fa78d397` ("Propose WI236
causal state runtime substrate" / "Add RepoPact ledger for WI236") — an
unrelated work item, independently created under the same id — while this
session's WI236 work was in progress locally. Discovered only at `git push`
time (`git status -sb` reporting "ahead 4, behind 2"), not during
development. Resolved by renumbering this session's work item to 237 across
every cross-reference (commit `658333d9`), merging (`b3668a32`), and
regenerating the dashboard (`4e9edc08`). See `07-concurrency-id-collision.md`
for why RepoPact's own tooling could not have prevented this pre-merge.

## 15. Final exact-HEAD closeout

**[HARD]** `scripts/ci.py closeout --work-item 237` passed completely
(every fast/full step, dashboard regeneration, and a clean-tree check) only
*after* the collision was resolved and pushed — the "clean tree" gate
correctly failed on every attempt while local commits were ahead of
`origin/main`, and passed once `origin/main` matched `HEAD` exactly.
Evidence: `evidence/runs/20260821-032210-237-ci-closeout.json`, committed at
`229156e1`.

## 16. Surviving README 236 / manifest 237 mismatch

**[HARD]** `work/completed/237-canonical-local-ci-and-repopact-enforcement/
README.md`'s heading still reads "# 236 — Canonical Local CI and RepoPact
Enforcement" while the sibling `work-item.json`'s `id` field reads `"237"`.
This mismatch was **not** caught by the batch text-substitution script used
to renumber every other WI236→WI237 reference in the same session (the
script matched `WI236`, `wi236`, `"id": "236"`, and `work item 236`; the bare
heading pattern `# 236 —` matched none of those patterns). It has not been
fixed, per this phase's explicit preservation instruction, and `repopact
validate --root .` passes cleanly with it present — confirmed both in
ForgeWire directly (this session, prior to the case-study directive) and
reproduced in an isolated throwaway fixture for this case study (see
`06-representation-drift.md`).
