# 03 — Version Delta: RepoPact 2.2.0 (ForgeWire's pin) vs. dev HEAD `8966ba06` (Phase 3)

Compares the exact `repopact==2.2.0` implementation ForgeWire consumed against
`C:\Projects\repopact` at HEAD `8966ba060b5fe4a7dedd5aa65879db9dec6be2b0`
(`VERSION` = `3.0.0`, branch `wi-032-ci-checkpoint-decision`). Every claim
below is a direct source-code comparison, not an inference from changelogs.

**Correction (post-Phase-10, before commit).** The first pass of this phase
compared 2.2.0 against the checked-out feature branch
`wi-032-ci-checkpoint-decision` only, and reported the worktree-walk
false-positive gap (`repo_model.IGNORED_PARTS`) as unaddressed in "dev HEAD."
That branch is **not** `origin/main`'s current tip. Re-checked directly
against `origin/main` (`0096d70`, "fix: exclude worktrees/ scratch checkouts
from contract scanning (#7)"): **this gap was already fixed upstream on
`origin/main` on 2026-07-28** — three weeks before WI230 (2026-08-18) and
over three weeks before ForgeWire's own WI236/237 session (2026-08-20). The
corrected finding is materially more specific than "still unaddressed," and
is folded into the "Changed" section below rather than "Unchanged." The
id-allocation gap, the README-heading-check absence, and the WI-032/decision-
0031 status were independently re-verified against `origin/main` directly
(not just the feature branch) and **do** hold as originally stated — see each
item's confirmation note.

## Unchanged (confirmed identical logic)

- **Work-item id allocation** (`new.py:_next_numeric`): byte-for-byte the same
  local-tree-scan-and-increment logic in both versions. `max(existing numeric
  prefixes) + 1`, computed from `(root/"work").glob("*/*/work-item.json")` at
  the moment `new` runs. No `git fetch`, no remote registry, no lock, in
  either version. **The concurrent-agent id-collision gap that produced the
  WI236/WI237 renumbering is present, unaddressed, in the current development
  head, not merely in the pinned 2.2.0.** See `07-concurrency-id-collision.md`.
- **`check-frozen`**: same `--base origin/main` default, same working-tree
  union (F-002's fix), same bare `--ack` flag with no separate authorization
  record, confirmed present in both 2.2.0 and `origin/main`.
- **`doctor`'s repair surface**: same seven repair classes (schema skew, stale
  registry, missing/unregistered contracts, incomplete audits, gitignored
  records, dead source of truth, preflight migration + provenance
  ratcheting), confirmed identical in both 2.2.0 and `origin/main`. No new
  repair class for the id-allocation or README-heading gaps (still absent —
  see below).
- **Work-item id allocation** (`new.py:_next_numeric`): confirmed
  byte-for-byte identical between 2.2.0 and `origin/main` directly (not just
  the feature branch) — same local-tree-scan-and-increment logic, no `git
  fetch`, no remote registry, no lock, in either. **The concurrent-agent
  id-collision gap that produced the WI236/WI237 renumbering is present,
  unaddressed, on RepoPact's actual current `main`.** See
  `07-concurrency-id-collision.md`.

## Changed (confirmed additions on `origin/main`, absent from 2.2.0)

- **PR #7, `0096d70` — "exclude worktrees/ scratch checkouts from contract
  scanning"** (merged to `origin/main` 2026-07-28). `repo_model.IGNORED_PARTS`
  on `origin/main` now reads `{".git", "__pycache__", "node_modules",
  ".venv", ".pytest_cache", "build", "dist", "fixtures", "worktrees"}` —
  `"worktrees"` added, absent from 2.2.0. **This is the single most important
  correction this phase produced.** The gap that generated 171 of ForgeWire's
  269 errors (~64%) was already fixed upstream, three weeks before WI230
  started and over three weeks before ForgeWire's own WI236/237 session —
  but ForgeWire never received it, because `requirements-repopact.txt` was
  pinned to `2.2.0` (released before this fix) and no upgrade occurred
  between the fix landing and the incident (WI236/237's own instructions were
  explicit that upgrading RepoPact was out of scope for that work; the
  question of *why* it hadn't already happened independently is a separate,
  open one this case study cannot answer from the sources inspected). **This
  is not "RepoPact left this unaddressed" — it is "the fix existed and the
  adopter's pin lagged it."** This reclassifies part of `01-paper-claims.md`
  C6 and part of `05-claim-evidence-matrix.md`'s C6 entry from "RepoPact
  design gap" toward "adopter version-currency gap" — precisely the failure
  mode GA-1's 2026-07-26 update and `fleet_verify.py` (below) already exist to
  detect (stale adopter pins), just not yet for *this specific* fixed defect,
  since `fleet_verify` checks version-string currency, not per-fix drift.
  Note also: `IGNORED_PARTS` still checks literal path-segment names, not
  git-tracked status generally — a worktree checked out under a
  differently-named directory (not literally `worktrees`) would still be
  walked and produce the same false positives. The class of defect (raw
  filesystem walk with no git-awareness) is narrowed for the one directory
  name this incident happened to use, not closed structurally.
- **Decision 0028 — "Pin the README's release line to VERSION."** Dev HEAD's
  `validate_repo.py` (lines ~165-204) adds `_README_RELEASE_RE`: the
  *repository-root* `README.md`'s "current release **X.Y.Z**" claim is now
  checked against the `VERSION` file, gated on the convention being present
  ("following README checkbox parity (decision 0014)"). **This is new since
  2.2.0** — 2.2.0's `validate_repo.py` has no such regex or function.
  **This is directly relevant to the Phase 6 finding**: it shows RepoPact's
  own maintainers independently recognized "a narrative document can drift
  from a source-of-truth record" as a recurring defect class worth a targeted
  fix — but scoped it, again narrowly, to one specific literal string pattern
  (a version-line in the *top-level* README), not to work-item READMEs, and
  not to headings/titles/ids in general. Two narrow instances of the same
  underlying class (decision 0014 for checkbox state, decision 0028 for the
  root README's version line) exist; a third instance (work-item README
  heading vs. manifest `id`/`title`) does not yet exist in either version.
- **`fleet_verify.py`** (new module, entirely absent from 2.2.0): a
  network-calling tool that checks public adopters' *declared* RepoPact
  version pin (`repopact==X.Y.Z` in their `requirements*.txt`) against
  RepoPact's actual current PyPI release, plus checksum/diff verification
  against declared remotes. This directly operationalizes GA-1's 2026-07-26
  finding ("ForgeLink, ForgeWire, and RepoPact Proving Ground still declaring
  2.2.0 while RepoPact's public current release is 2.3.0") into a repeatable
  check. It answers a different question from `validate_repo.py`'s own
  invocation, though: `fleet_verify` checks whether an adopter's *pin* is
  current, not whether that adopter's own `repopact validate` passes, and not
  whether the adopter's CI *invokes* the validator at all — it is a
  version-currency check, not an enforcement-closure check.
- **Work item 032 / decision 0031** (`C:\Projects\repopact\work\blocked\032-*`,
  `decisions\0031-*`): RepoPact's own repository has, since 2026-07-18, an
  **open, `blocked`** work item to restore its own CI enforcement, with all
  four acceptance criteria still `pending` as of the last update
  (`2026-07-27`). Decision 0031 (status `proposed`) documents that:
  - `ForgeWireLabs/repopact`'s GitHub Actions has been billing-locked
    (account-level, not repo-level) since before 2026-06-29's "billing-locked
    since June" note in `gap-audit-2026-07.md` GA-3, confirmed still failing
    in 2-6 seconds as of run `30218026017` (2026-07-27).
  - **`main` has no branch protection at all** (`404 Branch not protected`) —
    independent of the billing lock, so even a green CI run would not be a
    *required* gate today. AC-3 ("a deliberately invalid test branch proves
    the gate rejects drift before merge") cannot be satisfied by any provider
    until this is configured.
  - The decision's own text states plainly: "the maintainer's current
    workflow is direct-to-`main` commits" — RepoPact's own repository has
    been operating, for at least a month, in exactly the "governance
    declared, checkpoint not actually gating merges" state this case study
    investigates in ForgeWire, for a structurally different but
    phenomenologically identical reason (billing lock + absent branch
    protection, vs. ForgeWire's CI workflows simply never calling the CLI).
  - **Re-confirmed directly against `origin/main` (not just the feature
    branch), live, at the time of this correction**: `gh api
    repos/ForgeWireLabs/repopact/branches/main/protection` still returns
    `404 Branch not protected`; `gh run list` shows every recent Governance
    validation run on `main` — including the run for the worktree-exclusion
    fix itself (`0096d70`, run `30386882959`) and every run after it through
    2026-07-28 — completing in single-digit seconds with status `failure`.
    **This is not resolved as of `origin/main`'s actual current tip**, not
    merely as of a stale feature-branch snapshot. It remains the single
    most directly relevant piece of version-delta evidence this case study
    found: RepoPact's own maintainers are, concurrently with this case
    study's writing, mid-decision on the identical class of gap — and, per
    the freshly-confirmed CI run list, RepoPact's *own* governance workflow
    has been failing on every single push to its own `main` for at least the
    period 2026-07-26 through 2026-07-28 inclusive, immediately surrounding
    the worktree-exclusion fix itself.

## Absent in both versions (not merely unmentioned — verified absent)

- No mechanism, in 2.2.0 or `origin/main`, that itself verifies a CI workflow's
  *step bodies* call `repopact validate`/`check-frozen` (as opposed to
  merely existing as workflow files, or being named in an `adopt`-mapped
  policy/invariant). `adopt`'s CI mapping records that workflows *exist*; it
  does not and cannot introspect what each workflow step *does*.
- No self-requirement that RepoPact's own CLI participate in the loop that
  produces commits — the checkpoint-not-precondition design (C1 in
  `01-paper-claims.md`) is unchanged in dev HEAD; WI-032/decision-0031 is an
  attempt to close this gap operationally (branch protection + a required
  status check) for RepoPact's *own* repository specifically, not a change to
  the kernel model itself (no new `I_*` predicate, no schema change, no CLI
  surface change addresses this in dev HEAD as inspected).

## Conclusion for this phase

**Corrected conclusion.** Of the two structural gaps that produced the bulk
of ForgeWire's 269 errors: the worktree-walk false-positive class (~64% of
the total) **was already fixed on RepoPact's actual `origin/main`** three
weeks before either WI230 or WI236/237 — ForgeWire simply hadn't received it,
because its pin stayed at 2.2.0. This means roughly two-thirds of this
incident's error volume is better classified as a *version-currency* gap
(the fix existed; the adopter's pin lagged it) than as a RepoPact design gap
— an important distinction the first pass of this phase got wrong by
checking a stale feature branch instead of `origin/main` directly. The
concurrent-agent id-collision gap **is** confirmed still present,
unaddressed, on `origin/main`'s actual current tip, by direct source
comparison. Do not claim RepoPact 3.0.0-in-progress would have prevented
WI236/237's specific id collision — it would not have. Do claim it would have
prevented the worktree-driven 171 errors specifically, had ForgeWire's pin
been current. The one narrative-drift-adjacent improvement found (decision
0028) narrows a different, related class (root README's version claim)
without covering the work-item-README case this incident surfaced. The most
significant version
delta is not a code change at all: it is that RepoPact's own repository is,
as of dev HEAD, in active, self-acknowledged, unresolved deliberation over
the same "declared vs. actually-enforced checkpoint" gap this case study
investigates in ForgeWire (work item 032, decision 0031).
