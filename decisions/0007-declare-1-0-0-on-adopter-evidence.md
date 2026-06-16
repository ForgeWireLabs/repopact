---
id: 0007
title: Declare 1.0.0 on the Strength of Adopter Evidence
status: accepted
date: 2026-06-15
supersedes: ["0005"]
---

# 0007: Declare 1.0.0 on the Strength of Adopter Evidence

## Context

Decision [`0005`](0005-alpha-versioning-and-release.md) set the version to `0.1.0`
and released `v0.1.0-alpha`, explicitly because RepoPact had not been validated by
any external adopter: *"confidence is not evidence."* The roadmap made 1.0
conditional on adopters producing evidence that the record formats and rules hold.

That condition has now been met. The proving-ground evaluation (`research/`) built a
real, independent CLI project (`repopact-proving-ground`), adopted RepoPact into it
**from the published wheel** (not the source checkout), and drove every primitive,
including adversarial cases. Of six pre-registered hypotheses, five held outright:
adoptability (H1), evidence-gated completion (H3), binding authority at the CI
boundary (H4), filesystem state integrity (H5), and recoverability from the tree
alone (H6). The two defects found — `spec` crashing on a fresh repo (F-001) and
`check-frozen` being blind to working-tree changes (F-002) — are fixed under work
item `007` with regression tests.

Greenfield proof alone was judged insufficient for "ready." Work item `008` added
brownfield adoption (`repopact adopt`) and proved **H7** on a real, mature,
RepoPact-naive repository (forgewire, 4569 commits): its CODEOWNERS, four CI
workflows, and 19 nested `AGENTS.md` contracts were mapped into RepoPact records
non-destructively, and the result validated. All seven hypotheses now hold.

## Decision

- Set `VERSION` to **`1.0.0`**.
- Release **`v1.0.0`** (GitHub release + PyPI), superseding the alpha stance of `0005`.
- The 1.0 line is the set of record formats and rules exercised and held in the
  proving ground; changes that break them require a major-version bump.

## Alternatives considered

- **Stay at `0.x` pending multiple independent adopters.** Rejected: the bar set in
  `0005`/roadmap was *adopter evidence*, which now exists and is recorded; deferring
  further would move the goalposts. Future external adopters strengthen, not gate, 1.0.
- **Cut `1.0.0-rc` first.** Reasonable, but the defects are fixed and re-verified
  end-to-end from the rebuilt wheel; an rc adds latency without new information.

## Consequences

- `VERSION` becomes `1.0.0`; `SPEC.md` drops the alpha caveat and the dashboard
  surfaces 1.0.
- The proving-ground repository and `research/` are the durable evidence behind the
  claim and ship alongside the release.
- Breaking changes to the record formats now carry SemVer-major weight.
