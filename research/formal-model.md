# A formal model of RepoPact

*Companion to [`paper.md`](paper.md) and [`paper-outline.md`](paper-outline.md). This
document gives RepoPact an operational model intended to remain faithful to the
versioned specification, conformance corpus, and canonical implementation. If this prose,
the specification, the conformance fixtures, and the implementation disagree, the
disagreement is a defect to record and reconcile rather than a reason to silently choose
one description after the fact.*

> **Thesis.** RepoPact is a repository-native governance kernel. It keeps the durable,
> shared state of intent, authority, evidence, provenance, decisions, invariants, and
> lifecycle in the same version-controlled artifact that humans and agents must obtain to
> work on the project. The kernel has six layers, L0 through L5. Governance continuity
> is a cross-cutting recoverability property over those layers, not a seventh layer.

## 0. Kernel layers

| Layer | Name | Object | Role |
| --- | --- | --- | --- |
| **L0** | Record store | typed repository state `s` | stores governing source records |
| **L1** | Lifecycle FSM | per-work-item automaton `M_w` | models work state and authority transitions |
| **L2** | Invariant monitor | predicate `I`; language `R` | decides repository conformance |
| **L3** | Enforcement lattice | typed invariants | maps invariant kind to an appropriate enforcer |
| **L4** | Derive layer | projections `pi` | generates deterministic materialized views |
| **L5** | Adoption boundary | migration and external state | brings previously ungoverned state into the pact |

L0 through L3 operate on state already represented in the tree. L4 derives views from that
state. L5 is the boundary where the repository meets state it does not yet contain, such as
trackers, design documents, conversation history, and implicit human knowledge.

Two cross-cutting properties sit over these layers:

1. **Enforcement closure**, defined in §7, asks whether a configured admission boundary
   actually invokes and binds the applicable checker.
2. **Governance continuity**, defined in §8, asks whether the represented governed state
   remains recoverable when the worker, machine, or legitimate client changes.

Neither property creates a new source of truth.

## 1. State

A repository state is a finite typed record store. Write:

```text
s = <ver, Inv, Frz, Own, Reg, C, W, E, D, P, A, Prov>
```

| Symbol | Component | Typical source location |
| --- | --- | --- |
| `ver` | semantic version | `VERSION` |
| `Inv` | binding invariants | `governance/invariants.json` |
| `Frz` | frozen surfaces | `governance/frozen-surface.json` |
| `Own` | scopes, roles, concurrency rules | `governance/owners.json` |
| `Reg` | audit and contract registry | `audits/registry.json` |
| `C` | registered contracts | root and nested contract files |
| `W` | work items | `work/<status>/<id-slug>/work-item.json` |
| `E` | evidence runs | `evidence/runs/<id>.json` |
| `D` | decisions | `decisions/<id-slug>.md` |
| `P` | policies | `governance/policies/<id-slug>.md` |
| `A` | audit findings | `audits/findings/<id-slug>.json` |
| `Prov` | provenance typing | record fields plus semantic rules |

`Prov` can be treated as a function over record claims rather than an independent file set:

```text
prov(r) in {concrete, provisional, inferred}
```

A **concrete** claim is directly authored or backed by concrete evidence. A
**provisional** claim is valid but intentionally unfinished. An **inferred** claim is
reconstructed from available signals rather than directly proven.

The distinction is epistemic. It lets RepoPact represent uncertainty without turning
uncertainty into either silence or fabricated certainty.

### 1.1 Work items and authority

A work item is:

```text
w = (id, title, status, owner, affected_scopes, dependencies, AC, created, updated, prov)
```

with:

```text
status(w) in {proposed, active, blocked, deferred, completed}
```

The status carries authority semantics:

- `proposed`: candidate work is recorded but not authorized for implementation.
- `active`: work is accepted and authorized to proceed.
- `blocked`: accepted work cannot currently proceed for a recorded reason.
- `deferred`: accepted work is intentionally postponed.
- `completed`: delivered work is evidence-closed under the applicable completion rules.

A criterion is:

```text
c = (criterion_id, status, evidence_links)
status(c) in {pending, satisfied, waived}
```

A work item's declared status and its lifecycle directory are two independently observable
facts. Their equality is an invariant, not an assumption.

### 1.2 Derived projections

The derive layer produces materialized artifacts from source records:

```text
pi_dashboard(s) -> audits/reports/dashboard.md
pi_spec(s)      -> SPEC.md derived blocks
```

The principle is **derive over declare**. A view that can be computed from source records
should not become a second manually maintained authority.

## 2. The well-formedness predicate

Let the canonical validator compute a finite set of violations:

```text
Viol(s)
```

Define:

```text
I(s) iff Viol(s) = empty
R = { s | I(s) }
```

`R` is the recognized conformant repository language for a given RepoPact version.
Conformance is behavioral, not an implementation-language identity. The current release
line uses a canonical Rust semantic engine for the proven validation surface. The historical
Python validator can remain useful as an independent regression comparator, but it is not a
second canonical authority for surfaces already cut over to Rust.

A conforming alternate implementation must reproduce the versioned observable behavior
encoded by the specification and conformance corpus. It does not need to copy RepoPact's
internal code.

The predicate `I` includes at least the following classes:

| Predicate | Statement |
| --- | --- |
| `I_ver` | the version is well formed |
| `I_struct` | each record satisfies the applicable schema |
| `I_contract` | required contracts are present and registered |
| `I_id` | identifiers, record paths, and lifecycle directories agree |
| `I_ref` | dependencies, scopes, evidence, decisions, owners, and findings reference known records |
| `I_accept` | completed work has no pending criteria; satisfied criteria have evidence |
| `I_acyclic` | the work dependency graph is acyclic |
| `I_conc` | disjoint active-scope rules hold when configured |
| `I_orphan` | planning content does not silently exist outside the ledger where the rule applies |
| `I_prov` | provenance is valid and completion does not treat non-concrete proof as concrete |
| `I_derive` | enforced materialized views equal their canonical projections |
| `I_frozen` | protected changes receive required acknowledgement at diff time |

Some of these are one-tree predicates. `I_frozen` is shorthand for a transition property
that requires a base and head. This distinction is made explicit in §5.

## 3. Lifecycle automaton, L1

Per work item:

```text
M_w = (Q, Lambda, delta_w, Q0)

Q  = {proposed, active, blocked, deferred, completed}
Q0 = {proposed, active}
```

A work item can be born as a candidate or as accepted work. The transition:

```text
proposed -> active
```

is therefore an authority event. It changes recorded intent into accepted implementation
work.

RepoPact intentionally permits non-monotonic lifecycle motion. Work can become blocked,
deferred, reopened, or moved backward when reality changes. Degradation should be explicit
rather than hidden.

The completion edge is semantically guarded by a predicate such as:

```text
g_done(w, s) =
  every acceptance criterion is not pending
  and every satisfied criterion links evidence that exists
  and completed work is concrete
  and concrete completed work does not rest on non-concrete evidence
```

RepoPact is repository-native, not runtime-exclusive. A human or agent can still edit files
directly and temporarily create an invalid tree. The invariant monitor decides whether the
resulting state is admissible at an applicable checkpoint.

That means L1 supplies possible lifecycle transitions while L2 determines whether the
whole repository configuration after a transition belongs to `R`.

## 4. Repository transitions, adoption, and provenance

Model repository evolution as:

```text
T = (S, Init, Act, ->)
```

where `S` is the set of representable repository states, `Init` is the set of supported
bootstrap states, and `Act` contains governed operations plus arbitrary filesystem edits
that may occur outside RepoPact's clients.

Useful action classes are:

| Class | Examples | Intended property |
| --- | --- | --- |
| constructor | `init` | creates a conformant governed repository from a supported clean target |
| typed mutation | create/edit/transition | plans and applies bounded semantic changes |
| derive/read | `validate`, `dashboard`, analysis | observes state or regenerates derived views |
| repair | `doctor --fix` | conservative movement toward conformance |
| migration | `adopt`, `import-plan`, takeover flows | crosses L5 and reconstructs previously ungoverned state |
| diff-time enforcement | frozen-surface check | evaluates a transition against a base state |

### 4.1 The concrete-record adoption trilemma

A brownfield migration often encounters facts that are reachable but not fully proven. A
legacy roadmap may say a task is complete without evidence. A historical plan may indicate
ownership without a current authoritative owner record. A decision may be reconstructable
from history but lack a direct contemporaneous declaration.

For this **epistemic reconstruction problem**, consider three goals:

1. **Totality:** migration can process the reachable input signal.
2. **Faithfulness:** migration preserves what was observed without fabricating proof.
3. **Closure:** the emitted record is valid in the target language.

If every emitted claim must be concrete, those goals cannot always hold together. A
migration must either manufacture certainty, discard the signal, or emit an invalid claim.

This is the **concrete-record adoption trilemma**:

```text
total + faithful + closed cannot always hold
when every reconstructed claim must be concrete
```

Provenance typing changes the target language. A reconstructed claim may be admitted as
`inferred` or `provisional`, allowing the migration to preserve the observation without
pretending it is proof.

Let `R_p` be the provenance-aware recognized language. For uncertainty whose only conflict
is epistemic status, provenance typing permits:

```text
reachable signal -> inferred/provisional record in R_p
```

while completion remains stricter and requires the appropriate concrete proof.

### 4.2 What provenance does not solve

Provenance typing does **not** make every arbitrary legacy tree conformant.

A source project may contain structural contradictions that remain contradictions after the
facts are honestly typed: cyclic dependencies, impossible identifiers, conflicting
relationships, unsupported record shapes, or references that cannot be mapped without
changing their meaning. Those cases can still leave `Viol(s)` non-empty after migration.

This distinction is important. RepoPact resolves the concrete-record trilemma for
**epistemic reconstruction**. It does not claim a theorem that every arbitrary source tree
can be mapped losslessly into a conformant target with no residue.

A sound migration therefore has two responsibilities:

1. represent reconstructable uncertainty with honest provenance rather than fabricated
   certainty;
2. report structural residue it cannot faithfully map instead of hiding it.

This is the bounded claim the paper should make.

### 4.3 Repair and ratcheting

Let `rho` denote the repair behavior of `doctor --fix`. Its intended algebra is:

```text
Viol(rho(s)) subseteq Viol(s)
rho(s) = s for already healthy supported states
```

and repair should not silently overwrite differing source intent merely to obtain a green
validator result.

Provenance ratcheting is a related but distinct operation. An inferred or provisional
record may become concrete when the required concrete evidence arrives. The reverse should
not occur silently.

## 5. Typed invariant lattice, L3

RepoPact invariants are not one logical kind. Their type predicts what information an
enforcer needs.

| Type | Example | Needed information | Enforcer class |
| --- | --- | --- | --- |
| state | completed implies no pending criterion | one tree | validator |
| state | satisfied implies linked evidence | one tree | validator |
| state with provenance | completed proof is concrete | one tree | validator |
| state fixpoint | dashboard equals canonical projection | one tree plus generator | validator/generator |
| transition | frozen-surface change requires acknowledgement | base and head | diff-time checker |
| temporal | completed history is not rewritten to look cleaner | git trace | history analysis and review |
| relational | nested contract refines parent | contract pair and semantic order | review, future formalization |
| meta coverage | critical state does not live only outside the pact | repository plus external judgment | partial checks and review |

A single JSON Schema cannot decide a temporal property. A single-tree validator cannot
know whether a protected file changed relative to a base. Human review cannot efficiently
replace deterministic referential-integrity checks.

The lattice is therefore partly a restraint mechanism. RepoPact should make enforcement
boundaries explicit rather than claim total automation over semantic project intent.

## 6. Theorems and proof obligations

The labels below separate definitions, machine-checked properties, fixture-backed claims,
structural arguments, and empirical conjectures.

- **T1: Recognizer definition. `[def]/[fix]`**  
  For a given version, the canonical validator recognizes `R`. Alternative implementations
  are tested against the versioned conformance corpus rather than by code identity.

- **T2: Constructor correctness. `[ci]`**  
  `init` should land a supported clean target in `R` and should fail explicitly if it
  cannot.

- **T3: Surface closure. `[conj]`**  
  Advertised operations should either succeed or fail cleanly on the initialized surface
  without corrupting governed state. F-001 is the historical counterexample that made
  this a standing obligation.

- **T4: Completion safety. `[fix]/[conj]`**  
  A completed work item with pending criteria, missing evidence, non-concrete status, or
  non-concrete proof where concrete proof is required is not conformant.

- **T5: Checkpoint decision correctness. `[ci]/[conj]`**  
  When the applicable checkpoint actually executes on a candidate state, it should admit
  the state exactly when the enforced conformance rules are satisfied. §7 separates this
  decision correctness from deployment coverage, invocation, and effectiveness.

- **T6a: Concrete-record adoption trilemma. `[structural]`**  
  For reachable legacy claims whose uncertainty is epistemic, a migration that requires
  every emitted claim to be concrete cannot always be total, faithful, and closed at once.

- **T6b: Provenance-typed epistemic closure. `[ci]/[conj]`**  
  Where the only obstacle is epistemic status, a provenance-aware migration can emit an
  inferred or provisional record that remains faithful and valid while preserving stricter
  concrete completion rules. This does not imply closure for arbitrary structural
  contradictions.

- **T7: Repair monotonicity. `[conj]`**  
  Repair should not increase known violations, should preserve differing source intent,
  and should behave like identity on already healthy supported states.

- **T8: Governance continuity for represented state. `[conj]/[empirical]`**  
  A clean worker or supported-client transition should preserve recoverability of the
  repository's represented governance projection and current known violations without
  predecessor-private state. §8 defines this more precisely; H15/S8 is the prospective
  empirical test.

Open proof obligations continue to include stronger `new` preservation, lifecycle
preservation over all state invariants, trace semantics for historical invariants, a
refinement order for nested contracts, repair algebra, and explicit cataloging of every
implemented semantic predicate in the public specification.

## 7. Enforcement closure

RepoPact's repository checkpoint model leaves a deployment question outside the one-tree
recognizer. A validator can be correct and still be operationally irrelevant if the path
that admits state never calls it or ignores its result.

Let `A` be the deployment-designated set of consequential admission transitions, such as
merge to a protected branch, release publication, or another promotion the deployment
chooses to govern.

For `tau in A`, define:

```text
Cov(tau)   = the admission path routes through the applicable checker
Inv(tau)   = the checker actually executes for this candidate
Eff(tau)   = a rejecting result prevents the promotion
```

Then **enforcement closure** over `A` is:

```text
EC(A) := for all tau in A:
         Cov(tau) and Inv(tau) and Eff(tau)
```

The three predicates are independent. A path can lack coverage even when infrastructure is
healthy. A covered path can fail invocation because the runner or CI environment never
starts. A checker can invoke and reject correctly while an unprotected promotion ignores
that result.

Availability is therefore a possible cause of failed invocation, not a fourth logical
property.

`EC(A)` is substrate-neutral. GitHub required checks, self-hosted CI, a local promotion
runner, a release gate, or another mechanism can establish the property. The model does
not privilege one vendor.

### 7.1 Relationship to T5

T5 concerns the correctness of a checkpoint's decision when the checker executes. It does
not prove that the deployment provides `Cov`, `Inv`, or `Eff`.

A stronger operational non-bypass statement therefore needs both:

```text
checkpoint decision correctness + EC(A)
```

The naturalistic field case that motivated H14 showed why this distinction matters. It did
not confirm H14. S7 is the prospective comparative test.

## 8. Governance continuity

Governance continuity is the recoverability property that connects the repository-native
model to human-agent handoff.

Define the represented governance projection:

```text
G(s) = <Inv, Frz, Own, Reg, C, W, E, D, P, A, Prov>
```

and retain `Viol(s)` as the current known conformance violations.

Let `clone_v(s)` mean a clean clone of the versioned repository state together with the
RepoPact version needed to interpret it. A handoff `h` to a fresh legitimate worker or
supported client is governance-continuous for represented state when:

```text
recover_h(clone_v(s)) = <G(s), Viol(s)>
```

up to presentation-equivalent ordering and formatting.

This equality is semantic, not byte-for-byte UI identity. Two clients may render the same
work differently while agreeing on its id, authority state, dependencies, provenance,
evidence, scopes, and current violations.

### 8.1 Worker independence

For state RepoPact claims is repository authoritative, recovery should not require:

- the predecessor's chat transcript;
- a predecessor-only local database;
- a hidden UI cache;
- provider memory;
- one workstation's uncommitted index;
- an undocumented summary generated during a prior session.

A local cache or index is allowed as an acceleration artifact only if it is non-authoritative
and can be rebuilt from the repository without loss of governed meaning.

This distinction is directly relevant to future repository orientation work. A durable
repository graph may accelerate recovery, but it should not become a hidden second source
of truth. If committed, it should be a deterministic or provenance-bearing derived
projection. If local, it must be rebuildable.

### 8.2 Invalid-state continuity

Continuity does not mean a clean handoff must produce a green repository.

If `Viol(s)` is non-empty, a faithful handoff should preserve visibility of that fact. A
worker transition that silently turns known invalid state into a healthy report is a
continuity failure even when all source records were copied successfully.

This is why the recovery target includes both `G(s)` and `Viol(s)`.

### 8.3 Boundary of the claim

Governance continuity is not omniscience. State that never crossed L5 cannot be recovered
from the tree. The property is scoped to **represented, repository-authoritative state**.

It is also not an efficiency claim. A system could be perfectly recoverable yet require
painful repository-wide search every session. H15 states the structural property; S8
measures correctness and separately records orientation cost, including tokens, file reads,
tool calls, and repository-wide search operations.

This separation lets future orientation mechanisms be evaluated honestly. They may improve
cost without being allowed to redefine what counts as continuity.

## 9. Contributions and limits

The formal contributions are:

1. a recognized repository language and conformance target;
2. a lifecycle model in which status carries authority semantics;
3. a typed invariant lattice that explains why different rules need different enforcers;
4. the concrete-record adoption trilemma and provenance-typed epistemic resolution;
5. enforcement closure as a deployment property separate from validator correctness;
6. governance continuity as a clean-handoff recoverability property over represented state.

The model remains intentionally bounded.

RepoPact cannot govern facts it never receives. External ingestion can widen L5, but every
source needs provenance rather than automatic promotion to concrete truth. RepoPact also
does not replace runtime authorization, sandboxing, or execution control. Repository
conformance and runtime safety are different boundaries.

Finally, the model must not outpace the implementation. The canonical Rust engine is the
semantic authority only for surfaces actually cut over and proven. Migration-oriented or
historical commands that remain outside that surface should be named honestly. Likewise,
macOS and iOS targets are not treated as validated merely because the shared Tauri code can
target them.

## 10. Map to the paper and research protocol

This document is the formal spine for [`paper.md`](paper.md).

The key mappings are:

- §§0-5 map to the paper's repository-native governance kernel and typed enforcement model.
- §4 maps to the brownfield adoption and provenance discussion, with the structural-residue
  caveat that prevents overclaiming.
- §7 maps to H14 / S7, enforcement closure.
- §8 maps to H15 / S8, governance continuity.
- the conformance model supports the claim that multiple legitimate clients can share one
  semantic authority without requiring identical implementations.

The proving-ground program attempts to falsify the empirical claims. The formal model does
not turn a preregistered hypothesis into a theorem merely because the notation is clean.
