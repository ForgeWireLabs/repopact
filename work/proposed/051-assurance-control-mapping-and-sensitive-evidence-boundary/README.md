# Work Item 051 — Assurance Control Mapping and Sensitive Evidence Boundary

**Status:** Proposed

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
