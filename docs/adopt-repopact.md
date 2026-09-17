# Adopt RepoPact

*Diataxis mode: tutorial (learning-oriented). Follow it top to bottom once.*

By the end you will have a RepoPact-governed repository with one work item, one concrete verification evidence record, and a passing validator.

This tutorial assumes the stable RepoPact package is installed from PyPI. You do **not** need to clone the RepoPact source repository into the project you are governing.

## 1. Install RepoPact

```powershell
python -m venv .venv
.venv\Scripts\python -m pip install repopact
.venv\Scripts\repopact --help
```

On macOS or Linux, use the corresponding `.venv/bin/` executables.

## 2. Bootstrap a governed repository

```powershell
repopact init --target ./my-project
cd ./my-project
repopact validate
```

You now have a valid governed repository with a root contract, governance records, lifecycle directories, packaged-schema references, and a minimal local-first verification contract.

RepoPact tooling remains installed in the Python environment; the governed repository does not vendor a second copy of the implementation.

## 3. Make the governance yours

Open `governance/invariants.json` and inspect the seeded invariant. Add or revise an invariant that matters for your project: a guarantee you do not want silently weakened.

Inspect `governance/frozen-surface.json` and identify a path or symbol that should require explicit operator review before it changes.

Then run:

```powershell
repopact validate
```

The goal is not to add ceremony for its own sake. The governance records should describe boundaries that actually matter to the repository.

## 4. Capture work before implementation

```powershell
repopact new work-item "Wire up the health check"
```

RepoPact creates an active work-item directory under `work/active/` and stamps the required preflight marker showing that the work was recorded before implementation began.

Open its `work-item.json` and replace the starter acceptance criterion with one that can actually be checked. Use the sibling `README.md` for intent, context, tradeoffs, and implementation narrative.

Run `repopact validate` again before doing the implementation.

## 5. Do the work and run repository-defined verification

Make the implementation change, then run the repository's quick verification profile:

```powershell
repopact verify quick
```

A passing verification run is evidence about the checks that actually executed on this host. It does not automatically complete the work item or grant authority.

When you are ready to preserve a real verification run as RepoPact evidence, run the profile again with the work-item id:

```powershell
repopact verify quick --evidence-work-item NNN
```

Replace `NNN` with the work-item id RepoPact created.

RepoPact writes an immutable concrete evidence record under `evidence/runs/` and refreshes derived state as part of the write. If evidence recording cannot complete safely, it fails closed rather than leaving a partial record.

## 6. Link evidence to the acceptance criterion

Open the generated evidence record and copy its evidence id into the acceptance criterion's `evidence` list. Set the criterion state to `satisfied` only when that evidence actually supports the criterion.

If the verification profile does not prove the criterion you wrote, record or create the evidence that does. RepoPact does not treat a generic passing test run as proof of an unrelated claim.

Then run:

```powershell
repopact validate
```

## 7. Complete the work

When every acceptance criterion is satisfied with appropriate linked evidence:

1. move the work-item directory from `work/active/` to `work/completed/`;
2. set its machine-readable `status` to `completed`;
3. regenerate the dashboard;
4. validate the repository.

```powershell
repopact dashboard
repopact validate
```

The validator now confirms that the represented work is complete with evidence rather than merely because a worker said "done."

## What you just exercised

You used the core RepoPact loop:

```text
intent -> scoped authority -> work item -> implementation -> evidence -> audit/history
```

You also crossed several boundaries that RepoPact keeps deliberately separate:

- the work item granted represented implementation authority;
- the verification run produced evidence;
- the evidence did not complete the work item by itself;
- the lifecycle transition happened only after the acceptance criterion was evidence-closed.

## Next

- [Documentation map](README.md) — choose the path for using, understanding, integrating, or developing RepoPact.
- [Concepts](concepts.md) — understand authority, provenance, derived state, and enforcement classes.
- [Define your governance](guides/define-your-governance.md) — shape roles, invariants, and frozen surfaces for a real team.
- [Local-first verification and release](guides/local-ci-cd.md) — verification profiles, evidence recording, release operations, and hosted adapters.
- [`SPEC.md`](../SPEC.md) — normative reference.
