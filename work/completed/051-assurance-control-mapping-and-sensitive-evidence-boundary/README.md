# Work Item 051 — Assurance Control Mapping and Sensitive Evidence Boundary

**Status:** Completed

## Intent

Extend RepoPact's repository-native governance model so adopters can map
implemented controls to assurance/regulatory frameworks and preserve reviewable
evidence **without** turning RepoPact into a certification engine, legal expert,
or storage location for regulated production payloads.

RepoPact already models authority, invariants, work, evidence, provenance,
decisions, drift, and review. Those primitives are a strong substrate for
continuous assurance, but the distinction between a control, evidence that the
control operated, a framework requirement, an applicability conclusion, and an
independent attestation must remain explicit.

## Core boundary

RepoPact may say, for example:

- an adopter declares a control;
- the control maps to one or more external framework requirements;
- executable/review evidence supports the control for a particular scope and
  period;
- a gap, exception, or stale review remains open; and
- an external auditor/authority conclusion is referenced when one exists.

RepoPact must **not** infer from those facts that an organization is SOC 2
certified, HIPAA compliant, PCI DSS compliant, GDPR compliant, or legally
compliant with another regime.

## Sensitive-evidence rule

Compliance evidence often arises from systems that process PHI, cardholder data,
financial identity/KYC records, credentials, education records, personal data,
or confidential customer material. Git-backed RepoPact evidence should preserve
proof of the control while excluding the underlying regulated payload whenever
possible.

Preferred evidence forms include:

- hashes/digests;
- counts and bounded metadata;
- redacted fixtures;
- synthetic negative/positive tests;
- immutable run identifiers;
- external evidence references with access-control metadata;
- policy/configuration snapshots that contain no secrets; and
- auditor/operator attestations that identify the reviewed control without
  copying customer data.

Raw patient records, payment-card data, passports, customer secrets, message
bodies, access tokens, or equivalent production payloads are not acceptable
evidence merely because Git is private.

**Status (phase 2, Decision 0055):** implemented as bounded, scoped
guardrail diagnostics -- never a whole-repository scan, never a data
classifier. RepoPact inspects only content an assurance mapping already
references (`evidence_refs[kind=repository_artifact]` up to 256 KiB,
`evidence_refs[kind=external_reference].external.reference`), and reports
what pattern was observed, never a legal/compliance conclusion: private-key
markers and credential-bearing URIs are `error`; secret-assignment,
Luhn-valid card-number, and health/identity label shapes, and a
`restricted`-declared repository artifact, are `warning`; binary/oversize/
unreadable artifacts are `info` coverage notes. See ACM-004 in
`work-item.json`.

## Framework mapping model

Investigate a provider-neutral mapping record or template that can express:

- framework and version/source authority;
- requirement/control identifier;
- applicability status and rationale;
- adopter control/invariant/policy owner;
- implementation references;
- evidence references and evidence sensitivity;
- customer/operator responsibilities;
- third-party/subprocessor dependencies;
- gaps/exceptions/compensating controls;
- review date and freshness deadline;
- independent audit/certification reference, if any; and
- explicit `not assessed` / `requires authority review` states.

Framework definitions remain adopter-owned data or extensions. RepoPact should
not hard-code legal interpretations of HIPAA, PCI DSS, SOC 2, GDPR, KYC/AML,
FERPA, COPPA, FDA rules, or other regimes into its governance kernel.

## Drift and claim safety

The existing provenance and semantic-review machinery should be usable to flag:

- mappings whose external framework version changed;
- stale control reviews;
- evidence that no longer corresponds to current source/configuration;
- implementation changes that invalidate a control mapping; and
- documentation that makes a stronger claim than the stored evidence supports.

**Status (phase 2, Decision 0055):** implemented. `review.snapshot` records a
deterministic SHA-256 digest of the mapping's canonical review projection
plus per-reference digests for locally resolvable control/implementation/
evidence/documentation references; `repopact validate` recomputes and
compares them, distinguishing mapping drift, reference drift, and a
missing/deleted reference. Framework-version and review-due staleness are
computed from the existing date fields. `documentation_refs[].claim_basis`
is validated against what the mapping actually stores, catching a
documentation claim with no corresponding support. See ACM-004/ACM-005 in
`work-item.json` and the second checkpoint evidence.

## ForgeWire ecosystem use

ForgeWire WI248/WI263 are a first adopter/use case, not special cases baked into
the RepoPact core. The result must remain useful for unrelated repositories and
organizations.

## Non-goals

- issuing audit opinions or certifications;
- deciding whether a law applies to an adopter;
- bundling copyrighted standards text into RepoPact;
- storing production regulated data in Git evidence;
- replacing GRC systems, auditors, lawyers, or compliance authorities; or
- making SOC 2/HIPAA/PCI-specific logic mandatory for ordinary RepoPact users.

## Architecture review — 2026-09-14 (phase 2: diagnostics, drift, claim safety, adopter proof)

**Coding agent:** Claude Code. **Architecture reviewer:** GPT-5.6 Sol High.

Phase 1 (foundation checkpoint, evidence
`20260914-051-assurance-mapping-foundation-checkpoint`, accepted at commit
`4f85b5e`) established the record, its schema, and its authority boundary
(Decision 0054). Phase 2 adds sensitive-evidence guardrail diagnostics
(ACM-004), review/drift semantics (ACM-005), and an adopter proof
(ACM-007), governed by Decision 0055. Before writing any of that code,
this review restates the four distinctions the phase-2 architecture must
keep impossible to blur, because every new diagnostic in this phase is a
place that distinction could quietly erode:

- **An assurance mapping's existence is not certification.** A bounded
  guardrail diagnostic, a computed review state, or a resolved reference
  is a fact RepoPact observed about the record; none of them is, or
  contributes toward, a claim that an external framework requirement is
  satisfied or that an organization is compliant. Diagnostic wording is
  reviewed against this directly (Decision 0055, severity contract).
- **An evidence reference is not the raw evidence payload.** ACM-004's
  guardrails inspect only content an adopter already chose to commit
  behind an explicit `evidence_refs[kind=repository_artifact]`
  reference (or the reference metadata itself); they never read,
  recurse into, or classify arbitrary repository content, and they
  never turn evidence into a copy of a regulated record.
- **A warning signal is not a legal or data-protection classification.**
  Every ACM-004 detector is a bounded, false-positive-prone heuristic.
  Its diagnostic wording says what pattern was observed ("potential
  cardholder-data-shaped evidence"), never what law or standard it
  implicates ("PCI violation"). Only a small, explicitly justified set
  of structurally unambiguous cases (private-key markers, credential
  syntax in a URI, a path-containment violation) may use `Error`
  severity; everything else is `Warning` or `Info`.
- **A stale mapping is not an invalid JSON record.** ACM-005's
  freshness/drift states (`unreviewed`, `current`, `overdue`,
  `drifted`, ...) are diagnostics about the *review*, computed from
  deterministic local state and never persisted as canonical truth
  where they can be recomputed. A mapping with a fully overdue or
  drifted review remains schema-valid and remains a legitimate
  RepoPact record; drift is reported at `Warning`/`Info`, never as a
  schema rejection.

This phase's implementation is bound by Decision 0055, which also
restates the pre-existing authority split this phase must not weaken:
`repopact validate`'s only canonical implementation is the compiled Rust
engine (work/completed/056); `repopact/validate_repo.py` remains the
retained regression comparator. Every new diagnostic in this phase lands
in `repopact-validation` first; the Python mirror exists to keep the
conformance/regression suite meaningful, not as an independent source of
truth.
