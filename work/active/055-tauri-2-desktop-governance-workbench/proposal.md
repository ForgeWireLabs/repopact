# Work Item 055 — Tauri 2 Desktop Governance Workbench

**Status:** Proposed

**Owner:** work-coordinator

**Affected scopes:** work, tooling, docs, evidence

**Depends on:** WI053, WI054

## Intent

Build RepoPact's modern Tauri 2 desktop workbench as a thin, secure, user-facing client of the proven Rust repository/validation/analysis/mutation core.

The desktop is not a second RepoPact engine and is not a generic Markdown editor. Its purpose is to make repository-native governance understandable and operable for humans while preserving the same durable semantics used by CLI and agent integrations.

## Preconditions

WI055 must not assume governed write authority before WI054 proves the mutation plan/apply boundary. Read-only UI shell work may be explored earlier only if it does not duplicate repository or validation semantics in TypeScript.

## Product surface

Initial navigation should cover:

- repository/workspace switcher;
- Dashboard / repository health;
- Work;
- Decisions;
- Evidence;
- Graph;
- Validation;
- Analysis;
- Settings.

The design should support dense engineering/governance information with strong hierarchy, keyboard accessibility, responsive sizing, dark/light themes, and clear lifecycle/diagnostic state. Visual polish must not replace semantic clarity.

## Repository open/session model

The Rust backend owns repository root discovery, indexing, validation, analysis, mutations, and filesystem watching.

The frontend receives typed snapshots/view models/events. It must not recursively crawl the repository or implement RepoPact record interpretation itself.

## Narrow Tauri command surface

Expose operations comparable to:

- `open_repository`;
- `repository_snapshot`;
- `list_work_items`;
- `get_work_item`;
- `analyze_work_item`;
- `plan_work_item_create`;
- `plan_work_item_update`;
- `apply_mutation`;
- `validate_repository`;
- `get_relationship_graph`.

Exact names may change, but the capability boundary must remain operation-oriented and repository-scoped.

Do not expose generic arbitrary filesystem read/write, unconstrained path traversal, or unrestricted shell execution to the webview merely for convenience.

## Work browser/detail UX

Support lifecycle/scope/dependency filtering, search, active/blocked emphasis, acceptance-criterion state, evidence linkage, related decisions, frozen-surface implications, and relationship navigation.

A work-item detail view should include:

- overview;
- acceptance criteria;
- dependencies;
- scope/impact;
- evidence;
- decisions/relationships;
- validation;
- history/context;
- advanced raw representation, read-only by default.

## Guided work-item creation

The default creation path should be a governance workflow rather than an empty editor:

1. Intent
2. Repository analysis
3. Scope / ownership
4. Dependencies
5. Acceptance criteria
6. Evidence plan
7. Governance / frozen-surface review
8. Mutation preview
9. Create

The UI should surface deterministic findings from WI054, such as overlapping active work, dependency-state conflicts, affected frozen surfaces, related decisions, ownership issues, and evidence implications.

The user remains the authority for the durable work record.

## Editing and mutation preview

Before apply, render the core-produced mutation plan:

- fields changed;
- files/records changed;
- lifecycle transition;
- generated artifacts affected;
- relationship/downstream impact;
- diagnostics/warnings;
- canonical diff/preview;
- stale-plan/repository-state status.

The frontend may collect user intent but must not construct an alternate write path around the core.

## Decision UX

Show rationale/status, alternatives, supersession, work/records referencing the decision, and potential impact before supported edits.

Do not change decision semantics solely to make them easier to render.

## Evidence UX

Show associated work/criteria, run status, checks/commands, artifacts/references, provenance/timestamps, validation state, and durability/immutability expectations.

Completed evidence must not appear as an ordinary editable text document.

## Graph and analysis UX

Provide useful dependency/relationship visualization and source-backed impact explanations. A graph is an alternate view of canonical relationships, not a separate persistence model.

Analysis findings should link back to the source records/paths that caused them.

## Filesystem watching

Watching belongs in Rust below the frontend:

```text
filesystem event
 -> debounce/coalesce
 -> normalize/classify path
 -> incremental repository refresh
 -> affected validation/analysis refresh
 -> typed RepositoryChanged event
 -> UI update
```

Account for editor rename/replace saves, Git branch/worktree changes, generated-artifact bursts, and self-originated mutations without feedback loops.

## Tauri security model

Use Tauri 2 capabilities/permissions to expose only the narrow operations the app needs. Repository scope/path authority must be explicit.

This local UI security boundary is **not** equivalent to WI050's protected admission/enforcement substrate and must never be documented as such.

## Optional assistance

The UI may later display provider-neutral assisted-analysis suggestions through a separate adapter boundary, but deterministic analysis remains first-class and provider suggestions remain non-authoritative.

No provider SDK/secret belongs in canonical RepoPact records merely because the desktop can configure an integration.

## Cross-platform target

The workbench targets Windows, Linux, and macOS. Packaging, signing/notarization realities, native webview behavior, path semantics, and accessibility must be documented/tested to the extent available during this item.

Frozen CI/release surfaces still require their existing human-approval process.

## Explicitly out of scope

- implementing a second validator in TypeScript;
- direct frontend governed-file writes;
- unrestricted frontend shell/filesystem capability;
- Python compatibility/canonical cutover;
- WI050 protected enforcement migration;
- making any AI provider mandatory;
- silently rewriting durable/completed history.

## Closeout standard

WI055 closes when a user can open an adopted repository, understand its health/work/decisions/evidence/relationships, use the guided supported work-item operations through core-generated mutation plans, receive live repository updates, and do so through a constrained Tauri capability surface without duplicating RepoPact authority in the frontend.
