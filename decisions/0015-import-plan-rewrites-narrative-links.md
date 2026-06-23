---
id: 0015
title: Import-Plan Rewrites Narrative Links to the Work Ledger
status: accepted
date: 2026-06-23
supersedes: []
---

# 0015: Import-Plan Rewrites Narrative Links to the Work Ledger

## Context

`repopact import-plan` (decision [0010](0010-plan-import-maps-existing-trackers-to-work.md))
lifts each legacy plan item's README narrative into `work/<status>/<id>/README.md`. It
preserved the narrative **verbatim**, including its Markdown links.

Those links break in two ways the importer did not account for:

1. **Relative depth shifts on relocation.** A README that lived at `todos/<item>/` and
   linked to `../../docs/foo.md` now lives at `work/<status>/<id>/` — one directory
   deeper — so every relative link resolves to the wrong place.
2. **Cross-item links still point at the legacy tree.** A narrative linking to a sibling
   item as `../completed/104-execution-graph/` keeps pointing into `todos/`. Decision
   [0013](0013-takeover-delete-is-documented-and-git-guarded.md) then deletes that tree
   once everything is migrated — guarding only *recoverability of the deleted dir*, not
   *the references pointing into it*. The result: `work/` ships links that dangle the
   moment takeover runs, and so does every doc, test, and comment elsewhere in the repo
   that referenced the old paths.

This was observed in the wild: an `import-plan` + `takeover --delete` on a large repo
left dangling `todos/` references across the work ledger and the surrounding codebase,
none of which RepoPact validation flagged (it checks structural invariants, not arbitrary
link targets).

## Decision

`import-plan` now **rewrites each imported narrative's links** before writing the README,
using the full plan→work map built up front in a first allocation pass:

- **Relative links are rebased** from the source README's directory to the item's new
  `work/<status>/<id>/` home, so depth-sensitive links keep resolving.
- **Links that resolve into another migrated plan item are remapped** to that item's
  work location. Because only the README narrative is carried into `work/` (per
  [0013](0013-takeover-delete-is-documented-and-git-guarded.md), finer-grained detail
  files live on in git history), a link into another item **collapses to that item's
  `README.md`** — the unit RepoPact preserves.
- **URLs, in-page anchors, absolute paths, and mail links are left untouched**; `#anchor`
  fragments are preserved across a rewrite.

The guarantee: a freshly imported `work/` ledger contains no link that points back at the
legacy plan tree, so a later `takeover --delete` cannot strand a reference inside `work/`.

## Scope and limits

This fixes references **inside the imported narratives** (`work/`). It does **not** rewrite
references to the legacy tree that live *elsewhere* in the host repository (docs, tests,
code comments) — those are outside import-plan's authorship. Closing that gap on the
`takeover` side (scan/rewrite/report inbound references before delete) is a logical next
step but a separate decision; this one keeps RepoPact's own output self-consistent.

## Alternatives rejected

- **Copy every detail file into `work/` so links resolve as-is.** Rejected — contradicts
  [0013](0013-takeover-delete-is-documented-and-git-guarded.md): the README is the unit of
  the ledger; detail files belong in git history, not duplicated into `work/`.
- **Leave narratives verbatim and document the breakage.** Rejected — shipping known-dangling
  links in generated output is exactly the drift RepoPact exists to prevent.

## Consequences

- Imported `work/` READMEs are self-consistent and survive `takeover --delete`.
- Detail-file granularity inside cross-item links is intentionally lost (collapsed to the
  target README); the narrative summarizes what the detail file held, and git history holds
  the original.
