# Work Item 059 — Canonical Rust Validation Self-Hosting and Extended-Surface Parity

**Status:** Completed

**Owner:** tooling-owner

**Affected scopes:** tooling, governance, work, docs, evidence

**Depends on:** WI056

## Purpose

Close the remaining source-checkout validation parity gaps exposed by the WI056 canonical Rust-engine cutover so RepoPact's canonical Rust validator can validate RepoPact's own repository-validation contract without delegating to the legacy Python validator.

WI056 deliberately completed a surface-scoped authority cutover rather than pretending every historical Python operation had migrated. Its closeout evidence identifies three source-checkout validation gaps that should now be handled as focused follow-on work:

1. `governance/adopters.json` local validation remains Python-owned and the Rust validator reports it as unsupported.
2. `research/metadata.json` local validation remains Python-owned and the Rust validator reports it as unsupported.
3. Rust reports a README-checkbox parity discrepancy for completed WI036 that the legacy Python comparator does not report.

The objective is **self-hosting canonical validation**, not a broad remaining-Python rewrite.

## Success condition

After WI059, on a source checkout with no unrelated governed defect:

```text
repopact validate --root .
```

must be able to reach a valid result through the WI056 Rust engine path without invoking the legacy Python validator and without suppressing or downgrading genuinely unsupported/invalid semantics.

That result must come from actual parity closure, not a special-case allowlist for the RepoPact repository.

## Scope boundary

### In scope

- repository-local validation semantics for `governance/adopters.json`;
- repository-local validation semantics for `research/metadata.json`;
- exact Python/Rust parity for the Decision 0014 README-checkbox convention;
- Rust structured diagnostics and conformance/regression tests for those surfaces;
- removal/narrowing of `unsupported.semantic-surface` only where parity is proven;
- documentation/authority-map updates required by the parity change;
- proof that the canonical public validation route can validate RepoPact's own checkout.

### Explicitly out of scope

- cross-repository adopter fleet orchestration or verification;
- networked adopter checks;
- research experiment execution or benchmark orchestration;
- interpretation of research results;
- generic Python-to-Rust feature migration;
- WI050 admission/approval/guard/enforcement/IPC/platform/protected-service migration;
- generic decision/evidence/SPEC/takeover migration;
- frozen workflow changes;
- release publication/signing/notarization.

## Adopter validation boundary

Do not equate local adopter-manifest validity with fleet verification.

The canonical Rust validator should implement the repository-local semantics currently enforced when the Python repository validator encounters `governance/adopters.json`. This includes the schema and local invariants necessary to determine whether the record is valid in the current repository.

`fleet_verify.py` performs a broader operational role across adopter repositories. That remains Python-owned unless separately governed. WI059 must not add network or multi-repository orchestration to `repopact-validation` merely to eliminate the current unsupported diagnostic.

## Research metadata boundary

Port only the validation contract represented by the repository-local `research/metadata.json` and the current `validate_research.py` semantics needed to establish source-tree validity.

Do not pull experiment runners, benchmark tooling, external processes, result analysis, or other research execution into the validator.

The Rust implementation should reuse canonical schema/path/index machinery where possible and should produce deterministic structured diagnostics consistent with the rest of the Rust validator.

## WI036 checkbox discrepancy

Treat this as an unresolved parity finding until reproduced and classified.

The completed WI036 README visibly contains checklist entries for its criteria, including an `AC-7` entry whose bold label includes waiver explanation. The legacy Python validator does not report a mismatch, while the canonical Rust validator currently does.

Do **not** edit the completed WI036 narrative first just to make Rust quiet.

Required order:

1. construct a focused fixture reproducing the discrepancy;
2. compare Decision 0014's documented checklist convention and both implementations;
3. determine whether Rust is parsing the criterion identifier too broadly, Python is under-validating a genuine contradiction, or the source artifact itself is nonconforming;
4. fix the semantic defect at the correct layer;
5. add a regression that prevents the two validators from drifting again.

If a completed artifact truly requires correction, preserve its historical provenance and record the correction explicitly. Do not silently rewrite completed history to accommodate an implementation bug.

## Authority rule

Decision 0042 remains binding.

The Rust engine is canonical for migrated validation semantics. The historical Python validator remains an explicit comparator/reference, not fallback authority.

Therefore WI059 must not solve parity by:

- invoking Python from Rust;
- automatically falling back from Rust to Python;
- treating unsupported diagnostics as warnings merely because RepoPact's own checkout contains the records;
- hard-coding RepoPact repository paths/IDs as exceptions;
- deleting independent Python comparator coverage to make parity appear cleaner.

## WI050 boundary

WI050 remains separately owned and security-critical.

The Rust validator already distinguishes WI050 admission/enforcement/operator-authority surfaces from ordinary validation. WI059 does not authorize their migration. Keep the current 8/8 corpus green and preserve explicit rejection/unsupported behavior where Decision 0042 requires it.

## WI057 boundary

The validation extension must continue to consume the immutable cached repository snapshot/index/topology introduced by WI053-WI057.

Do not add direct filesystem/Git subprocess calls per adopter, research record, work item, or criterion. Validation should remain a deterministic projection over one repository generation, and the constant Git-query/process behavior proven by WI057 must remain green.

## Test strategy

Before implementation, capture the exact three-gap source-checkout result at the current baseline.

Then add focused fixtures for:

- valid/invalid/absent adopter manifest cases;
- adopter schema/local-invariant failures currently covered by Python;
- valid/invalid/absent research metadata where optional;
- configured research path/reference failures;
- README checklist identifiers with explanatory text after the canonical criterion ID;
- actual checkbox-state contradictions;
- missing mirrored criteria when the checklist convention is present;
- malformed/non-convention text that should not accidentally opt into parity validation.

The published canonical conformance corpus should be extended if these semantics belong in the language-neutral product contract; otherwise paired Rust/Python executable regression fixtures must still prove parity and document why the narrower location is correct.

## Documentation outcome

When parity is proven, update the WI056-era authority/deferred-surface documentation so it no longer describes local adopter-manifest validation or local research-metadata validation as unsupported Rust semantics.

Continue to say plainly that cross-repository fleet verification, research execution/orchestration, WI050 security authority, and any other retained Python surfaces have not migrated.

## Closeout standard

WI059 closes only when:

- the initial three-gap state is durably reproduced;
- the two local validation surfaces have canonical Rust parity;
- the WI036 discrepancy is correctly classified and fixed at the right layer;
- unsupported diagnostics are removed only for proven semantics;
- canonical `repopact validate` can validate the RepoPact source checkout without legacy fallback when no unrelated governed defect exists;
- Python/Rust/conformance/WI050/linked-worktree/process/package regressions remain green;
- documentation describes the new authority boundary accurately;
- closeout evidence states any residual unsupported canonical validation surface exactly.

This item was activated on 2026-09-10 after WI058 completed. The proposal and preflight rationale above are preserved as originally recorded. Before semantic implementation began, a full-baseline attempt on a separate machine discovered a regression-fixture storage-amplification defect unrelated to WI059 semantics; that finding is recorded as CVP-017 above and detailed in [fixture-storage-finding.md](fixture-storage-finding.md).

## Implementation status (semantic-parity phase, 2026-09-11)

The three semantic gaps (adopter-manifest parity, research-metadata parity,
the WI036 checkbox discrepancy) have been implemented and proven with
executable coverage; see [semantic-parity-inventory.md](semantic-parity-inventory.md)
for the exact functions ported, what remains Python-owned, and the
conformance/unit-test evidence. `repopact validate --root .` now validates
this checkout through the canonical Rust engine with zero diagnostics, no
`--legacy-python`, and no unsupported-surface entries for either migrated
file.

A subsequent architecture review found one containment flaw in the snapshot
layer (a metadata-directed adopter-overlay or research-metadata path could be
read into the immutable generation before an escape could be classified);
that was corrected with one reusable `resolve_within_root` choke point in
Rust and an equivalent containment check added to the Python comparator, with
regression coverage proving out-of-repository content is never ingested. See
[semantic-parity-inventory.md](semantic-parity-inventory.md) for detail.

## Closeout

WI059 closed on 2026-09-11. Durable closeout evidence is recorded as
[`20260911-059-canonical-rust-validation-self-hosting`](../../../evidence/runs/20260911-059-canonical-rust-validation-self-hosting.json),
which every acceptance criterion in `work-item.json` references. That
evidence records: the original three-gap baseline; the CVP-017 fixture-
amplification finding and fix; the containment-flaw finding and fix; adopter/
research/checkbox parity results on Windows and Debian 13 (WSL2, a real
Linux-native checkout, not a DrvFS/Windows path); the direct
`repopact-engine` stdio protocol proof (handshake + validate) on both
platforms; the public Python compatibility route with no legacy fallback;
21/21 canonical conformance and legacy-comparator results; WI050's untouched
8/8 corpus; WI057's unchanged process/scaling bounds; and the residual
unsupported Rust surfaces (WI050 records only).
