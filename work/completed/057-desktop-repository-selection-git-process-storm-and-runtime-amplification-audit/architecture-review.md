# WI057 Sol Architecture Review

The operator-reported terminal storm is a confirmed architecture-level amplification defect, not merely a Windows window-style problem.

The binding diagnosis and complete process inventory are in `process-amplification-audit.md`. The implementation direction is:

1. bound and centralize native Git subprocess execution;
2. compute Git/worktree/tracked-path topology once per repository generation;
3. make validation genuinely snapshot-backed;
4. cache one immutable native desktop generation and project all reads from it;
5. move expensive refresh work outside the global desktop mutex and publish generation-safely;
6. make watcher refresh single-flight, coalesced, generated-path-aware, and unchanged-token suppressing;
7. audit every remaining production Rust/Python process launch for amplification and timeout/containment behavior;
8. preserve WI050 ownership rather than changing protected security substrate under this work item;
9. prove the fix first with deterministic process-count instrumentation, then with controlled native Windows repository selection.

WI056 is intentionally blocked until WI057 closes. Decision 0042 remains architecturally valid; the sequencing block prevents the current flawed repository/process behavior from becoming canonical through the Python/Rust cutover.

The fix must remove the process storm itself. Merely hiding child console windows is insufficient.
