# Capture 008 — plan-import sources: section roadmaps + GitHub issues

Raw transcript. 2026-06-16. Extends `import-plan` (capture 007) to the remaining
common trackers. Tests: 41/41.

## Section-based roadmap (no checkboxes)

A synthetic `ROADMAP.md` with `## Now / ## Later / ## Done` and plain bullets imported
by section lifecycle (regression test `test_import_plan_section_roadmap_without_checkboxes`):

```
## Now    -> work/active/*-ship-the-api
## Later  -> work/deferred/*-mobile-app
## Done   -> work/completed/*-initial-release
```

## GitHub issues (opt-in) + roadmap, one pass on repopact

```
$ python scripts/plan_import.py --target . --issues --dry-run
Would import 16 plan item(s) into work/; skipped 0.
  + work/completed/200-...        # from ROADMAP.md "Now — shipped"/"Earlier — shipped"
  + work/active/205-...           # ROADMAP "Next"
  + work/deferred/208-...         # ROADMAP "Later"
  + work/active/211-document-path-handling-in-check-frozen-surface-across-oses   # GitHub issue
  + work/active/212-provide-a-reusable-github-action-for-the-governance-gates    # GitHub issue
  + work/completed/213-add-conformance-fixtures-valid-invalid-example-repos       # closed issue -> completed
```

Both new sources fire in a single pass: open issues → active, closed → completed
(waived). `--issues` is opt-in and a safe no-op when `gh` or a GitHub remote is absent.
(Read-only dry-run; repopact's own `work/` was not modified.)

## Sources now covered by `import-plan`

1. Plan directories (`todos/`, `tasks/`, … with lifecycle subfolders) — capture 007.
2. Checklist files (`- [ ]` / `- [x]`).
3. Section roadmaps/backlogs (`##` Now/Next/Later/Done/Blocked → bullets).
4. GitHub issues (opt-in).

Still open: nested sub-tasks (intentionally not flattened), and non-GitHub trackers
(Jira/Linear) — future adapters.
