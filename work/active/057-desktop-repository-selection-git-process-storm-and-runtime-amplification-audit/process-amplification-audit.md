# WI057 Process and Refresh Amplification Audit

Date: 2026-09-10
Reviewer: Sol architecture pass
Scope: production Rust runtime, Tauri/React desktop read/event paths, production Python subprocess helpers

## Classification standard

Each process-launch site is classified by whether it can multiply with repository size, UI/read fan-out, watcher/event cadence, or an unbounded loop. A site may be acceptable even without a short timeout when it is intentionally long-running and bounded by explicit user action; conversely, a very fast process is unsafe if invocation count can explode.

## Confirmed critical finding A — Rust worktree discovery multiplies by work-item count

`rust/crates/repopact-repository/src/lib.rs`:

- `RecordIndex::build` adds every work-item directory to `source_paths` via `repository.files_under(directory)`.
- `Repository::files_under` calls `discover_embedded_worktree_roots(&self.root)`.
- `discover_embedded_worktree_roots` calls `registered_worktree_roots`.
- `registered_worktree_roots` executes `git -C <root> worktree list --porcelain`.

Result: one snapshot can issue approximately N identical `git worktree list` processes for N work items, in addition to other topology/validation calls. This is the primary process-amplification defect.

Disposition: fix in WI057. Git topology must be computed once per repository generation and reused by all traversals.

## Confirmed critical finding B — desktop read fan-out multiplies fresh snapshots

`rust/apps/repopact-desktop/src/App.tsx` calls `loadRepository()` immediately after selection. It performs, in parallel:

- repository overview;
- work-item list;
- decision list;
- evidence list.

The Rust desktop service currently implements those reads by calling `active.core.snapshot()` or `overview(active)`, which itself constructs a fresh snapshot. Work-item detail, validation, graph and analysis similarly acquire new snapshots.

Result: frontend read fan-out multiplies backend filesystem/Git recrawls.

Disposition: fix in WI057. One native `ActiveSession` generation must cache one immutable snapshot and all reads must project from it.

## Confirmed critical finding C — `validate_snapshot` recrawls

`rust/crates/repopact-validation/src/lib.rs` accepts `RepositorySnapshot` but creates a new `Validator` from `snapshot.repository().clone()` and runs the ordinary repository validator. That validator rediscovers contracts/work/evidence and runs Git-backed ownership checks.

The validator also calls `repository.iter_contracts()` in more than one semantic path, including validation/dashboard rendering, so the same worktree topology can be rediscovered within one nominal validation.

Disposition: fix in WI057. Snapshot validation must consume the supplied snapshot/index/topology and cached Git facts rather than reopening the repository.

## Confirmed critical finding D — raw Windows GUI child creation

Production Rust uses direct `std::process::Command::new("git").output()` in repository/validation code. There is no shared child-process policy, timeout, non-interactive environment, invocation instrumentation, or Windows `CREATE_NO_WINDOW` configuration.

Result: when the Tauri GUI triggers repeated Git calls, Windows may display a console for each child process, matching the operator report.

Disposition: fix in WI057. Centralize captured Git execution below reusable Rust repository semantics. Eliminate raw production Git launches elsewhere.

## Confirmed high finding E — watcher refresh can amplify and block UI access

The Tauri process has a background thread polling repository events every 200ms. On a watcher change, `DesktopService::poll_repository_events` holds the global desktop mutex and performs a full snapshot/overview computation before returning the event.

Current behavior lacks explicit refresh single-flight, stale-generation publication checks, and unchanged-snapshot-token suppression. Watcher ignore behavior uses the canonical ignore set and does not include Rust `target/`.

Disposition: fix in WI057. Drain/coalesce under short lock, compute refresh outside global lock, publish only if session generation is still current, and suppress unchanged-token events. Add watcher-specific generated-output exclusions.

## Confirmed architectural finding F — `RepoPactCore` convenience API is fresh-snapshot per method

`RepoPactCore::{validate,graph,analyze,plan_mutation}` each call `self.snapshot()` independently. This is acceptable for isolated one-shot callers but is unsafe as a multi-view session API because callers can accidentally create repeated full crawls.

Disposition: WI057 must make the distinction explicit and provide/use a generation/session projection path for multi-read consumers. The one-shot API may remain if its semantics are documented/tested.

## Rust process inventory

Current production Rust code search found external-process creation only in:

1. `repopact-repository` — Git common-dir/worktree topology queries.
2. `repopact-validation` — Git tracked-path/repository-history queries.

No production process spawning was found in graph, analysis, mutation, desktop API, or React itself. Tests contain expected Git setup commands and are not runtime authority.

Disposition: after remediation, raw Git process launch outside the approved Rust process helper should be regression-forbidden.

## Python process inventory and disposition

### `repopact/repo_model.py`

Runs Git common-dir and `worktree list --porcelain` queries. Invocation count is bounded per `iter_contracts` call rather than per work item, so the same multiplicative defect is not present. However these calls have no explicit timeout.

Disposition: review/add bounded query timeout/non-interactive behavior where safe. This code becomes reference/legacy for migrated surfaces after WI056, but remains user-reachable before cutover.

### `repopact/validate_repo.py`

Runs several bounded Git metadata queries for release/source identity and tracked-path ownership. The shared `_git` helper and a direct `git ls-files --cached` call do not currently establish an explicit timeout.

Disposition: safe query calls should be bounded or routed through a bounded helper. No loop-based process explosion was identified.

### `repopact/check_frozen_surface.py`

Runs a finite set of Git diff queries across a small fixed range set. No repeated unbounded loop was identified; helper has no explicit timeout.

Disposition: add safe finite timeout if it does not alter frozen-surface semantics.

### `repopact/adopt_repo.py`

Git statistics call a fixed group of repository metadata commands. No size-proportional subprocess loop was identified; no explicit timeout is visible in the helper.

Disposition: bound metadata query calls where safe.

### `repopact/takeover.py`

Git safety/recoverability checks execute bounded repository commands. No unbounded process loop was identified; helper lacks explicit timeout.

Disposition: bound read/query calls where safe; preserve mutation/recoverability semantics.

### `repopact/plan_import.py`

Uses `gh issue list` as a single explicit import action. No repeat/event amplification was identified. Explicit timeout should be considered for the query operation.

Disposition: bounded user-triggered query, not a storm; add timeout only if behavior remains correct.

### `repopact/fleet_verify.py`

Git `ls-remote`, `gh api`, and local remote inspection already use explicit timeouts.

Disposition: good existing pattern; preserve.

### `repopact/release_build.py`

Runs build and Git commands through a common helper. Build commands are intentionally potentially long-running and should not receive an arbitrary short timeout. They are one-shot release operations, not UI/event paths.

Disposition: classify as intentionally long-running. Git-only metadata suboperations may be bounded separately if useful, but no storm was identified.

### `repopact/adapters.py`

Uses `subprocess.Popen` to launch an explicitly requested admitted child process. This is intentional launcher authority, not a query loop.

Disposition: outside WI057 storm remediation; preserve admission semantics.

### WI050 protected platform/guard code

`platform_backends.py` and `guard_ipc.py` invoke host tools such as `icacls` and `sc.exe`. ACL verification may invoke `icacls` once per path ancestor, which is a bounded loop based on path depth. This is separately owned protected security substrate, not reachable from the current Tauri repository picker.

Disposition: audit only under WI057. Do not change security semantics here. If timeout/process containment is materially deficient, record an explicit WI050-owned finding/criterion before WI057 closeout.

## Frontend/event inventory

No JavaScript `setInterval` polling loop was found. The significant frontend multiplier is command fan-out after repository selection and watcher-event follow-up reads. Once reads become cached-generation projections, those calls are cheap and semantically coherent.

The native Tauri background thread is the only continuous desktop polling loop found. Its 200ms cadence is acceptable only if event polling remains cheap and refresh work is single-flight/bounded.

## Required closeout proof

WI057 evidence must include:

- complete production process-launch inventory after changes;
- count/instrumentation proving Git subprocess count does not grow with work-item count;
- count/instrumentation proving repeated overview/work/decision/evidence/graph/validation/analysis reads on one generation create no new Git processes;
- watcher burst test proving bounded single-flight refresh;
- Windows proof that captured Git children do not create visible consoles;
- timeout/failure tests for the centralized Rust Git query runner;
- disposition of every Python subprocess helper listed above;
- explicit WI050 ownership record for any protected-substrate issue found;
- full regression/conformance/desktop security baseline.

## Current conclusion

The operator-reported process storm has a concrete architectural cause in the Rust desktop/repository path. Static review found no second production Rust process-spawn subsystem with the same multiplicative structure. Several Python helpers have weaker timeout hygiene but are finite one-shot paths, not watcher/UI amplification loops. WI057 will remediate the critical Rust path and close the remaining process-safety audit gaps before WI056 resumes canonical-engine cutover.
