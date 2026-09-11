# Finding: regression-fixture storage amplification (CVP-017)

**Discovered:** during a WI059 baseline attempt, before any WI059 semantic work began.

**Not a WI059 semantic defect.** This is a harness/architecture problem in the
Python regression suite that WI059 (and every future work item) inherits, so
it is fixed and proven here before WI059 semantic implementation starts.

## What happened

A first WI059 baseline attempt on a separate machine ran the canonical Python
suite and observed multi-gigabyte disk growth, later recovered (~17 GB) by
cleanup. The cause was not WI059 semantics — WI059 had not started.

## Root cause

Several tests and one production harness script created disposable "repository"
fixtures by doing, in effect:

```python
shutil.copytree(REPO_ROOT, dest, ignore=shutil.ignore_patterns(".git", "__pycache__", "*.pyc"))
subprocess.run(["git", "init"], cwd=dest, ...)
...
```

That ignore list only ever excluded `.git`, `__pycache__`, and `*.pyc`. It had
no entry for `rust/target` — the Rust workspace's build output directory,
introduced by WI052-056 and not (at the time) covered by `.gitignore` either —
so once a Rust build had run, every fixture in every test that used this
pattern also copied the full compiled `rust/target` tree (gigabytes) into a
throwaway directory. Dozens of tests used this pattern, so one Python test
run copied that multi-gigabyte tree dozens of times. Each disposable
repository also ran a normal `git init`, which leaves Git's default automatic
maintenance/gc behavior enabled, adding process/repack activity on top of the
storage growth.

Sites found (all confirmed and fixed in this pass):

- `tests/test_admission.py`
- `tests/test_admission_security.py`
- `tests/test_opt_in_provider.py`
- `tests/test_protected_substrate.py`
- `tests/test_guard_authority.py` (two independent fixtures per test class;
  the second was previously built by copying the *first fixture* rather than
  the real checkout — a fixture-of-fixture pattern that would have
  re-multiplied any future amplification)
- `tests/test_research_metadata.py`
- `tests/test_validate_repo.py` (the single largest contributor: most of its
  ~113 tests used this pattern)
- `repopact/run_admission_platform_conformance.py`

Sites inspected and found **not** to be part of the problem (they copy small,
fixed conformance-corpus fixtures, not the developer checkout, so they were
left unchanged):

- `repopact/run_conformance.py`
- `tests/test_conformance.py`

## Fix

See [repopact/dev_fixtures.py](../../../repopact/dev_fixtures.py) for the
implementation and [tests/test_dev_fixtures.py](../../../tests/test_dev_fixtures.py)
for its regression coverage. Summary:

- One centralized materializer (`materialize_source_tree` /
  `FixtureRepo` / `open_fixture_repo`) replaces every ad hoc
  `shutil.copytree` + `git init` block above.
- File selection is git-aware: `git ls-files --cached --others
  --exclude-standard` against the real checkout, then each selected path is
  copied from the *working tree* (so uncommitted edits participate, not just
  the last commit). This automatically excludes `.git` (never listed) and any
  ignored/generated output (`rust/target`, `node_modules`, `__pycache__`,
  `dist`, ...) without hand-maintaining a parallel ignore list, and lets new,
  non-ignored development files participate.
- `.gitignore` gained `/rust/target/` and `node_modules/`/`/rust/apps/*/dist/`
  entries — they were previously absent, so `--exclude-standard` would not
  have excluded them either.
- Every fixture created by this module gets automatic Git maintenance/gc
  disabled locally (`git config --local gc.auto=0`,
  `gc.autoPackLimit=0`, `maintenance.auto=false`) — never globally, and never
  on the real checkout.
- Cleanup is unconditional: `FixtureRepo` removes its temp holder in
  `close()`/`__exit__`, wrapped so a failure partway through materialization
  still cleans up, and `open_fixture_repo(self)` registers that cleanup with
  `unittest.TestCase.addCleanup` so it runs even if the test body or a later
  subprocess call fails.
- A fixture can never be built from another fixture: every holder directory
  carries a marker file, and materialization refuses to run if the requested
  source is under (or is) a previously created fixture.

## Proof

The regression suite (`tests/test_dev_fixtures.py`) proves tracked-file
inclusion, untracked-non-ignored-file inclusion, `.git` exclusion, ignored
build-output exclusion, `__pycache__`/`*.pyc` exclusion, non-inflating repeated
materialization, disposable-repo usability, and the disabled-maintenance
config — using a small synthetic source repo so the tests are fast and don't
depend on the real checkout's state.

One test (`test_real_checkout_fixture_excludes_rust_target_when_present`)
specifically exercises the real checkout's actual `rust/target` when present,
so the fix is proven against genuine multi-gigabyte build output, not just a
synthetic stand-in. On the Dell Precision, this was run with an actual `cargo
build --workspace` output present (`rust/target` ≈ 2.7-4.3 GB across the two
full baseline runs in this pass) and the full canonical Python suite passed
without the developer needing to delete that build output first. See the
WI059 activation baseline evidence for exact before/after disk and process
numbers.

## A second, related leak found during the same audit

Auditing every `TemporaryDirectory`/`mkdtemp` call in `tests/` (not just the
files already suspected of the `rust/target` amplification bug) turned up
`tests/test_takeover_refs.py`: both its `MapRef` and `RewriteInboundReferences`
test classes called `tempfile.mkdtemp()` in `setUp` with **no cleanup at
all** — no `tearDown`, no `addCleanup`. These fixtures are small (a handful of
synthetic work-item directories, not a checkout copy), so they were not part
of the multi-gigabyte failure, but every run of the suite left every one of
them behind permanently, which is exactly the "guarantee cleanup across
success and failure" property CVP-017 requires. Fixed by registering
`self.addCleanup(lambda: shutil.rmtree(self.tmp, ignore_errors=True))` right
after each `mkdtemp()` call.

That audit also found `FixtureRepo.close()`'s own robustness gap: the
synthetic source repos `tests/test_dev_fixtures.py` builds for its own
`setUp` were being torn down with a bare
`shutil.rmtree(holder, ignore_errors=True)`, which — unlike `FixtureRepo`'s
internal cleanup — had no read-only retry, so a `.git/objects` file Git had
just written could still be present (and locked/read-only) when cleanup ran,
and `ignore_errors=True` silently left the directory behind. `dev_fixtures.py`
now exports `robust_rmtree()` (the same chmod-and-retry `shutil.rmtree` used
internally by `FixtureRepo.close()`) so any caller building a throwaway
directory that will contain a Git repository can get the same guarantee
instead of reinventing it inconsistently.
