# 004 — Specification, documentation, and first public milestone

> **Status**: ✅ **CLOSED 2026-06-15.** Evidence `20260615-spec-docs-release`. Turns RepoPact from "an idea you can agree with" into "a standard you can use and verify": a normative SPEC, a Diataxis documentation set, OSS hygiene, and an honest `v0.1.0-alpha` release. The repository is now both product and evidence.
> **Owners**: Governance (lead), Tooling, Docs.
> **Depends on**: Work item 003 ✅ (adoption surface and primitive hardening).
> **Frozen surface**: touches `schemas/**`-adjacent docs, `.github/**` (templates + workflow), and `SPEC.md` (governance) — operator-directed establishing change (INV-6).
> **Charter binding**: serves *"systems before sessions"* (a recoverable, adoptable standard) and *"completion requires proof"* (the alpha label — confidence is not evidence).

---

## Why this work item exists

RepoPact had the intellectual foundation but no usable surface for outsiders: no
single normative reference, no docs, no community scaffolding, and an overconfident
`1.0.0` version it had not earned. This item closes that gap and ships the first
public milestone, incorporating the "turn RepoPact into evidence" suggestion.

### Core principle for this item

```
Visibility comes from use, not agreement. Ship something people can run,
verify, and contribute to — and label its maturity honestly.
```

---

## What shipped

### Reference
- `SPEC.md` — normative conformance contract (record types, semantic rules,
  lifecycle, invariant/escalation and frozen-surface semantics, conformance).
- `scripts/generate_spec.py` — generates the SPEC's record-type catalog and
  invariant list from `schemas/*.json` + `governance/invariants.json`; CI checks
  freshness (decision `0004`, policy `001`).

### Documentation (Diataxis, `docs/` scope)
- Tutorial `docs/adopt-repopact.md`; explanation `docs/concepts.md`; how-to guides
  `define-your-governance`, `author-records`, `ci-integration`, `extend-the-validator`.
- `docs/AGENTS.md` registered in `audits/registry.json`; `docs` scope + `docs-owner`
  role added to `owners.json`. Governed lightly — no per-page audit inventories.

### Community and hygiene
- `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`.
- `.github/ISSUE_TEMPLATE/` (bug, task, Discussions link) and a PR template.
- `ROADMAP.md` (curated Now/Next/Later), `examples/README.md` (repo-as-example +
  `init` generator), and a runnable demo (`scripts/demo.sh`/`.ps1`, `docs/demo.md`).
- GitHub **Discussions** enabled; five tightly-scoped **good first issues** (#2–#6).

### Release & honest versioning
- `VERSION` lowered `1.0.0` → `0.1.0`; first release tagged `v0.1.0-alpha`
  (decision `0005`). A days-old, externally-unproven standard does not claim 1.0.

## Codex suggestion mapping

The "turn RepoPact into evidence" checklist mapped to existing + new work: `init`,
one invariant, one frozen surface, one work item, one evidence record, and one
reconciliation finding already existed (000–003); this item added the SPEC, docs,
hygiene, roadmap, demo, Discussions, good-first-issues, and the alpha release. The
animated GIF is scripted and ready to record (issue #3) — the recording itself is a
manual pass outside this environment.

## Closeout

All six acceptance criteria satisfied with evidence `20260615-spec-docs-release`.
Validator green; tests 23 → 24; SPEC and dashboard regenerate cleanly.
