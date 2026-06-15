# Charter

## Purpose

Build systems that remain understandable, operable, and changeable when people,
agents, tools, and implementation details change.

## Principles (human judgment)

Principles orient decisions. They are not mechanically checkable; they require
judgment and are upheld by review.

1. **Systems before sessions**: no critical state exists only in chat context.
2. **Authority is local**: constraints and validation live near governed code.
3. **One kind of truth per record**: plans, execution logs, and audits are distinct.
4. **Completion requires proof**: confidence is not evidence.
5. **History is an asset**: rejected alternatives and failures remain discoverable.
6. **Drift is a defect**: disagreement between architecture and reality is tracked.
7. **Degradation is explicit**: blocked and deferred are legitimate states, not silence.
8. **Derive over declare**: anything computable from source records is generated,
   not hand-maintained (see policy `001`).

## Invariants (binding, with escalation)

Invariants are the *pact*. They are guarantees that an agent may not silently
weaken. Each is declared in [`invariants.json`](invariants.json) with a rationale,
an escalation path, and — where possible — a machine enforcer. A request that would
weaken any invariant must be flagged and confirmed with the human operator before
it is implemented, regardless of which role is active.

The distinction from principles is deliberate: a principle is a value you weigh; an
invariant is a line you do not cross without explicit approval. See decision
[`0001-repository-as-pact`](../decisions/0001-repository-as-pact.md).

## Non-goals

- Replacing source control, issue trackers, or CI providers.
- Encoding every judgment in automation.
- Producing process artifacts that do not improve decisions or recoverability.
