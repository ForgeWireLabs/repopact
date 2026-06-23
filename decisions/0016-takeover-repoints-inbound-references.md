---
id: 0016
title: Takeover Repoints Inbound References Before Retiring a Plan Directory
status: accepted
date: 2026-06-23
supersedes: []
---

# 0016: Takeover Repoints Inbound References Before Retiring a Plan Directory

## Context

`repopact takeover` (decisions [0012](0012-tracking-import-and-takeover.md),
[0013](0013-takeover-delete-is-documented-and-git-guarded.md)) retires a fully-migrated
legacy plan directory by archiving or deleting it. Its safety gates covered the *deleted
directory* — git-recoverability, no registered audit scope pointing in, an ADR recording
recovery — but said nothing about the **references pointing at it from the rest of the
repo**: docs links, frontmatter `source_of_truth:`, code `Path()` calls, comments.

Decision 0015 fixed the import side so `work/` ships no dangling links. The retirement
side was still blind: deleting `todos/` left every inbound reference across docs, tests,
and code dangling, and RepoPact validation did not flag it (it checks structural
invariants, not arbitrary link targets). Observed in the wild on forgewire — dozens of
broken references and a silently-broken test suite after a single `takeover --delete`.

## Decision

Before retiring any directory, `takeover` **repoints inbound references** across the repo,
using each work item's `source` provenance (basename-indexed, so a reference that omits the
legacy lifecycle segment — `todos/107-x` vs `todos/completed/107-x` — still resolves):

1. **Auto-rewrite the unambiguous navigational forms.** Markdown link targets `](...)` and
   `source_of_truth:` frontmatter values are repointed to `work/<status>/<id>/`. Because
   `todos/` and `work/` are repo-root siblings, a leading `../` run is preserved verbatim —
   only the `<dir>/<item>` head is swapped. Detail files never migrated into `work/`
   collapse to the item's `README.md` (the unit RepoPact preserves, per
   [0013](0013-takeover-delete-is-documented-and-git-guarded.md)).
2. **Report everything else.** Code `Path()` calls, prose mentions, and cross-repo
   `../other/<dir>` links are listed (`file:line: token`) for the operator to fix by hand.
   Editing code or historical prose automatically is unsafe — a path string in a test may
   be asserting on retired content, and a sentence describing what *was* deleted should not
   be silently rewritten.

Only references to directories **actually being retired this run** are touched; a plan dir
kept because it still has un-migrated items is left alone. Runs under `--dry-run`. The
existing post-retirement re-validation still applies.

## Scope and limits

This auto-rewrites the safe navigational forms only. It deliberately does **not** rewrite
code or prose — it surfaces them. An operator who wants those changed makes them with the
report in hand. This is the retirement-side complement to
[0015](0015-import-plan-rewrites-narrative-links.md)'s import-side rewrite; together they
close the inbound-reference drift class on both ends.

## Alternatives rejected

- **Block retirement until references are zero.** Rejected — many references are in prose or
  historical audit records that legitimately name the old path; a hard block would make
  takeover unusable on any real repo.
- **Auto-rewrite code paths too.** Rejected — a `Path("todos/...")` in a test may assert on
  content that is *meant* to be gone; rewriting it would paper over a real breakage. Report,
  don't guess.
- **Leave it to validation.** Rejected — RepoPact validation checks structural invariants,
  not arbitrary link/path targets, and adding a whole-repo link checker is a far larger,
  noisier change than fixing references at the one moment they break.

## Consequences

- `takeover` no longer silently strands references; the safe ones are repointed and the rest
  are reported at the moment of retirement, when the operator has the most context.
- Auto-rewriting is conservative by design: navigational links and frontmatter only. Code
  and prose remain the operator's call, with an exact `file:line` worklist.
