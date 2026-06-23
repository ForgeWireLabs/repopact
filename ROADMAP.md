# Roadmap

A human-curated forward view. The authoritative status of in-flight work is the
derived [dashboard](audits/reports/dashboard.md) and the `work/` ledger; this file
adds editorial intent (what we mean to do next and why) that is not derivable.

## Now — shipped in v1.7.0

Closed the **inbound-reference drift class** found in the wild on forgewire — a
`takeover --delete` retired a heavily-referenced `todos/` tree and left dangling
references across docs, code, and a silently-broken test suite, none of which RepoPact
validation flagged. Three fixes, both ends of the lifecycle:

- **Import-plan rewrites narrative links** (decision `0015`): `import-plan` rebases each
  imported narrative's relative links to the item's new `work/<status>/<id>/` home and
  remaps cross-item links to the work ledger, so a freshly imported `work/` ships no link
  pointing back at the legacy plan tree.
- **Takeover repoints inbound references** (decision `0016`): before retiring a directory,
  `takeover` auto-rewrites the safe navigational forms (Markdown link targets,
  `source_of_truth:`) across the repo and **reports** everything it cannot safely touch
  (code `Path()` calls, prose, cross-repo links) as a `file:line` worklist. Runs under
  `--dry-run`.
- **Import-plan stops doubling titles**: source headings that embed the tracker number
  (`# 107 — Foo`) no longer produce `# 107 — 107 — Foo` once the allocated id is prepended.

## Now — shipped in v1.0.0

- **Adopter evidence + 1.0**: an independent proving-ground project
  (`repopact-proving-ground`) adopted RepoPact from the published wheel and
  exercised every primitive, including adversarial cases. Five of six hypotheses
  held outright; the two defects found (`spec` crash, `check-frozen` working-tree
  blindness) were fixed and re-verified. Recorded in [`research/`](research/);
  1.0 declared in decision `0007`. Work item `007`.
- **Brownfield adoption**: `repopact adopt` brings an *existing* repo under RepoPact,
  mapping CODEOWNERS → scopes, CI workflows → binding gates, and nested `AGENTS.md`
  → registered contracts, non-destructively. Proven on the real forgewire repo
  (4569 commits, 19 contracts). Work item `008`, decision `0008`.

## Earlier — shipped in v0.1.0-alpha

- **Governance core**: binding invariants + escalation, frozen surface, decisions,
  policies, parameterized roles (work items `001`–`002`).
- **Adoption surface**: `init_repo.py`, templates + `new.py`, Apache-2.0, audit
  findings, schema-enforced validation, spec version, cycle detection (`003`).
- **Specification & docs**: `SPEC.md` (generated catalog), Diataxis docs, OSS
  hygiene, first alpha release (`004`).

## Next

- **PyPI distribution**: publish the `repopact` wheel so `pip install repopact`
  works for anyone (Trusted Publishing from CI preferred).
- **Animated demo**: record `scripts/demo.sh` as an asciinema cast / GIF and embed
  it in the README (the script is ready; the recording is a manual pass).
- **Multi-adopter evidence**: invite external projects to adopt RepoPact and
  contribute their proving-ground-style evidence, broadening the 1.0 base.

## Later

- **Alternative validator** in a compiled language, proving the SPEC is
  implementation-independent.
- **GitHub Action** that runs the gates as a reusable workflow.
- **Conformance test kit** packaged for third-party implementations (the
  `tests/fixtures/` corpus shipped in `006` is the seed).

## How to influence it

Open a [task issue](https://github.com/ForgeWireLabs/repopact/issues/new/choose) or
start a [discussion](https://github.com/ForgeWireLabs/repopact/discussions). Scoped
contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md).
