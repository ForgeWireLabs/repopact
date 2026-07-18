# Conformance fixtures

The corpus referenced by [`SPEC.md`](../../SPEC.md) §9. A RepoPact-conformant
implementation must **accept** each `valid*` fixture and **reject** each invalid one.

Fixtures deliberately do **not** vendor `schemas/`. `scripts/run_conformance.py`
injects the canonical `schemas/` from the repository root before validating, so a
fixture can never pass against a stale schema copy.

## valid fixtures

`valid/` is a minimal, self-contained repository: a root contract, one binding
invariant, one frozen-surface entry, a scope/owner map, an audit registry, and one
completed work item with a satisfied criterion linked to evidence.

`valid-proposed/` and `valid-provisional/` are overlays proving those intermediate
states are admitted without granting completion authority.

## invalid/

Each directory is an **overlay** on `valid/`: only the file(s) that introduce the
violation, plus a `meta.json` declaring the expected rejection message. The runner
copies `valid/`, injects schemas, applies the overlay, and requires exactly one
reference violation containing the declared message. It canonically regenerates
the dashboard unless a dashboard fixture explicitly selects `preserve` or `remove`.

| Fixture | Rule (SPEC §4) | Expected message |
| --- | --- | --- |
| `active-depends-on-proposed` | Reference resolution / lifecycle authority | `depends on proposed work item` |
| `completed-with-pending` | Acceptance | `completed item has pending criterion` |
| `satisfied-without-evidence` | Acceptance | `satisfied without evidence` |
| `unknown-dependency` | Reference resolution | `unknown dependency` |
| `unregistered-contract` | Contract coverage | `not registered in audits/registry.json` |
| `dependency-cycle` | Acyclic dependencies | `dependency cycle` |
| `completed-provisional-work` | Provenance | `cannot be completed; ratchet to concrete first (P2)` |
| `status-directory-mismatch` | Identity agreement | `status 'active' does not match directory 'completed'` |
| `invalid-semantic-version` | Version | `must be semantic (MAJOR.MINOR.PATCH)` |
| `schema-invalid-evidence` | Record validity | `schema result: 'unknown' is not one of` |
| `orphan-work-directory` | Ledger visibility | `work directory holds planning content` |
| `disjoint-active-scopes` | Optional concurrency | `active scope conflict` |
| `missing-dashboard` | Dashboard integrity / INV-7 | `missing generated dashboard` |
| `stale-dashboard` | Dashboard integrity / INV-7 | `dashboard is stale` |

To add a rule to the corpus: create `invalid/<name>/` with the violating file(s),
add the rule inventory and case mapping to `manifest.json`, and keep `meta.json` in
sync for fixture readability. Tests fail if a rule is uncovered, a case names an
unknown rule, a fixture directory is undeclared, or a reject fixture produces a
secondary violation.
