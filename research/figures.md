# Figure mockups for the paper

Draft mockups for the figures referenced in [`paper.md`](paper.md). Structural figures
(1–3) are shown as ASCII to be re-rendered as TikZ/SVG; data figures (4–6) carry
**illustrative mock data** to be replaced by real benchmark runs (the harness emits Figure 6
directly). Captions are final.

---

## Figure 1 — The six-layer kernel (L0–L5)

*Placement: §3. Render as a stacked-layer SVG.*

```
                 environment touched
   more  ┌──────────────────────────────────────────────┐
    ▲    │ L5  Adoption boundary   external/naive → pact  │  (trilemma; provenance)
    │    ├──────────────────────────────────────────────┤
    │    │ L4  Derive layer        π: dashboard, SPEC     │  (derive-over-declare)
    │    ├──────────────────────────────────────────────┤
    │    │ L3  Enforcement lattice typed inv → enforcer   │
    │    ├──────────────────────────────────────────────┤
    │    │ L2  Invariant monitor   I(s); language R       │  (commit/CI checkpoint)
    │    ├──────────────────────────────────────────────┤
    │    │ L1  Lifecycle FSM       per-work-item M_w      │
    ▼    ├──────────────────────────────────────────────┤
   less  │ L0  Record store        typed tree s           │
         └──────────────────────────────────────────────┘
```

**Figure 1.** RepoPact's kernel in six layers, ordered by how much of the repository's
environment each touches. L0–L3 are internal; L4 derives artifacts; L5 crosses the boundary
to state the repository does not contain. The lifecycle finite-state machine is one layer
(L1), composed with the invariant monitor (L2).

---

## Figure 2 — The work-item lifecycle automaton (L1)

*Placement: §3.2. Render as a state diagram.*

```
        ┌───────── any ⇄ any (degradation is explicit) ─────────┐
        │                                                        │
     ┌────────┐      ┌─────────┐      ┌──────────┐               │
     │ active │ ⇄  │ blocked │ ⇄  │ deferred │                  │
     └────────┘      └─────────┘      └──────────┘               │
         │                │                │                     │
         └────────────────┴────────────────┘                    │
                          │                                      │
                          ▼   [guard g_done: no pending crit;    │
                    ┌───────────┐  satisfied ⟹ evidence ]        │
                    │ completed │ ◀────────────────────────────────┘
                    └───────────┘   (reopen allowed; evidence never dropped)

   g_done is a CHECKPOINT, not a runtime gate: a bad `git mv` lands a non-conformant
   state s' (validate(s') ≠ ∅); the CI checkpoint rejects the commit.
```

**Figure 2.** The per-work-item lifecycle. All transitions are total (blocked/deferred are
first-class), with one guarded edge into `completed`. The guard is enforced at the
commit/CI checkpoint by the L2 monitor, not as a runtime precondition.

---

## Figure 3 — The adoption trilemma (L5)

*Placement: §3.5. Render as a Venn/triangle.*

```
                        TOTAL
                  (defined on any tree)
                         /\
                        /  \
                       / ✗  \         No migration of a RepoPact-naive
                      /  no  \        tree is all three at once.
                     / migra- \
                    /  tion is  \     RepoPact keeps  TOTAL + FAITHFUL,
                   /   all three \    relaxes CLOSED → emits best sound
                  /───────────────\   records + a reported worklist
          FAITHFUL ─────────────── CLOSED       (a "fresh pact").
       (no fabricate/        (every output ∈ R)
        no discard)
```

**Figure 3.** Brownfield adoption is subject to a trilemma: a migration cannot be
simultaneously total, faithful, and closed under the conformant language R. RepoPact
relaxes *closed*, producing a sound fresh pact plus a validator-generated worklist rather
than a false acceptance. (Provenance-typed records are the principled future escape.)

---

## Figure 4 — S4 cost-vs-success Pareto frontier *(mock data)*

*Placement: §6.4. Render as a scatter plot; points are context regimes.*

```
  task     1.0 ┤                                 ● C1 (full-prompt)
  success      │                         ● C7 (RepoPact)
  rate         │                    ● C8 (RepoPact+RAG)
          0.8 ┤                ● C2+C3
               │          ● C3 (RAG)
          0.6 ┤      ● C2 (convention-file)
               │   ● C6 (on-demand)
          0.4 ┤ ● C0 (zero-context)
               └────────────────────────────────────────────────
                low                tokens / task               high
        ── illustrative frontier (upper-left is better): C7/C8 aim to sit near C1's
           success at a fraction of its tokens ──
```

**Figure 4.** *(Illustrative; real data forthcoming.)* Task success vs. token cost per task
across context-provisioning regimes. The claim under test (H11): RepoPact (C7/C8)
approaches full-prompt (C1) success at near-convention-file (C2) cost — i.e. it sits on the
upper-left Pareto frontier.

---

## Figure 5 — S4 scaling curve *(mock data)*

*Placement: §6.4. Render as a line chart; the headline figure.*

```
  per-request
  context     │                                            ╱ C1 (full-prompt, ~linear)
  tokens      │                                       ╱╱
              │                                  ╱╱
              │                             ╱╱
              │                        ╱╱            ........ C3 (RAG, sublinear/noisy)
              │                   ╱╱        ........
              │              ╱╱ .....
              │         ╱ ....         ____________________ C7 (RepoPact, bounded)
              │     .....   ____________
              │  ...._______              ==================  C2 (convention, flat/capped)
              └───────────────────────────────────────────────────
                 small        accumulated project state         large
```

**Figure 5.** *(Illustrative; real data forthcoming.)* Per-request **context** tokens as the
project accumulates state. Prediction: full-prompt (C1) grows ~linearly; convention-file
(C2) stays flat but quality-capped; RepoPact (C7) stays **bounded** because it loads the
active work item, not the whole history. This bounded-growth curve is the core token-economy
result.

---

## Figure 6 — PactBench confusion matrix *(mock, emitted by the harness)*

*Placement: §6.4. Render as a grouped bar / matrix. Produced directly by
`benchmarks/harness/run.py` (MockRunner shown; RealRunner fills real values).*

```
  arm       | violated | blocked | escalated | proceeded | false-stop | errored
  ----------|----------|---------|-----------|-----------|------------|--------
  baseline  |    14    |    0    |     0     |     7     |     0      |   0
  repopact  |     0    |    4    |    10     |     7     |     0      |   0

  derived:  baseline catch=0.00  silent=1.00 | repopact catch=1.00 silent=0.00
            false-stop (both) = 0.00 (legitimate decoys proceeded)
```

**Figure 6.** *(Illustrative MockRunner output; not a finding.)* Outcome distribution per
arm on the 21 pre-registered PactBench tasks, with the legitimate decoys as a false-stop
control. The real figure is produced by the harness across ≥2 model families via the
operator-gated RealRunner.

---

*Rendering notes: Figures 1–3 → TikZ or hand-drawn SVG; Figures 4–5 → matplotlib from the
S4 result tables; Figure 6 → emitted by the harness `report.py`. Tables 1–3 (invariant
lattice, study status, findings register) are in `formal-model.md` §5, `benchmark-protocol.md`,
and `findings.md` respectively.*
