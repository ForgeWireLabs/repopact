# Conformance fixtures

The corpus referenced by [`SPEC.md`](../../SPEC.md) §9. A RepoPact-conformant
implementation must **accept** the valid fixture and **reject** each invalid one.

Fixtures deliberately do **not** vendor `schemas/`. `tests/test_conformance.py`
injects the canonical `schemas/` from the repository root before validating, so a
fixture can never pass against a stale schema copy.

## valid/

A minimal, self-contained valid repository: a root contract, one binding
invariant, one frozen-surface entry, a scope/owner map, an audit registry, and one
completed work item with a satisfied criterion linked to evidence.

## invalid/

Each directory is an **overlay** on `valid/`: only the file(s) that introduce the
violation, plus a `meta.json` declaring the expected rejection message. The test
copies `valid/`, injects schemas, applies the overlay, and asserts the message
appears.

| Fixture | Rule (SPEC §4) | Expected message |
| --- | --- | --- |
| `completed-with-pending` | Acceptance | `completed item has pending criterion` |
| `satisfied-without-evidence` | Acceptance | `satisfied without evidence` |
| `unknown-dependency` | Reference resolution | `unknown dependency` |
| `unregistered-contract` | Contract coverage | `not registered in audits/registry.json` |
| `dependency-cycle` | Acyclic dependencies | `dependency cycle` |

To add a rule to the corpus: create `invalid/<name>/` with the violating file(s)
and a `meta.json`; the test picks it up automatically.
