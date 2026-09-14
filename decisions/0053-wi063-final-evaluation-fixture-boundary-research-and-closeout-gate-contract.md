---
id: 0053
title: WI063 final evaluation, fixture-boundary, research, and closeout gate contract
status: accepted
date: 2026-09-14
supersedes: []
---

# 0053: WI063 final evaluation, fixture-boundary, research, and closeout gate contract

## Context

Ten WI063 acceptance criteria remain pending entering this checkpoint:
ROG-019, ROG-028, ROG-032, ROG-033, ROG-034, ROG-035, ROG-036, ROG-037,
ROG-038, ROG-040. Two of them -- ROG-028 (macOS portability evidence)
and ROG-034 (a registered S8 R1 comparative result) -- depend on
external resources this checkpoint may not have: an authorized macOS
execution environment, and an authorized, funded multi-agent benchmark
run. Fabricating either would produce a false closure. This decision
binds how every remaining criterion is resolved, and states plainly
that WI063 stays active rather than close falsely.

## Decision

### 1. Fixture-boundary resolution (ROG-019) without weakening ROG-021

`fixtures`-named directories remain fully content-excluded from the
source projection (Decision 0044, proven by ROG-021) -- that exclusion
is never weakened or removed. Instead, the *existence and
classification* of a fixture boundary becomes a small piece of
deterministic metadata the projection contract itself covers:

- `SourceProjection` gains a sorted, bounded `excluded_boundaries` list
  of `{path, classification}` pairs (classification is currently only
  `test_fixture_boundary`), populated by the same directory walk that
  already computes `IGNORED_PARTS` exclusions -- no second filesystem
  pass.
- The fingerprint (Decision 0044 section 8) is extended to cover each
  boundary's repository-relative path and classification, so creating,
  removing, or renaming a fixture directory changes the fingerprint
  exactly like any other tracked fact. Editing a file *inside* an
  excluded fixture tree changes nothing the projection observes, by
  construction (fixture bytes are never read).
- The graph gains one narrow fact per boundary: a `Directory` node at
  that path carries `node_role = "test_fixture"` (Decision 0049's
  open, bounded-syntax role vocabulary -- no new closed enum, no new
  `GraphNodeKind` variant). The graph may know *that* a fixture
  boundary exists at a path; it never knows what is inside it.

This directly satisfies ROG-019's "test targets/fixtures" clause
without contradicting ROG-021's exclusion guarantee: the graph
distinguishes "a fixture boundary is here" from "here is what the
fixture contains," and never crosses that line.

### 2. ROG-032 benchmark methodology

A dedicated, source-controlled, deterministic benchmark harness (not
ad hoc shell timings as the sole evidence) measures: full build time,
incremental update time by mutation class, peak process memory, durable
graph bytes, node/edge counts, clean-clone verification/load time,
representative query latency, parser coverage, graph/source size ratio,
and branch/merge rebuild cost -- against real RepoPact and against at
least two larger, deterministically generated fixture scales. Generated
fixtures are specified by a seeded generator, not committed as
thousands of manually authored files. Any product limit changed as a
result of this data is justified by the measurement, not intuition;
an unchanged limit is recorded as still appropriate given the data,
not silently left unexamined.

### 3. ROG-033 pre-registration ordering

The S8 R1 graph-enabled condition is defined in a dated amendment to
`research/benchmark-protocol.md`, committed as its own narrow commit,
before any exploratory or comparative R1 result is collected. The
amendment does not rewrite B0, R0, the existing task set, or any
already-registered metric definition. The amendment's commit SHA must
chronologically precede every R1 result artifact; if that ordering is
ever violated, ROG-033 is not satisfied by that run.

### 4. ROG-034 R1 execution gate

S8's B0 and R0 conditions have never been executed (confirmed via
`research/amendments/2026-09-12-governance-continuity.md`: "No S8 run
had been performed when H15/S8 was added"). S8 requires a real
multi-worker/multi-client clean-clone handoff matrix with actual
agent/model execution and a frozen scorer -- not a synthetic or
in-repository substitute. Running it requires resources (funded model/
provider API usage, an authorized benchmark runner) this checkpoint
has no standing operator authorization to spend. Per this decision:

- No paid API credits, no new paid hosted runner, and no new
  third-party account may be acquired to produce an R1 result without
  explicit operator approval obtained outside this session.
- If R1 cannot be run under this constraint, ROG-033 (the committed,
  pre-registered amendment) may still be satisfied on its own; ROG-034
  (the executed, reported comparison) is recorded as pending, blocked
  on authorized benchmark execution -- never as satisfied by a
  synthetic, partial, or fabricated substitute.
- A future checkpoint that does have authorization must still run the
  full registered task set, report every registered correctness and
  cost measure (never cherry-picked), and accept any outcome --
  better, neutral, mixed, worse, or inconclusive -- as valid. Reduced
  search at the cost of degraded correctness is explicitly not a win.

### 5. macOS evidence policy (ROG-028)

No macOS runner exists in this repository's CI (`governance.yml`/
`release.yml` both run `ubuntu-latest` only) and none is available in
this local environment. This checkpoint does not create or run a
hosted macOS job to manufacture a checkbox, and does not incur new
cloud/provider spend without explicit operator approval. Every other
ROG-028 clause (wide/compact usability, no hover/right-click
dependency, Windows/Linux portable graph semantics, mobile boundary
composition) is proven independently; macOS execution is recorded as
the sole named residual gap, and the criterion stays pending until a
real macOS execution environment is authorized and used.

### 6. No-paid-provider / no-unapproved-hosted-run rule

This rule is general, not S8-specific: no checkpoint may spend
operator money (paid model APIs, paid hosted CI/runners, new
third-party accounts) to close an acceptance criterion without the
operator's explicit, contemporaneous approval. Absence of that
approval is a legitimate reason to leave a criterion pending; it is
never a license to fabricate the missing evidence.

### 7. Authority/security closeout tests (ROG-036/037/040)

Closing ROG-036, ROG-037, and ROG-040 requires executable proof, not
architectural narrative alone:

- ROG-036: static/executable evidence that the deterministic ROG core
  (`repopact-graph`, `repopact-repository`, `repopact-core`, the query
  kernel, metadata/semantic adapters, the durable builder/loader) has
  no LLM/embedding/vector-DB/cloud/network dependency, plus a
  networkless-execution proof of the real build/verify/query/
  clean-clone-load lifecycle.
- ROG-037: an authority-boundary matrix classifying every graph
  operation as read-only, derived-artifact-write, or authoritative-
  source-mutation, plus adversarial tests proving a graph fact naming
  itself "owner"/"approved"/"frozen acknowledged"/etc. changes nothing
  about actual owner resolution, lifecycle, criterion state, frozen-
  surface enforcement, evidence truth, or mutation authorization.
- ROG-040: API-design review plus a tampered-shard test (in a
  disposable repository, never authoritative main) proving governance/
  mutation APIs ignore a synthetic authority-like graph fact entirely,
  and explicit tests that canonical owners/AC-state/evidence truth/
  frozen state always trace to governance/work-item/evidence/frozen-
  surface records, never to graph shards.

### 8. When ROG-038 and WI063 may actually close

ROG-038's closeout evidence may be produced only after every other
criterion's disposition is known (satisfied or explicitly, honestly
pending). If ROG-028 and/or ROG-034 remain pending at that point, the
record produced is a **closeout-readiness checkpoint**, explicitly
named as such, not a final closeout record -- and WI063 remains
`active`. WI063 transitions through canonical lifecycle tooling only
when all 40 acceptance criteria are genuinely satisfied; a long
document is not by itself sufficient to mark ROG-038 satisfied.

## Consequences

- ROG-019 gains a real, bounded fixture-boundary fact without ever
  reading fixture bytes, closing the last named gap in Decision 0044's
  metadata coverage.
- ROG-032 produces reproducible, source-controlled benchmark evidence
  instead of one-off developer-machine numbers.
- The S8 R1 pre-registration is committed and auditable even though
  its execution may remain blocked, so a future authorized run has an
  unambiguous, already-frozen contract to execute against.
- This checkpoint may legitimately end with WI063 still active. That
  outcome is treated as correct governance behavior, not a shortfall
  to paper over.

## Alternatives considered

- **Indexing fixture directory contents to fully close ROG-019.**
  Rejected: this directly contradicts ROG-021 and Decision 0044's
  fixture-exclusion guarantee and risks exposing fixture-internal
  content (potentially including deliberately-seeded secrets/violation
  material) as graph facts.
- **Treating S8 B0/R0's absence as irrelevant and running only R1.**
  Rejected: a graph-enabled result with no baseline is not a
  comparison, and Decision 0053 section 4 requires the full registered
  matrix, not a partial substitute presented as if it were complete.
- **Running a smaller, single-model, no-budget-impact "R1-like" probe
  and calling it ROG-034 evidence.** Rejected: this is exactly the
  fabricated/partial substitute this decision forbids; it would let a
  future reader believe H15/S8 had been evaluated when it had not.
- **Marking ROG-028 satisfied on Windows+Linux plus architectural
  reasoning alone.** Rejected: the AC's text names macOS explicitly;
  architectural portability is evidence toward the criterion, not a
  substitute for its execution clause.
