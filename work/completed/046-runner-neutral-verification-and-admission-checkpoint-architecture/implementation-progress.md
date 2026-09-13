# WI046 implementation progress — 2026-09-13 architecture-review validation pass

This record captures implementation and validation state. Durable closeout
evidence lives in `evidence/runs/20260913-046-architecture-review-validation-pass.json`
and `evidence/runs/20260913-031628-497968-verify-ci.json`.

## What this pass found and fixed

An architecture-review-informed audit (not a filename/prose inference) found
two real defects and fixed both:

1. **`coverage: complete` did not mean complete.** `governance/verification.json`'s
   `release` profile declared `coverage: complete` with no platform-specific
   step declarations anywhere. `_coverage()`'s `satisfied` formula for
   `complete` mode reduced to "no required step was unavailable on this
   host" — a single Windows host could report `coverage.satisfied: true`
   without Linux, macOS, Android, or iOS ever being represented. Fixed by
   adding a required, non-empty profile-level `required_platforms`
   declaration (schema + Python + Rust semantic validation reject a
   `complete` profile without one), and by making `_coverage()` compute
   `satisfied` for `complete` mode as "this run's platform covers the
   entire declared `required_platforms` set", reporting anything else as
   `missing_platforms` rather than assuming it ran elsewhere.
   `governance/verification.json`'s `release` profile now declares
   `required_platforms: ["windows", "linux", "macos"]` — the actual
   platform-specific wheel matrix maturin's `bindings = "bin"` build
   produces, not `KNOWN_PLATFORMS` wholesale (Android/iOS are correctly
   excluded).

2. **`repopact verify` and `repopact release ...` did not exist.** Despite
   README.md and `docs/guides/local-ci-cd.md` documenting `repopact verify
   quick|ci|release` and `repopact release verify|build|inspect|publish` as
   the canonical operator workflow, `repopact/cli.py` never wired a
   `verify` or grouped `release` subcommand — only the old hyphenated
   `release-build` compatibility command existed. `verify_cli.py` and
   `release_local.py` both had working standalone `main()` functions
   (which is why the hosted workflow adapters, using `python -m
   repopact.verify_cli` / `python -m repopact.release_local` directly,
   worked correctly) but the documented unified operator command silently
   did not. Fixed by adding thin `verify`/`release` subparsers that pass
   the remaining argv through to each module's own `main()`, so no second
   argument grammar or semantic list is maintained.

A third, smaller defect was found and fixed in the test suite itself:
`tests/test_conformance.py` imported `validate` from the pre-WI046
`repopact.validate_repo` module directly instead of the new
`repopact.legacy_validate` composite oracle, so its own copy of the
`verification-default-profile` negative fixture check never actually
exercised the WI046 verification-contract validator. Fixed by correcting
the import.

## Governed release surface (Architecture issue B)

Investigated whether `release_local`'s wheel/sdist focus honestly
represents RepoPact's release surface now that WI060/WI062 proved native
Tauri Workbench packages install correctly on Windows, Linux, and Android.
Conclusion: **the Python wheel/sdist pair (built by maturin with a
platform-native compiled engine binary, published as `pip install
repopact`) remains RepoPact's only current governed, public release
artifact.** The Workbench native packages are explicitly local-operator
install artifacts (WI062 itself scoped signing and public distribution as
separate, un-done work). `release_local.py`'s scope is therefore correct
as-is; no native-artifact release contract was added, and none should be
invented here.

## Validation results

- Rust workspace: green, including both cross-caller parity tests
  (`verification_validation_parity.rs`, `verification_post_validation.rs`)
  run and reported individually, and the two new
  `repopact-validation::verification` coverage-completeness tests.
- Focused WI046 Python tests: 36/36 pass (`test_verification.py`,
  `test_release_local.py`, `test_wi046_hosted_adapters.py`,
  `test_conformance.py`), including 6 new coverage tests, 4 new
  release-manifest negative tests, and 1 new dashboard-rollback test.
- Canonical Rust conformance: 22/22. Python compatibility comparator:
  22/22. WI050 admission corpus: 8/8. (An initial run showed 21/22 canonical
  because the local debug `repopact-engine.exe` binary was stale from a
  prior session and had not picked up this session's Rust changes;
  rebuilding it resolved the discrepancy — not an architecture defect.)
- Full Python suite: 253 tests, 13 failures, 2 skipped (down from 14
  failures before the `test_conformance.py` fix). All 13 remaining
  failures are fully explained: 1 is the previously-documented,
  Precision-only isolated-interpreter `cryptography` environment gap; the
  other 12 all trace to one pre-existing, out-of-scope issue below.

## Blocking finding: this is not WI046's to fix

The live checkout at the directed baseline (`d1d26e5688c4d5ec61b2c78ea07c8143d339ff26`)
fails canonical `repopact validate` with 6 errors, all in `research/*.md`
content that arrived in the same upstream sync as WI046 itself (git history
confirms these research commits predate this validation pass and are
unrelated to it). Per this pass's own explicit instruction, research files
were not edited to make validation pass.

This blocks a genuine `pass` result for the `quick`/`ci`/`release`
profiles on the live checkout, and blocks `repopact release build` from
producing real artifacts (it correctly refused: "release verification
profile did not pass (fail); artifact build was not started" — proving the
fail-closed gate, not a defect). The mechanism's behavior here is exactly
correct: it detects a real problem and reports it honestly rather than
claiming success. `evidence/runs/20260913-031628-497968-verify-ci.json`
records this real, non-fabricated `ci` profile failure as durable evidence
of the mechanism working as designed.

## Why WI046 remains active

AC-6, AC-7, and AC-22 require proving the local CI and release-preparation
paths end-to-end, including an actual passing run and (for AC-7/AC-22)
real release-build artifact/hash/reproducibility evidence. Neither is
honestly achievable on this checkout right now, through no fault of
WI046's implementation. Per this pass's explicit governing instruction,
research-file content is out of scope for this pass and is reported here
as a separate blocker rather than fixed. AC-17 (a single derived view
aggregating local-contract/hosted-CI/hosted-CD/admission/release-readiness
state) also remains an implementation gap: today that state is represented
across evidence records and `docs/guides/local-ci-cd.md` rather than one
unified command.

All other acceptance criteria (AC-1 through AC-5, AC-8 through AC-16,
AC-18 through AC-21) are satisfied with concrete evidence recorded in
`evidence/runs/20260913-046-architecture-review-validation-pass.json`.
