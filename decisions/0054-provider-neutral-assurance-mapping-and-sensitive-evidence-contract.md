---
id: 0054
title: Provider-neutral assurance mapping and sensitive-evidence contract
status: accepted
date: 2026-09-14
supersedes: []
---

# 0054: Provider-neutral assurance mapping and sensitive-evidence contract

## Context

WI051 asks RepoPact to let an adopter map an implemented control to an
external assurance/regulatory framework requirement, and to preserve
reviewable evidence for that mapping, without RepoPact becoming a
certification engine, a data-loss-prevention product, or a storage
location for regulated production payloads. RepoPact already models
authority, invariants, work, evidence, provenance, decisions, and
review; the risk is conflating five facts that must stay distinct: an
implemented control, evidence that the control operated, a framework's
stated requirement, an adopter's applicability conclusion, and an
independent auditor's attestation. This decision settles the record's
placement, shape, and authority boundary before any schema or validator
code is treated as final.

Architecture discovery (before this decision) found:

- Canonical JSON schemas live in `repopact/schemas/*.schema.json` and are
  embedded byte-for-byte into the Rust engine (`repopact-schema`,
  `EMBEDDED_SCHEMAS`, with a test asserting the embedded copy matches the
  checked-in file). `repopact validate` runs exclusively against the
  compiled Rust engine (`repopact-engine`) via `engine_client.py`'s
  fail-closed protocol; `repopact/validate_repo.py` is retained as an
  explicit regression comparator (`repopact/legacy_validate.py`, used by
  `doctor`/`adopt`/`init`/`import-plan`/`takeover` and by
  `tests/test_conformance.py`) but is not itself product authority
  (work/completed/056, `command-authority-inventory.md`). A new record
  family is therefore validated in **both** places, mirroring the
  existing evidence-run pattern in each: JSON-Schema structural check,
  then hand-written cross-reference checks.
- Record discovery is convention-based (fixed directory + filename
  pattern), never a manifest; there is no dispatch-by-`$id` registry in
  either language, only an ordered list of `validate_*` calls.
- Lifecycle transition (`proposed` → `active`) is `git mv` + a status-field
  edit, validated after the fact by a directory/status-match check; there
  is no CLI "activate" verb, by design.
- `RecordKind` (`repopact-types`) is a closed Rust enum used only for
  repository-index/record-reference identity (`RecordIndex`,
  `RecordRef`); it is unrelated to the durable, schema-versioned
  Repository Orientation Graph (`GraphNodeKind`/`GraphEdgeKind`,
  Decision 0045), so adding a variant carries none of that decision's
  same-major compatibility hazard.
- `governance/record-types.md`'s own placement heuristic (decision =
  point-in-time choice; policy = continuous rule, no escalation gate;
  invariant = continuous binding rule) does not fit an assurance mapping:
  it is many independently addressable, individually identified
  instances with their own applicability/review state, which is the same
  shape as evidence-run and work-item, not decision/policy/invariant.

## Decision

### 1. Canonical record placement

A **dedicated, optional, versioned JSON record family** at
`assurance/mappings/<id>.json`, one file per mapping, schema
`repopact/schemas/assurance-mapping.schema.json`. Absence of the
`assurance/` directory is a fully valid RepoPact repository (same
contract as `evidence/runs/` conceptually being present, except this
family may be entirely absent with no seed requirement).

### 2. Record identity/versioning

`id` is an adopter-chosen opaque slug (`^[a-z0-9]([a-z0-9-]*[a-z0-9])?$`)
that must equal the filename stem; duplicates across the directory are
rejected. A top-level `version` field (`const: 1` today) is the record's
own schema-version contract, independent of the overall RepoPact
specification version — an unrecognized future major version must be
reported as unsupported, never interpreted optimistically (mirroring
`rog-capability.schema.json`/`verification-profile.schema.json`'s
`version` convention, not the whole-specification `SPEC.md` version).

### 3. Framework/source-authority semantics

`framework.id`, `framework.version`, `framework.source_authority`, and
`framework.source_ref` are open, adopter-owned strings. RepoPact defines
no enum of named regimes (no `SOC2`/`HIPAA`/`PCI_DSS`/`GDPR`/`KYC`/`AML`
constants anywhere in the schema or validators). `source_authority`
records *where the requirement definition came from* (e.g.
"official-publication", "customer-contract"); RepoPact never evaluates
whether that source is legally controlling.

### 4. Requirement identity

`requirement.id` is a non-empty open string; RepoPact validates presence
only, never a framework-specific identifier grammar (`CC6.1`,
`164.312(a)(1)`, and `CUSTOM-AUTH-04` are all equally valid).

### 5. Applicability semantics

`applicability.status` is a closed, adopter-facing enum:
`unassessed | applicable | not_applicable | conditional | partial`
(house style: short, lowercase, closed, terminal-state-bearing, matching
work-item/decision/policy/evidence enum conventions surveyed before
choosing it; no existing enum already fit, so this one is new but
consistent). Any status other than `unassessed` requires non-empty
`rationale` and `determined_by`, enforced by both validators (not by pure
JSON Schema, matching decision 0003's "schema is authoritative for
shape, validator functions are authoritative for cross-record semantics"
split). A `not_applicable` conclusion is stored exactly as an *adopter
assertion with provenance*, never as RepoPact-derived legal advice.

### 6. Adopter-control references

`control_refs[]` reference RepoPact's own canonical governance records —
`policy`, `invariant`, `decision`, `work_item`, `contract` (an
`AGENTS.md` repository-relative path), or `role` (an adopter-owned scope
name) — rather than duplicating control text RepoPact already owns.
`policy`/`invariant`/`decision`/`work_item` references are resolved
against the same id sets the rest of validation already computes
(decisions/policies/invariants/work items discovered by the existing
record index); a dangling reference is rejected by both engines with an
`assurance.unknown-<kind>` diagnostic. `contract` references are resolved
through the existing repository-relative containment check
(`resolve_within_root` / `_resolve_repo_relative`, mirroring the WI059
containment idiom) and must exist as a file.

### 7. Implementation references

`implementation_refs[]` support `source`, `configuration`, `workflow`,
`test`, `runtime_surface` (all repository-relative paths, containment-
and existence-checked the same way as `contract` control references) and
`decision`/`work_item` (canonical id references). No implementation
reference field accepts an absolute host path or requires source
ingestion into the record; the reference is a pointer, never a copy.

### 8. Evidence references

`evidence_refs[]` is a closed union by `kind`:
`evidence_run` (a canonical `evidence/runs/<id>.json` id, cross-
referenced against the existing evidence-id set), `hash` (algorithm +
digest + optional size/count/content-descriptor), `repository_artifact`
(a repository-relative, containment- and existence-checked path to an
already-committed bounded/redacted/synthetic file), or
`external_reference` (system + identifier + optional reference/date/
digest, structurally forbidden from embedding credentials by having no
credential field at all). **No evidence-reference kind has a field
capable of carrying an inline raw payload** — this is enforced
structurally (`additionalProperties: false`, no free-form content/body
property in any branch), not by content scanning. Every evidence
reference also carries an explicit, adopter-declared `sensitivity`
(`ordinary | sensitive | restricted`) — required, never inferred by
RepoPact.

### 9. Responsibilities

`responsibility[]` is a small structured list (`role`:
`adopter_owned | shared | third_party_dependent`, optional free-text
`description`, optional `owner_ref` into RepoPact's owner/role scopes),
not a single global enum and not cloud-shared-responsibility-specific
vocabulary.

### 10. Third-party dependencies

`third_party_dependencies[]` are generic (`name`, free-text
`description`, nested `evidence_refs[]` reusing the same closed evidence-
reference union). Naming a dependency never implies that dependency is
itself compliant; nothing in the schema lets an adopter assert that.

### 11. Gaps/exceptions

`gaps[]` (`known_gap | accepted_exception | planned_remediation |
not_yet_evidenced | external_dependency`, each with a free-text
`description` and optional dates/`owner_ref`) are structurally distinct
from evidence and from applicability. A record with an open gap is a
valid RepoPact record; record validity never implies satisfaction of the
external requirement.

### 12. Review freshness

`review` (`reviewed_at`, `reviewed_by`, `review_due_at` or
`review_interval_days`, `framework_version_reviewed`) stores the fields a
future drift/staleness engine (ACM-005) needs. This checkpoint stores and
schema-validates the fields; it implements no staleness computation, no
`repopact doctor` diagnostic, and no dashboard surface for them yet.

### 13. Independent attestation

`attestation` (`issuer`, `type`, `date`, `reference`) is optional and
kept structurally separate from `evidence_refs`. Its presence means only
that the adopter recorded a pointer to an external attestation; RepoPact
never independently verifies it and never treats its presence as
RepoPact-issued certification.

### 14. Sensitive-evidence handling

The schema has no field, in any branch of any object, that can hold an
arbitrary raw byte/text payload. This is the entire enforcement
mechanism for this checkpoint's sensitive-evidence boundary (ACM-003):
not a classifier, not a regex scanner, but the *absence of a place to
put the payload*. A `repository_artifact` reference still points at a
file the adopter chooses to commit — RepoPact documents (schema
description, SPEC.md, this decision) that such a file must be bounded,
redacted, or synthetic, and never a raw regulated/customer payload, but
does not and cannot verify that claim about arbitrary file content. That
verification (ACM-004: pattern-based guardrails for obvious hazards —
PEM markers, credential-shaped strings, checksum-valid card-number-shaped
digits, disallowed absolute paths, credential-bearing URIs) is
explicitly deferred to the next WI051 checkpoint; this schema is
designed so adding it later does not require a breaking schema change.

### 15. Backward compatibility

Every change in this checkpoint is additive: a new optional schema, a
new optional discovery path, a new optional validator that returns
immediately when `assurance/mappings/` does not exist, a new optional
`RecordIndex` field, a new `RecordKind` enum variant used only for
repository-index identity (not the durable graph). No existing schema,
validator, evidence-run semantics, or `provenance` semantics changes.
Repositories without assurance mappings, and all pre-existing evidence
runs, remain valid and unmigrated.

### 16. Kernel/legal-authority boundary

RepoPact records that a mapping, a control reference, an evidence
reference, a gap, or an attestation reference *exists*. It never infers
from that presence that a control *operated*, that a framework
requirement is *satisfied*, that an organization is *legally compliant*,
or that RepoPact itself *certifies* or *independently audits* anything.
No field, validator rule, or generated doc claims otherwise; SPEC.md's
new §4 rule 15 states this explicitly, and `governance/record-types.md`
records "certification/legal compliance conclusions" and "raw regulated
evidence payloads" as things this record type must not own.

## Alternatives considered

### A. Extend `evidence-run`

Rejected. Evidence-run is deliberately execution-oriented (`id`,
`timestamp`, `work_item`, `result`, `provenance`, `commands`,
`artifacts`, `environment`) and is the leaf record every other governed
artifact references, never the other way around. Overloading it to also
carry framework/applicability/responsibility/gap semantics would make
"evidence exists" and "framework requirement mapped" the same field,
directly contradicting the ACM-002 requirement that these stay distinct
states, and would force every existing evidence-run consumer (dashboard,
`work_ids` cross-reference, timestamp-vs-commit chronology check) to
reason about an unrelated concern.

### B. Extend policy/contract records

Rejected. A policy is "a continuous operating rule with no escalation
gate" (`record-types.md`) — a single adopter-owned statement, not a
collection of individually applicable-or-not, evidenced-or-not framework
requirement instances. A contract (`AGENTS.md`) has no id, no status
field, and no lifecycle at all; it is the loosest, most narrative record
type RepoPact has. Folding assurance mappings into either would conflate
"RepoPact's own operating rule" or "authority narrative" with "an
external party's requirement text and an adopter's conclusion about it"
— exactly the adopter-policy-authority-vs-external-framework-mapping
conflation this decision must avoid (WI051 §5).

### C. Dedicated assurance/control-mapping record

**Selected.** Matches the evidence-run/work-item shape (many
independently identified, individually addressable instances, each with
its own lifecycle-adjacent state) that the housekeeping heuristic in
`record-types.md` already uses to distinguish record families. Requires
no new ontology beyond one schema, one discovery convention, and one
`RecordKind` variant used purely for index identity.

### D. Separate framework + mapping records

Considered and rejected for this checkpoint. Splitting `framework`
identity into its own record family (so multiple mappings could share
one `framework` record by reference) is a legitimate future
normalization if adopters accumulate many mappings against the same
framework, but it is premature ontology today: WI051's ACM-001..003
foundation does not require cross-mapping framework deduplication, no
adopter fixture yet demonstrates the need, and the inline
`framework: {...}` object can be extracted into a referenced record
later as a strictly additive schema change (the object shape does not
have to change, only its location) if evidence emerges that it is
needed.

## Consequences

- `assurance/mappings/*.json` is validated by both engines
  (`repopact/validate_repo.py:validate_assurance_mappings`,
  `repopact_validation::Validator::validate_assurance_mappings`) using
  the same schema, the same diagnostic wording, and the same
  cross-reference semantics, so `repopact validate`'s canonical Rust
  path and the Python regression comparator agree.
- The record can grow ACM-004 (sensitive-evidence guardrail
  diagnostics), ACM-005 (freshness/drift), and ACM-007 (a full adopter
  conformance proof, e.g. a ForgeWire-shaped fixture) without a breaking
  schema change, because every field those need already exists
  (`sensitivity`, `review.*`, the closed evidence-reference union).
- Graph integration (WI054/063) is deliberately not touched beyond the
  `RecordKind::AssuranceMapping` variant needed for repository-index
  identity: no `GraphNodeKind`/`GraphEdgeKind` variant, no
  `repopact-graph` or `repopact-desktop-api` wiring. If assurance
  mappings need graph visibility later, they extend the existing
  canonical graph rather than acquiring a second governance authority.

## Alternatives considered for the sensitive-evidence boundary

- **A closed `evidence_storage_mode` enum** (`repository_metadata |
  hash_only | redacted | synthetic | controlled_external`) as a separate
  field from `kind`. Rejected as redundant: `evidence_refs[].kind`
  already IS the handling-mode field (each kind maps 1:1 to a handling
  strategy); adding a second field describing the same fact invites the
  two to disagree.
- **Content-classification fields** (`contains_phi: bool`,
  `pci_scope: bool`, etc.). Rejected outright — RepoPact cannot reliably
  determine legal data classification, and a field inviting an adopter
  to assert one would misrepresent an unverifiable claim as
  RepoPact-checked fact (WI051 §21, non-goal: "issuing audit opinions").
