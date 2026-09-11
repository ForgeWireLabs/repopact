# WI059 semantic-parity inventory

Authority/parity inventory for the three source-checkout validation gaps
identified at WI056 closeout (CVP-002). States exactly which functions/rules
migrated to canonical Rust and which retained Python authority is
deliberately untouched.

## Canonical Rust after WI059

### Local `governance/adopters.json` validity — `rust/crates/repopact-validation/src/adopters.rs`

Ported from `repopact.validate_repo.validate_adopter_manifest`
([validate_repo.py:347](../../../repopact/validate_repo.py)):

- schema conformance against `adopter-fleet.schema.json` (now embedded in
  `repopact-schema`, see `EMBEDDED_SCHEMAS` in
  [repopact-schema/src/lib.rs](../../../rust/crates/repopact-schema/src/lib.rs));
- `adopters[].id` uniqueness (`adopters.duplicate-id`);
- `adopters[].repository` uniqueness after lowercase + trailing-`.git`
  normalization (`adopters.duplicate-repository`);
- for every vendored `consumption.files[]` contract with `mode == "overlay"`:
  the `overlay_path` resolves inside the repository (`adopters.overlay-path-escape`
  otherwise), the overlay is readable from the indexed generation
  (`adopters.overlay-missing` otherwise), and its CRLF-normalized SHA-256
  matches `overlay_sha256` (`adopters.overlay-checksum-mismatch` otherwise).

`unsupported.semantic-surface` for `governance/adopters.json` is removed from
`Validator::report_unsupported_surfaces` in
[repopact-validation/src/lib.rs](../../../rust/crates/repopact-validation/src/lib.rs).

### Local `research/metadata.json` and registered research-fact validity — `rust/crates/repopact-validation/src/research.rs`

Ported from `repopact.validate_research`
([validate_research.py](../../../repopact/validate_research.py)), function by
function:

| Python | Rust |
| --- | --- |
| `_load_metadata` (+ `validate_repo.validate_research_records`'s paper.md/protocol.md gate) | `Validator::validate_research` |
| `_validate_freshness` | `research::validate_freshness` |
| `_validate_lifecycle` | `research::validate_lifecycle` |
| `_validate_benchmark` | `research::validate_benchmark` |
| `_validate_threats` | `research::validate_threats` |
| `_validate_trace` | `research::validate_trace` |
| `_section` (regex lookahead) | `research::markdown_section` (rewritten without lookahead — the `regex` crate's linear-time engine does not support `(?=...)`; ported as a manual heading-boundary scan instead) |

Gating is self-contained in `validate_research`: silent when neither
`research/metadata.json` nor the upstream `research/paper.md` +
`research/protocol.md` convention is present; `research.metadata-missing`
when the convention is present but metadata is absent; full validation
otherwise. Deterministic "today" for freshness expiry is
`Validator::with_today` (defaults to the real date via `today_utc()` in
production; tests inject a fixed date, mirroring Python's
`validate(root, today=date(...))`).

`unsupported.semantic-surface` for `research/metadata.json` is removed the
same way as adopters, above.

### Decision 0014 checkbox semantics — `decision_0014_criterion_id` in `rust/crates/repopact-validation/src/lib.rs`

Root cause: the Rust README-checkbox scanner took the *entire* bold label
between `**...**` as the criterion identifier and required every character in
it to be alphanumeric-or-`-`. Python instead recognizes the canonical
identifier **prefix** `[A-Za-z][A-Za-z0-9]*-[0-9]+` with a word boundary
after the digits (`CHECKBOX_LINE` in
[validate_repo.py](../../../repopact/validate_repo.py)). A label like
`AC-7 (waived by decision \`0029\`)` therefore matched in Python but not in
Rust, producing a false `work.readme-checkbox-missing` for completed WI036's
AC-7 that the legacy comparator never reported.

Classification (per the WI059 architecture review's required order): this
was a **Rust parsing bug**, not a Python under-validation and not a
nonconforming repository artifact. WI036's README was correct under decision
0014's documented convention. Fixed at the parsing layer; WI036 itself was
not edited.

## Retained outside this migration (deliberately not touched)

- **Adopter fleet/network verification** — `repopact/fleet_verify.py` and any
  multi-repository/network orchestration. Nothing in `adopters.rs` performs
  network or subprocess access; it only reads the local manifest and local
  overlay files already in the repository snapshot.
- **Remote repository access** — no Rust validation code opens a network
  connection, spawns `git` against a remote, or inspects another checkout.
- **Research execution and benchmark running** — PactBench itself,
  experiment execution, and result interpretation remain Python/operational
  concerns; `research.rs` only checks that already-written facts (task
  counts, hypothesis ranges, threat identifiers, trace targets) are
  internally consistent, the same "fact validation only" scope
  `validate_research.py` had.
- **WI050 protected authority** — admission, guard, operator-authority,
  repository-registration, and protected-service semantics are untouched;
  their `unsupported.semantic-surface` entries remain in
  `report_unsupported_surfaces`, and the WI050 8/8 corpus is unaffected (see
  closeout evidence).
- **Unrelated Python commands** — `repopact/adopt_repo.py`,
  `repopact/release_build.py`, `repopact/takeover.py`, and other retained
  Python-only workflows are untouched.

## Snapshot/index extension (architectural prerequisite, done before any validator code)

`RecordIndex` (`rust/crates/repopact-repository/src/lib.rs`) gained:

- `adopters: Option<IndexedRecord>` and `research_metadata: Option<IndexedRecord>`,
  populated the same way as `owners`/`invariants`/`frozen_surface`;
- `adopter_overlay_bytes: BTreeMap<PathBuf, Vec<u8>>` — raw bytes (not the
  UTF-8-decoded `text_files` copy) for every vendored overlay path, because
  checksum verification must hash exact bytes and overlay content is not
  guaranteed to be valid UTF-8;
- `source_paths`/`text_files` extended to include every top-level
  `research/*.md` file (README included, unlike the existing
  `discover_markdown_records` helper which excludes it — the README's
  *membership in the set* is itself part of the freshness-coverage contract)
  and every local file referenced by an adopter overlay contract or research
  metadata field (freshness policy, lifecycle set/figure documents,
  benchmark source/documents/range/mapping documents, threat documents, and
  every proposed-state trace target).

No validator in `adopters.rs`/`research.rs` calls `std::fs::read_to_string`,
`Path::is_file`, or any Git operation directly; all of them read through
`RecordIndex::text`/`RecordIndex::overlay_bytes` against the snapshot already
built once per `validate()` call. WI057's scaling/process-bound tests
(`repopact-repository`'s `snapshot_git_invocation_count_is_bounded_independent_of_work_items`
and siblings) re-ran green after this change with no new Git query behavior.

## Conformance and parity evidence

- **Decision 0014 checkbox parity**: added to the language-neutral corpus as
  `readme-checkbox-mismatch` (`conformance/manifest.json` /
  `conformance/fixtures/invalid/readme-checkbox-mismatch/`), since the
  checklist convention is a general repository contract feature, not
  maintainer-only. 13 additional Rust unit tests in
  `repopact-validation/src/lib.rs` cover the explanatory-suffix and
  malformed-suffix boundary cases directly.
- **Adopter manifest**: maintainer-only/optional surface — paired Rust unit
  tests only (`repopact-validation/src/adopters.rs`, 9 tests: absent, valid,
  schema-invalid, duplicate id, duplicate normalized repository, valid/invalid/
  missing overlay checksum, path escape).
- **Research metadata**: RepoPact upstream/maintainer governance surface —
  paired Rust unit tests only (`repopact-validation/src/research.rs`, 14
  tests covering gating, freshness expiry with an injected date, coverage
  drift, lifecycle/benchmark/threat/trace drift, and the provenance-figure
  regression check), deliberately built against synthetic fixtures rather
  than only the live RepoPact repository files.
