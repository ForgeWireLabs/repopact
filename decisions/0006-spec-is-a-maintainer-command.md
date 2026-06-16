---
id: 0006
title: spec Is a Maintainer Command, Not an Adopter Command
status: accepted
date: 2026-06-15
supersedes: []
---

# 0006: `spec` Is a Maintainer Command, Not an Adopter Command

## Context

The proving-ground evaluation (finding **F-002**, research record) installed the
RepoPact wheel into a clean environment and ran every advertised CLI subcommand
against an `init`-fresh repository. `repopact spec` crashed with a
`FileNotFoundError`: `init` seeds no `SPEC.md`, but the command called
`SPEC.md.read_text()` unconditionally. The tool's output was not closed under the
tool's own command surface.

The deeper question is whether an adopter repository should have a `SPEC.md` at
all. `SPEC.md` is *RepoPact's own* specification, derived by `generate_spec.py` from
the schemas and invariants. An adopter governs their project with RepoPact; they do
not republish the RepoPact standard.

## Decision

`spec` is a **maintainer** command: it regenerates the derived blocks of an
*existing* `SPEC.md` for repositories that publish a RepoPact specification (i.e.
RepoPact itself, or an alternative implementation). When no `SPEC.md` is present it
exits non-zero (1) with a one-line explanation, rather than raising. `init` does
**not** seed a `SPEC.md`.

## Alternatives considered

- **Seed a `SPEC.md` in `init`.** Rejected: it would copy RepoPact's spec into every
  adopter repo, implying they must maintain it, and would drift from the source.
- **Remove `spec` from the adopter CLI entirely.** Rejected: the same console entry
  point serves maintainers; a clean no-op-with-message is less surprising than a
  missing subcommand for those who do publish a spec.

## Consequences

- Running `repopact spec` on an adopter repo prints guidance and exits 1; no traceback.
- Covered by a regression test (`test_cli_spec_fails_cleanly_without_spec_file`).
- Documented as adopter-vs-maintainer surface in the CLI help and guides over time.
