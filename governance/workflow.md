# Operating Workflow

## 1. Capture intent

Create `work/active/NNN-short-name/` containing `README.md` and `work-item.json`.
State the outcome, boundaries, dependencies, risks, and acceptance criteria.

## 2. Resolve authority

Read applicable `AGENTS.md` files. Record `owner_scope` and any affected scopes.
One scope leads; additional scopes are reviewers or separately sequenced changes.

## 3. Lock decisions

Material decisions belong in the work-item README with rationale and rejected
alternatives. Do this before implementation when reversal would be expensive.

## 4. Execute

Keep changes inside declared scope. Update the work item as facts change. A plan
may evolve, but the record must show the evolution rather than overwrite it.

## 5. Produce evidence

Create an evidence run manifest from `schemas/evidence-run.schema.json`. Evidence
records commands, results, environment, artifacts, and the acceptance criteria it
supports. Failed runs are retained when they explain a decision or blocker.

## 6. Reconcile

Update affected audit inventory rows and findings. An audit answers whether the
declared system and implemented system agree; it is not a second backlog.

## 7. Transition state

- `active`: being designed or executed.
- `blocked`: cannot progress until a named condition changes.
- `deferred`: intentionally postponed with rationale and revisit trigger.
- `completed`: acceptance criteria satisfied with evidence.

Move the entire directory and update `status`. Never copy only the final summary.

