# 02 — RepoPact 2.2.0 Enforcement Model (Phase 3)

Inspected directly against the installed `repopact==2.2.0` package (identical
copy present in both `C:\Projects\forgewire\.venv\Lib\site-packages` and
`C:\Projects\repopact-release\verify-2.2.0\venv\Lib\site-packages` — same
wheel, confirmed by `repopact-2.2.0.dist-info` in both). Module names below
are the flat top-level modules the wheel installs
(`repopact_cli.py, repo_model.py, validate_repo.py, generate_dashboard.py,
adopt_repo.py, init_repo.py, doctor.py, check_frozen_surface.py, new.py,
plan_import.py`) — RepoPact 2.2.0 is not a namespaced `repopact` package on
disk; that namespacing appears only in the current dev tree (see
`03-version-delta.md`).

## Modeled / enforced state (checked by `repopact validate`, no CI needed)

All of these are one-tree, `validate_repo.py`-resident checks, confirmed by
direct source read:

- `I_ver`, `I_struct` (JSON Schema, Draft 2020-12).
- `I_contract`: root `AGENTS.md` present; every nested contract registered in
  `audits/registry.json`; `_audit/` companions complete.
- `I_ID`: record id matches its path prefix; `status(w) == dir(w)`; ids unique
  per record type (`validate_work`'s `seen` dict — confirmed at
  `validate_repo.py:349`, "duplicate id also used by {seen[...]}"). **This
  check is single-tree**: it only fires if two conflicting ids coexist in the
  filesystem being validated *at that moment*. It cannot fire across two
  branches that have not yet been merged (see `07-concurrency-id-collision.md`).
- `I_ref`: dependency/scope/evidence/owner referential integrity; authorized
  work (`active`/`completed`) may not depend on `proposed` work.
- `I_accept`: `satisfied` requires non-empty `evidence`; `completed` forbids
  any `pending` criterion.
- `I_acyclic`: DFS 3-color cycle detection over the dependency digraph.
- `I_conc`: disjoint-active-scopes, opt-in via `owners.json.concurrency`.
- `I_orphan`: a `work/<status>/<dir>` with planning content (`README.md`,
  `AGENTS.md`, or `_audit/`) but no `work-item.json` is rejected
  (`validate_orphan_work_dirs`, confirmed at `validate_repo.py:600-607`).
- `I_prov`: `completed ⟹ concrete`; `concrete ⟹` all evidence backing
  `satisfied` criteria is `concrete`.
- **Preflight** (`validate_work_preflight`): conditionally required —
  disabled by default; enabled via `governance/owners.json.preflight.enabled`,
  with an optional `required_from_id`/`required_from_date` grandfather clause.
  When required, only checks *presence* of a `preflight` object with the
  schema-required shape (`created_before_work_started`, `created_at`,
  `note`); it cannot and does not verify the *truthfulness* of
  `created_before_work_started` or the timestamp — a fabricated preflight
  block (backdated) would pass exactly as well as an honest, disclosed
  retroactive one. Honesty here is a documented convention (this session's own
  disclosed-retroactive notes, and the `import-plan` `waived`-not-`satisfied`
  convention), not a machine-checked property.
- **`I_derive_dash`** (2.2.0's headline change): `exists(dashboard) and
  read(dashboard) == π_dashboard(s)` — decided by the validator on one tree,
  confirmed operationally multiple times in the ForgeWire WI236/237 session
  (`repopact validate` rejecting a stale `audits/reports/dashboard.md` with no
  CI involved at all).
- **README-manifest checkbox parity** (`validate_readme_checkbox_parity`,
  decision 0014): *narrowly scoped*. Only checks the `- [ ] **ID** ...`
  checklist convention's checked/unchecked state against the manifest's
  criterion `state` — gated entirely on that convention being present
  (`if not boxes: return`). It does **not** check the README's title/heading,
  or any other prose, against the manifest's `id` or `title` fields. See
  `06-representation-drift.md` for the direct consequence.

## Modeled but advisory / diff-time / human-gated state

- **`check-frozen`** (`check_frozen_surface.py`): a *separate CLI entry
  point*, not invoked by `repopact validate`. Diffs `--base` (default
  `origin/main`) `...HEAD` **unioned with** working-tree/staged changes
  (F-002's fix). Requires `--ack` to pass once a frozen-surface path is
  touched — `--ack` is a bare boolean CLI flag with **no** separate
  authorization record (no signature, no linked decision, no operator
  identity check); anyone or anything invoking the CLI can pass it. This is
  structurally the same "self-attested" shape T9 (provenance misuse) warns
  about for evidence records, applied to frozen-surface approval — not named
  as such in the research corpus.
- **INV-4** (no history rewrite): human review + git, by logical type
  (temporal). Unmechanized (`formal-model.md` O-4).
- **INV-5** (nested contract refinement): human review. Unmechanized (O-6).
- **INV-1** (no critical state only in conversation): human judgment plus
  `I_orphan` as a partial machine proxy.

## Unmodeled state (confirmed absent by source inspection, not merely undocumented)

- **No awareness of non-`.git`-tracked filesystem content.**
  `repo_model.iter_contracts`'s `IGNORED_PARTS` is a hardcoded set:
  `{".git", "__pycache__", "node_modules", ".venv", ".pytest_cache", "build",
  "dist", "fixtures"}`. It does not consult `.gitignore`, `git ls-files`, or
  any git-worktree-awareness. A local `git worktree` checkout anywhere under
  the validated root (e.g. `.claude/worktrees/<name>/`) is walked exactly as
  if it were governed content, because it contains its own copies of every
  `AGENTS.md` from whatever commit it has checked out. This produced 171 of
  the 269 errors reconciled in the ForgeWire WI236/237 session (see
  `06`/`08` and `01-paper-claims.md` C6).
- **No cross-branch/remote-aware work-item id allocation.**
  `new.py:_next_numeric` (identical in current dev HEAD — see
  `03-version-delta.md`) computes `max(existing numeric prefixes) + 1` by
  scanning **the local working tree at the moment `new` is invoked**. No
  `git fetch`, no remote id registry, no lock, no reservation. Confirmed
  directly in source; not an inference. See `07-concurrency-id-collision.md`.
- **No requirement that RepoPact's own CLI be invoked by anything.**
  There is no "meta-invariant" or self-check in `validate_repo.py` that
  asserts a CI workflow or pre-commit hook actually calls `repopact
  validate`/`check-frozen`. The presence of a `.github/workflows/*.yml` file
  that *mentions* RepoPact is not itself checked against whether that
  workflow's steps *invoke* the CLI. (ForgeWire's own four pre-WI236
  workflows are the concrete case: none called `repopact validate` or
  `check-frozen` despite `REPOPACT-ADOPTION.md`'s "Next steps" naming this as
  an open item since adoption.)
- **No README/narrative-vs-manifest identity check beyond the one checkbox
  convention** described above.
- **`doctor`'s repair surface does not cover** the two gaps above: it repairs
  schema skew, stale registry paths, missing/unregistered contracts,
  incomplete audits, gitignored records (F-008's specific fix), a "dead
  source of truth" case, preflight migration, and provisional→concrete
  ratcheting — all *content*-level repairs on records already inside the
  governed tree. It has no notion of "prune extraneous filesystem content
  outside the governed tree" or "reconcile a duplicate id across two
  branches."

## External workflow assumptions (stated in the model, not enforced by it)

- The paper and formal model are explicit that L1/L2 composition is
  *checkpoint-based*: "RepoPact does not require this to be enforced as a
  runtime gate" (`paper.md` §3.2). The model's safety property (T5, monitor
  non-bypass) is conditioned on a checkpoint actually running; RepoPact
  assumes but does not itself guarantee that some checkpoint (CI, pre-commit,
  a human running `validate`) exists and fires within a bounded window. WI-032
  in RepoPact's own dev repo (`C:\Projects\repopact\work\blocked\032-...`)
  is precisely an attempt to close this assumption for RepoPact's *own*
  repository, and remains open (all 4 acceptance criteria `pending` as of
  `2026-07-27`) — see `03-version-delta.md`.
- CI integration is assumed to be wired by the adopter; RepoPact ships example
  workflow content via `adopt`'s CI-workflow mapping (policies + INV-2 +
  frozen-surface entries recording that CI *workflows exist*), but does not
  verify those workflows' *step bodies* actually call the CLI.
