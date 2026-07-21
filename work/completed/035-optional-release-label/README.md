# 035 — Optional RELEASE_LABEL pre-release identity

> **Status**: Complete — 2026-07-20
> **Owners**: tooling-owner (lead); governance-owner affected.
> **Depends on**: `019`, `030`.

## Intent

Adopters shipping a pre-release build (e.g. SkillForge's `1.4.1-beta.1`) need one
shared pre-release string across their product surfaces without loosening
`VERSION`. `VERSION` must stay a clean, totally-ordered `MAJOR.MINOR.PATCH` triple
because adopter equality and vendored-overlay targeting compare against it exactly;
a pre-release suffix in `VERSION` would silently break those checks.

## Decisions

Add an optional root `RELEASE_LABEL` record (decision `0026`) rather than relaxing
`VERSION`. It is a full SemVer pre-release whose `MAJOR.MINOR.PATCH` core is pinned
to `VERSION`, so it adds a maturity label without letting the release line diverge.
Absent means unconstrained — the rule is purely additive and no existing repository
changes validity.

## Scope

- `validate_release_label` in the reference validator, wired into `validate()`.
- SPEC repository layout, §4.7, and §8 prose.
- Conformance rule `SPEC-4-release-label` with an accept and a core-mismatch reject
  fixture.
- No `VERSION`, `suite_version`, or adopter-fleet change; that belongs to the next
  release decision (cf. `0023` add → `0024` release).

## Acceptance criteria

- [x] **AC-1** — validator accepts a VERSION-pinned label, rejects a divergent or malformed one, and ignores absence.
- [x] **AC-2** — SPEC and the conformance suite inventory the rule with accept and reject fixtures under bidirectional coverage.
- [x] **AC-3** — the change is additive and every repository gate passes.

## Closeout

Evidence `20260720-optional-release-label` links every criterion to the validator
rule, the SPEC additions, the expanded conformance corpus, and the passing
governance, conformance, and unit gates. The item is complete and lives under
`work/completed/`.
