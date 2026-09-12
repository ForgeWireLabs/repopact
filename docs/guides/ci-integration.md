# Guide: CI integration

*Diataxis mode: how-to (task-oriented).*

RepoPact's CI contract is repository-defined and provider-neutral. The source of truth is
`governance/verification.json`; local execution is canonical. A hosted CI system is an
optional adapter that installs RepoPact and invokes the same named profile.

For the complete local-first workflow, release preparation, evidence recording, and hosted
opt-in switches, see [`local-ci-cd.md`](local-ci-cd.md).

## Minimum local gate

Run the repository's quick profile for a fast governance check:

```console
repopact verify quick
```

Run the normal CI profile before integration:

```console
repopact verify ci
```

The upstream RepoPact `ci` profile owns canonical validation, Python tests, conformance,
Rust workspace tests when required, and SPEC freshness. The dashboard is already part of
canonical repository validation. Do not restate that command sequence in provider YAML.

## Record a run when it is evidence

For work that needs a durable verification record, attach the actual invocation to an existing
work item:

```console
repopact verify ci --evidence-work-item 046
```

Ordinary exploratory runs remain read-only when `--evidence-work-item` is omitted.

## Hosted adapter shape

A hosted provider should do venue setup only, then call the same local contract. Conceptually:

```yaml
- uses: actions/checkout@v4
  with:
    fetch-depth: 0
- uses: actions/setup-python@v5
  with:
    python-version: "3.13"
- run: python -m pip install -e .
- run: repopact verify ci
```

The checked-in GitHub adapter is disabled by default and allocates a hosted runner only when
repository variable `REPOPACT_GITHUB_CI` is exactly `true`. Workflow presence by itself does
not prove that the adapter is enabled, available, invoked, effective, or bound to a merge
boundary.

## Keep derived artifacts honest

INV-7 is enforced through repository-owned semantics rather than a GitHub-only diff recipe:

- `repopact validate` rejects a stale generated dashboard;
- the `ci`/`release` verification profiles run the `spec_freshness` check;
- provider adapters invoke those same local semantics.

Regeneration remains explicit when you intentionally change source records:

```console
repopact dashboard
repopact spec
```

## Surface frozen-surface changes

Frozen-surface approval remains a human authority boundary (INV-6). A local or hosted caller
may report changes with:

```console
repopact check-frozen --base origin/main
```

That report does not synthesize operator approval. Hosted pull-request adapters may expose it
to reviewers, but the human review boundary remains the binding authority.

## Do not infer remote enforcement from a green run

A passing `repopact verify ci` run proves that invocation on its recorded executor. It does
not prove branch protection or another remote admission boundary is closed. RepoPact keeps
checkpoint definition, venue availability, invocation, result, and admission effectiveness as
separate facts.
