# Work Item 056 — Python Compatibility and Canonical Rust-Core Cutover

**Status:** Proposed

**Owner:** work-coordinator

**Affected scopes:** work, tooling, governance, docs, evidence

**Depends on:** WI053, WI054, WI055

## Intent

Decide and execute RepoPact's transition from a Python-reference/Rust-alternate migration state to a single canonical Rust executable core with a deliberately maintained Python compatibility surface, only after Rust read/validation/mutation and desktop-client boundaries have been proven.

This work item exists specifically to prevent an accidental cutover caused by implementation momentum. A Rust implementation does not become canonical merely because it is feature-rich or because Tauri uses it.

## Preconditions

WI056 must not activate until the evidence needed from WI053 and WI054 is available and the desktop/client boundary from WI055 has demonstrated that the core API is usable without duplicating semantics.

If the program determines Tauri delivery should not be a hard prerequisite for canonical-core cutover, that dependency may be amended through normal governance with explicit rationale; it must not simply be ignored.

## Cutover decision

Record a durable decision that identifies:

- the exact Rust surfaces that become canonical;
- remaining Python-only or deferred surfaces;
- conformance/parity evidence;
- the Python compatibility mechanism;
- version/protocol compatibility rules;
- packaging/distribution model;
- rollback strategy;
- release implications;
- WI050 boundary status;
- what claims documentation may make after cutover.

## Python compatibility alternatives

Evaluate at least these two supported architectural families against actual release/operational evidence.

### Option A — PyO3/native binding

```text
Python CLI/package
      |
      v
PyO3 binding
      |
      v
canonical Rust core
```

Evaluate wheel/build matrix, Python versions, source builds, Windows/Linux/macOS release burden, error/diagnostic mapping, API stability, and development ergonomics.

### Option B — stable Rust executable/protocol

```text
Python CLI/package
      |
      v
versioned local executable/protocol
      |
      v
canonical Rust core
```

Evaluate process startup, binary discovery/versioning, structured I/O, cancellation/timeouts, error transport, packaging, offline use, and compatibility guarantees.

A third approach is acceptable only if it preserves the one-authority requirement and is supported by better evidence.

## No permanent dual engine

The end state must not leave independently evolving Python and Rust implementations both authorized to mutate/validate the same supported semantic surface indefinitely.

Temporary compatibility/reference code may remain for regression tests or unsupported legacy operations, but authority must be explicit and documentation must not imply two interchangeable canonical engines.

## Parity gates before authority flip

At minimum require durable evidence for:

1. read/discovery parity for supported records/repository topology;
2. validation accept/reject + diagnostic parity;
3. canonical mutation before/after parity;
4. generated projection parity for Rust-owned outputs;
5. transactional failure/recovery behavior;
6. Python compatibility behavior through the selected binding/protocol;
7. supported-platform packaging/install behavior;
8. explicit known non-parity/deferred list.

## Python CLI behavior

Preserve existing user-facing command semantics where practical, but do not preserve Python implementation structure merely for compatibility.

For commands whose canonical semantics move to Rust, Python should become a client/wrapper of the Rust authority rather than silently retaining an independent implementation.

Commands or security-critical surfaces intentionally remaining outside the cutover must be labeled clearly.

## Distribution and release engineering

Define reproducible Windows/Linux/macOS delivery for the selected architecture, including as applicable:

- Rust binaries/libraries;
- Python package/wheels;
- checksums/signing metadata;
- version coupling;
- installation/update behavior;
- offline/source-build story;
- artifact provenance;
- backward/forward compatibility;
- rollback/recovery.

Frozen CI/release paths require their existing operator approval before modification.

## WI050 boundary

WI056 does not automatically migrate admission/enforcement.

If WI050 or later security work has not separately proven a Rust protected substrate, the cutover decision must explicitly state that those operations remain on their existing authority path. Do not route them through an unproven compatibility shim just to advertise a single implementation language.

## Documentation claim safety

After cutover, documentation must distinguish:

- canonical Rust-supported semantics;
- Python wrapper/client status;
- intentionally deferred surfaces;
- platform/package limitations;
- WI050 protected enforcement status;
- migration/compatibility guarantees.

Do not use "Rust rewrite complete" or equivalent language if meaningful supported RepoPact semantics remain under a different authority.

## Rollback

The cutover needs an explicit release rollback plan. A failed packaging or compatibility release must not require rewriting repository history or downgrading durable record formats without a governed migration.

## Explicitly out of scope

- using cutover to redesign RepoPact schemas without separate governance;
- retaining permanent duplicate canonical Python/Rust engines;
- porting WI050 security substrate without its own proof;
- coupling canonical authority to a specific UI or LLM provider;
- rewriting durable completed work/evidence history.

## Closeout standard

WI056 closes when RepoPact has one explicitly documented canonical executable authority for the migrated surfaces, Python is a tested compatibility consumer or deliberately bounded legacy surface, distribution/rollback is proven to the available platform level, known non-parity is explicit, and release/documentation claims match the evidence.
