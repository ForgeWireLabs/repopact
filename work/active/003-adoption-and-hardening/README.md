# 003 — Adoption surface and primitive hardening

> **Status**: 📋 **Planning — backlog captured 2026-06-15.** The governance kernel (000), publication (001), and governance primitives (002 ✅) make RepoPact self-consistent: it validates, tests, runs CI, and dogfoods its own records. This item captures everything between *"a repo that governs itself"* and *"a standard another repo can adopt."* Eight deliverables across three tiers; none blocks the kernel, three block adoption.
> **Owners**: Governance (lead), Tooling.
> **Depends on**: Work item 002 ✅ (binding invariants, frozen surface, decisions/policies, parameterized roles, derive-over-declare).
> **Frozen surface**: `governance/invariants.json`, `governance/charter.md`, `schemas/**`, `.github/workflows/**` — changes here need operator approval (`INV-6`). Several items below touch `schemas/**` and are expected, reviewed changes.
> **Charter binding**: every item restores or completes a primitive the charter already promises; none adds new scope. B-tier serves *"systems before sessions"* (adoption), H-tier serves *"drift is a defect"* (declared-but-unenforced primitives), N-tier serves *"derive over declare"* and robustness.

---

## Why this work item exists

RepoPact's stated end-state is a **reusable standard**, not just ForgeWire's externalized brain. The kernel proves the model on one repo (itself). Three things keep it from being adoptable by a second repo, three primitives are declared but not yet enforced, and two are robustness gaps. They are captured together so the path to "RepoPact v1, adoptable" is visible in the filesystem rather than living in a conversation.

### Core principle for this item

```
A primitive that is declared but not enforced is drift waiting to happen.
A standard that cannot be installed is documentation, not a standard.
Close the gap between what RepoPact claims and what RepoPact checks/installs.
```

---

## Status legend

| Status | Meaning |
| ------ | ------- |
| 📋 Planning | Captured, not started |
| 🚧 In Progress | Active development |
| ✅ Complete | Finished, acceptance satisfied with evidence |
| ⏸️ Deferred | Intentionally postponed with revisit trigger |

---

## Backlog ledger

> Each row maps to an acceptance criterion in `work-item.json`. Anchors are verified against `main` at work item 002.

| ID | Tier | Item | Why it matters | Status | Anchor / evidence |
|----|------|------|----------------|--------|-------------------|
| **B1** | Blocks adoption | No bootstrap path | Adoption today means hand-copying files; ForgeWire shipped `_audit_template/` + `agents-md-template.md`, RepoPact ships neither. | 📋 | no `init` script, no skeleton in repo |
| **B2** | Blocks adoption | No record templates | Authors must hand-write JSON to match schemas for work items, decisions, policies, evidence. | 📋 | no `templates/` directory |
| **B3** | Blocks adoption | No LICENSE | Repo is public; without a license, adoption is legally ambiguous. | 📋 | no `LICENSE` at root |
| **H1** | Half-built | Audit finding is a stub | `record-types.md` describes "audit finding" but it has no schema, no validation, no example. | 📋 | `audits/findings/` contains only `.gitkeep` |
| **H2** | Half-built | Symbol-level frozen surface unenforced | `frozen-surface.json` accepts a `symbols` array; the checker matches paths only, so a frozen *symbol* change is not caught. | 📋 | `scripts/check_frozen_surface.py` has no symbol handling |
| **H3** | Half-built | Schemas are docs, not enforcement | `validate_repo.py` re-implements every check in Python and never loads `schemas/*.json`; schema and validator can silently drift. | 📋 | `validate_repo.py` imports `json` only, not the schema files |
| **N1** | Nice-to-have | No spec version | Adopters cannot say "we are on RepoPact v1"; schemas have `$id` but no version. | 📋 | no version constant or `VERSION` |
| **N2** | Nice-to-have | No dependency-cycle check | `depends_on` existence is validated; cycles are not. | 📋 | `validate_work` checks membership only |

---

## Tier 1 — Blocks adoption

### B1 — Bootstrap path
**Problem.** There is no supported way to install RepoPact into a *new* repository. The whole value proposition ("recover project state without a prior conversation") assumes the structure already exists.
**Proposed approach.** A `scripts/init_repo.py` (or documented skeleton) that writes the minimal source records — root `AGENTS.md`, `governance/{charter,invariants,frozen-surface,owners}.json|md`, empty `work/` lifecycle dirs, `evidence/runs/`, `audits/registry.json`, schemas — into a target directory, then runs `validate_repo.py` against it to prove the seed is valid.
**Acceptance (AC-1).** Running the bootstrap into an empty directory produces a repository that passes `validate_repo.py`.

### B2 — Record templates
**Problem.** Creating a work item, decision, policy, or evidence run means hand-authoring JSON/front matter that matches a schema from memory.
**Proposed approach.** A `templates/` directory with a commented exemplar for each record type, referenced from the relevant README. Optionally a `scripts/new.py <type> <slug>` that stamps one out with today's date and the next free id.
**Acceptance (AC-2).** A template exists for work item, decision, policy, and evidence run; a freshly stamped record validates.

### B3 — LICENSE
**Problem.** The repository is public on GitHub with no license; reuse rights are undefined.
**Proposed approach.** Choose and add a license appropriate to a standard meant for adoption (operator decision — likely permissive, e.g. Apache-2.0 or MIT). Record the choice as a decision record.
**Acceptance (AC-3).** A `LICENSE` file is present and the choice is captured in `decisions/`.

---

## Tier 2 — Half-built primitives

### H1 — Audit finding as a real record type
**Problem.** The audit-finding record type is documented but unimplemented; `audits/findings/` is empty.
**Proposed approach.** Add `schemas/audit-finding.schema.json` (id, scope, observed drift, risk, reconciliation, state), validate findings in `validate_repo.py`, and seed one example finding.
**Acceptance (AC-4).** Findings are schema-defined, validated, and exemplified.

### H2 — Symbol-level frozen-surface enforcement
**Problem.** A frozen *symbol* can be declared but not enforced.
**Proposed approach.** Extend `check_frozen_surface.py` to grep changed files in the diff for declared `symbols` and report matches alongside path hits.
**Acceptance (AC-5).** A change touching a declared frozen symbol is reported by the checker.

### H3 — Resolve schemas-as-enforcement
**Problem.** Schemas and the hand-rolled validator can drift; right now schemas are effectively documentation.
**Proposed approach.** Decide between (a) validating records against the JSON Schemas — which means a dependency (`jsonschema`) against the current stdlib-only constraint — or (b) explicitly declaring schemas reference-only with the validator authoritative, and adding a test that the two agree on required fields. Either way, record the decision.
**Acceptance (AC-6).** The relationship is resolved and recorded in `decisions/`, with the chosen mechanism in place.

---

## Tier 3 — Nice-to-have / robustness

### N1 — Spec version
**Problem.** No way to reference a version of the standard.
**Proposed approach.** A single source of truth for the spec version (e.g. `governance/owners.json` already uses `version`; add a top-level `VERSION` or a `repopact_version` field) surfaced on the dashboard.
**Acceptance (AC-7).** The standard declares an explicit version, surfaced in a derived view.

### N2 — Dependency-cycle detection
**Problem.** `depends_on` can form a cycle undetected.
**Proposed approach.** Add a cycle check over the work-item dependency graph in `validate_work`, with a test.
**Acceptance (AC-8).** A dependency cycle fails validation.

---

## Sequencing

Recommended order: **B3 → B1 → B2 → H1 → H2 → H3 → N1 → N2.** LICENSE first (cheap, unblocks public reuse), then the bootstrap + templates that make adoption real, then close the half-built primitives, then robustness. B3 and H3 each warrant a decision record; the rest are scoped implementation.

## Closeout criteria

Each AC is satisfied by linked evidence; when all eight are satisfied, move this directory to `work/completed/`. Per `INV-7`/policy `001`, regenerate the dashboard rather than editing it.
