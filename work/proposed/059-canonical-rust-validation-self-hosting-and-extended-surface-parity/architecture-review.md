# WI059 Architecture Review — Canonical Rust Validation Self-Hosting and Extended-Surface Parity

Date: 2026-09-10
Reviewer: Sol High

## Finding

WI056 correctly completed a surface-scoped Rust authority cutover, but its closeout evidence leaves the canonical validator unable to report the RepoPact source checkout as valid because two repository-local validation surfaces remain explicitly unsupported and one README-checkbox rule disagrees with the legacy comparator.

That is acceptable as WI056 closeout state, but it is not a desirable permanent product state. RepoPact's canonical validator should ultimately be able to validate the repository that defines and ships RepoPact without consulting a second semantic authority.

The correct follow-on is narrow parity work, not reopening WI056 and not migrating every retained Python operation.

## Confirmed baseline

At WI056 closeout:

- canonical Rust validation passes the published 20/20 conformance corpus;
- the explicit legacy Python comparator also passes 20/20;
- `governance/adopters.json` is emitted as `unsupported.semantic-surface` by Rust;
- `research/metadata.json` is emitted as `unsupported.semantic-surface` by Rust;
- Rust reports a README-checkbox parity issue in completed WI036 that Python does not;
- WI050 remains separately owned and 8/8;
- no Python fallback is permitted on migrated public validation;
- the source-checkout exit-1 behavior is therefore explicit, not hidden divergence.

## Architectural decision

WI059 should make canonical Rust validation **self-hosting for RepoPact's repository-validation contract**.

Self-hosting here means that all local governed records which the normal RepoPact repository validator treats as part of repository validity are understood by the canonical Rust validator, except surfaces deliberately and explicitly separated by an accepted authority boundary such as WI050 security enforcement or operational cross-repository tooling.

It does not mean every RepoPact command or Python module must move to Rust.

## Adopter manifest split

There are two distinct concepts that must not be collapsed:

1. **Local adopter-manifest validation** — reading `governance/adopters.json`, validating its schema and repository-local invariants/version relationships. This belongs in canonical repository validation if the presence of that file affects `repopact validate`.
2. **Adopter fleet verification** — operationally inspecting/verifying external adopter repositories. This is orchestration and remains Python-owned unless separately migrated.

The Rust validator should gain (1), not (2).

No network access, repository fan-out, subprocess-based remote checks, or persistent fleet state belongs in `repopact-validation` for WI059.

## Research metadata split

Likewise separate:

1. **Local research metadata validity** — whether `research/metadata.json` and its configured local references satisfy RepoPact's governed metadata contract.
2. **Research execution** — running experiments, benchmarks, tools, or interpreting results.

Only (1) belongs in WI059.

Where `validate_research.py` mixes pure validation helpers with execution assumptions, port the pure repository-validity semantics rather than transliterating Python structure mechanically.

## Checkbox discrepancy

The WI036 discrepancy must be treated as a parity bug until classified.

The completed WI036 README contains an entry of the form:

`- [x] **AC-7 (waived by decision 0029)** ...`

while the manifest criterion ID is `AC-7`.

Decision 0014's intent is to mirror criterion state using the criterion identifier. A validator that treats explanatory parenthetical text inside the bold label as part of the identifier may produce a false missing-criterion result even though the human-readable checklist clearly names AC-7.

However, WI059 must prove that hypothesis with executable fixtures. Do not assume Rust is wrong merely because Python is quiet, and do not assume the README is wrong merely because Rust is stricter.

The resolution should define one language-neutral interpretation of checklist identifiers and then make both implementations/fixtures reflect it.

## Canonical parsing rule

Avoid two independent ad hoc parsers.

For each migrated surface:

- prefer typed structures and existing schema infrastructure;
- reuse repository snapshot/index text/JSON already loaded for the current generation;
- centralize any new parser/helper where multiple Rust validation rules need it;
- return structured stable diagnostic codes rather than Python prose copies;
- preserve semantic equivalence even if diagnostic wording differs.

If the Python implementation contains historical quirks that contradict SPEC/accepted decisions, do not automatically clone the quirk. First classify it as specification, implementation bug, or intentional compatibility behavior and record the choice.

## Conformance strategy

The 20-case corpus being green does not prove these extended source-checkout surfaces because the corpus currently omits them.

WI059 should add coverage at the most language-neutral level practical. Prefer new canonical fixtures when the rule is part of RepoPact's general repository contract. If a rule is maintainer-only or optional in a way that makes corpus inclusion inappropriate, add paired Rust/Python fixtures and document that boundary.

The test set must demonstrate both acceptance and rejection, not only make the current source checkout green.

## Process/session constraints

WI057 is binding.

The new validation passes must operate entirely over the supplied `RepositorySnapshot`/index/topology where those facts are already present. Do not re-open the repository or invoke Git merely because adopter/research metadata exists.

A validation call should remain one bounded repository generation with no item-count-proportional external process creation.

## Authority constraints

Decision 0042 remains unchanged.

Do not:

- invoke Python from the Rust engine for these checks;
- add legacy fallback to `repopact validate`;
- downgrade unsupported/invalid semantics to warnings to obtain a green self-check;
- special-case the upstream RepoPact repository;
- migrate WI050 protected semantics;
- pull `fleet_verify.py` orchestration into Rust validation;
- pull research execution into Rust validation.

## Completion interpretation

A green source checkout is necessary but not sufficient.

Closeout requires proof that the canonical validator is green because the missing semantics were implemented correctly, with negative fixtures and parity evidence, not because the exact current files were allowlisted.

If additional repository-local validation surfaces are discovered while doing the inventory, WI059 should classify them. Small directly related omissions may be included if they are required for the same self-hosting contract and do not cross WI050/fleet/research-execution boundaries; materially new domains should become separate work rather than silently expanding this item.
