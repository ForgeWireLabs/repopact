# 030 — Complete conformance rule coverage

> **Status**: Active
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

- [ ] **AC-1** — fail when any machine-enforceable rule has no conformance fixture.
- [ ] **AC-2** — add the currently missing rule cases.
- [ ] **AC-3** — isolate fixture signals and report unexpected violations.
- [ ] **AC-4** — align conformance documentation and pass every repository gate.

## Closeout

Each acceptance criterion is satisfied by linked evidence. When all are satisfied,
move this directory to `work/completed/` and regenerate the dashboard.
