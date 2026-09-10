# Codex Directive — WI057 Desktop Git Process Storm and Runtime Amplification Audit

Implementation agent: Codex

## Start state

Begin from the exact `main` SHA that activates WI057 and blocks WI056. Fetch/verify remote `main` equals that SHA and require a clean worktree before implementation.

Read, in order:

1. root `AGENTS.md` and applicable nested contracts;
2. `work/active/057-desktop-repository-selection-git-process-storm-and-runtime-amplification-audit/README.md`;
3. `work/active/057-desktop-repository-selection-git-process-storm-and-runtime-amplification-audit/process-amplification-audit.md`;
4. completed WI053, WI054, WI055 work/evidence;
5. Decision 0040 and Decision 0041;
6. blocked WI056 and Decision 0042 so the remediation preserves the planned canonical-engine boundary.

Do not implement WI056 while WI057 is active.

Work directly on `main`. Preserve concurrent work. No force-push, reset, history rewrite, or rewriting completed WI055 evidence.

## Safety rule

Do not intentionally trigger the uncontrolled Windows terminal/process storm before containment exists. Reproduce the multiplication first with deterministic instrumentation/counting tests.

If a live native check begins spawning uncontrolled children, terminate the Workbench process tree and treat the run as failed evidence; do not continue stressing the host.

## Baseline

Before implementation, establish the current baseline with target output outside the checkout:

- `python -m pip install -e ".[dev]"`
- Python repository validation using the current pre-WI056 authority path
- `python -m unittest discover -s tests -v`
- `cargo fmt --manifest-path rust/Cargo.toml --all -- --check`
- `cargo check --manifest-path rust/Cargo.toml --workspace`
- `cargo test --manifest-path rust/Cargo.toml --workspace --quiet`
- legacy conformance 20/20
- WI050 admission corpus 8/8
- linked-worktree/reference parity suite
- desktop `npm ci --ignore-scripts`, generated type drift check, TypeScript check, tests, production build
- frozen-surface check against the WI057 activation base.

Record any existing failure before modifying code.

## Phase 1 — executable invocation-count reproduction

Add test instrumentation around native Git execution before optimizing it.

Build a realistic repository fixture with many work items (at least 50; preferably test 1, 10, and 100) and prove the current architecture's Git query count is size-dependent, or otherwise produce an equivalent executable trace that demonstrates the multiplication mechanism.

The final regression must assert a small constant upper bound independent of work-item count. Do not encode an unnecessarily brittle exact number if platform/repository form legitimately changes one query; assert the semantic bound and identify the commands counted.

Also instrument a desktop-service read fan-out: after one repository generation is opened, repeated overview/work/decision/evidence/validation/graph/analysis reads must not launch new Git queries.

## Phase 2 — one native Git query boundary

Centralize production Rust Git query execution. A small internal module/type inside the repository crate is preferred unless a genuinely reusable cross-crate process abstraction is required; do not create a broad generic shell framework.

Required behavior:

- argv execution only, no shell interpolation;
- captured stdout/stderr/exit status;
- explicit operation labels/diagnostics;
- non-interactive Git query environment (`GIT_TERMINAL_PROMPT=0` or equivalent safe setting);
- finite timeout for local metadata queries;
- child termination and reap on timeout;
- Windows GUI-safe creation using `CREATE_NO_WINDOW` for captured children;
- portable behavior on non-Windows;
- injectable/countable runner for tests.

Remove raw production `Command::new("git")` call sites from repository and validation code after migration. Add a static/regression test preventing them from reappearing outside the approved helper/test code.

Do not add generic shell/process authority to Tauri or the frontend.

## Phase 3 — snapshot-owned repository topology

Introduce explicit snapshot-generation facts, conceptually `RepositoryTopology` or equivalent, containing only the Git/filesystem facts required by repository/validation semantics.

Compute worktree/common-dir/linked-worktree exclusion facts once per fresh snapshot. Compute tracked-path facts once when required by ownership validation. Reuse these facts through indexing, contract traversal, per-work-item source collection, validation, dashboard generation, graph, analysis, and mutation planning.

Refactor `files_under`/walkers so per-record traversal receives/reuses the already computed linked-root/topology set. It must not rediscover worktrees.

Preserve exact WI053 semantics for:

- normal repositories;
- linked worktrees;
- non-conventional nested linked-worktree paths;
- stale `.git` worktree metadata;
- independent nested repositories;
- ignored directories;
- Windows path comparison;
- record-relative references.

## Phase 4 — make snapshot validation real

`validate_snapshot` must validate the supplied generation, not merely recover a repository pointer and start a new crawl.

Refactor validator inputs so supported WI053/WI054 semantics consume snapshot/index/topology facts. Eliminate duplicate `iter_contracts`, evidence/work discovery, and Git tracked-path queries within one validation generation where the snapshot already owns those facts.

Dashboard projection used during validation/mutation should derive from the same generation. Do not change canonical dashboard bytes except where a pre-existing bug is proven.

Tests must prove validation of an existing snapshot performs no new Git query and no new repository crawl that changes its observed generation.

## Phase 5 — desktop cached immutable generation

Refactor `ActiveSession` to own the current immutable `RepositorySnapshot` (or a typed generation object containing snapshot plus safe derived immutable views).

Normal desktop read operations must project from that generation:

- overview;
- work list/detail;
- decision/evidence list/detail;
- validation;
- graph;
- analysis.

Do not call `active.core.snapshot()` on each read.

Initial repository open computes one generation. Explicit refresh computes a replacement generation. Successful mutation computes/publishes the authoritative post-apply generation. Accepted watcher changes compute a replacement generation.

A user can still explicitly ask for freshness; do not turn the cache into hidden stale state. Freshness is generation-based and explicit.

## Phase 6 — lock discipline and stale-publication safety

Do not hold the global desktop mutex across avoidable filesystem traversal, validation, graph build, or Git waits.

Use a short-lock pattern:

1. capture current session id/generation/root;
2. release lock;
3. compute candidate refresh;
4. reacquire lock;
5. publish only if the same session/generation is still current.

A stale refresh from repository A must never overwrite a newer repository B session.

Plan-handle/session semantics from WI055 remain intact.

## Phase 7 — watcher convergence

Keep watcher input collection cheap. Drain/coalesce relevant paths, then perform at most one refresh computation for a burst.

Required:

- explicit single-flight behavior per session generation;
- debounce/coalescing preserved;
- watcher-specific generated-path ignore for `.git`, `target`, `node_modules`, `.venv`, `__pycache__`, `.pytest_cache`, build/dist and any other demonstrated noisy generated roots;
- unchanged refreshed snapshot token => no duplicate semantic event;
- self-apply classification preserved;
- external invalid edits remain observable, not repaired silently;
- switch/close cleanly tears down the old watcher;
- no refresh feedback loop from the Workbench's own read/build activity.

Keep watcher ignore policy distinct from canonical governance contract-discovery rules unless parity evidence supports a shared change.

## Phase 8 — core API amplification review

Review every production caller of `RepoPactCore::snapshot`, `validate`, `graph`, `analyze`, and mutation planning.

One-shot convenience methods may remain, but document/test that they are fresh-snapshot operations. Multi-read consumers must use an explicit generation/snapshot path.

Ensure the planned WI056 one-request-per-process engine can use the corrected APIs without creating hidden repeated recrawls inside one request.

## Phase 9 — Python process-safety audit

Use `process-amplification-audit.md` as the minimum inventory and search again after implementation.

For non-WI050 Python helpers:

- `repo_model.py`
- `validate_repo.py`
- `check_frozen_surface.py`
- `adopt_repo.py`
- `takeover.py`
- `plan_import.py`
- `fleet_verify.py`
- `release_build.py`
- other production sites discovered during the audit

Classify each process call. Add finite timeout/non-interactive behavior to bounded metadata/query operations when semantics permit. Do not put arbitrary short timeouts on intentional builds or launchers. Preserve existing `fleet_verify` timeout behavior.

Do not change admission/guard/platform/protected-service semantics under WI057. Audit those sites; if a material issue exists, add a durable WI050-owned finding/criterion and stop short of an unreviewed security patch.

## Phase 10 — static safety guardrails

Add executable tests that make this defect class harder to reintroduce. At minimum:

- production native Git spawning exists only in the approved helper;
- desktop/Tauri/frontend have no generic shell/process capability;
- Git query count for snapshot build is O(1) with respect to work-item count;
- unchanged-generation read fan-out launches zero Git children;
- watcher burst produces bounded refresh work;
- timeout path kills/reaps the native Git child;
- Windows creation flags are applied by the native helper under `cfg(windows)`.

Prefer behavior tests over brittle source-text checks, but a narrow source guard is acceptable as a backstop for raw process-launch reintroduction.

## Phase 11 — native Windows verification

Only after phases 1-10 are green, build the Windows desktop application and exercise repository selection against:

1. the RepoPact repository itself;
2. a scratch/adopted RepoPact repository.

Record:

- app remains responsive;
- repository loads and navigation works;
- manual refresh works;
- switching repositories works;
- watcher external refresh works;
- no visible Git console windows appear;
- observed maximum concurrent Git child count;
- total Git invocations for initial load/refresh via instrumentation or equivalent trace;
- process tree is clean after close.

Do not claim Linux/macOS visual/runtime verification from the Windows host.

## Phase 12 — full regression and closeout

Run the complete baseline again plus all new WI057 process/snapshot/watcher tests.

Regenerate dashboard through canonical current tooling. Run frozen-surface verification. `.github/workflows/**` must remain untouched unless separately approved.

Evidence must explicitly list every production process-launch site and its final disposition. “No other issues found” is not sufficient without the inventory.

Move the complete WI057 directory to completed only after all RPS-001..RPS-020 criteria have concrete evidence.

Then unblock WI056 by moving it back to active, restoring its implementation directive, removing WI057 as an unsatisfied dependency only if the work-item semantics call for that, and regenerating the dashboard. Do not begin WI056 implementation in the same commit unless WI057 closeout is complete and the handoff state is clean.

## Required final report

Return:

- final main SHA and commits;
- exact confirmed root cause;
- before/after Git invocation counts for 1/10/50-or-100 work-item fixtures;
- maximum concurrent Git process observation;
- Windows no-console/timeout implementation;
- snapshot/topology architecture changes;
- desktop generation/cache and mutex changes;
- watcher changes and convergence evidence;
- complete Rust process-launch inventory;
- complete Python process-launch disposition;
- any WI050-owned finding created;
- native Windows repo-picker result;
- Python/Rust/conformance/WI050/frontend/frozen regression results;
- exact residual risks;
- confirmation whether WI056 is safely unblocked and its resulting status/SHA.
