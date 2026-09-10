# Work Item 057 — Desktop Repository-Selection Git Process Storm and Runtime Amplification Audit

**Status:** Active

**Owner:** tooling

**Affected scopes:** work, tooling, docs, evidence

**Depends on:** WI055

## Incident

An operator reported that selecting a repository in RepoPact Workbench can hang the application while repeatedly opening and closing terminal windows whose title/path references Git. The loop is severe enough to interfere with Windows Task Manager and normal process termination.

Treat this as a host-impacting defect, not a cosmetic UI issue.

WI055 remains immutable completed history. WI057 owns diagnosis, remediation, the broader runtime-amplification audit requested after the incident, and durable regression evidence.

## Confirmed defect chain

Static inspection establishes a credible multiplicative path:

1. `RecordIndex::build` iterates every work item and calls `repository.files_under(record.directory)` to collect source paths.
2. `Repository::files_under` currently calls `discover_embedded_worktree_roots` every time.
3. `discover_embedded_worktree_roots` calls `registered_worktree_roots`, which executes `git -C <root> worktree list --porcelain`.
4. Therefore one snapshot can launch Git approximately once per work item before counting other Git-backed validation work.
5. The React `loadRepository` path immediately issues overview, work, decision, and evidence reads concurrently.
6. Those native read operations currently construct fresh snapshots rather than projecting from one cached desktop generation.
7. `validate_snapshot` is not genuinely snapshot-native: it reopens validator state from `snapshot.repository()` and rediscovers repository facts.
8. Rust process launchers use `Command::new("git").output()` directly without a shared timeout/process-containment helper; Windows GUI-origin children have no `CREATE_NO_WINDOW` handling.
9. Watcher refresh computes a full snapshot while the shared desktop mutex is held and lacks explicit single-flight/unchanged-token suppression.

This combination can turn what should be a small number of metadata queries into a visible process storm and UI starvation.

## Architecture target

```text
Repository open / accepted refresh
          |
          v
bounded Git/process facts + filesystem discovery
          |
          v
immutable RepositorySnapshot generation
          |
          +-- validation
          +-- dashboard projection
          +-- graph
          +-- analysis
          +-- overview
          +-- work/decision/evidence reads
          |
          v
Desktop ActiveSession cached generation
```

A repository generation is computed once, then read many times.

Frontend fan-out must not imply backend recrawl fan-out.

## Process-safety rule

A production external process reachable from repository reads, UI actions, watcher refresh, or repeated service calls must have an explicit execution policy. At minimum classify:

- command owner;
- whether it is a query, mutation, launcher, service operation, or test harness;
- maximum expected invocation cardinality per user operation/generation;
- timeout/cancellation semantics where appropriate;
- Windows console behavior when launched below a GUI;
- prompt/network behavior;
- whether it is safe to retry;
- whether it may cross WI050/protected authority.

Do not add a blanket timeout to intentionally long-running build or launcher operations merely to satisfy this rule. The point is bounded behavior with correct semantics, not arbitrary time limits.

## Rust correction

Centralize native Git execution below repository/validation semantics. The helper must support captured output, no shell interpolation, non-interactive query behavior, finite termination for local metadata queries, Windows no-console creation, and test instrumentation.

Compute worktree/common-dir/tracked-path facts once per snapshot generation and reuse them. Per-record traversal must accept/reuse topology rather than rediscovering it.

`validate_snapshot` must become truly snapshot-backed. Validation and dashboard rendering must not independently rediscover the same repository generation.

## Desktop correction

`ActiveSession` owns one current immutable snapshot generation. Overview/work/decision/evidence/validation/graph/analysis reads project from it.

Expensive refresh work should occur outside the global state mutex where practical. Publishing a refresh must verify the session/generation is still current.

Watcher refresh is coalesced and single-flight. Generated-path churn, including Rust `target/`, must not induce semantic refresh storms. If the resulting snapshot token is unchanged, do not publish a duplicate semantic repository-change event.

## Repository-wide audit

WI057 also audits the same failure shape outside the immediate desktop path.

Current static review has found:

- production Rust external process launches are concentrated in `repopact-repository` and `repopact-validation`, both Git-backed;
- `RepoPactCore` convenience methods each acquire a fresh snapshot, so multi-view callers must not casually use them as a repeated read session;
- older Python repository/validator/frozen/adopt/takeover/import helpers contain bounded subprocess calls, several without explicit timeouts;
- `fleet_verify.py` already uses explicit timeouts for Git/GitHub process queries;
- release builds and adapter launchers are intentionally long-running/launcher operations and require classification rather than blanket short timeouts;
- WI050 platform/guard code contains host-command calls, including bounded per-path ACL inspection, but it is separately owned security substrate and must not be altered under WI057 without an explicit WI050 finding/criterion.

The closeout audit must enumerate all production subprocess launch sites and show that no repeated/high-frequency unbounded pattern remains unreviewed.

## Safe reproduction

Do not intentionally reproduce the uncontrolled process storm before safeguards exist.

First prove the multiplicative behavior with injected/counting process tests and realistic many-work-item fixtures. After the fix, perform native Windows verification against the RepoPact repository and a scratch/adopted repository while recording Git invocation/concurrency behavior.

## Immediate operator workaround

Do not select a repository in an affected Workbench build.

If the storm starts, terminate the Workbench process tree instead of killing Git globally:

```text
taskkill /F /T /IM repopact-desktop.exe
```

## Explicitly out of scope

- rewriting completed WI055 evidence;
- changing governance semantics just to optimize UI reads;
- generic webview shell/filesystem authority;
- porting or weakening WI050 authority;
- PyO3/canonical Python cutover implementation owned by WI056;
- persistent repository-local cache/watch databases;
- hiding console windows while leaving an unbounded process storm intact.

## Sequencing

WI056 is blocked on WI057 because WI056 would make the Rust engine the canonical compatibility authority. The underlying repository/process behavior must be corrected before that authority cutover is implemented.

## Closeout standard

WI057 closes only when process count is demonstrably bounded independently of work-item count and frontend read fan-out, Windows captured Git children no longer open consoles, repository generations are reused correctly, watcher refresh converges, the broader process inventory is dispositioned, native Windows selection is responsive, and all relevant WI053-WI055/WI050 regression gates remain green.
