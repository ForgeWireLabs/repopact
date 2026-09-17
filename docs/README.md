# RepoPact documentation

*Diataxis mode: explanation (orientation and documentation map).*

RepoPact is repository-native governance for durable human-agent software engineering. The repository carries the project state that must survive a session change: intent, authority, work state, decisions, invariants, provenance, evidence, and known drift.

This page is the entry point for the documentation set. Choose the path that matches what you are trying to do.

## Use RepoPact

Start here if you want to govern a repository rather than develop RepoPact itself.

- [Adopt RepoPact](adopt-repopact.md) — end-to-end first use on the stable package.
- [Define your governance](guides/define-your-governance.md) — roles, scopes, invariants, and frozen surfaces.
- [Author records](guides/author-records.md) — work items, evidence, decisions, and policies.
- [Local-first verification and release](guides/local-ci-cd.md) — repository-defined verification, evidence recording, release build/inspection, and optional hosted adapters.

## Understand RepoPact

Start here if you want the model and its boundaries before operating it.

- [Concepts](concepts.md) — authority, invariants, lifecycle, provenance, derived state, and enforcement classes.
- [`SPEC.md`](../SPEC.md) — normative repository model and machine-enforced rules.
- [Conformance](../CONFORMANCE.md) — implementation-independent acceptance/rejection contract.
- [Repository Orientation Graph](repository-orientation-graph.md) — optional derived orientation state and its authority boundary.
- [Charter](../governance/charter.md) — project principles and non-goals.

## Integrate RepoPact

Start here if another tool, agent host, IDE, execution system, or repository service needs to participate in the pact.

- [Pre-execution admission](guides/pre-execution-admission.md) — optional admission, guard, adapter, and confinement model.
- [CI integration](guides/ci-integration.md) — hosted checks as adapters rather than authority.
- [GitHub App setup](guides/github-app-setup.md) and [GitHub snapshot import](guides/github-snapshot-import.md) — GitHub-specific integration surfaces.
- [Install and build the engine](guides/packaging-and-engine.md) — canonical Rust engine packaging and compatibility boundary.

RepoPact does not require a particular model provider, coding agent, editor, CI vendor, or runtime. Those systems integrate with RepoPact; they do not become RepoPact's source of truth.

## Develop RepoPact

Start here if you are changing RepoPact itself.

1. Read the root [`AGENTS.md`](../AGENTS.md) and the deepest applicable nested contract.
2. Use the [`work/`](../work/) ledger to capture intent before implementation.
3. Treat [`SPEC.md`](../SPEC.md), schemas, decisions, and governance records as the normative sources for their respective concerns.
4. Run the required validation and conformance checks before claiming completion.
5. Preserve the distinction between the canonical Rust semantic engine and retained Python compatibility/comparator surfaces.

See [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution mechanics.

## What is authoritative?

RepoPact deliberately separates source records from projections.

Authoritative project state lives in records such as `governance/`, `work/`, `decisions/`, and `evidence/`, plus source code and explicitly governed configuration. The dashboard, Repository Orientation Graph, Workbench views, generated specification blocks, and other indexes are derived views. They may accelerate orientation and operation, but they do not acquire authority merely because they are easier to query.

If a derived view conflicts with its source records, the view is stale or wrong and must be rebuilt or reconciled.

## Enforcement is a ladder, not a boolean

RepoPact uses explicit assurance classes rather than treating every integration as equivalent:

- **instruction-only** — durable rules and state exist, but no pre-execution host boundary is claimed;
- **session-start** — an integration gates creation/start of a covered session or child process;
- **pre-action** — a covered mutation is checked before its callback/action begins;
- **sandbox/process-enforced** — a real operating-system boundary constrains the launched process tree for the capability being claimed.

Higher classes do not make lower classes false, and lower classes must never be described as if they provide stronger confinement. The portable reference baseline for optional admission remains `pre-action`; stronger process/path confinement is provider- and platform-dependent. See [Pre-execution admission](guides/pre-execution-admission.md) and decision [`0060`](../decisions/0060-optional-sandbox-process-enforced-reference-confinement.md).

## Release and development boundaries

The repository can contain active work that is newer or broader than the stable package contract. Public documentation should identify whether a statement refers to:

- the current stable package;
- implementation present on `main`;
- an integration-dependent reference guarantee;
- active, deferred, or proposed work; or
- research infrastructure/results.

Do not infer production availability, cross-platform proof, or benchmark results merely from code existing in the repository.
