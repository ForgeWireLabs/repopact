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
`generate_spec.py` from the schemas and invariants. An adopter repo does not
obviously need to vendor the RepoPact spec. So the defect is one of two things,
to be settled by a decision record:

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
