# 064 — Research Metadata Consistency Repair after H15/S8 Governance-Continuity Publication

> **Status**: ✅ Completed
> **Owners**: governance (lead).
> **Depends on**: none.

## Intent

The September 12, 2026 governance-continuity/H15/S8 publication pass rewrote
large parts of `research/*.md` (preregistering H15 and S8, adding threats T11
and T12, and restyling the lifecycle-set notation) without updating
`research/metadata.json`, the canonical cross-document consistency contract.
Canonical `repopact validate` now correctly detects the resulting drift as 6
diagnostics. This work item repairs *representation consistency only*: it
makes the metadata contract and the research prose agree again. It does not
add a hypothesis, does not move H15/S8's 2026-09-12 preregistration date,
does not run S8, does not reinterpret any result, and does not implement or
depend on WI063.

**In scope:** `research/metadata.json`'s study-hypothesis map, benchmark
range markers, lifecycle-notation pattern, and threat-identifier list;
matching canonical-range sentences in `research/benchmark-protocol.md` and
`research/paper-outline.md`; T11/T12 sections in
`research/threats-to-validity.md`; the semantic-claim-freshness review;
`tests/test_research_metadata.py` and the Rust research validator's tests.

**Out of scope:** WI063 (Repository Orientation Graph) in any form; any new
research hypothesis or benchmark result; rewriting the historical
H8–H14/S1–S7 narrative; WI046.

## Decisions

- Accept `in` as an additional canonical lifecycle-notation spelling
  alongside `∈` in the metadata regex, rather than reverting the paper's
  prose back to the mathematical symbol, since the September 12 rewrite is
  an intentional style choice, not a data error.
- Preserve the historical H8–H14 sentence in `benchmark-protocol.md`
  verbatim; add a new, separate present-tense H8–H15 canonical summary
  sentence rather than editing the historical one.

## Scope

- `research/metadata.json`
- `research/benchmark-protocol.md`
- `research/paper-outline.md`
- `research/threats-to-validity.md`
- `tests/test_research_metadata.py`
- `rust/crates/repopact-validation/src/research.rs` (tests only, if fixture literals require it)
- `work/active/064-.../` (this item)
- `evidence/runs/` (WI064 closeout evidence)

## Closeout

Each acceptance criterion (AC-1 through AC-10) is satisfied by linked
evidence in `evidence/runs/20260913-064-research-metadata-consistency-repair.json`.
`repopact validate --root .` reports zero `research.*` diagnostics because
the corpus is genuinely consistent, not because a check was weakened; the
Rust engine and the explicit Python legacy comparator agree. The freshness
policy's review window had not expired, so `verified_on`/`review_by` were
left unchanged rather than mechanically bumped.
