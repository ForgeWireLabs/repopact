# Contributing to RepoPact

RepoPact governs its own development, so contributing means using it. The
repository is the source of truth; conversations only initiate work.

## Before you start

Read [`AGENTS.md`](AGENTS.md) (the contract), then
[`governance/charter.md`](governance/charter.md) and
[`governance/workflow.md`](governance/workflow.md). New to the model? Follow the
[adoption tutorial](docs/adopt-repopact.md).

## The loop

1. **Capture intent.** `repopact new work-item "Short title"`, then fill in
   the README and at least one acceptance criterion.
2. **Stay in scope.** Keep each change within one role's scope
   (`governance/owners.json`). Honor the invariants and the frozen surface.
3. **Implement.**
4. **Prove it.** Record an evidence run under `evidence/runs/` and link it from the
   acceptance criterion.
5. **Reconcile.** Regenerate derived artifacts; do not hand-edit them.
6. **Transition.** Move the work-item directory to `work/completed/`.

## Required checks

Install the development environment once, then run the repository-defined local
CI profile:

```console
python -m pip install -e ".[dev]"
repopact verify ci
```

The profile is defined in [`governance/verification.json`](governance/verification.json),
not in GitHub Actions. It covers the canonical validation, regression tests,
conformance, Rust workspace checks, and derived-artifact checks registered for
this repository. Use `repopact verify quick` for a fast development check and
`repopact verify release` for release-readiness verification.

The `dev` extra supplies the test and release tools (`pytest`, Maturin, `twine`)
inside the active environment. Individual test commands remain useful for focused
development, but they are not a second definition of the repository's CI contract.

GitHub-hosted validation is an optional adapter and is disabled by default. It runs
only when repository variable `REPOPACT_GITHUB_CI` is exactly `true`, and when
enabled it invokes the same local `ci` profile. A local pass is concrete local
evidence; it does not claim that a remote merge gate is enabled or effective.

## Release preparation

Release verification, build, inspection, and publication are separate operations.
Artifacts must come from a clean committed tree:

```console
repopact release verify
repopact release build --outdir release-out
repopact release inspect --dist release-out
```

Building does not publish. Local publication requires explicit operator intent:

```console
repopact release publish --dist release-out --confirm-publish
```

Publication credentials stay outside the repository and are supplied by the
operator's environment/keyring configuration. GitHub-hosted publication is a
separate optional adapter, disabled unless `REPOPACT_GITHUB_CD` is exactly `true`.
Enabling hosted CI does not enable hosted CD.

See [Local-first verification and release workflow](docs/guides/local-ci-cd.md)
for the full execution and evidence model.

## Touching the frozen surface

Changes to paths or symbols in
[`governance/frozen-surface.json`](governance/frozen-surface.json) require
maintainer approval (INV-6). Call it out explicitly in your PR.

## Decisions

If your change makes a material, hard-to-reverse choice, add a decision record
(`repopact new decision "..."`) so the rationale outlives the work item.

## Conduct

By participating you agree to the [Code of Conduct](CODE_OF_CONDUCT.md). Questions
and ideas are welcome in [Discussions](https://github.com/ForgeWireLabs/repopact/discussions).
