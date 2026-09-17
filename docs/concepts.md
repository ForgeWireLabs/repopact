# Concepts

*Diataxis mode: explanation (understanding-oriented).*

## The problem: governance discontinuity

Human and agent work increasingly begins in conversations whose state does not reliably survive the session. The next contributor may recover the source code but not what was authorized, which constraints are binding, why a decision was made, what remains unfinished, or what evidence actually supports a completion claim.

RepoPact treats that as a software-engineering problem. Its answer is to put the load-bearing engineering state in versioned repository records so legitimate workers can recover it without inheriting a private conversation or vendor-specific memory system.

The repository is the durable coordination surface. That is the "pact."

## Why "pact"

The distinguishing primitive is the **binding invariant**: a guarantee a worker may not silently weaken. An invariant carries rationale, an escalation path, and, where its logical type and integration boundary permit it, a machine enforcer.

See decision [`0001`](../decisions/0001-repository-as-pact.md).

## Principles vs invariants

The charter separates the two deliberately:

- A **principle** is a value you weigh, such as "drift is a defect." It is upheld by judgment.
- An **invariant** is a line you do not cross without the required authority, such as "a completed work item has no pending criteria." It is upheld by a machine check or a named escalation.

This distinction prevents a useful design preference from being accidentally treated as a hard rule, and prevents a binding rule from being softened into advice.

See [`governance/charter.md`](../governance/charter.md).

## One truth per record

RepoPact avoids maintaining several competing copies of the same fact. Each record type owns one kind of truth:

- invariants bind;
- decisions record durable choices and rejected alternatives;
- policies record durable operating rules;
- work items track authorized units of work and lifecycle;
- evidence records concrete observations or executions;
- findings report drift or unresolved problems;
- the dashboard derives a view from source records.

See [`governance/record-types.md`](../governance/record-types.md).

## Authority is not the same as presentation

RepoPact deliberately separates **authoritative state** from **derived views**.

Authoritative project state lives in the source record or repository surface that owns a fact: `governance/`, `work/`, `decisions/`, `evidence/`, source code, and explicitly governed configuration.

The following are useful but derived:

- the generated dashboard;
- Repository Orientation Graph data;
- Workbench views;
- generated specification blocks;
- indexes, summaries, and other read models.

A derived view can be durable and highly structured without becoming authoritative. If a derived view conflicts with its source records, the derived view is stale or wrong and must be regenerated or reconciled.

This is why RepoPact can add better interfaces and semantic indexes without creating a second project truth.

## Derive over declare

Anything computable from source records is generated, not hand-maintained. The dashboard, generated specification catalog, and audit-freshness views are examples.

The lesson behind policy [`001`](../governance/policies/001-derived-artifacts-are-generated.md) is that hand-maintained mirrors of derivable state eventually drift, and the resulting staleness becomes indistinguishable from a real disagreement about project state.

Canonical generation has a precise boundary: byte-equality with a freshly generated dashboard proves an exact projection of the source manifests. It does **not** prove that a human-authored manifest, external-service claim, research assertion, or audit alignment statement remains true about external reality. Those semantic claims need verification dates and review deadlines under policy [`002`](../governance/policies/002-semantic-claim-freshness.md).

## Work lifecycle carries authority

RepoPact lifecycle states are not cosmetic labels:

- **`proposed`** captures an idea durably without authorizing implementation;
- **`active`** represents accepted work authorized to proceed;
- **`blocked`** is accepted/current work that cannot proceed until a named condition changes;
- **`deferred`** is accepted work intentionally postponed with rationale;
- **`completed`** is delivered work whose acceptance criteria are evidence-closed.

This matters because planning and authorization are different. Recording an idea should not silently grant permission to implement it.

A work item also records owner scope and affected scopes so cross-scope work has an explicit lead instead of relying on whoever happens to be running the current session.

## Completion requires proof

A work item is not complete because a human or agent is confident. Its acceptance criteria must be satisfied with linked evidence according to the repository's rules.

Evidence does not automatically grant authority. A passing test cannot activate a proposed work item, approve a frozen-surface mutation, or mint an operator approval receipt. RepoPact keeps **evidence**, **authority**, and **lifecycle** distinct even when one operation produces information relevant to another.

Confidence is not evidence, and evidence is not authority.

## Provenance makes uncertainty explicit

Brownfield adoption requires reconstruction. RepoPact does not solve that by pretending reconstruction is perfect.

Records can carry provenance such as:

- **`concrete`** — directly established state or evidence;
- **`provisional`** — usable state that has not yet been fully ratified;
- **`inferred`** — reconstructed from indirect evidence.

This lets an adopted repository become usable without laundering uncertain history into fact. Later evidence can ratchet provisional or inferred state toward concrete state.

## RepoPact is not `AGENTS.md++`

`AGENTS.md`, `CLAUDE.md`, editor rules, skills, and system prompts are instruction surfaces. They tell a worker how it should behave within the environment that reads them.

RepoPact is the durable governance substrate around those instructions: work authority, lifecycle, binding invariants, frozen surfaces, decisions, provenance, evidence, audits, and derived state.

Instruction files can participate in the pact and can be governed by it. They do not replace the rest of the pact, and RepoPact does not require one particular instruction-file format.

## Enforcement is a ladder

RepoPact avoids a single marketing boolean called "enforced." Different integrations provide different boundaries, and the claim must match the actual boundary.

### `instruction-only`

The repository records durable rules and state, but no pre-execution host boundary is claimed. Validation can still reject invalid repository state after or around a change.

### `session-start`

A covered integration checks authority before starting a session or child process. This is stronger than instruction-only operation, but it does not imply that every later filesystem action by the process tree is intercepted.

### `pre-action`

A covered mutation is checked before its callback or action begins. A denial means that covered action does not start.

This is RepoPact's portable reference baseline for the optional admission plane. It is a meaningful pre-execution guarantee for the action families the adapter actually covers, but it is **not** arbitrary-process path confinement.

### `sandbox/process-enforced`

A real operating-system boundary constrains the launched process tree for the capability being claimed. This requires native, executable proof of the boundary rather than an adapter declaring that it is protected.

Decision [`0060`](../decisions/0060-optional-sandbox-process-enforced-reference-confinement.md) establishes optional Linux Landlock as the first reference path for this higher class. Platform support and proof remain separate from the portable policy semantics.

Higher assurance classes consume RepoPact authority. They do not become a second work ledger, and they may narrow authority but may not widen it.

## Verification is not admission

Repository-defined verification answers questions such as "did the required checks run successfully on this host?"

Admission answers a different question: "may this session or action begin under the represented authority?"

A local verification pass is useful evidence. It is not proof that a remote merge gate, protected guard, or process-confinement boundary exists or is effective.

See [Local-first verification and release](guides/local-ci-cd.md) and [Pre-execution admission](guides/pre-execution-admission.md).

## The Rust engine and clients

RepoPact's canonical semantic authority is the Rust engine and its versioned language-neutral protocol for migrated product surfaces. The retained Python layer is a compatibility, packaging, and comparator surface rather than a second semantic authority.

Workbench and other clients should consume the canonical semantic operations instead of reimplementing repository meaning in each UI or integration.

## The Repository Orientation Graph is derived

The Repository Orientation Graph can preserve deterministic, incremental semantic and metadata relationships and expose bounded orientation queries. It is still a projection.

ROG state cannot authorize a frozen mutation, activate work, change ownership, create a binding invariant, or grant execution authority. If graph state conflicts with authoritative repository state, the graph must be rebuilt or reconciled.

See [Repository Orientation Graph](repository-orientation-graph.md).

## Stable product, active work, and research are different claims

A repository can simultaneously contain:

- capabilities in the stable package;
- newer or broader implementation on `main`;
- integration-dependent reference guarantees;
- active, blocked, deferred, or proposed work;
- research infrastructure whose comparative results are still pending.

RepoPact documentation keeps those categories explicit. Code existing in the repository is not, by itself, evidence of stable packaging, complete cross-platform support, production readiness, or benchmark-proven benefit.
