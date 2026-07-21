# RepoPact Conformance

RepoPact conformance is tied to the semantic version in [`VERSION`](VERSION).
An implementation may claim **RepoPact 2.3.0 conformant** when it passes the
published conformance suite in [`conformance/`](conformance/).

The manifest contains the normative machine-enforced rule inventory and its valid
or invalid fixtures. Coverage is bidirectional: every inventoried rule must have at
least one case, every case must reference a known rule, and every fixture directory
must be declared. The coverage gate fails before implementation results are
accepted when the inventory or fixture mapping drifts.

Each reject fixture must isolate exactly one primary diagnostic from the reference
validator. The runner reports the declared diagnostic and any unexpected secondary
violations deterministically; a secondary violation fails the case even when the
expected text is also present. Dashboard absence and drift cases explicitly opt out
of canonical fixture regeneration so dashboard enforcement itself can be tested.

Reference run:

```powershell
python scripts/run_conformance.py
```

Third-party implementation run:

```powershell
python scripts/run_conformance.py --command "your-validator --root {repo}"
```

The runner replaces `{repo}` with a temporary materialized fixture repository.
A conformant implementation must accept every `accept` case and reject every
`reject` case with the expected diagnostic. Passing the suite is a compatibility
claim for the named RepoPact version, not a claim about future versions.
