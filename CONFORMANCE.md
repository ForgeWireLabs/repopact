# RepoPact Conformance

RepoPact conformance is tied to the semantic version in [`VERSION`](VERSION).
An implementation may claim **RepoPact 2.0.1 conformant** when it passes the
published conformance suite in [`conformance/`](conformance/).

The suite contains one valid repository fixture and invalid overlays. Each case
declares the rule or invariant it exercises, the expected outcome, and, for
rejections, the required diagnostic text.

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
