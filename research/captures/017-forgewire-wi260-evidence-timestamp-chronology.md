# Capture 017 — ForgeWire evidence timestamp chronology (WI049)

Captured 2026-09-03 from the locally available ForgeWire checkout at
`C:\Projects\forgewire`. The checkout does not contain a literal `WI260`
work-item directory or reachable commit, so this capture does not invent one.
It records the retained ForgeWire evidence artifact that exposes the same field
failure shape cited by the RepoPact hotfix and independently reproduces that
shape in the WI049 evidence run.

## Retained field artifact

`evidence/runs/20260826-252-bdr004-endpoint-tariff-predicate-semantics-correction.json`
was originally committed with `timestamp: 2026-08-26T21:30:00+00:00`. Git commit
`f9c894bb54e99356e5ae866a4ab1aa6c4c459270` already contained that file and was
authored at `2026-08-26T17:39:16Z` (`2026-08-26T12:39:16-05:00`). The claimed
evidence time therefore postdated the commit that recorded it by approximately
3 hours 51 minutes. The file's own `chronology_correction` explains that the
value was invented/approximated, and records the recovered write timestamp
`2026-08-26T17:26:59.391Z`, which precedes the commit and matches the observed
write → validation → commit sequence.

This is a generic RepoPact concern: an evidence-run JSON record has the same
`id`, `timestamp`, `commands`, `artifacts`, and `environment` shape regardless
of whether its work item belongs to ForgeWire, RepoPact, or another adopter.
The defect is in the relationship between a claimed execution time and the
repository event that records the claim, not in ForgeWire's pricing domain.

## Independent reproduction

WI049 created a copied RepoPact fixture, initialized Git with a fixed commit
time, and then changed one evidence record to a timestamp six minutes after
that commit. Repeated `validate()` calls returned byte-identical diagnostics;
the record remained rejected without waiting for wall-clock time to pass. A
second fixture used an exported/no-`.git` tree and deliberately far-future
timestamp; it retained schema/ISO validation and skipped the unavailable
history comparison as specified by decision 0037.

## Disposition

The original current-wall-clock check from `f2c80b7` is not retained as the
generic contract. Decision 0037 adopts an explicit
`timestamp_basis: "git-recording"` opt-in: normalize aware offsets to UTC,
interpret naive values as UTC, and reject only when the claimed execution time
exceeds the first recording commit by more than five minutes. Legacy records
without that opt-in remain structurally validated so completed backfill history
is not rewritten. This capture is research evidence for WI049, not a
retroactive preflight record for `f2c80b7` or a claim that WI260 itself was
present locally.
