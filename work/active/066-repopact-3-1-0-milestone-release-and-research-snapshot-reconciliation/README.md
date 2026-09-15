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
