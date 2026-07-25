---
id: 0028
title: Pin the README release line to VERSION
status: accepted
date: 2026-07-25
supersedes: []
---

# 0028: Pin the README release line to VERSION

## Context

RepoPact's thesis is that load-bearing state must not drift silently. The
validator already enforces that for the two derived artifacts — the dashboard
(decision `0026`) and the derived blocks of `SPEC.md` — both regenerated from
source records and diffed in CI.

The README was not covered, and it drifted. At 2026-07-25 it advertised
"current release **2.2.0**" and linked the 2.2.0 changelog while `VERSION` read
`2.3.0` and v2.3.0 was already published to PyPI. `ROADMAP.md` had drifted the
same way, describing the `proposed` lifecycle state as unreleased three releases
after it shipped.

This is worse than an ordinary stale-docs bug. The README is the first thing an
evaluator reads, the claim it got wrong was a *release* claim, and the project
sells drift detection. A tool whose own front door misreports its version has an
evidence problem, not a typo.

The obvious repair — edit the two files — fixes today's instance and leaves the
mechanism that allowed it untouched. It would drift again on the next release,
because nothing but maintainer memory connects `VERSION` to the prose.

## Decision

Make the README release line a checked record.

Where `README.md` uses the convention `current release **X.Y.Z**`, the validator
requires that `X.Y.Z` equal `VERSION`. Where that line carries a changelog link
into `decisions/`, the target must exist and its front-matter title must name the
same version.

The rule is **gated on the convention being present**, following the precedent
set for README checkbox parity (decision `0014`). A repository that does not
advertise a release line in its README is unaffected, so the rule costs adopters
nothing while pinning the surface that actually drifted here. A link that points
somewhere other than `decisions/` (an adopter's `CHANGELOG.md`, say) is checked
for existence only — RepoPact does not impose its changelog convention on
adopters.

## Alternatives considered

- **Generate the README from source records.** The strongest form of the
  guarantee, and how the dashboard works. Rejected: a README is persuasive prose
  whose structure is not derivable from records, and templating it would degrade
  the document that does the most work for the project. Pinning the one factual
  claim gets most of the benefit at none of that cost.
- **Just fix the two files.** Rejected as the whole point of the finding: it
  repairs the instance and preserves the failure mode.
- **Warn instead of fail.** Rejected. RepoPact has no warning tier in the
  validator, and a soft signal on a release claim is one that gets ignored.
- **Extend the rule to every version string in every document.** Rejected as
  over-reach: prose legitimately discusses historical releases ("shipped in
  2.1.0"), and a blanket rule would force awkward rewrites of accurate history.
  The release *line* is the claim about the present, so it is the one pinned.

## Consequences

- Releasing now means updating `VERSION` and the README release line together,
  or validation fails. That coupling is the point.
- The failure arrives at `repopact validate` time — locally, and in CI once
  remote enforcement is restored (work item `032`) — rather than from a reader
  noticing.
- `ROADMAP.md` is *not* covered. Its drift is editorial (forward-looking intent
  that ages), not a checkable claim pinned to a source record, so it is repaired
  by hand here and left to review. Recording that boundary matters: the rule
  covers the claim that has a single source of truth, and nothing more.
- Adopters who do not use the convention see no change in behaviour.
