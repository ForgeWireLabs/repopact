# RepoPact Conformance Suite

This directory is the versioned conformance corpus for RepoPact. It promotes the
fixture set from internal tests into a public suite that alternative
implementations can run.

The suite is defined by [`manifest.json`](manifest.json):

- `valid/` is a minimal conformant RepoPact repository.
- `invalid/*` are overlays on `valid/`.
- each case names the rule or invariant it exercises and the expected validator
  outcome.

Run the reference implementation:

```powershell
python scripts/run_conformance.py
```

Run another implementation by passing a command template. The runner replaces
`{repo}` with a temporary fixture repository path:

```powershell
python scripts/run_conformance.py --command "my-repopact-validator --root {repo}"
```

An implementation claiming conformance must accept every `accept` case and reject
every `reject` case with the declared diagnostic text.
