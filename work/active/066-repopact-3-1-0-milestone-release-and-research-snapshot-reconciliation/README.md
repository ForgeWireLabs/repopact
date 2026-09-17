# RepoPact 3.1.0 Milestone Release and Research Snapshot Reconciliation

## Why this work item exists

The 3.0.2 stable line has accumulated a substantial additive implementation
milestone: the canonical Rust engine and local-first release architecture,
Repository Orientation Graph capability, provider-neutral assurance mapping,
Workbench repository-map surfaces, benchmark RealRunner smoke infrastructure,
and the mobile app-private workspace/acquisition implementation checkpoint.

This cross-scope release item is the durable lead record for determining whether
that delta warrants the proposed 3.1.0 MINOR release, reconciling the research
paper to the final release candidate, refreshing the arXiv preparation package,
and executing the governed release process. The governance owner leads the
decision and identity surfaces; affected scopes are work coordination, evidence,
tooling/package implementation, and project-facing documentation.

WI021 remains the launch umbrella and is not reopened or hijacked. WI046 remains
completed. WI022, WI063, and WI065 remain independent records; this release does
not close their unfinished comparative, graph-evaluation, or mobile runtime and
export obligations.

## Release boundaries

- The compatibility audit must precede version mutation. A genuine breaking
  change stops this item before the version cut and requires a MAJOR decision.
- The stable package boundary is determined from WI046 and current release
  policy. Workbench, Android, and active research surfaces must not be presented
  as production-validated PyPI contents unless the artifact contract proves that
  they are included.
- ArXiv publication is not authorized by this item. The package may be rebuilt
  and verified, but its durable status remains **ARXIV NOT SUBMITTED**.
- Active work is described as active or incomplete where its own record says so;
  release publication cannot manufacture completion evidence.
- Publication and tag/release actions happen only after the exact candidate
  passes the canonical pipeline and are recorded with venue-specific evidence.

## Acceptance evidence

Each criterion in `work-item.json` remains pending until a concrete evidence run
or durable release artifact proves it. The final evidence must include the exact
candidate and post-release identities, hashes, PyPI clean-install smoke result,
tag/release result, and every known incomplete WI022/WI063/WI065 surface.

## Superseded by 3.1.1 and 3.1.2

The `repopact-3.1.0.tar.gz` sdist upload attempt this item's AC-9 anticipated
was rejected by PyPI at publication time (a `LICENSE`-file packaging defect;
see decision 0063). `3.1.0`'s wheel is live, but `3.1.0` was never the current
stable install identity in practice: `3.1.1` (decision 0063) and then `3.1.2`
(decision 0064, work item 069) were published as the corrective successors,
each with its own real publication/clean-install/tag/release evidence. AC-9,
AC-10, and AC-11 above correctly remain `pending` as literal 3.1.0-specific
publication criteria -- they are not rewritten to say `3.1.2` -- but current
stable-release reality is `3.1.2`, not this item's unpublished `3.1.0`
candidate. See `work/active/069-post-3-1-1-validation-remediation-and-development-identity-reconciliation/`
for the current release record.
