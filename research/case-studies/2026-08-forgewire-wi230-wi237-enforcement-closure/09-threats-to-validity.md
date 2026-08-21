# 09 — Threats to Validity for This Case Study (Phase 9)

This case study is itself subject to threats, some inherited directly from
RepoPact's own `threats-to-validity.md` (T1–T10, already read in full for
Phase 2) and some specific to how this incident was observed and analyzed.

## Inherited from RepoPact's own T1 (reflexivity/progenitor adoption)

ForgeWire is RepoPact's progenitor: RepoPact "was distilled from real
practices in the author's agentic development workflow" — ForgeWire's own
tiered `AGENTS.md` contracts, `_audit` system, and planning trees, per
`paper.md` §1 and `findings.md` F-007's explicit validity caveat. This case
study is therefore **not an independent field trial**. It is an *endogenous
longitudinal* observation: the same project, over time, both gave rise to the
governance model and is now the subject whose drift under that model is
being studied. Every finding in this case study inherits T1's caveat: it
demonstrates something about how *this* project's *this* history unfolded
under RepoPact, not something proven to generalize to an unrelated adopter.
The parallel finding in RepoPact's own repository (WI-032/decision 0031,
`03-version-delta.md`) partially mitigates this specific worry for the
*enforcement-closure* finding only: that finding now has two independent
loci (ForgeWire and RepoPact's own repo), which is stronger than one, but
both are still ForgeWireLabs-controlled repositories (RepoPact's own T1/GA-10
concern — "all conformant adopters are ForgeWireLabs repositories" — applies
here without modification).

## Operator and agent familiarity

The operator (Jeremy Shows) and the agents that did this session's work have
varying but generally *high* familiarity with RepoPact's concepts — this is
the project whose practices were distilled into RepoPact in the first place.
This is a threat in the conventional sense (an unfamiliar adopter might
behave differently — make different mistakes, or the same mistakes for
different reasons, or notice the gaps sooner via unfamiliarity-driven
caution) but it cuts in a specific direction here: the fact that this
incident occurred *despite* high familiarity with RepoPact's intent is
arguably stronger evidence that the gap is structural (in the tooling, not
merely a naive-adopter mistake) than if it had occurred with an unfamiliar
team. This should be stated as a two-sided observation, not resolved in
RepoPact's favor or against it.

## Not a preregistered intervention

WI236/237 was not designed, hypothesized, or registered in advance as a test
of any RepoPact claim — it was ordinary engineering work (building a
canonical CI runner) that happened to surface the incident described here as
a byproduct, and this case study itself was commissioned *after* the fact,
observing what already happened rather than designing a controlled trial.
This is the single largest methodological difference from RepoPact's own
`protocol.md`/`benchmark-protocol.md` discipline, which explicitly sets
hypotheses and falsification criteria *before* runs, precisely so results
aren't shaped to fit a foregone conclusion. This case study's evidence should
be read as **naturalistic field observation**, not as a benchmark run, and
should not be represented as satisfying S1–S6's pre-registration discipline.

## Multiple agents/models participated

This incident's timeline spans several distinct agent sessions (the original
WI230 work, the WI236/237 canonical-CI work, this case-study phase) and at
least one independently-operating concurrent process (the causal-state-
runtime-substrate WI236 collision, `07-concurrency-id-collision.md`) whose
model/agent identity is not established by any source this case study
located. Findings that depend on "what an agent did or didn't do" (e.g.,
whether `doctor` was run between GA-1 and WI230, item 6 in
`04-forgewire-case-timeline.md`) cannot be attributed to a single agent's
behavior pattern; the evidence trail does not resolve who ran what, when,
across sessions.

## Hosted GitHub CI availability/billing was impaired

Directly relevant and worth stating plainly: **both** subject repositories in
this case study had hosted-CI impairment during the relevant window.
ForgeWire's own workflows simply never called the RepoPact CLI (a wiring gap,
not an availability gap — GitHub Actions ran fine for ForgeWire, just without
invoking RepoPact). RepoPact's *own* repository had genuine hosted-CI
unavailability (the billing lock, `03-version-delta.md`). These are different
mechanisms producing a structurally similar symptom (checkpoint not actually
gating merges), and conflating them would overstate the case. This case study
treats them as two independently-caused instances of the same *symptom*
(enforcement closure absent), not as one shared root cause.

## WI237 was intentionally designed after observing WI230

The canonical-CI-runner work (WI236/237) was explicitly motivated by having
already seen WI230's 26/297-error state — i.e., the intervention was a
direct, deliberate response to the very drift this case study analyzes, not
an independent variable applied blind to the outcome. This is the expected
and correct order of events for real engineering (see a problem, fix it) but
means WI236/237's "success" (269→0, `full`/`closeout` passing) should not be
read as a controlled test of "does building a canonical CI runner generically
fix drift" — it is one team's specific, informed response to one specific,
already-diagnosed problem, evaluated on the same repository that motivated it.

## Code/test debt predates the intervention

The pre-existing gate violations WI236/237 surfaced (forbidden imports since
2026-05-05, Rust lint drift, the CRLF/`git diff --check` miscalibration, the
orphaned test) were not created by this case study or by WI230 — they
accumulated over a longer period the evidence trail does not fully
reconstruct. Their discovery is attributable to *this session actually
running* previously-declared-but-unexercised gates for the first time, which
is itself the central finding (declared ≠ invoked ≠ effective), but the
underlying debt's age and origin are not independently dated here beyond
what each defect's own git blame would show (not run for this case study).

## Post-intervention observation window is short

`scripts/ci.py closeout` passed cleanly once, at one HEAD (`229156e1`), one
day after the intervention began. This case study has **zero** evidence about
whether the new canonical-CI arrangement (decision 0009, the `full`/`closeout`
profiles, the new pre-commit hook) actually holds up over weeks or months of
subsequent ordinary work — which is exactly the timescale (34+ days between
GA-1 and WI230) over which the *previous* arrangement's failure became
visible. Claiming the intervention "worked" in any durable sense would be
premature; it is accurate only to say it produced a real, verified,
zero-error state at one point in time, with real negative-control evidence
that the new gates function when deliberately broken (`scripts/ci.py`'s own
FAST/FULL runs correctly rejecting a reintroduced forbidden import and a
frozen-surface change without `--ack`, per this session's own record).

## Why the case is still valuable, despite the above

- **A real multi-agent project**, not a constructed benchmark fixture —
  ForgeWire is a genuine, long-lived, multi-subsystem application (GTK
  shell, HTTP gateway, two separate Rust workspaces, a distributed
  fabric/loom control plane) with real users' time and real architectural
  stakes, not a toy.
- **Long-lived repository** — 4569+ commits at RepoPact's own original
  adoption measurement (`findings.md` F-007), now materially more; the drift
  this case study examines occurred over real calendar time (at minimum the
  34-day GA-1-to-WI230 window), not a single synthetic session.
- **Architecture-changing work** — WI230's own scope (AgentBus retirement,
  a leaf-boundary reduction from 15 to 14 substrate-to-application imports)
  is genuine, consequential architectural work, not a scripted task designed
  to be temptable.
- **Non-toy consequences** — the forbidden imports, orphaned test, and Rust
  lint drift this case study's own intervention uncovered were real defects
  with real (if modest) blast radius, not manufactured for the study.
- **A natural pre/post intervention boundary** — WI230 (pre) and WI236/237
  (post) supply a genuine before/after structure this case study did not
  have to construct, even though it is uncontrolled in the sense described
  above.
- **A durable git/evidence trail** — nearly every claim in this case study
  traces to a specific commit SHA, evidence-run JSON, or work-item record,
  independently re-checkable by a third party without access to this
  conversation — which is itself a demonstration of the very recoverability
  property (H6) the paper claims RepoPact provides.
