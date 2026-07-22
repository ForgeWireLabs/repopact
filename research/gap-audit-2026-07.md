# Pre-publication gap audit — 2026-07-15

A hard look at everything backing the paper — adopter repositories, conformance
suite, tests, benchmarks, research record, and release/CI substrate — with the
paper deadline set for **end of August 2026**. Each finding states what was
verified, why it matters, and the concrete move. Severity reflects impact on the
paper's credibility, not effort to fix.

**Method.** Every adopter repo was validated against the current (2.1.0)
validator; the conformance manifest was diffed against the SPEC §4 rule set and
the formal model's claims; the proving-ground harness and drift selftests were
executed; the research record was cross-checked against the paper's citations;
work-ledger and CI state were inspected directly. Numbers below are from those
runs, not from memory.

## Current disposition — 2026-07-22

This table preserves the original audit findings below and records later closure
without rewriting what was observed on 2026-07-15.

| Gap | Disposition | Durable owner or evidence |
| --- | --- | --- |
| GA-1 adopter validation | **closed** | five-adopter 2.2.0 rollout, capture 015 / WI 028 |
| GA-2 missing proposed-state trace | **closed** | F-014, capture 013 / WI 031 |
| GA-3 CI and cross-platform enforcement | **open, operator-gated** | WI 032; direct PyPI publication is not CI restoration |
| GA-4 conformance coverage | **closed** | 15 rules / 17 cases, WI 030 |
| GA-5 statistical plan | **open** | WI 022 AC-4 |
| GA-6 RealRunner and comparative results | **open, operator-gated** | WI 022 AC-3/AC-5 |
| GA-7 publication logistics | **open** | WI 021 and publication work remain active |
| GA-8 stale research facts | **closed** | canonical metadata + regression checks, WI 031 |
| GA-9 ledger/audit reconciliation | **open** | WI 033 |
| GA-10 independent reproduction | **open** | WI 034 |

---

## GA-1 — Two flagship adopters fail current validation (critical)

Validated with `python scripts/validate_repo.py --root <repo>` on 2026-07-15:

| Adopter | Result | Error classes |
| --- | --- | --- |
| repopact-proving-ground | **pass** | — |
| ForgeLink | **pass** | — |
| moto-one-hyper | **pass** | — |
| forgewire | **39 errors** | 16 × unknown `affected_scope`, preflight-marker drift, others |
| skillforge-academy | **42 errors** | all preflight-marker drift (2.0 mandatory-preflight epoch) |

The paper cites forgewire (F-007) and SkillForge (F-012, F-013) as adoption
evidence. A reader who clones them and runs current `repopact validate` finds the
paper's own evidence base non-conformant. This is the F-011 longitudinal-drift
class recurring in the wild — which is an opportunity: run `doctor` upgrades on
both and capture the episode as fresh T7/ratchet evidence (currently `[conj]`,
partial). If `doctor` cannot repair the unknown-scope class, that is a new honest
finding about `doctor` coverage. Either outcome strengthens the paper.

**Move.** `doctor --fix` both repos from the 2.1.0 package; capture; new findings
register entries; re-validate.

## GA-2 — The proposed-state episode has no finding or capture (critical)

Paper §6.4 narrates the `proposed` lifecycle episode (decision `0023`, work item
`025`, downstream adopter `moto-one-hyper-forgewire-rom-lab`) as a result — but
[`findings.md`](findings.md) stops at F-013 and [`captures/`](captures/) stops at
012. A paper whose thesis is "every claim traces to a typed record" cites an
episode with no register entry.

**Move.** Add **F-014** to the register and **capture 013** documenting the
adopter pressure, the gap analysis, and the resolution chain (decision 0023 →
WI 025 → schema/CLI/conformance → release 2.1.0, decision 0024).

**2026-07-22 update.** Closed by F-014 and capture 013. The trace now includes the
originating Moto adopter commit, decision 0023, WI 025 implementation evidence,
schema/CLI/semantic/conformance behavior, decision 0024 and release tag 2.1.0, and the
later public fleet rollout evidence.

## GA-3 — CI is dead, and CI is the thesis (critical, operator-gated)

GitHub Actions has been **billing-locked since June**; `governance.yml` fails in
~5 s on every push and `release.yml` cannot publish. The paper's enforcement
story is "invalid states are rejected at the commit/CI checkpoint" and Appendix B
tags T5 as `[ci]` — but the flagship repo's checkpoint has not executed for a
month, and a reviewer who opens the Actions tab sees a wall of red. This also
means all recent testing is Windows-local only, so the cross-platform claim is
currently unexercised (the 2.0.2 seed-lookup bug was itself Windows-specific —
platform coverage is not hypothetical).

**Move (operator).** Clear the billing lock, or move the gates to a free CI
alternative before submission. Re-enables trusted publishing as a side effect.

**2026-07-18 update.** The billing lock remains. RepoPact 2.2.0 therefore uses the
documented direct-PyPI fallback, with exact artifact hashes, public-index verification,
and a clean-environment install captured as release evidence. This restores package
publication but does **not** close GA-3: cross-platform CI and the repository checkpoint
remain unavailable. Separately, 2.2.0 moves dashboard projection equality into the
one-tree validator, so that particular fixpoint no longer depends exclusively on CI.

## GA-4 — Conformance suite: 8 cases; the 2.0 headline feature has zero (high)

Current cases: valid-minimal, valid-proposed-work, active-depends-on-proposed,
completed-with-pending, satisfied-without-evidence, unknown-dependency,
unregistered-contract, dependency-cycle.

**Not covered by any fixture:**

- **Provenance (`I_prov`) — nothing.** No rejection for completed-but-provisional,
  none for concrete work resting on inferred evidence, no valid
  inferred/provisional acceptance case. The central 2.0 contribution is
  conformance-untested.
- **Status–directory mismatch (`I_ID`)** — F-004 is a flagship "holds" in the
  paper with no conformance case.
- Version format (`I_ver`), schema-invalid record (`I_struct`), orphan work dirs
  (`I_orphan`), disjoint active scopes (`I_conc`).

[`formal-model.md`](formal-model.md) T1 claims "one invalid overlay per §4 rule"
— currently false.

**Move.** Grow the suite to ~14–15 cases (one rejection per machine-checkable
rule, plus provenance accept/reject pairs); until then, weaken the T1 wording.

## GA-5 — No statistical plan in the benchmark protocol (high)

[`benchmark-protocol.md`](benchmark-protocol.md) pre-registers hypotheses,
metrics, and falsification criteria, but contains **no** sample sizes,
repetition counts, seed policy, temperature policy, effect-size or
confidence-interval plan, or multiple-comparison handling across six studies.
Agents are stochastic; one run per task per arm is an anecdote, and
pre-registration without an analysis plan erodes the pre-registration claim.

**Move.** Dated amendment before real runs: runs-per-task-per-arm (e.g. 3–5
seeds), rubric version pinning, analysis plan (proportions with Wilson CIs,
paired per-task comparison), and a stopping rule.

## GA-6 — All comparative results remain operator-gated; RealRunner untested (high, schedule risk)

S1 (24 PactBench tasks) and S5 (drift harness) selftests pass and are runnable
today — blocked only on **API keys for ≥2 model families**. Working back from
end-of-August: keys by ~end of July → S1+S6 early August → S5 mid-August →
freeze. If keys slip, every value claim ships "forthcoming."

**RealRunner has never executed a real agent.** A 3-task, one-model smoke run
now would de-risk the "subprocess interface for real agent runs" claim before
the results section is bet on it.

S2/S3/S4 have no harness. Realistic triage: implement **S4** (token economy —
no SWE-bench dependency; Figure 5 is the headline figure) as the second study;
explicitly defer S2/S3 in the paper.

## GA-7 — Publication logistics with lead times (high, operator-gated)

- **No renderable figures exist.** [`figures.md`](figures.md) is ASCII mockups;
  arXiv needs SVG/TikZ for Figures 1–3 and a matplotlib pipeline for 4–6.
- **No markdown→LaTeX/PDF pipeline; no bibliography.** Appendix E says of itself
  "not citation-complete." BibTeX collection is slow — start now.
- **arXiv cs.SE endorsement** for a first-time submitter can take weeks; the
  account + endorsement request should happen in July, not August. Same for the
  2–3-week HN karma warm-up (work item `021`).

## GA-8 — Stale and inconsistent documents (medium)

- [`threats-to-validity.md`](threats-to-validity.md) has a **duplicate T7**
  (two "Token-measurement fairness" sections) and is **missing T9 (provenance
  misuse) and T10 (standard–implementation coupling)**, which exist in the paper.
- [`figures.md`](figures.md) predates 2.0: Figure 2 shows the four-state
  lifecycle (no `proposed`); Figure 3's caption calls provenance "the principled
  *future* escape" (it shipped in 2.0); Figure 6 says "21 pre-registered tasks"
  (there are 24).
- [`benchmark-protocol.md`](benchmark-protocol.md) intro says the studies
  "operationalize hypotheses H8–H10" but defines six studies through H13.
- **The proving ground pins a git commit, not PyPI**
  (`repopact @ git+…@d1d5f81`) while the paper says it "consumes RepoPact from
  PyPI." Re-pin to `repopact==2.1.0`; re-run selftests and validation.

**2026-07-18 update.** The Proving Ground now pins formal PyPI release
`repopact==2.2.0`; governance, unit, PactBench, and drift selftests pass. The rollout
also verified ForgeLink, SkillForge, ForgeWire, and Moto One Hyper on 2.2.0 at their
public default-branch heads. This closes the package-pin bullet, not the other stale-doc
items in GA-8.

**2026-07-22 update.** Closed. `research/metadata.json` now owns the lifecycle states,
24-task PactBench count, S1–S6/H8–H13 mapping, T1–T10 identifiers, and proposed-state
trace. The repository gate cross-checks the paper, figures, protocols, threat register,
and benchmark ledger; regressions deliberately reintroduce every stale-fact class.

## GA-9 — Ledger hygiene in our own repos (medium)

- Work items `020`/`021`/`022` have acceptance criteria with **empty
  descriptions** in `work-item.json`; the meaning lives only in READMEs — a mild
  H6 (recover-from-records) violation in our own ledger.
- The proving ground's work ledger holds one completed item while an entire
  benchmarks tree was built there. Either backfill work items or explicitly
  record that PG work is authorized from the repopact ledger — a cross-repo
  authority gap at L5 the paper could name rather than hide.
- Work item `021` AC-3 (PyPI launch release) is `pending` although 2.1.0
  published 2026-07-15; decide whether 2.1.0 is the launch release and, if so,
  satisfy AC-3 with an evidence run.

## GA-10 — Independence of the evidence base (medium, standing)

All conformant adopters are ForgeWireLabs repositories. Flask (F-009) is the
only non-org datum and exists only as a local capture. Threats T1/T2 remain the
paper's soft underbelly. One genuinely third-party adopter before August would
be worth more than any documentation polish; publishing the flask adoption as a
reproducible script is the cheap version.

---

## Recommended sequencing

| # | Action | Owner | Target |
| --- | --- | --- | --- |
| 1 | `doctor` upgrades on forgewire + skillforge, captured (GA-1) | agent | mid-July |
| 2 | ~~F-014 + capture 013 for the proposed episode (GA-2)~~ **closed 2026-07-22** | agent | mid-July |
| 3 | ~~Conformance suite to full rule coverage incl. provenance (GA-4)~~ **closed by WI 030** | agent | mid-July |
| 4 | Statistical amendment to benchmark protocol (GA-5) | agent | mid-July |
| 5 | ~~Stale-docs batch: threats, figures, protocol intro, PG pin (GA-8)~~ **closed 2026-07-22** | agent | mid-July |
| 6 | RealRunner smoke run, 3 tasks × 1 model (GA-6) | agent + keys | late July |
| 7 | Clear GitHub billing lock (GA-3) | **operator** | ASAP |
| 8 | Provision model API keys, ≥2 families (GA-6) | **operator** | end July |
| 9 | arXiv account + cs.SE endorsement request (GA-7) | **operator** | July |
| 10 | S1 + S6 real runs (GA-6) | agent | early Aug |
| 11 | S5 comparative runs; S4 harness if time (GA-6) | agent | mid-Aug |
| 12 | Figures rendered; LaTeX + BibTeX; paper freeze (GA-7) | agent + operator | mid–late Aug |

This document is a point-in-time audit; re-run the verification commands before
trusting any row after further changes land.
