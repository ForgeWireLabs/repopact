# RepoPact Conformance

RepoPact conformance is tied to the semantic version in [`VERSION`](VERSION).
An implementation may claim **RepoPact 3.0.2 conformant** when it passes the
published conformance suite in [`conformance/`](conformance/).

The manifest contains the normative machine-enforced rule inventory and its valid
or invalid fixtures. Coverage is bidirectional: every inventoried rule must have at
least one case, every case must reference a known rule, and every fixture directory
must be declared. The coverage gate fails before implementation results are
accepted when the inventory or fixture mapping drifts.

Each reject fixture must isolate exactly one primary diagnostic from the canonical
fixture oracle. The runner reports the declared diagnostic and any unexpected secondary
violations deterministically; a secondary violation fails the case even when the
expected text is also present. Dashboard absence and drift cases explicitly opt out
of canonical fixture regeneration so dashboard enforcement itself can be tested.

WI046 adds the optional provider-neutral `governance/verification.json` contract to
the canonical Rust validation surface. The conformance corpus includes a negative
profile-reference fixture while focused Rust/Python tests cover containment,
placeholder, capability, host/complete coverage, and execution semantics that are
not meaningful to fabricate inside a metadata-only fixture.

Canonical Rust engine run:

```powershell
python -m repopact.run_conformance
```

Add `--legacy-python` to run the independent Python compatibility comparator. The
comparator composes the historical Python validator with small explicit compatibility
validators for semantic surfaces migrated after the Rust cutover, including WI046.
It remains regression evidence rather than product authority.

Third-party implementation run:

```powershell
python -m repopact.run_conformance --command "your-validator --root {repo}"
```

The runner replaces `{repo}` with a temporary materialized fixture repository.
A conformant implementation must accept every `accept` case and reject every
`reject` case with the expected diagnostic. Passing the suite is a compatibility
claim for the named RepoPact version, not a claim about future versions.

WI050 admission conformance is additive: `tests/test_admission.py` exercises
canonicalization, Ed25519 verification, protected registration/tamper failure,
policy denials, lease revocation, delegation subset checks, and two reference
pre-action adapter families. The OS-neutral policy contract is shared by
Windows, Linux, and macOS backends; this checkout records unavailable host proof
rather than overstating sandbox coverage.
