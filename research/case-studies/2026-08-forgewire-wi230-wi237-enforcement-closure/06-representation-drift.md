# 06 — The WI237 README/Manifest ID Mismatch (Phase 6)

**Not fixed, per instruction.** `work/completed/237-canonical-local-ci-and-
repopact-enforcement/README.md` still reads "# 236 — Canonical Local CI and
RepoPact Enforcement"; the sibling `work-item.json`'s `id` is `"237"`.
Preserved as case-study evidence.

## Why this happened, mechanically (from `04-forgewire-case-timeline.md` item 16)

The renumbering was done with a batch text-substitution script matching the
literal strings `WI236`, `wi236`, `"id": "236"`, and `work item 236`. The
README's heading text is `# 236 — Canonical Local CI and RepoPact
Enforcement` — a bare number with no `WI` prefix, no `"id":` JSON key
context, and no `work item` phrase preceding it. None of the four patterns
matched. This is a straightforward string-matching miss during a manual
renumbering operation, not a RepoPact defect by itself — but *that
RepoPact's validator did not catch the miss* is the fact worth investigating,
and is what this document reproduces and explains.

## Reproduction (isolated fixture, not ForgeWire)

Built and validated in `C:\Projects\repopact-case-study\_fixture-repro`, a
throwaway `repopact init`-fresh repository, entirely outside the governed
subject repositories:

1. `repopact new work-item "Fixture Item" --status active --root .` — this
   itself writes the same id into both the JSON (`"id": "001"`) and the
   generated README heading (`# 001 — Fixture Item`), confirming the two are
   *sourced from the same variable at creation time* and start in sync by
   construction, not by any cross-check.
2. `repopact validate --root .` → `Repository governance validation passed.`
3. Edited **only** the README heading to `# 999 — Fixture Item`, leaving
   `work-item.json`'s `"id": "001"` untouched — reproducing the exact class
   of drift WI237 exhibits (heading and manifest id disagree).
4. `repopact validate --root .` → `Repository governance validation passed.`
   **Unchanged.** The mismatch is completely invisible to the validator.

## Why, exactly (source-level, from `02-repopact-2.2-enforcement-model.md`)

The only README-content check in `validate_repo.py` (both 2.2.0 and current
dev HEAD) is `validate_readme_checkbox_parity` (decision 0014). Its own
docstring states its scope precisely: it checks the `- [ ] **CRIT-1** ...`
checklist convention's checked/unchecked state against the manifest's
criterion `state` field — nothing else — and is gated entirely on that
checkbox convention being present in the README (`if not boxes: return`).
WI237's README does not use the checkbox convention for its acceptance
criteria (it lists them as prose bullets in the closeout section, not as
`- [ ]` items), so this check does not even activate for it. There is no
separate check, in either 2.2.0 or current dev HEAD, that parses a work-item
README's title/heading and compares it against the sibling manifest's `id` or
`title` fields.

## Answering the prompt's questions directly

**Is README deliberately noncanonical?** Yes, explicitly, by the validator's
own design and its docstring's own words: "The manifest stays the source of
truth; this only stops a README from silently disagreeing with it" — stated
in the context of the one narrow dimension (checkbox state) it actually
checks. The design intent is that `work-item.json` is canonical and the
README is a human-facing narrative that *may* drift on anything the
validator doesn't specifically check.

**Does RepoPact promise consistency here?** No. Nowhere in `paper.md`,
`formal-model.md`, or the SPEC-referencing comments in `validate_repo.py` is
there a claim that README content is validated wholesale against its
manifest. The only consistency claims made are the narrow, named ones:
checkbox-state parity (decision 0014) and, as of current dev HEAD only, the
*root* README's release-version line against `VERSION` (decision 0028 — see
`03-version-delta.md`). Both are additive, specific, and dated — evidence of
an incremental pattern (fix the one thing that broke), not a general
"narrative documents are cross-validated" guarantee.

**Is the README intended as a human projection of the typed ledger?** Partly,
and inconsistently across the codebase. `repopact new` *generates* the
initial README with content sourced from the manifest (title, id), which is
projection-like behavior at creation time. But the README is then a
free-form editable file — `paper.md` §3.5's derive-over-declare principle
("anything computable from source records should be generated, not hand-
maintained") is stated for the *dashboard* and *SPEC.md*, both of which are
fully regenerated, byte-checked artifacts (`π_dashboard`, `π_spec`). The
work-item README is not treated the same way: it is seeded once, then
hand-maintained indefinitely, with no regeneration step and no fixpoint
check analogous to `I_derive_dash`. This is an inconsistency in how strictly
derive-over-declare is applied across artifact types, not a contradiction of
the principle itself — the principle was never claimed to apply to
work-item READMEs specifically.

**Is this consistent with "derive over declare"?** No — this is the crux.
The work-item README's title/heading is exactly the kind of fact that is
"computable from source records" (it is nothing but the manifest's `id` and
`title`, concatenated) and therefore, per the *stated principle*, should be
generated rather than hand-maintained. That it currently is *not* treated
this way (seeded once, then free-editable, with the id portion effectively
duplicated rather than derived) is a direct instance of the exact drift class
derive-over-declare exists to prevent — applied inconsistently, not
violated in principle. The dashboard and SPEC got the fixpoint treatment;
the work-item README's own identity line did not.

**Should the heading be generated rather than manually duplicated?** Given
the principle already stated in the paper, yes — this would be the
consistent extension, not a new idea. Two ways to do it without full
regeneration machinery: (a) derive the heading line specifically (`# {id} —
{title}`) from the manifest at `dashboard`/`validate` time, the same way
`I_derive_dash` derives the dashboard, checked as a narrow fixpoint; or (b)
generalize `validate_readme_checkbox_parity`'s existing narrow-convention
pattern (decision 0014) with a decision 00XX for "README heading parity,"
following the exact precedent decision 0028 already set for the *root*
README's version line. Option (b) is more consistent with how RepoPact has
actually evolved this class of check so far (narrow, convention-gated,
one decision per pattern) — see `03-version-delta.md`.

**Would cross-validating it improve correctness or merely overconstrain
prose?** It would improve correctness for the *one line* that duplicates
already-typed data (the id/title heading) without touching genuinely free
prose (the "Intent," "Decisions," "Scope" sections, which are not derivable
from any record and should stay hand-authored). The distinction the paper
already draws — source records vs. derived artifacts — supports checking
exactly the derivable line and nothing more; a blanket "the whole README must
match" rule would indeed overconstrain prose that has no source-of-truth
counterpart to check against.

**Does this expose a general "representation coverage" limitation?** Yes.
The pattern across `06` and `03` (decision 0014 → decision 0028 → this
finding) shows RepoPact's narrative-consistency coverage growing by
one-narrow-fix-per-discovered-drift-instance rather than by a general
principle applied uniformly to every place a record's identity is restated
in prose. This is not evidence of carelessness — each fix so far has been a
reasonable, scoped response to a real discovered case — but it does mean the
current state is a patchwork of specific checks rather than a general
"any prose that restates typed data must match" invariant. A fourth instance
of the same underlying class (work-item README headings) is exactly what
this incident surfaces, and it is reasonable to predict a fifth elsewhere
(e.g., a decision record's own title vs. its filename slug, not checked here
either — not independently verified in this case study, flagged as a
plausible further instance rather than confirmed).
