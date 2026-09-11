"""Regression coverage for the centralized disposable-repository materializer
(WI059 CVP-017). See repopact/dev_fixtures.py for the storage-amplification
history this replaces.
"""
from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

from repopact.dev_fixtures import (
    FixtureRepo,
    default_source_root,
    materialize_source_tree,
    open_fixture_repo,
    repository_source_files,
    robust_rmtree,
)


def _init_synthetic_source(root: Path) -> None:
    """A tiny standalone git repo standing in for a real dev checkout, so these
    tests do not depend on (or mutate) the actual RepoPact working tree.
    """
    root.mkdir(parents=True, exist_ok=True)
    (root / ".gitignore").write_text("ignored_build_output/\n__pycache__/\n*.pyc\n", encoding="utf-8")
    (root / "tracked.txt").write_text("tracked content v1\n", encoding="utf-8")
    (root / "pkg").mkdir()
    (root / "pkg" / "module.py").write_text("value = 1\n", encoding="utf-8")
    subprocess.run(["git", "init", "-q"], cwd=root, check=True, capture_output=True)
    subprocess.run(["git", "config", "--local", "user.email", "fixture@example.invalid"], cwd=root, check=True, capture_output=True)
    subprocess.run(["git", "config", "--local", "user.name", "Fixture Source"], cwd=root, check=True, capture_output=True)
    subprocess.run(["git", "add", "-A"], cwd=root, check=True, capture_output=True)
    subprocess.run(["git", "commit", "-q", "-m", "initial"], cwd=root, check=True, capture_output=True)

    # An ignored build-output directory, standing in for rust/target: present on
    # disk, never tracked, and must never be copied into a fixture.
    build_output = root / "ignored_build_output"
    build_output.mkdir()
    (build_output / "artifact.bin").write_bytes(b"\x00" * 1024)

    # Python bytecode caches, also ignored, also must never be copied.
    cache_dir = root / "pkg" / "__pycache__"
    cache_dir.mkdir()
    (cache_dir / "module.cpython-000.pyc").write_bytes(b"\x00" * 64)

    # A new, non-ignored development file that was never committed. It should
    # still participate, same as a file the developer just created.
    (root / "untracked_new_file.py").write_text("new = True\n", encoding="utf-8")


class DevFixturesTests(unittest.TestCase):
    def setUp(self) -> None:
        holder = tempfile.mkdtemp(prefix="repopact-dev-fixtures-test-")
        self.addCleanup(lambda: robust_rmtree(holder))
        self.source = Path(holder) / "source"
        _init_synthetic_source(self.source)

    def test_tracked_source_is_included_with_current_working_tree_content(self) -> None:
        # Modify a tracked file's working-tree content without committing; the
        # fixture must reflect the current content, not the last commit.
        (self.source / "tracked.txt").write_text("tracked content v2 (uncommitted)\n", encoding="utf-8")
        dest = self.source.parent / "dest-tracked"
        materialize_source_tree(self.source, dest)
        self.assertEqual((dest / "tracked.txt").read_text(encoding="utf-8"), "tracked content v2 (uncommitted)\n")

    def test_untracked_non_ignored_file_participates(self) -> None:
        dest = self.source.parent / "dest-untracked"
        materialize_source_tree(self.source, dest)
        self.assertTrue((dest / "untracked_new_file.py").is_file())

    def test_git_directory_is_never_copied(self) -> None:
        dest = self.source.parent / "dest-no-git"
        materialize_source_tree(self.source, dest)
        self.assertFalse((dest / ".git").exists())

    def test_ignored_build_output_is_excluded(self) -> None:
        dest = self.source.parent / "dest-no-build-output"
        materialize_source_tree(self.source, dest)
        self.assertFalse((dest / "ignored_build_output").exists())

    def test_python_bytecode_cache_is_excluded(self) -> None:
        dest = self.source.parent / "dest-no-pycache"
        materialize_source_tree(self.source, dest)
        self.assertFalse((dest / "pkg" / "__pycache__").exists())

    def test_repeated_materialization_does_not_inflate(self) -> None:
        first = self.source.parent / "dest-first"
        second = self.source.parent / "dest-second"
        materialize_source_tree(self.source, first)
        materialize_source_tree(self.source, second)
        first_files = sorted(p.relative_to(first) for p in first.rglob("*") if p.is_file())
        second_files = sorted(p.relative_to(second) for p in second.rglob("*") if p.is_file())
        self.assertEqual(first_files, second_files)
        self.assertEqual(set(repository_source_files(self.source)), {str(p).replace("\\", "/") for p in first_files})

    def test_fixture_cannot_be_materialized_from_another_fixture(self) -> None:
        with FixtureRepo(source_root=self.source) as fixture:
            with self.assertRaises(RuntimeError):
                materialize_source_tree(fixture.root, fixture.root.parent / "nested")

    def test_fixture_is_usable_as_a_disposable_git_repository(self) -> None:
        with FixtureRepo(source_root=self.source) as fixture:
            status = subprocess.run(["git", "status", "--porcelain"], cwd=fixture.root, check=True, capture_output=True, text=True)
            self.assertEqual(status.stdout.strip(), "")
            log = subprocess.run(["git", "log", "--oneline"], cwd=fixture.root, check=True, capture_output=True, text=True)
            self.assertIn("fixture", log.stdout)

    def test_fixture_disables_automatic_git_maintenance(self) -> None:
        with FixtureRepo(source_root=self.source) as fixture:
            for key, expected in (
                ("gc.auto", "0"),
                ("gc.autopacklimit", "0"),
                ("maintenance.auto", "false"),
            ):
                value = subprocess.run(
                    ["git", "config", "--local", "--get", key], cwd=fixture.root, check=True, capture_output=True, text=True,
                ).stdout.strip()
                self.assertEqual(value, expected, key)

    def test_fixture_cleanup_removes_the_temp_directory_on_close(self) -> None:
        fixture = FixtureRepo(source_root=self.source)
        fixture.open()
        root = fixture.root
        holder = root.parent
        self.assertTrue(holder.exists())
        fixture.close()
        self.assertFalse(holder.exists())

    def test_cleanup_runs_even_when_materialization_fails(self) -> None:
        # source_root pointing at a non-repository directory makes `git ls-files`
        # fail partway through open(); the temp holder it already created must
        # still be removed rather than orphaned.
        not_a_repo = self.source.parent / "not-a-repo"
        not_a_repo.mkdir()
        fixture = FixtureRepo(source_root=not_a_repo, prefix="repopact-dev-fixtures-fail-")
        with self.assertRaises(subprocess.CalledProcessError):
            fixture.open()
        self.assertIsNone(fixture.root)

    def test_addcleanup_helper_removes_fixture_after_test_case_tears_down(self) -> None:
        source = self.source

        class Inner(unittest.TestCase):
            def runTest(self) -> None:
                self.captured_root = open_fixture_repo(self, source_root=source)

        case = Inner()
        result = case.run()
        self.assertTrue(result.wasSuccessful())
        self.assertFalse(case.captured_root.parent.exists())

    def test_real_checkout_fixture_excludes_rust_target_when_present(self) -> None:
        real_root = default_source_root()
        target_dir = real_root / "rust" / "target"
        if not target_dir.exists():
            self.skipTest("rust/target not present on this checkout; run after a Rust build to exercise exclusion")
        with FixtureRepo(source_root=real_root, init_git=False) as fixture:
            self.assertFalse((fixture.root / "rust" / "target").exists())
            self.assertFalse((fixture.root / ".git").exists())
            self.assertTrue((fixture.root / "pyproject.toml").is_file())


if __name__ == "__main__":
    unittest.main()
