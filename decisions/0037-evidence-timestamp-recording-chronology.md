---
id: "0037"
title: Deterministic evidence timestamp recording chronology
status: accepted
date: 2026-09-03
supersedes: []
---

# 0037: Deterministic evidence timestamp recording chronology

## Context

The post-3.0.2 change in `f2c80b7dcdc54ff9f4753bc996ef0b6dfba539bf` rejected
an evidence timestamp that was more than five minutes after the validator's
wall clock. That check was an ungoverned historical change, not a preflight-
authorized RepoPact implementation. It also made an unchanged repository
change validity as time passed: a record could fail at one invocation and pass
five minutes later. The retained ForgeWire chronology evidence that motivated
the change shows the underlying shape: an evidence record originally claimed
`2026-08-26T21:30:00Z` even though commit `f9c894bb` containing it was authored
at `2026-08-26T17:39:16Z`; the value was later corrected to the recovered write
time `2026-08-26T17:26:59.391Z`. The local ForgeWire checkout does not contain a
literal WI260 work-item record, so that retained correction is cited as the
available field artifact rather than presented as a fabricated WI260 file.

## Decision

RepoPact keeps ISO 8601 validation for every evidence-run `timestamp` and adds
a deterministic, history-sensitive chronology check for records that explicitly
set `timestamp_basis` to `git-recording`:

1. Parse the timestamp and normalize an offset-aware value to UTC. A
   timezone-less value is deliberately interpreted as UTC, never as the local
   machine timezone.
2. In a Git checkout, find the first commit that recorded the evidence file
   (`git log --follow --diff-filter=A --format=%ct:%H --reverse -- <path>`).
3. Accept an execution timestamp no later than that commit's authored/recorded
   time plus five minutes. The five-minute bound covers ordinary writer/commit
   clock drift and the short write-to-commit interval exposed by the field
   correction. A timestamp later than that bound is rejected with a diagnostic
   containing only the fixed commit identity and time.
4. If Git metadata is unavailable, the file is uncommitted, or the tree is an
   export, retain structural/schema/ISO validation but skip the history-
   dependent comparison. Such a tree has no recording fact from which to derive
   the ordering, and inventing one would be less deterministic than accepting
   the explicitly disclosed limitation.

`timestamp_basis` is an opt-in compatibility marker. Existing records without
that field include historical backfills whose execution time predates later
import into the repository; they remain structurally validated and are not
rewritten to manufacture chronology. New concrete evidence that claims this
ordering uses `timestamp_basis: "git-recording"`. An unknown basis is invalid.

## Alternatives evaluated

- **Current wall clock (rejected).** It is simple and catches a far-future value,
  but validity depends on invocation time, machine clock, timezone configuration,
  and test scheduling. An unchanged record can heal merely through passage of
  time, and an export cannot reproduce the original result.
- **Git recording time for every historical record (rejected as a migration).**
  It is deterministic and reproduces the incident, but existing backfilled
  evidence was intentionally recorded hours after its stated historical run.
  Applying the rule retroactively would require rewriting completed history,
  contrary to INV-4 and would misclassify known records.
- **Opt-in Git recording time with structural-only fallback (chosen).** It
  applies the narrow generic guarantee to records that claim recording-backed
  chronology, reproduces the WI260-shaped failure permanently, remains
  deterministic on unchanged trees, and keeps exported/uncommitted behavior
  explicit rather than dependent on an unavailable clock or fabricated metadata.

## Consequences and limits

The rule establishes only an upper bound from execution claim to first Git
recording. Git commit dates are author-supplied metadata and are not an
independent trusted clock; a dishonest author can still lie consistently. A
later amendment does not heal a timestamp that is already beyond the first
recording bound, because the first recording commit remains stable. Recovered
or amended evidence should use a new file (or retain a documented correction)
and an explicit basis rather than rewriting completed records silently.

This decision governs WI049's reconciliation of the historical hotfix. It does
not retroactively authorize `f2c80b7`, does not alter WI048 or the `v3.0.2` tag,
and does not implement WI044, WI046, WI047, or any ForgeWire feature.
