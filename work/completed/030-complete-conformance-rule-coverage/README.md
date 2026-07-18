# 030 — Complete conformance rule coverage

> **Status**: Complete — 2026-07-18
> **Owners**: tooling-owner (lead); governance-owner affected.
> **Depends on**: `019`, `024`, `025`, `026`.

## Intent

Turn conformance completeness into a checked property. The published suite currently has
eight cases and does not exercise several machine-enforced rules, including the central
provenance contribution. A manifest that merely lists existing fixtures cannot detect an
omitted rule.

## Decisions

The normative rule inventory must be machine-readable and coverage must be bidirectional:
every enforceable rule has a fixture, and every fixture maps to a real rule. Fixtures must
isolate their primary signal so canonical-dashboard enforcement cannot confound results.

## Scope

- Conformance rule inventory and coverage validation.
- Missing acceptance/rejection fixtures.
- Runner diagnostics, documentation, and regression tests.

## Acceptance criteria

- [x] **AC-1** — fail when any machine-enforceable rule has no conformance fixture.
- [x] **AC-2** — add the currently missing rule cases.
- [x] **AC-3** — isolate fixture signals and report unexpected violations.
- [x] **AC-4** — align conformance documentation and pass every repository gate.

## Closeout

Evidence `20260718-complete-conformance-rule-coverage` links every criterion to
the normative rule inventory, bidirectional coverage gate, expanded 17-case
corpus, exact reference-diagnostic isolation, aligned conformance documentation,
and passing repository gates. The item is complete and lives under
`work/completed/`.
