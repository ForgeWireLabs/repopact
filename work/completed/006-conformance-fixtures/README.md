# 006 — Conformance fixtures

> **Status**: ✅ **CLOSED 2026-06-15.** Evidence `20260615-conformance-fixtures`. Closes issue #4: a machine-checkable corpus that makes the SPEC's conformance claim provable rather than asserted.
> **Owners**: Tooling (lead), Governance.
> **Depends on**: 004 ✅ (SPEC).
> **Frozen surface**: none touched (SPEC.md prose only).

## Intent

SPEC §9 claimed `validate_repo.py` is the reference implementation, but nothing
let a second implementation (or a reviewer) check that claim. This work item adds
a conformance corpus: a valid baseline and one invalid variant per major rule.

## What shipped

- `tests/fixtures/valid/` — a minimal, self-contained valid repository.
- `tests/fixtures/invalid/<rule>/` — five overlays (completed-with-pending,
  satisfied-without-evidence, unknown-dependency, unregistered-contract,
  dependency-cycle), each just the violating file(s) plus a `meta.json` with the
  expected message.
- `tests/test_conformance.py` — accepts the valid fixture; rejects each invalid
  overlay with its declared message.
- `tests/fixtures/README.md` and `SPEC.md` §9 reference the corpus.

## Decisions in flight

- **No vendored schemas in fixtures.** The test injects the canonical `schemas/`
  before validating, so a fixture can never pass against a stale schema copy — the
  main rot risk of a committed corpus (policy 001). One source of truth preserved.
- **Overlays, not full copies.** Invalid cases carry only the differing file(s),
  keeping the corpus small and the violation self-evident.
- **Validator ignores `fixtures/`.** `repo_model.iter_contracts` skips ignored
  subtrees (relative to root) so the main repo self-validates while each fixture is
  still validated as its own root.

## Verification

Main repo validates (fixtures ignored); tests 26 → 28 including the data-driven
conformance suite. Evidence `20260615-conformance-fixtures`.

## Closeout

All acceptance criteria satisfied. Issue #4 closed.
