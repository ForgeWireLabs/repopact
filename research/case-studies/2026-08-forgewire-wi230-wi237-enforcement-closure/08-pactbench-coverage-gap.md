# 08 — PactBench / Proving Ground Coverage Gap Analysis (Phase 8)

Inspected: `C:\Projects\repopact-proving-ground\benchmarks\pactbench\tasks\`
(24 task files, `0001`–`0024`, matching the paper's stated count) and
`C:\Projects\repopact-proving-ground\benchmarks\drift\{MUTATION-SET.md,
mutations.json}` (S5/H12's 15-mutation set, M1–M15). Not modified. Each of the
eight incident features named in the prompt is scored below.

## 1. RepoPact installed but never invoked

**Absent.** No PactBench task (0001–0024) or drift mutation (M1–M15) models
"the repository is adopted, valid, and conformant, but the validator is
simply never run as part of ordinary workflow for an extended period." Every
PactBench task is a single-shot, single-session "agent is given a task with
a tempting shortcut" scenario, scored at the end of that one session — none
model a multi-session or long-duration *absence* of invocation. The drift
harness's mutations (M1–M15) each model a single discrete edit, immediately
followed by a detection-latency measurement; none model "zero validate/CI
invocations for N days/commits while unrelated work proceeds," which is the
actual shape of the WI230 incident (34+ days between GA-1's discovery of 39
errors and WI230's own 297-error starting point).

## 2. Declared CI gate that is not actually exercised

**Absent**, with one close-but-distinct cousin. M4 ("Add a CI workflow not
reflected as a policy/invariant... enforcement exists, governance silent") is
the *inverse* shape: a CI workflow exists and *does* run something, but
RepoPact's own records don't know about it. ForgeWire's incident is the
other direction: RepoPact's records exist (declared invariants, CI workflow
files that reference RepoPact by name in `REPOPACT-ADOPTION.md`'s narrative)
but the CI workflow's actual step bodies never call `repopact
validate`/`check-frozen` — governance is declared, the gate is declared, but
nothing in the executed pipeline invokes the checker. No mutation or task
models "a CI workflow claims to enforce X but its steps don't actually call
the enforcer."

## 3. Governance state drifting while ordinary tests remain green

**Absent.** No task or mutation scores test-suite pass/fail status alongside
governance-conformance status as two independent, potentially-diverging
axes. WI230's own closeout line ("11059 passed, 0 failed. RepoPact: 0 WI230
errors (26 at the start of closeout)") makes this divergence explicit and
measurable in the field case; nothing in PactBench or the drift harness
currently produces an analogous joint metric.

## 4. Validator invocation only at late closeout

**Partially covered**, via the general shape of S5's "time/edits-to-
detection" metric — if a scenario were constructed where validate is *never*
run until a deliberate late point, S5's scoring machinery (`latency`,
`silent_staleness`) could in principle measure it. But no *existing* mutation
or task specifically constructs this scenario; all 15 mutations assume
detection is attempted promptly (or is a documented blind spot detected only
by a later `doctor`/`validate`, e.g. M9) rather than deliberately deferred to
a "closeout" event as a matter of workflow design (as WI230's own closeout-
time validation, and this session's own `scripts/ci.py closeout` profile,
both are). The *instrument* exists; the *specific scenario* does not.

## 5. Human-readable projection disagreeing with canonical typed record

**Partially covered.** M13 ("Hand-edit a derived artifact (dashboard)")
covers exactly this for the *dashboard*, and is explicitly caught by
`I_derive_dash` per `02-repopact-2.2-enforcement-model.md`/`03-version-
delta.md`. But no mutation or task covers the narrower, more common instance
this case study found: a **work-item's own README** disagreeing with its
own sibling `work-item.json` on a fact the manifest already contains (id,
title) — as opposed to the dashboard, which is a separate, wholly-generated,
repo-wide artifact. `06-representation-drift.md` shows this is invisible to
`repopact validate` in both 2.2.0 and current dev HEAD. This is the most
concrete, reproducible gap this phase identified: a real, cheaply-added
mutation (M16 candidate: "edit a work-item README's heading id/title without
updating the sibling manifest") is directly implied by existing evidence and
not yet present in `mutations.json`.

## 6. Concurrent agents choosing the same work-item ID

**Absent from the drift harness (M1–M15); nominally in scope for S3 but not
enumerated as a concrete task.** `benchmark-protocol.md` S3's metrics
("Conflicting/clobbering edits; duplicated work; scope-collision rate") are
broad enough to *include* an id collision, but no S3 harness, fixture, or
task file was found under `benchmarks/` implementing this specific scenario
— S3 is listed in the paper (`paper.md` §5.2 table) as "protocol defined;
results pending," with no runnable artifact located in the Proving Ground for
this study, unlike S1 (PactBench, implemented) and S5 (drift, implemented).
`07-concurrency-id-collision.md` shows the underlying mechanism
(`new.py`'s local-scan allocation) is a clean, deterministic, easily-
constructed two-agent scenario: this is a concrete, low-effort candidate for
the first S3 fixture once S3 moves from "protocol defined" to implemented.

## 7. A validation implementation that exists but is ineffective/miscalibrated

**Absent**, and this is the most surprising gap given how much of this
case study's *own* debugging fell into this category. This session directly
encountered: a `git diff --check` step that was technically "there" but
miscalibrated for the repository's actual line-ending convention (flagging
every CRLF line as a whitespace error); a Rust `clippy` configuration that
had apparently never been run with `--all-features` locally, so its
declared lint policy (`workspace.lints.clippy`) was silently unexercised for
an entire dependency-feature surface; and (in RepoPact's own repository, per
`03-version-delta.md`) a CI workflow that dispatches and fails in 2-6 seconds
due to an account-level billing lock — present, invoked even, but never
actually completing a check. None of PactBench's 24 tasks or the drift
harness's 15 mutations scores "the gate exists, is invoked, and produces a
result — but the result is wrong, incomplete, or the gate fails closed for
an unrelated reason (missing toolchain, billing lock, miscalibrated rule)
rather than by correctly detecting the tested violation." This is a distinct
failure mode from both "not invoked" (item 1/2 above) and "invoked, detects
correctly" (what M1-M3, M6, M8, M10-M12, M14-M15 test) — it is "invoked,
fails for a reason unrelated to the thing being tested," and it is currently
unscored.

## 8. Hosted CI unavailable while local work continues

**Absent from PactBench/drift; present as a live, first-party RepoPact
engineering concern (work item 032), not as a benchmark scenario.**
`gap-audit-2026-07.md` GA-3 and work item 032 document RepoPact's own
GitHub Actions being billing-locked for over a month, with local (Windows)
testing continuing regardless — a real, ongoing instance of exactly this
scenario, happening to RepoPact's own repository rather than to a subject
under test. It has not been converted into a PactBench task or drift
mutation, despite being lived, dated, and fully documented in RepoPact's own
`research/` and `work/` trees. This is arguably the single cheapest and most
authentic scenario to add to the benchmark, since RepoPact would not need to
construct a synthetic case — its own history already contains one.

## Summary table

| # | Feature | Coverage |
| --- | --- | --- |
| 1 | RepoPact installed but never invoked | absent |
| 2 | Declared CI gate not actually exercised | absent (M4 is the inverse case) |
| 3 | Governance drift while tests stay green | absent |
| 4 | Validator invoked only at late closeout | partially covered (instrument exists, scenario doesn't) |
| 5 | Human-readable projection vs. typed record | partially covered (M13 covers the dashboard; work-item README case does not exist) |
| 6 | Concurrent agents choose the same work-item id | absent as a concrete task (in scope for S3's stated metrics, no runnable fixture found) |
| 7 | Validation implementation exists but is ineffective/miscalibrated | absent |
| 8 | Hosted CI unavailable while local work continues | absent from the benchmark; present as lived, first-party RepoPact history (WI-032) |

Six of eight incident features that this real, longitudinal field case
actually exhibited have no PactBench task or drift mutation; the other two
are only partially covered by adjacent-but-distinct existing scenarios. This
is not a criticism of PactBench's current 24 tasks, each of which tests a
real and worthwhile single-session shortcut-temptation scenario — it is a
observation that the *longitudinal, multi-session, invocation-availability*
class of failure this case study documents is a materially different shape
from what S1 and S5 currently score, and sits closer to S3's stated scope
(coordination, conflict, collision) without yet having runnable artifacts
there.
