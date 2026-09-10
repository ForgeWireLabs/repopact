# Work Item 056 — Python Compatibility and Canonical Rust-Core Cutover

**Status:** Blocked

**Blocked by:** WI057 — Desktop Repository-Selection Git Process Storm and Runtime Amplification Audit

WI056 was activated after WI053-WI055 completed, but implementation is intentionally paused after an operator reproduced a host-impacting native repository-selection defect in the Rust desktop/repository path.

The previously approved WI056 implementation plan is preserved in `active-plan.md`, the Sol architecture review remains in `architecture-review.md`, and the original implementation directive is preserved as `codex-directive.pre-wi057.md`.

Do not implement the canonical Rust engine/Python cutover until WI057 closes with concrete evidence that repository snapshot construction, Git subprocess behavior, desktop refresh, and the broader runtime-amplification audit are safe. This prevents WI056 from making a flawed process/refresh implementation the new canonical authority.

Decision 0042 remains accepted; this is a sequencing block, not a reversal of the selected compatibility architecture.
