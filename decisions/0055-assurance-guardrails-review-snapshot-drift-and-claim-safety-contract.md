---
id: 0055
title: Assurance guardrails, review snapshot, drift, and claim-safety contract
status: accepted
date: 2026-09-14
supersedes: []
---

# 0055: Assurance guardrails, review snapshot, drift, and claim-safety contract

## Context

Decision 0054 established the assurance/control-mapping record and its
authority boundary but deliberately deferred three acceptance criteria:
ACM-004 (sensitive-evidence guardrail diagnostics), ACM-005 (review
freshness and drift), and ACM-007 (a full adopter proof). This decision
settles the architecture for all three before implementation, and
restates the boundary this phase must not blur: an assurance mapping's
existence is not certification; an evidence reference is not the raw
evidence payload; a warning signal is not a legal or data-protection
classification; a stale mapping is not an invalid JSON record (see the
WI051 README's 2026-09-14 phase-2 architecture-review section, reviewed
by GPT-5.6 Sol High).

`repopact validate`'s only canonical implementation remains the compiled
Rust engine (work/completed/056); `repopact/validate_repo.py` remains the
retained regression comparator. Every diagnostic and computation this
decision introduces lands in `repopact-validation` first; Python mirrors
it for conformance/regression parity, not as an independent authority.

## Decision

### 1. Diagnostic scan scope

Sensitive-evidence diagnostics (ACM-004) are scoped **only** to content an
assurance mapping already explicitly references:
`evidence_refs[kind=repository_artifact]` file content, and
`evidence_refs[kind=external_reference].external.reference` /
`third_party_dependencies[].evidence_refs[...]` metadata strings. RepoPact
never walks the source tree, `.git`, or any file an assurance mapping does
not name. RepoPact is not a data-loss-prevention product; it inspects
exactly what an adopter chose to declare as evidence for exactly the
mapping that declared it.

### 2. Severity contract

The engine's existing `Error | Warning | Info` severities (already
protocol-level: `valid = error_count == 0`, `engine_client.render_validation`
now prints the real severity) are used deliberately:

- **Error** only for cases where the reference itself violates an
  explicit RepoPact contract with negligible false-positive risk:
  private-key PEM markers in a declared repository-artifact evidence
  file, a repository-relative reference that escapes the repository, and
  a syntactically unambiguous credential-bearing external URI
  (`scheme://user:password@host`).
- **Warning** for potential sensitive-data indicators RepoPact cannot
  classify conclusively: generic secret-assignment-shaped text, a
  Luhn-valid card-number-shaped digit sequence, high-signal health/
  identity/financial structural labels, a secret-shaped query parameter,
  a `restricted`-declared `repository_artifact` evidence reference, and
  every ACM-005 drift/staleness state.
- **Info** for coverage limitations: an artifact too large or too binary
  to inspect, or an external/attested reference RepoPact never fetched.

No diagnostic message names a legal regime or compliance conclusion
("PCI violation", "HIPAA violation", "certified"). Diagnostic wording is
reviewed against this rule directly, not left to detector-author
discretion (WI051 §7).

### 3. Bounded artifact inspection

A referenced `repository_artifact` is read only up to **256 KiB**. There
is no prior byte-size convention elsewhere in RepoPact to align to (the
codebase's only existing size constant, `PLAN_LIMIT`, bounds an unrelated
in-memory list, not file content); 256 KiB is a new, deliberately chosen
bound: generous headroom over any legitimate bounded/redacted/synthetic
evidence artifact (Decision 0054 already required evidence to be
"bounded metadata," so a genuinely bounded artifact is expected to be far
smaller), while small enough that the validator can never become an
unbounded content scanner regardless of what an adopter commits. Content
that decodes as valid UTF-8 and contains no NUL byte in the read window
is treated as text and scanned; anything else (decode failure, embedded
NUL, oversize, unreadable) is skipped with an `Info` coverage diagnostic
naming which condition applied. No detector reads more than one bounded
window per referenced artifact, and no detector is applied to unreferenced
files.

### 4. Review-freshness derivation without a snapshot

`review.reviewed_at` / `review_due_at` / `review_interval_days` /
`framework_version_reviewed` (Decision 0054 §12) already support two
purely date-based facts without any new schema: whether a review is
`unreviewed`, `current`, or `overdue` (explicit `review_due_at` wins over
a computed `reviewed_at + review_interval_days`; with neither, there is
no automatic time-expiry rule), and whether `framework.version` disagrees
with `framework_version_reviewed`. Both are deterministic local
comparisons; `now` is always injectable in tests and never taken as
ambient authority for anything computed inside a snapshot itself.

### 5. Deterministic reference snapshots

Date-based freshness cannot prove a reviewed reference has not changed
since review. `review.snapshot` (additive, schema stays version 1) adds:

- `mapping_digest`: SHA-256 of the mapping's **canonical review
  projection** -- the full mapping JSON object with `review.snapshot`,
  `created`, and `updated` removed (bookkeeping timestamps are not
  review-relevant semantics; every other field, including `notes`,
  participates), object keys recursively sorted, serialized compactly.
  `serde_json`'s workspace-wide `preserve_order` feature means `Value`
  objects are insertion-ordered, not alphabetical, so both engines
  implement an explicit recursive key-sort before serializing; arrays
  are never reordered (their order is semantic content, not
  incidental). Rust and Python are each internally deterministic and
  reproducible across repeated runs and across OS (proven by test); they
  are not required to produce byte-identical digests to each other for
  exotic Unicode/number edge cases, since nothing in this decision's
  acceptance criteria depends on cross-language digest equality, only on
  each engine's own determinism.
- `references[]`: one entry per **locally resolvable** control,
  implementation, evidence, and documentation reference at snapshot
  time, each carrying its own current-content digest: canonical
  RepoPact records (decision/policy/invariant/work-item) are hashed by
  their own source file's bytes; repository-relative paths
  (implementation refs, `repository_artifact` evidence, documentation
  refs) are hashed by file content via the same digest primitive
  `Repository::path_state` already uses (`repopact-repository`,
  WI054/059); a missing/unresolvable reference records `"absent"` or
  `"unresolvable"` rather than a digest, so a delete is distinguishable
  from a change.

### 6. External references are never fetched

`external_reference`, `attestation.reference`, and `framework.source_ref`
are never dereferenced over the network during validation or snapshot
computation, at any point in this phase. Their adopter-supplied metadata
(system, identifier, digest, date) participates in `mapping_digest` like
any other field; an adopter-supplied external digest is retained
verbatim, never independently verified. RepoPact does not call cloud
APIs to decide assurance freshness, matching WI046's local-primary
verification stance.

### 7. Drift classes

Given a stored `review.snapshot`, recomputing the current snapshot and
comparing yields four distinct, separately diagnosed conditions, all
`Warning` severity (drift is a review-staleness fact, never a schema
violation):

- **Mapping drift** (`assurance.review-mapping-drift`): the current
  canonical projection's digest differs from `mapping_digest`. Catches
  any semantic change (applicability, framework identity/version,
  control refs, gaps, responsibility, third-party dependencies,
  evidence metadata, attestation metadata) without interpreting whether
  the change is legally material.
- **Reference drift** (`assurance.review-reference-drift`): a
  previously-`references[]`-recorded item's current digest differs from
  its stored digest. Reports mapping id, category, kind, ref, reviewed
  digest, and current digest -- never source content.
- **Reference missing** (`assurance.review-reference-missing`): a
  previously-resolvable reference is now absent/unresolvable. Reported
  distinctly from drift (a delete is not a content change) and distinctly
  from "unresolvable at snapshot time" (which the snapshot itself already
  recorded as `"unresolvable"` rather than silently omitting).
- **Stale framework review** (`assurance.review-framework-version-stale`):
  `framework.version` differs from `framework_version_reviewed`, exactly
  as in §4, still emitted even when a snapshot exists.

### 8. Documentation claim-basis model

ACM-005's "documentation claims stronger than stored evidence" clause is
implemented as an explicit, deterministic, structural check -- never
natural-language interpretation of prose. `documentation_refs[]`
(additive) declares a repository-relative path and a `claim_basis` drawn
from a closed vocabulary (`mapping | control_reference |
implementation_reference | evidence_reference |
external_attestation_reference`), each requiring the corresponding array/
object to be genuinely non-empty on the same record
(`assurance.documentation-claim-basis-unsupported` when it is not).
There is deliberately no `compliant`/`certified`/`legally_satisfied`
basis value: the check validates that a claimed *kind of support* exists,
never that a claim is legally true. Documentation-ref paths participate
in the review snapshot under `category: "documentation"`, so an edited
documentation file surfaces as reference drift even when implementation
and evidence are unchanged.

A second, narrower, explicitly optional and advisory mechanism -- a
bounded keyword heuristic over the *declared* `documentation_refs` files
only (never arbitrary repository prose) for phrases like "certified" /
"fully compliant" / "complies with" / "meets all requirements" appearing
alongside an open gap or no corresponding evidence/attestation -- emits
`assurance.documentation-claim-exceeds-support` at `Warning`. Its message
says only that a claim may exceed recorded support; it never asserts the
claim is false, illegal, or a named violation. This heuristic's absence
never becomes an "all documentation verified" claim, and Decision 0054's
authority boundary already forbids treating it as more than advisory.

### 9. Snapshot computation surface

`repopact-validation` gains the canonical, read-only computation
(`compute_review_snapshot`); the engine gains a new operation,
`assurance.snapshot`, returning the computed `review.snapshot` object for
one mapping id without writing anything. The Python CLI adds `repopact
assurance snapshot <mapping-id>` as a thin wrapper over that engine call
(`repopact-cli` pattern already used for `analyze`/`graph.*`), printing
the JSON for the adopter to merge into `review.reviewed_at` /
`review_snapshot` themselves. This intentionally does not go through
WI054's mutation-plan authority: computing a snapshot has no
before/after repository state to diff, and silently writing an adopter's
JSON record on their behalf is exactly the kind of authority overreach
Decision 0054 §16 forbids. A future checkpoint may wire this into the
mutation system if adopters want an apply-in-place command; this phase
ships the read-only primitive only.

### 10. Time handling

Every date/time comparison in this phase accepts an injectable "now"
(mirroring `Validator::with_today`, already used for research
claim-freshness) in both engines' test surfaces. No test depends on the
real wall clock. No field in `review.snapshot` itself is time-based; only
`review.reviewed_at`/`review_due_at` (adopter-authored) carry time
semantics.

### 11. ForgeWire/adopter-proof boundary

ACM-007's ForgeWire-shaped fixture is public-safe, synthetic, and
identical in engine/validator code path to the adopter-neutral
`example-framework` fixture already proven in phase 1. It demonstrates
capability ("this mapping is expressible"), never a certification claim
("ForgeWire is compliant") -- its own fixture documentation says so
explicitly. No branch anywhere in `repopact-validation`,
`repopact/validate_repo.py`, or this schema names a specific framework;
the same code paths, unmodified, process both fixtures.

## Consequences

- `review.snapshot` and `documentation_refs` are additive; every phase-1
  mapping (with neither field) remains valid and is reported as
  `unreviewed`/date-based-only freshness with no drift diagnostics,
  since there is nothing to compare against.
- The digest-based drift model catches uncommitted, dirty-working-tree
  changes to a reviewed reference (it hashes current file bytes, not a
  Git commit), matching WI051 §5's requirement that a change the adopter
  is currently reviewing is visible immediately, not only after commit.
- `repopact assurance snapshot` is the first CLI/engine surface unique to
  WI051; it is read-only by design, so it carries none of WI054's
  mutation-authority or WI050 admission-gate concerns.

## Alternatives considered

- **A general-purpose content classifier or NLP-based claim checker.**
  Rejected outright (WI051 §21, §27): RepoPact cannot reliably determine
  legal data classification or grade prose, and pretending otherwise
  would misrepresent an unverifiable claim as RepoPact-checked fact.
- **Persisting derived review state (`current`/`overdue`/`drifted`) as
  canonical JSON on the record.** Rejected: it is fully recomputable from
  `review.snapshot` plus current repository state; persisting it invites
  the two to disagree and duplicates authority the record's own content
  already carries (WI051 §20).
- **Hashing Git commit metadata (mtime/commit SHA) instead of content.**
  Rejected: it cannot see uncommitted edits, which is exactly the
  "adopter is currently reviewing this" case §5's drift model exists to
  catch (WI051 §44 test requirement).
- **Wiring `assurance snapshot` through the WI054 mutation-plan system to
  write the mapping in place.** Rejected for this phase: a snapshot
  computation has no repository-state diff to plan/apply, and an
  in-place write bypasses the adopter's own authorship of their record.
  Left as explicit future work if adopter demand justifies it.
