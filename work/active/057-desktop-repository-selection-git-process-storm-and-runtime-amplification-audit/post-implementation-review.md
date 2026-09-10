# WI057 Post-Implementation Architecture Review

Date: 2026-09-10
Reviewer: Sol architecture pass
Implementation reviewed: `02df051b05c4c16e356a46ffbe3c2a3563204bb0`
Implementation agent: Codex

## Status

The primary WI057 remediation is materially successful but WI057 is not yet ready for closeout. The measured Git invocation collapse from `2/11/51/101` to `3/3/3/3`, cached-read zero-additional-Git behavior, watcher coalescing, snapshot-backed validation, Windows no-console Git creation, Python query timeout hygiene, and WI050 finding are consistent with the intended architecture.

Two implementation-level containment issues remain in addition to the already declared native GUI picker verification gap. They are covered by existing RPS-004 and RPS-013 and do not require a new architecture decision or acceptance criterion.

## Finding 1 — Windows timeout termination delegates to an unbounded `taskkill.exe`

`rust/crates/repopact-repository/src/git.rs::terminate_child_tree` starts:

```text
Command::new("taskkill").args(["/PID", <pid>, "/T", "/F"]).output()
```

The Git query itself is bounded, but this failure-path helper has no timeout/termination bound of its own. If `taskkill.exe` wedges, `wait_bounded` can remain blocked after the nominal Git deadline and the "finite timeout/termination" guarantee is no longer strict.

### Required correction

Prefer a Windows-native process-tree containment primitive that does not require launching an unbounded helper process. A Job Object with kill-on-close semantics is the preferred design if it can be implemented without weakening portability or introducing unsafe lifecycle behavior. If Codex retains `taskkill`, the taskkill invocation itself must be bounded and its own failure must not strand the caller or leave an unbounded helper process.

Tests must cover the timeout/termination path rather than only the normal Git query path.

## Finding 2 — stale refresh can clear a newer session's single-flight guard

In both explicit refresh and watcher refresh, expensive work is correctly performed outside the global desktop mutex. However, when publication discovers that `active.id` or `active.generation` no longer matches the captured session, current code executes:

```text
active.refresh_in_flight = false
```

At that point `active` may be a newly selected repository/session. A stale refresh computation from repository A can therefore clear repository B's independently owned `refresh_in_flight` flag. If B is itself refreshing, this permits another B refresh to start concurrently and violates the generation-isolation intent of RPS-013.

### Required correction

A stale computation must never mutate the current active session. Clear `refresh_in_flight` only when the active session id and captured generation still identify the computation that set it. Repository switching should install a new session whose refresh guard is independent of abandoned work from the prior session.

Add a deterministic concurrency regression:

1. open repository A;
2. hold/delay A's refresh computation after it marks A in flight;
3. switch to repository B;
4. start/mark a B refresh in flight;
5. allow A's stale computation to return;
6. prove B remains in flight and a second B refresh cannot start;
7. complete B and prove normal refresh behavior resumes.

Also review concurrent double-open ordering. Normal React `busy` state reduces the likelihood, but the Rust service boundary should not allow an older slow `open_repository` computation to overwrite a newer repository selection if commands are invoked concurrently. If this can occur, serialize repository-open publication or use a monotonically reserved request/session epoch before expensive open work.

## Native picker closeout

The existing RPS-018 gap remains real. Do not weaken it merely because the external GUI automation helper is unavailable. Replace automation dependency with a safe operator-assisted evidence path:

- provide a small Windows verification harness that launches the already-built RepoPact Workbench and records the Workbench PID plus descendant/related Git process counts over time;
- the harness must not automate picker clicks and must not itself spawn repeated Git queries;
- operator manually selects the RepoPact repository, navigates between several views, explicitly refreshes, switches to a scratch/adopted repository, navigates/refreshes again, then switches back;
- operator records whether any Git console flashes occur and whether Windows remains responsive;
- harness records peak observed Git process count/concurrency and terminates/cleans up safely;
- final evidence identifies this as operator-assisted native validation rather than automated GUI validation.

The earlier uncontrolled storm must not be deliberately recreated.

## Machine-record reconciliation

At implementation SHA `02df051...`, WI057 correctly remains `active`, but all 20 acceptance criteria are still machine-recorded as pending. Before formal closeout, reconcile criteria against concrete evidence. Do not mark RPS-018 or RPS-020 satisfied until the operator-assisted native picker verification succeeds and the final closeout evidence exists. The remaining criteria may be ratcheted only when their evidence records are durable and validation passes.

## WI056

WI056 remains blocked. It may resume only after:

1. the timeout termination path is strictly bounded;
2. stale refresh/session concurrency cannot weaken a newer session;
3. any concurrent repository-open publication race is resolved or proven absent;
4. native operator-assisted picker verification passes on RepoPact and a second scratch/adopted repository;
5. WI057 criteria/evidence are reconciled and the complete work item is moved to completed.

No Decision 0042 amendment is currently required.
