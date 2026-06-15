# Roadmap

A human-curated forward view. The authoritative status of in-flight work is the
derived [dashboard](audits/reports/dashboard.md) and the `work/` ledger; this file
adds editorial intent (what we mean to do next and why) that is not derivable.

## Now — shipped in v0.1.0-alpha

- **Governance core**: binding invariants + escalation, frozen surface, decisions,
  policies, parameterized roles (work items `001`–`002`).
- **Adoption surface**: `init_repo.py`, templates + `new.py`, Apache-2.0, audit
  findings, schema-enforced validation, spec version, cycle detection (`003`).
- **Specification & docs**: `SPEC.md` (generated catalog), Diataxis docs, OSS
  hygiene, first alpha release (`004`).

## Next

- **Animated demo**: record `scripts/demo.sh` as an asciinema cast / GIF and embed
  it in the README (the script is ready; the recording is a manual pass).
- **Conformance fixtures**: a corpus of valid/invalid repositories so alternative
  implementations can self-test against the SPEC.
- **`repopact` console entry point**: package so `repopact init` works without the
  `python scripts/...` prefix.

## Later

- **Alternative validator** in a compiled language, proving the SPEC is
  implementation-independent.
- **GitHub Action** that runs the gates as a reusable workflow.
- **1.0**: declared only after external adopters have produced evidence that the
  record formats and rules hold up (confidence is not evidence).

## How to influence it

Open a [task issue](https://github.com/ForgeWireLabs/repopact/issues/new/choose) or
start a [discussion](https://github.com/ForgeWireLabs/repopact/discussions). Scoped
contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md).
