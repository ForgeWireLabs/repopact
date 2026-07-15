# A formal model of RepoPact

*Companion to [`paper-outline.md`](paper-outline.md) §3 ("The model"). This document
gives RepoPact an operational semantics, stated to remain faithful to the reference
implementation ([`scripts/validate_repo.py`](../scripts/validate_repo.py)) and SPEC
§3–§7. Where this prose and the reference implementation disagree, the discrepancy is a
defect, resolved by an audit finding rather than by silent divergence (SPEC §1).*

> **Thesis.** RepoPact's kernel is a layered governance system for repository-level
> memory: the durable, shared record of intent, authority, evidence, and history through
> which independent agents and humans coordinate. The kernel comprises six layers (§0).
> One of them — the work-item lifecycle (L1) — is a finite-state machine. The others are
> a record store (L0), an invariant monitor over states checked at commit/CI boundaries
> (L2), a typed enforcement lattice (L3), a derive layer (L4), and an adoption boundary
> between the repository and the external systems it does not yet contain (L5). The
> sections below give each layer an operational semantics.

---

## 0. The kernel in layers

RepoPact's kernel comprises six layers. The work-item lifecycle (L1) is a finite-state
machine; the remaining layers constitute the governance substrate. Each has a formal
home below.

| Layer | Name | Object | Formal home |
| --- | --- | --- | --- |
| **L0** | Record store | the typed tree `s` (state algebra) | §1 |
| **L1** | Lifecycle FSM | per-work-item automaton `M_w` | §3 |
| **L2** | Invariant monitor | the predicate `I`; `R` = recognized language | §2 |
| **L3** | Enforcement lattice | invariants typed state / transition / temporal / relational → tiered enforcers | §5 |
| **L4** | Derive layer | projections `π` (dashboard, SPEC); derive-over-declare | §1, §4 |
| **L5** | Adoption boundary | migration of naive/external reality → the pact; the trilemma; provenance | §4, §7 |

The layers are ordered by how much of the repository's environment they touch. L0–L3 are
internal: everything they govern already lives in the tree. L4 derives artifacts from the
tree. L5 is the boundary at which the repository meets state it does not contain —
external trackers, design documents, and intent or history that were never committed. The
limits of RepoPact in its current version (§7) are L5 limits.

## 1. State

A repository state `s` is a finite typed record store. We write it as the tuple

```
s = ⟨ ver, Inv, Frz, Own, Reg, C, W, E, D, P, A ⟩
```

| Symbol | Component | Source location |
| --- | --- | --- |
| `ver` | semantic version string | `VERSION` |
| `Inv` | declared invariants | `governance/invariants.json` |
| `Frz` | frozen surface (globs, symbols, reasons) | `governance/frozen-surface.json` |
| `Own` | scopes `Σ`, roles, concurrency flag `δ` | `governance/owners.json` |
| `Reg` | audit registry (scope→contract, review dates) | `audits/registry.json` |
| `C` | set of contracts (`AGENTS.md` nodes) | `**/AGENTS.md` (minus `IGNORED_PARTS`) |
| `W` | set of work items | `work/<status>/NNN-slug/work-item.json` |
| `E` | set of evidence runs | `evidence/runs/<id>.json` |
| `D` | set of decisions | `decisions/NNNN-slug.md` |
| `P` | set of policies | `governance/policies/NNN-slug.md` |
| `A` | set of audit findings | `audits/findings/NNN-slug.json` |

Let `Σ = scopes(Own)` be the scope identifiers and `δ = Own.concurrency.enforce_disjoint_active_scopes ∈ {⊥,⊤}`.

A **work item** is

```
w = (id, title, σ, owner, aff, dep, AC, created, updated)
```

with status `σ(w) ∈ Q = {proposed, active, blocked, deferred, completed}`, `owner ∈` (intended) `Σ`,
`aff ⊆ Σ`, `dep ⊆` (intended) `Ids(W)`, and acceptance set `AC(w)` of criteria

```
c = (cid, st, ev),   st ∈ {pending, satisfied, waived},   ev ⊆ Ids(E).
```

Write `dir(w)` for the name of `w`'s lifecycle directory (`work/<dir(w)>/…`). The
distinction between `σ(w)` (the JSON field) and `dir(w)` (the filesystem) is deliberate:
their *agreement* is an invariant (§2, `I_ID`), not an assumption.

`S` is the set of all such states. `S` is infinite: `|W|`, `|E|`, and the other record
sets are unbounded. The repository as a whole is therefore modeled as an infinite-state
transition system (§4); finite-state structure is confined to the per-item lifecycle (§3).

The **derive projections** are total functions `S → Artifact`:

```
π_dash(s)  = generate_dashboard.generate(s)      → audits/reports/dashboard.md
π_spec(s)  = generate_spec.render(…, s)          → SPEC.md derived blocks
```

These are functions of `s` rather than transitions of governed state: the formal content
of *derive over declare* (charter principle 8, policy 001).

---

## 2. The well-formedness predicate `I` (what the validator decides)

The reference validator computes a finite set `Viol(s)` of atomic violations
([`validate`](../scripts/validate_repo.py)). Define

```
I(s)  ≡  Viol(s) = ∅           accept(s) ≡ I(s)           R = { s ∈ S : I(s) }.
```

`R` is the set of **conformant** repositories. By SPEC §1, RepoPact is defined as the
recognizer of `R`: a conformant implementation "accepts exactly the repositories that
satisfy every rule in §3–§7 and rejects the rest." `validate_repo.py` is therefore the
**characteristic function `χ_R`**, and `R` is the language RepoPact recognizes.

`I` decomposes as a conjunction of atomic predicates, each tied to a SPEC §4 rule and a
code site:

| Predicate | Statement | SPEC | Enforcer (`validate_repo.py`) |
| --- | --- | --- | --- |
| `I_ver` | `ver` matches `MAJOR.MINOR.PATCH` | §4.7 | `validate_version` |
| `I_struct` | every record satisfies its JSON Schema | §3 | `check_schema` (Draft 2020-12) |
| `I_contract` | root `AGENTS.md` exists; every nested contract is registered; `_audit/` companions complete | §4.1 | `validate_contracts` |
| `I_ID` | `id(r) = prefix(path(r))` ∀ record; `σ(w) = dir(w)`; ids unique per type | §4.2 | `validate_work`, `_validate_records`, `validate_*` |
| `I_ref` | `dep ⊆ Ids(W)`, `c.ev ⊆ Ids(E)`, `e.work_item ∈ Ids(W)`, `finding.scope ∈ Σ`, `role.scopes ⊆ Σ`, `decision.supersedes ⊆ Ids(D)`, `owner/aff ⊆ Σ`; and σ(w) ∈ {active, completed} ⟹ ∀d ∈ dep(w): σ(d) ≠ proposed (authorized work may not depend on unauthorized candidates) | §4.3 | `validate_work`, `validate_evidence`, `validate_findings`, `validate_owners`, `validate_decisions` |
| `I_accept` | ∀`c`: `c.st=satisfied ⟹ c.ev≠∅`; and `σ(w)=completed ⟹ ∄c∈AC(w): c.st=pending` | §4.4 | `validate_work` |
| `I_acyclic` | the `dep` digraph `G(s)=(Ids(W), dep)` is a DAG | §4.5 | `detect_dependency_cycles` (DFS 3-color) |
| `I_conc` | `δ ⟹ ∀` distinct non-terminal `w,w'`: `scopes(w) ∩ scopes(w') = ∅` | §4.6 | `validate_disjoint_scopes` |
| `I_orphan` | no dir under `work/` carries planning content (`README`/`AGENTS`/`_audit`) without a `work-item.json` | — | `validate_orphan_work_dirs` |

`I_orphan` carries no §4 number because it operationalizes INV-1: a planning artifact
invisible to the ledger is critical state held outside any tracked record. It connects a
governance invariant to a machine-checkable state predicate, and is a candidate for an
explicit SPEC §4 entry (proof obligation O-7).

`I_accept` comprises `{INV-2, INV-3}`: the two machine-enforced governance invariants are
exactly the two clauses of the acceptance predicate (§4).

---

## 3. The work-item lifecycle (L1): a guarded automaton

Per work item, the lifecycle is a finite automaton

```
M_w = (Q, Λ, δ_w, Q₀),   Q = {proposed, active, blocked, deferred, completed},   Q₀ = {proposed, active}.
```

`Q₀` is a *set* of initial states because `new` may create an item either as accepted
work (`active`, the default) or as a captured-but-unauthorized candidate (`proposed`,
via `--status proposed`; decision 0023). `proposed` records possible intent without
granting implementation authority; acceptance is the `proposed → active` move.

`Λ` is the alphabet of lifecycle moves (a directory relocation + a `σ` rewrite). `δ_w` is
**total** — any state may move to any state — with one guard, on edges into `completed`:

```
g_done(w, s) ≡ (∀ c ∈ AC(w): c.st ≠ pending)
             ∧ (∀ c ∈ AC(w): c.st = satisfied ⟹ c.ev ≠ ∅ ∧ c.ev ⊆ Ids(E)).
```

```
        ┌──────────────── any ⇄ any (degradation is explicit: charter P7) ──────────────┐
        │                                                                                │
   proposed  ──►  active  ⇄  blocked  ⇄  deferred                                       │
  [no authority]     │          │           │                                            │
                     └──────────┴───────────┴──────►  completed  [edge guarded by g_done]│
                                       completed ────────────────────────────────────────┘
                                       (reopen is allowed; evidence is never dropped — INV-4)
```

**Composition with the invariant monitor (L2).** The lifecycle automaton is the control
structure of a single work item; it delegates two concerns to the invariant monitor (L2).

1. **Acceptance.** Every `q ∈ Q` is legitimate: `blocked` and `deferred` are first-class
   states (charter principle 7, "degradation is explicit"). The automaton therefore has no
   rejecting states. Correctness is not reachability of an accepting final state
   (`◇accept`) but an invariant held across all states (`□I`), and that invariant resides
   in L2. L1 supplies the reachable control points; L2 determines which configurations of
   the whole tree are well-formed.
2. **Data constraints.** `g_done` quantifies over `AC(w)` and `Ids(E)`, which are
   unbounded. L1 is thus an *extended* (guarded) automaton whose guard is the L2 predicate
   `I_accept` evaluated on the post-state.

Composition is checkpoint-based rather than precondition-based. RepoPact does not evaluate
`g_done` as a runtime gate: a work item may be moved into `work/completed/` with pending
criteria by an ordinary `git mv`. The resulting state `s'` satisfies `validate(s') ≠ ∅`,
so `s' ∉ R`, and the CI checkpoint rejects the commit. L1 transitions freely; L2 decides
admissibility at the commit boundary. The lifecycle automaton models one coordinate of
`s`; the semantics of the whole repository is the transition system of §4 composed with
the monitor over all of `s`.

---

## 4. The repository as a transition system

```
T = (S, Init, Act, →)
```

- `Init ⊆ S`: the bootstrap images (output of `repopact init`).
- `Act`: the parameterized CLI actions.
- `→ ⊆ S × Act × S`: `s --a--> s'` iff `s' = effect_a(s)` (no runtime guards; see §3).

Actions partition by their relationship to `R`:

| Class | Actions | Property w.r.t. `R` |
| --- | --- | --- |
| **Constructor** | `init` | `effect(⊥) ∈ R` (lands valid) |
| **Invariant-preserving** | `new`, lifecycle move *with* `g_done`, `doctor --fix` | `s ∈ R ⟹ effect(s) ∈ R` (claimed; O-2/O-3/O-5) |
| **Migration (best-effort)** | `adopt`, `import-plan` | `effect(s)` **may leave `R`**; residual `Viol` is *reported*, not prevented |
| **Derive / read (no governed effect)** | `validate`, `dashboard`, `spec`, `check-frozen` | governed projection unchanged; may rewrite `π(s)` |

`adopt` and `import-plan` are not invariant-preserving. On a RepoPact-naive tree they emit
records mapping the existing signals and then report residual violations: the CLI prints
*"produced N validation error(s) to resolve"* and exits non-zero
([`repopact_cli.py`](../scripts/repopact_cli.py)). Their guarantee is verdict soundness —
the result never passes the validator while violating a rule (which would falsify
H3/H4) — together with the reported worklist.

`doctor --fix` is the dual: a **repair / retraction** operator `ρ`. Intended algebra
(O-5): `ρ` is conservative (never overwrites a *differing* source record — capture 010's
ForgeLink schema lesson), violation-monotone (`Viol(ρ(s)) ⊆ Viol(s)`), and a retraction
onto `R` (`ρ|_R = id`: healthy repos are fixed points). `adopt`/`import` map naive trees
*toward* `R`; `ρ` returns *drifted* trees *to* `R`.

### Adoption cannot preserve `R`: a trilemma

That `adopt` and `import-plan` may leave `R` is structural, not an implementation
limitation. A migration over a RepoPact-naive (or partially external) project is subject
to three requirements:

- **Total** — defined on any input tree.
- **Faithful** — maps existing signals to records without fabricating records or
  discarding signals (the non-destructive guarantee; decision 0008).
- **Closed** — every output lies in `R`.

No migration satisfies all three. Input trees contain configurations `R` forbids: a nested
`AGENTS.md` naming a team that no `CODEOWNERS` entry establishes as a scope (violating
`I_ref`); a roadmap with cyclic *blocked-by* edges (`I_acyclic`); a checklist item marked
done with no corresponding evidence (`I_accept`). Forcing such a tree into `R` requires
either inventing the missing record — for instance synthesizing an evidence run to satisfy
a criterion, which both breaks faithfulness and manufactures false proof in violation of
INV-3 — or discarding the offending signal, which breaks the non-destructive guarantee.
RepoPact relaxes *closed*, retains *total* and *faithful*, and reports the residue as a
validator-generated worklist, yielding a fresh pact rather than a false acceptance. This
is the guarantee formalized as T6 (§6).

The taxonomy above follows from each action's domain:

| Action | Domain | Closed under `R`? | Why |
| --- | --- | --- | --- |
| `init` | `{⊥}` (fresh target) | **yes, trivially** | authors the *entire* output; no prior input to remain faithful to |
| `new`, guarded move, `doctor --fix` | `R` (already-valid trees) | **yes (claimed)** | bounded deltas applied within `R` |
| `adopt`, `import-plan` | **arbitrary trees** | **no** | the trilemma |

`init` is closed under `R` because its domain is a single empty target and it authors the
entire output. `new`, guarded moves, and `doctor --fix` are closed because their domain is
`R` and they apply bounded deltas within it. `adopt` and `import-plan` operate on arbitrary
trees and are bound by the trilemma. These are the actions that cross the L5 boundary, and
they are central to real-world adoption (H7).

**Provenance typing (implemented in 2.0; decision 0021).** The trilemma
`{total, faithful, closed}` admits two members only while *faithful* requires every record
to be concrete — to assert a fact. A provenance type on records —
`concrete` versus `inferred`/`provisional` — lets a migration emit inferred/provisional
records that declare themselves reconstructed rather than proven. Such a record remains
faithful (it labels itself as not-yet-proof rather than fabricating proof) *and* lies in
`R` (it is a valid state, admitted by L2 rule P1). `adopt` now emits a **provisional** work
item backed by **inferred** evidence, so the migration is **both Closed and Faithful** — the
trilemma is resolved in the implementation, not merely relaxed. The L2 monitor enforces P2
(a `completed` item must be concrete) and P3 (a `concrete` item may not rest on non-concrete
evidence); `doctor` ratchets `provisional → concrete` (conservative, monotone) once concrete
evidence is attached. §7 develops the still-open direction (external project memory).

---

## 5. The invariant lattice is typed, and the type predicts the enforcer

RepoPact's seven invariants are not of a single logical kind, and the kind determines both
whether a machine can enforce an invariant and which mechanism does. A predicate over one
tree is decidable by the validator; a property of a change requires a diff; a property of
history requires the trace.

| INV | Statement (SPEC §6) | Logical type | Form | Enforcer |
| --- | --- | --- | --- | --- |
| INV-2 | completed ⟹ no pending criterion | **state** | `□ I_accept(s)` | `validate_repo.py` |
| INV-3 | satisfied ⟹ linked evidence | **state** | `□ I_accept(s)` | `validate_repo.py` |
| INV-7 | derived artifacts are generated, not hand-edited | **state (fixpoint)** | `□ (dash(s)=π_dash(s) ∧ spec(s)=π_spec(s))` | CI dashboard-diff (`governance.yml`) |
| INV-6 | frozen-surface change ⟹ operator approval | **transition (2-state)** | `□ (touch(Δ, Frz) ⟹ ack)` over diff `Δ=(s,s')` | `check_frozen_surface --base` |
| INV-4 | completed work is never rewritten to look cleaner | **temporal / historical** | `□ ¬rewrite(history)` over the git trace | human review + git |
| INV-5 | deepest `AGENTS.md` refines parents, never weakens | **relational / refinement** | `∀ c≺c': ⟦c⟧ ⊆ ⟦c'⟧` | human review |
| INV-1 | no critical state lives only in conversation | **meta (coverage)** | every load-bearing fact ∈ some tracked record | human + `I_orphan` (partial) |

As the logical type ascends from state to two-state to temporal to relational and meta, the
enforcer moves from the validator to a diff-time checker to human review. The progression
reflects what is decidable on a single tree. INV-6 takes a `--base` argument because a
change to the frozen surface is a two-state property not present in a single snapshot.
INV-4 is human-gated because it quantifies over history, which a single tree does not
contain. Mechanizing INV-4 and INV-5 would require, respectively, a trace semantics over
git history and a refinement order `≺` on contracts (O-4, O-6).

---

## 6. Theorems and proof obligations

Each is tagged with its discharge status. `[def]` true by definition for the reference
implementation; `[ci]` machine-checked on every run; `[fix]` covered by the fixture corpus
`tests/fixtures/`; `[conj]` a conjecture whose falsification is a proving-ground target
(mapped to the protocol hypotheses H1–H7).

- **T1 — Recognizer soundness & completeness.** `validate(s)=∅ ⟺ s ⊨ I`.
  `[def]` for the reference impl (SPEC §1 defines `R` as its accept set). For an
  *alternative* implementation it is a theorem, tested against the corpus: one valid
  baseline that must be accepted and one invalid overlay per §4 rule that must be rejected
  with a declared message. `[fix]` via `tests/test_conformance.py` (SPEC §9, conformance).

- **T2 — Constructor correctness (H1).** `effect_init(⊥) ∈ R`.
  `[ci]` — `repopact init` validates its own output and exits non-zero otherwise
  ([`repopact_cli.py`](../scripts/repopact_cli.py)); every invocation is a proof instance.
  Capture 001.

- **T3 — Surface closure (H2).** For every advertised action `a` and every `s ∈ Init`,
  `a(s)` is defined (terminates without crash or corruption): the tool's output is closed
  under the tool's own surface. `[conj]` — the original counterexample was F-001 (a
  documented command absent from the dispatcher). This is totality, weaker than
  `R`-preservation.

- **T4 — Completion safety (H3).** For a lifecycle move into `completed`:
  `s ∈ R ∧ g_done(w,s) ⟹ effect(s) ⊨ I_accept`. Equivalently, since the move is unguarded
  at runtime: `s' with σ(w)=completed ∧ (∃c: c.st=pending) ⟹ s' ∉ R`. This is the formal
  statement of "completion requires proof." `[fix]` (the `completed`-with-pending overlay),
  `[conj]` adversarially (¬H3 = the validator accepting unproven completion).

- **T5 — Monitor non-bypass (H4/H5).** For *any* edit trace `s₀ → s₁ → … → s_k` (arbitrary
  filesystem edits, not just `Act`), the CI checkpoint admits the commit producing `s_k`
  iff `s_k ∈ R`. No invalid state is admitted past a checkpoint. The state invariants
  (INV-2/3/7, all of §4) are enforced this way; the two-state invariant INV-6 is enforced
  on the diff `(s_{k-1}, s_k)` by `check_frozen_surface`. `[ci]`/`[conj]` (¬H4 = a
  frozen/invariant change passing unacknowledged; ¬H5 = a status/dir mismatch, cycle, or
  scope clash accepted).

- **T6 — Migration is not invariant-preserving.** `adopt` and `import-plan` are not
  `R`-preserving: `∃ s. effect_adopt(s) ∉ R`. The result is structural — the adoption
  trilemma (§4) shows no total, faithful migration is closed under `R` — not a defect.
  Their guarantee is verdict soundness (T1 holds on the result) together with a reported
  worklist; they establish a fresh pact and do not fabricate conformance. `[ci]` — the CLI
  reports residual violations and exits non-zero.

- **O-1 … O-7 — Open obligations.**
  - **O-2** `new`-correctness: `s ∈ R ⟹ new(s) ∈ R` (a stamped template lands valid). `[conj]`
  - **O-3** lifecycle preservation: a *guarded* move preserves all *state* invariants, not only `I_accept`. `[conj]`
  - **O-5** `doctor` algebra: conservative ∧ violation-monotone ∧ `ρ|_R = id`. `[conj]`, partial evidence in capture 010.
  - **O-4** trace semantics for INV-4 (mechanize "no history rewrite" over git). *open / unmechanized.*
  - **O-6** a refinement order `≺` on contracts to mechanize INV-5. *open / unmechanized.*
  - **O-7** give `I_orphan` a SPEC §4 number (it is enforced but uncatalogued).

---

## 7. Contributions and limits

**Contributions.** (1) A precise definition of the validator's decision (T1) and a
conformance target for alternative implementations. (2) The action taxonomy of §4 and T6.
(3) The typed invariant lattice (§5), which accounts for the three enforcement mechanisms
and identifies what a fourth would require. (4) A backlog of open obligations: O-4 and O-6
are the path to mechanizing INV-4 and INV-5.

**Faithfulness.** The model's value depends on its correspondence to the implementation. If
`I` here and `Viol` in the code diverge, this document becomes a second hand-maintained
mirror of derivable state — the failure mode policy 001 prevents. The clause table of §2 is
kept row-aligned to the validator's functions, and any discrepancy between this prose and
the code is treated as an audit finding (SPEC §1). INV-4, INV-5, and INV-1 lie outside the
validator by their logical type (§5); the model marks that boundary explicitly. The
lifecycle automaton (§3) models a single coordinate of the state; the semantics of the
whole repository is the transition system of §4.

**The L5 boundary: brownfield adoption.** RepoPact governs only what is in the repository.
The memory that makes coordinated agentic work possible — plans with real-time updates,
design intent, prior results, the history of *why* — frequently does not live in the repo.
It lives in external systems: trackers (Jira, Trello, Linear), engineering documents,
company-trajectory documents, and conversations that were never committed. `adopt` reads
CODEOWNERS, workflows, nested contracts, and git history, and from them builds the best
sound starting point obtainable at the moment of adoption — a fresh pact. It cannot read
what is not present. The gap between the pact and the project's true state is real adoption
drift, and it is not fabricable: the correct response is to state the gap, not to close it
with invented records (which would violate INV-3).

This is the central design tension for an agentic operating system at the kernel level.
Coordinated multi-agent work degrades without shared, current memory in the same way that
distributed teams without communication do: agents given partial views of the problem
produce fractured systems — conflicting schemas, incompatible stacks, and duplicated or
contradictory implementations. The repository-as-memory is the coordination substrate, and
the value of the pact is proportional to how much of the working memory it holds. Two
directions follow, both at L5:

1. **External ingestion.** Bring external sources across the boundary as first-class,
   evidence-bearing records (a tracker export → work items + provenance; a design document
   → a decision or contract). This widens what `adopt` can faithfully capture and shrinks
   the unfabricable gap.
2. **Provenance-typed, evolvable records.** Adoption emits `inferred`/`provisional` records
   (the trilemma escape, §4) that are explicitly reconstructed rather than asserted, and
   designed to be completed by a human- or agent-led pass — including the tiered
   `AGENTS.md` contracts, which adoption scaffolds as deterministically as possible while
   leaving completion hooks where determinism runs out. The kernel remains governing and
   transitional rather than immutable: records ratchet from `inferred` to `concrete` as
   evidence arrives.

Of these, **provenance-typed records are implemented in 2.0** (decision 0021): adoption
emits provisional/inferred records and `doctor` ratchets them, resolving the trilemma in the
implementation. **External ingestion remains future work.** Hypothesis H7 is correspondingly
bounded: adoption is non-destructive and sound at the time it is performed, and now
*honestly typed* (provisional, not falsely concrete), but completeness is still limited by
how much of the project's memory is reachable from within the repository.

---

## 8. Map to the paper

This document is the formal spine of [`paper-outline.md`](paper-outline.md) §3 (the model)
and §4 (the validator as reference semantics). In its vocabulary, §5–§6 of the outline (the
evaluation) become: the proving ground attempts to falsify T3–T5 and O-2/O-3/O-5, and each
falsification is a `findings.md` entry citing a capture.

The contributions beyond the SPEC are the kernel-layer model (§0), which presents the
architecture with the lifecycle automaton as L1; the adoption trilemma and provenance
typing (§4); the typed invariant lattice (§5); and the L5 boundary (§7), which states the
repository-as-kernel thesis precisely together with its present limits. In each, the
architecture's enforcement tiers and adoption guarantees follow from the logic rather than
from convenience.
