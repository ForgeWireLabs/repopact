# 040 — Refresh Research Metadata Contract Tests

> **Status**: Complete — see evidence
> `20260821-040-refresh-research-metadata-contract-tests`.
> **Owners**: governance-owner (lead).
> **Depends on**: `039`.

## Intent

Narrow regression repair. Commit `ec8edbd` (work item 039) legitimately
advanced `research/metadata.json`'s canonical `claim_freshness.review_by`
(2026-08-09 → 2026-09-18) and `benchmark.study_hypotheses` range (H8–H13 →
H8–H14). Two negative tests in `tests/test_research_metadata.py` hardcode
those prior canonical values as literals, so they began failing — not
because anything is broken, but because the tests' own fixtures went stale
the moment the canonical values they exist to test against changed. This
item updates only those two tests' fixture literals to the new canonical
values, preserving each test's semantic purpose as a genuine negative test.

**In scope**: `tests/test_research_metadata.py` (the two specific tests
named below only) and the evidence-run semantics review requested alongside
this fix. **Out of scope**: any other test, any RepoPact behavior change,
H14/S7/the case study/paper conclusions/the formal model (none found
inconsistent during this pass), and WI039 itself (not reopened, not
rewritten).

## Decisions

None beyond the two literal-value updates. The evidence-run semantics
question (does `result: passed` permit non-gating diagnostic command
failures) was investigated and resolved with no change required — see
Closeout.

## Scope

Files this work changed: `tests/test_research_metadata.py` (2 tests),
`audits/reports/dashboard.md` (regenerated). No file under `research/`,
`repopact/`, `schemas/`, or ForgeWire was touched.

## Closeout

Both acceptance criteria are satisfied by evidence
`20260821-040-refresh-research-metadata-contract-tests`: the full suite is
green (138 tests, 0 failures, 2 pre-existing legitimate skips; 20/20
conformance; validate/dashboard/spec/check-frozen all clean).

**Evidence-run semantics conclusion (AC-3):** direct inspection of
`repopact/schemas/evidence-run.schema.json` and `validate_repo.py`'s
`validate_evidence()` confirms neither the schema nor the validator imposes
any relationship between the top-level `result` field and individual
`commands[].exit_code` values — `commands` is a free-form log, `result` is
an independent summary judgment. A real, already-accepted precedent
(`evidence/runs/20260726-semantic-ledger-freshness-reconciliation.json`,
WI-033's own closeout evidence) already records `result: "passed"` alongside
two diagnostic commands with `exit_code: 1`. **Conclusion: this is an
intentional, already-established pattern, not an inconsistency.** No change
was made to the evidence-run schema, `validate_repo.py`, or WI039's own
evidence run as a result of this review.
