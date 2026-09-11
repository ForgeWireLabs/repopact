"""Disposable Git-tracked source-tree fixtures for the regression suite.

Several tests and conformance harnesses need a throwaway *copy* of (most of) the
RepoPact source tree that they can freely mutate, `git init`, and commit to. The
old approach in each caller was an ad hoc ``shutil.copytree(repo_root, dest,
ignore=shutil.ignore_patterns(".git", "__pycache__", "*.pyc"))``, which recursively
copies every other byte under the checkout — including generated build output
such as ``rust/target`` that is not covered by that hand-rolled ignore list. With
a real Rust build present, that turned every fixture into a multi-gigabyte copy,
and with dozens of fixtures created per run (plus nested `git add -A`/gc activity
inside each disposable repo) the effect was multi-gigabyte-per-run storage and
process amplification unrelated to whatever the test was actually checking.

This module is the single place that decides what belongs in a disposable
fixture. It selects files the same way ``git`` itself would report the working
tree: currently tracked files (read from the working tree, not the index blob,
so uncommitted edits participate) plus untracked files that are not ignored.
That automatically excludes ``.git`` (never listed by ``git ls-files``) and any
present-or-future ignored build output (``rust/target``, ``node_modules``,
``__pycache__``, ``dist``, ...) without hand-maintaining a parallel ignore list,
while still letting new, non-ignored development files participate.

Fixtures created here also get automatic Git maintenance disabled locally
(never globally, and never on the real checkout) so a disposable repository
cannot spawn background `gc`/repack activity, and cleanup is unconditional:
callers get a context manager or a `unittest.TestCase.addCleanup`-based helper
so a fixture is removed whether setup, the test body, or a subprocess call
fails.
"""

from __future__ import annotations

import os
import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

# Local-only (git config --local) settings applied inside every disposable
# fixture repository to prevent automatic maintenance/gc/repack activity that
# is irrelevant to what the test under it is checking. Never applied globally
# and never applied to the real RepoPact checkout.
DISPOSABLE_GIT_MAINTENANCE_CONFIG: dict[str, str] = {
    "gc.auto": "0",
    "gc.autoDetach": "false",
    "gc.autoPackLimit": "0",
    "maintenance.auto": "false",
}


def default_source_root() -> Path:
    """The real RepoPact checkout this process is running from."""
    return Path(__file__).resolve().parents[1]


FIXTURE_MARKER_NAME = ".repopact-fixture-marker"


def _guard_not_a_fixture(source_root: Path) -> None:
    """Refuse to materialize a fixture using another fixture created by this
    module as the source. Every holder directory :class:`FixtureRepo` creates
    carries a marker file; walking up from ``source_root`` catches both "this
    path is a fixture root" and "this path is somewhere inside one", so
    fixtures can never recursively include earlier fixtures. A source that
    merely happens to live under the system temp directory (e.g. a synthetic
    git repo built by a unit test) is unaffected.
    """
    resolved = source_root.resolve()
    for ancestor in (resolved, *resolved.parents):
        if (ancestor / FIXTURE_MARKER_NAME).exists():
            raise RuntimeError(
                f"refusing to materialize a fixture repo from another disposable "
                f"fixture ({resolved} is under a directory created by FixtureRepo); "
                "fixtures must be built from the real checkout (default_source_root())."
            )


def repository_source_files(source_root: Path) -> list[str]:
    """Repo-relative paths that belong in a fixture: tracked files (current
    working-tree content) plus untracked-but-not-ignored files. Ignored/generated
    output and ``.git`` are excluded because git itself excludes them from this
    listing.
    """
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=source_root,
        check=True,
        capture_output=True,
    )
    return [p.decode("utf-8") for p in result.stdout.split(b"\0") if p]


def materialize_source_tree(source_root: Path, dest_root: Path) -> Path:
    """Copy ``source_root``'s current git-tracked/non-ignored working tree into
    a fresh ``dest_root`` (must not already exist).
    """
    _guard_not_a_fixture(source_root)
    dest_root.mkdir(parents=True, exist_ok=False)
    for rel in repository_source_files(source_root):
        src_file = source_root / rel
        if not src_file.is_file():
            continue
        dest_file = dest_root / rel
        dest_file.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src_file, dest_file)
    return dest_root


def configure_disposable_git_maintenance(repo_root: Path) -> None:
    """Disable automatic Git maintenance/gc/repack inside a disposable fixture
    repository. Uses ``git config --local`` only: never touches global config,
    and only ever runs against paths this module created.
    """
    for key, value in DISPOSABLE_GIT_MAINTENANCE_CONFIG.items():
        subprocess.run(
            ["git", "config", "--local", key, value],
            cwd=repo_root,
            check=True,
            capture_output=True,
        )


def init_disposable_git_repo(
    repo_root: Path,
    *,
    user_email: str = "fixture@example.invalid",
    user_name: str = "RepoPact fixture",
    commit_message: str = "fixture",
) -> None:
    """``git init`` a disposable fixture, disable its automatic maintenance, and
    commit everything currently materialized in it.
    """
    subprocess.run(["git", "init", "-q"], cwd=repo_root, check=True, capture_output=True)
    configure_disposable_git_maintenance(repo_root)
    subprocess.run(["git", "config", "--local", "user.email", user_email], cwd=repo_root, check=True, capture_output=True)
    subprocess.run(["git", "config", "--local", "user.name", user_name], cwd=repo_root, check=True, capture_output=True)
    subprocess.run(["git", "add", "-A"], cwd=repo_root, check=True, capture_output=True)
    subprocess.run(["git", "commit", "-q", "-m", commit_message], cwd=repo_root, check=True, capture_output=True)


def _clear_readonly_and_retry(func, path, _exc_info) -> None:
    """``shutil.rmtree`` onerror hook: some files inside a fixture's ``.git``
    directory can end up read-only; drop that bit and retry once instead of
    leaving an orphaned temp directory behind.
    """
    try:
        os.chmod(path, stat.S_IWRITE)
        func(path)
    except OSError:
        pass


def robust_rmtree(path: Path | str) -> None:
    """``shutil.rmtree`` with the same read-only retry :class:`FixtureRepo`
    uses internally. Any caller building a throwaway directory that will end
    up containing a Git repository (read-only objects included) should use
    this instead of a bare ``shutil.rmtree(..., ignore_errors=True)``, which
    silently leaves the directory behind on the exact same failure this
    module was written to eliminate.
    """
    shutil.rmtree(path, onerror=_clear_readonly_and_retry)


class FixtureRepo:
    """A disposable, task-owned copy of the RepoPact source tree.

    Materializes on ``__enter__``/:meth:`open`; guarantees removal on
    ``__exit__``/:meth:`close`, including when materialization itself fails
    partway through.
    """

    def __init__(
        self,
        *,
        source_root: Path | None = None,
        prefix: str = "repopact-fixture-",
        init_git: bool = True,
        tmp_root: Path | None = None,
    ) -> None:
        self._source_root = source_root or default_source_root()
        self._prefix = prefix
        self._init_git = init_git
        self._tmp_root = tmp_root
        self._tmp_holder: str | None = None
        self.root: Path | None = None

    def open(self) -> "FixtureRepo":
        self._tmp_holder = tempfile.mkdtemp(
            prefix=self._prefix,
            dir=str(self._tmp_root) if self._tmp_root else None,
        )
        try:
            (Path(self._tmp_holder) / FIXTURE_MARKER_NAME).touch()
            root = Path(self._tmp_holder) / "repo"
            materialize_source_tree(self._source_root, root)
            if self._init_git:
                init_disposable_git_repo(root)
            self.root = root
        except Exception:
            self.close()
            raise
        return self

    def close(self) -> None:
        holder, self._tmp_holder = self._tmp_holder, None
        self.root = None
        if holder is not None:
            robust_rmtree(holder)

    def __enter__(self) -> "FixtureRepo":
        return self.open()

    def __exit__(self, exc_type, exc, tb) -> None:
        self.close()


def open_fixture_repo(
    test_case: unittest.TestCase,
    *,
    init_git: bool = True,
    prefix: str = "repopact-fixture-",
    tmp_root: Path | None = None,
    source_root: Path | None = None,
) -> Path:
    """Materialize a :class:`FixtureRepo` and register its cleanup with
    ``test_case.addCleanup`` so it is removed even if the rest of ``setUp``, the
    test body, or a subprocess call later fails. Returns the fixture root.
    """
    fixture = FixtureRepo(source_root=source_root, prefix=prefix, init_git=init_git, tmp_root=tmp_root)
    fixture.open()
    test_case.addCleanup(fixture.close)
    return fixture.root
