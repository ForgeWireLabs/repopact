# Tooling Alignment Report

## 2026-09-03 WI050 admission architecture review

- Reviewed validate_repo.py, check_frozen_surface.py, init_repo.py,
  adopt_repo.py, doctor.py, and the CLI against the f2c80b7/WI049
  first-write bypass.
- Current validation and frozen checks remain deterministic repository
  backstops; check-frozen --ack is a caller assertion and is not operator
  proof in the accepted WI050 design.
- Decision 0038 selects a protected guard plus vendor-neutral adapter SPI for
  a later implementation pass. No tooling/runtime behavior or frozen path was
  changed in this architecture phase.

## 2026-09-03 deterministic evidence timestamp chronology

- `validate_repo.validate_evidence` retains ISO-8601 validation for all runs and
  applies the explicit `timestamp_basis: "git-recording"` rule against the first
  recording commit plus a five-minute clock/write tolerance.
- Naive timestamps are UTC; aware timestamps normalize to UTC. Git-free,
  exported, and uncommitted records retain structural validation without a
  fabricated history comparison, preserving deterministic behavior and legacy
  evidence history.
- Regression coverage proves far-future rejection, tolerance/offset/naive and
  historical acceptance, malformed input rejection, Git-free behavior, and
  repeated validation that cannot heal through passage of time. Decision 0037
  records the alternatives and the WI049 reconciliation boundary.

## 2026-09-02 structural worktree-aware contract discovery

- `repo_model.iter_contracts` now prunes same-repository linked worktrees using
  both Git's porcelain worktree registry and embedded `.git` files pointing into
  the primary `.git/worktrees` directory.
- The literal `worktrees` entry in `IGNORED_PARTS` remains as a compatibility
  fallback for stale/orphaned conventional scratch trees with no usable metadata.
- Independent nested repositories with `.git/` directories remain discoverable;
  exported trees and Git-unavailable environments retain deterministic fallback
  behavior.
- Regression coverage includes Windows paths with spaces, real conventional and
  non-conventional linked worktrees, stale linked metadata, nested repositories,
  negative controls, and worktree cleanup.

## 2026-09-02 source_of_truth resolution semantics

- `doctor._dead_source_of_truth` now resolves every path token relative to the
  declaring record's directory, matching decision 0016 and `takeover.py`'s
  preserved leading `../` behavior.
- Bare names are deliberately record-relative; a coincident root-level file
  cannot make a missing sibling target appear healthy.
- Regression coverage exercises nested `../` targets, valid and invalid bare
  targets, root coincidence, and the non-destructive `doctor --fix` contract.

## 2026-09-02 stable source/artifact identity reconciliation

- `validate_repo` now recognizes the exact matching stable tag as a valid
  unlabeled release tree and rejects later package/runtime source at the same
  `VERSION` unless a valid VERSION-pinned `RELEASE_LABEL` is present.
- `package_version` gives labeled development builds deterministic PEP 440
  metadata while preserving the strict `VERSION` adopter compatibility core.
- `release-build` uses the derived artifact identity for wheel/sdist names and
  still performs its independent-export and structural package checks.
- Regression tests cover exact-tag acceptance, unlabeled post-tag rejection,
  labeled development acceptance, and deterministic metadata mapping.

## 2026-07-26 RepoPact 3.0 release boundary

- Decision 0029's package/CLI boundary is released as the approved major version,
  rather than remaining an unreleased source change under the immutable public
  2.3.0 artifact.
- `release-build` constructs artifacts twice from independent exports of the
  committed tree and structurally rejects stale flat modules even if
  `top_level.txt` is misleading.
- Adopter-manifest validation checks declaration structure and local overlay
  integrity; remote version currency remains the fleet verifier's responsibility,
  so package publication and ecosystem rollout are genuinely separate phases.
- The declared development extra installs pytest, build, and twine while the
  required repository suite remains standard-library unittest.

## 2026-07-26 semantic freshness and ledger reconciliation

- Audit registry deadlines now block validation after expiry even if the
  dashboard was regenerated; source review, not projection refresh, is required.
- Upstream research metadata registers every top-level claim document under a
  dated, maximum-30-day review contract. Missing documents and expired contracts
  have regression coverage.
- `repopact new` stamps upstream work items against
  `repopact/schemas/work-item.schema.json` after the package-resource move while
  retaining the conventional root `schemas/` URI in adopter repositories.
- WI 020–022 preserve partial evidence criterion by criterion without converting
  missing launch, benchmark, statistical, or real-model proof into completion.

## 2026-07-26 package-resource seed closure

- Canonical schemas and templates live inside the `repopact` package and ship
  through setuptools package data; the deprecated `data-files` install surface
  is removed.
- `init`, `adopt`, `doctor`, validation, record stamping, conformance, and fleet
  verification resolve packaged resources through `importlib.resources`, while
  adopter repositories retain their conventional root `schemas/` and
  `templates/` copies.
- The protected schema surface moved from `schemas/**` to
  `repopact/schemas/**` with explicit operator approval for WI-036 AC-2.

## 2026-07-26 single-package execution and ownership closure

- The distribution exposes one top-level package, `repopact`; all internal modules use
  package-relative imports and the console script remains the supported interface.
- Seeded repositories contain governed state but no vendored tooling. The installed
  package executes validation, generation, and record-stamping (decision 0029).
- RepoPact enables exact Git-tracked path ownership in `governance/owners.json`.
  Deterministic diagnostics reject both unowned paths and overlapping ownership,
  while adopters can opt into the checkout-relative rule after mapping their tree.
- Test copies exclude virtual environments and build caches, schema validators
  are reused by content, and exported trees skip unnecessary Git probes. These
  remove the dominant local suite-time cost while preserving isolated repository
  state per test.

## 2026-07-22 canonical research metadata and trace repair

- `research/metadata.json` is the machine source for lifecycle states, PactBench task
  count, study/hypothesis mappings, threat identifiers, and the proposed-state trace.
- `validate_research.py` cross-checks repeated human-authored facts without generating
  or token-substituting semantic research claims.
- The normal repository gate activates this check only for the upstream research record,
  so adopters are not required to carry RepoPact's paper metadata.
- Mutation tests reject duplicate/missing threats, a four-state figure, stale task count,
  stale hypothesis range, and pre-2.0 provenance wording.

## 2026-07-18 complete conformance rule coverage

- The conformance manifest now inventories every repository-tree rule named by the
  SPEC and machine-enforced invariants covered by the reference validator.
- Coverage is bidirectional: omitted-rule and unknown-rule mappings fail before
  conformance execution, and repository tests reject undeclared fixture directories.
- Reject fixtures must produce exactly one intended reference violation. The runner
  reports unexpected secondary diagnostics deterministically and fails the case.
- Added provenance acceptance/rejection, lifecycle identity, semantic version,
  schema validity, orphan work, disjoint scope, and missing/stale dashboard cases.

## 2026-07-18 deterministic adopter fleet verification

- Added a versioned, schema-validated public adopter manifest covering exact PyPI
  pins and vendored consumers as distinct contracts.
- `repopact fleet-verify` resolves each declared public default branch, reads its
  version marker at the resolved commit, and fails closed on stale or unreachable
  state while reporting unregistered local candidates separately.
- Vendored parity is checksum-backed: exact files must remain byte-identical and
  declared overlays must reconstruct the adopter bytes from an immutable upstream
  revision. A version marker alone cannot pass.
- `repopact release-closeout` reports package publication and ecosystem rollout as
  separate phases and succeeds only when both have evidence.

## 2026-06-29 proposed lifecycle state (025)

- Added `proposed` to the shared lifecycle model as candidate work that does not
  grant implementation authority.
- Validator accepts structurally valid proposed work items but rejects active or
  completed work that depends on proposed work.
- Bootstrap and CLI record-stamping now create/use `work/proposed/`; conformance
  covers the new lifecycle rule.

## 2026-06-15 adoption surface and hardening (003)

- Records are now validated against `schemas/*.json` via `jsonschema` (decision
  0003); the validator retains cross-record semantic checks. Finding 001 closed.
- Added: audit-finding validation, spec-version check, dependency-cycle detection,
  symbol-level frozen-surface enforcement.
- Added bootstrap (`init_repo.py`) and record-stamping (`new.py`) tooling plus
  `templates/`, making RepoPact installable into a new repository.

## 2026-06-15 governance primitives (002)

- Validator enforces invariants, frozen-surface structure, role/scope references,
  decision and policy front matter, and registry-driven contract coverage.
- Mandatory per-contract `_audit` triples relaxed; an `_audit/` companion is
  validated for completeness only when present (INV-7, policy 001).
- Optional disjoint-active-scope check, off by default.

## 2026-06-14 bootstrap

- The validator is read-only and reports deterministic path-scoped errors.
- Dashboard generation writes only `audits/reports/dashboard.md`.
- Lifecycle-blocking rules have unit-test coverage.

## 2026-07-18 deterministic dashboard enforcement

- `validate_repo.py` compares the committed dashboard with a fresh canonical render
  and rejects missing or stale output.
- The generator no longer embeds its run date, so output stays byte-stable until a
  displayed source value or audit-cadence state changes.
- Bootstrap, adoption, record stamping, plan import, takeover, conformance
  materialization, and doctor repair refresh the derived dashboard as part of their
  governed mutation path.
- Regression tests cover missing/stale rejection, stable rendering, doctor repair,
  and command compatibility.

## 2026-09-03 WI050 implementation pass

- Added the six approved admission schemas only; existing frozen schemas,
  invariants, charter, and workflows remain unchanged.
- Added the vendor-neutral canonical policy core, Ed25519 approval receipts,
  external registration/trust pin, short leases, revocation and delegation
  subset checks, plus fail-closed guard and truthful adapter/platform SPI.
- CLI, validator, doctor, SPEC, guide, tests, and audit inventory now expose the
  opt-in admission plane. Reference adapters are pre-action gates and do not
  claim arbitrary process or filesystem confinement.
