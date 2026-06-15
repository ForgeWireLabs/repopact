# Concepts

*Diataxis mode: explanation (understanding-oriented).*

## The problem

Agent and human work increasingly starts in conversations whose state evaporates
when the session ends. The next contributor — human or agent — cannot recover what
was decided, what is binding, or what was actually proven. RepoPact's answer: put
all of it in versioned files, and make the binding parts enforceable.

## Why "pact"

A README is advice. A pact has teeth. The distinguishing primitive is the
**binding invariant**: a guarantee an agent may not weaken without flagging it and
getting explicit human approval. See decision
[`0001`](../decisions/0001-repository-as-pact.md).

## Principles vs invariants

The charter separates the two deliberately:

- A **principle** is a value you weigh (e.g. "drift is a defect"). Upheld by
  judgment.
- An **invariant** is a line you do not cross without approval (e.g. "a completed
  work item has no pending criteria"). Upheld by a machine check or a named
  escalation.

This separation is the pact. See [`governance/charter.md`](../governance/charter.md).

## Derive over declare

Anything computable from source records is generated, not hand-maintained — the
dashboard, the SPEC catalog, audit freshness. The lesson behind it (policy
[`001`](../governance/policies/001-derived-artifacts-are-generated.md)) is a scar:
hand-maintained mirrors of derivable state rot faster than humans reconcile them,
and the rot then masquerades as drift.

## One truth per record

Each record type owns one kind of truth: invariants bind, decisions record choices,
policies state durable rules, work items track units of work, evidence proves,
findings observe drift, the dashboard derives. See
[`governance/record-types.md`](../governance/record-types.md).

## Completion requires proof

A work item is not done because someone is confident; it is done when its
acceptance criteria are satisfied with linked evidence. The validator enforces
this. Confidence is not evidence.
