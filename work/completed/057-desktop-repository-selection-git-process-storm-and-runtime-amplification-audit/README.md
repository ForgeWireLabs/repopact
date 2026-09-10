# Work Item 057 — Desktop Repository-Selection Git Process Storm and Runtime Amplification Audit

**Status:** Completed

**Owner:** tooling

**Affected scopes:** work, tooling, docs, evidence

**Depends on:** WI055

## Closeout

WI057 was opened after the native RepoPact Workbench repository picker reproduced a host-impacting Git process storm on Windows.

The remediation replaced repository-size-dependent Git discovery and repeated desktop recrawls with bounded Git topology queries, immutable cached repository generations, snapshot-backed validation, generation-safe refresh publication, watcher coalescing, and bounded Windows child-process containment.

Deterministic instrumentation proved snapshot Git invocation counts changed from `2/11/51/101` to `3/3/3/3` for repositories containing 1/10/50/100 work items. Cached desktop read fan-out added zero Git calls, deterministic maximum Git query concurrency was 1, and a watcher burst produced one refresh.

The operator-assisted native Windows validation was then completed against the real Workbench. The operator navigated several if not all sidebar subcategories and reported that everything populated as expected. The process harness remained at zero Git descendants for almost the entire run, observed only short-lived bounded Git descendants during repository activity, recorded a maximum of 2, and cleaned up the Workbench process tree normally. No recurrence of the original sustained process storm or host lockup was observed.

All RPS acceptance criteria are satisfied. Final closeout evidence is recorded in `evidence/runs/20260910-057-native-picker-closeout.json` together with the remediation evidence in `evidence/runs/20260910-057-process-amplification-remediation.json`.

WI056 may resume from the corrected snapshot/session/process substrate. WI050 host-tool timeout containment remains separately owned by `audits/findings/003-wi050-host-tool-timeout-containment.json`.
