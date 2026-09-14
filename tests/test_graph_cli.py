from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from repopact import graph_cli


class GraphCliTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="repopact-graph-cli-")
        self.root = Path(self._tmp.name)
        (self.root / "src").mkdir()
        (self.root / "README.md").write_text("# fixture\n", encoding="utf-8")
        (self.root / "src" / "lib.rs").write_text("pub fn x() {}\n", encoding="utf-8")

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def run_cli(self, *args: str) -> int:
        return graph_cli.main([*args, "--root", str(self.root), "--json"])

    def capture(self, *args: str) -> tuple[int, dict]:
        import io
        import contextlib

        buffer = io.StringIO()
        with contextlib.redirect_stdout(buffer):
            code = self.run_cli(*args)
        return code, json.loads(buffer.getvalue())

    def test_status_on_disabled_repository_is_absent_and_exits_zero(self) -> None:
        code, result = self.capture("status")
        self.assertEqual(code, 0)
        self.assertEqual(result["freshness"], "absent")
        self.assertEqual(result["diagnostics"], [])

    def test_build_then_status_is_fresh(self) -> None:
        build_code, build_result = self.capture("build")
        self.assertEqual(build_code, 0)
        self.assertGreater(build_result["node_count"], 0)

        status_code, status_result = self.capture("status")
        self.assertEqual(status_code, 0)
        self.assertEqual(status_result["freshness"], "fresh")

    def test_source_change_after_build_is_reported_stale(self) -> None:
        self.capture("build")
        (self.root / "src" / "lib.rs").write_text("pub fn x() { /* changed */ }\n", encoding="utf-8")
        code, result = self.capture("verify")
        self.assertEqual(code, 1)
        self.assertEqual(result["freshness"], "stale")

    def test_update_with_no_prior_graph_is_a_full_fallback(self) -> None:
        code, result = self.capture("update")
        self.assertEqual(code, 0)
        self.assertEqual(result["mode"], "full_fallback")
        self.assertEqual(result["fallback_reason"], "graph_absent")

    def test_update_after_build_with_no_change_is_a_true_no_op(self) -> None:
        self.capture("build")
        code, result = self.capture("update")
        self.assertEqual(code, 0)
        self.assertEqual(result["mode"], "no_op")
        self.assertEqual(result["semantic_reparsed"], 0)

    def test_update_after_one_file_change_reuses_the_rest(self) -> None:
        (self.root / "src" / "other.rs").write_text("pub fn y() {}\n", encoding="utf-8")
        self.capture("build")
        (self.root / "src" / "lib.rs").write_text("pub fn x() { /* changed */ }\n", encoding="utf-8")
        code, result = self.capture("update")
        self.assertEqual(code, 0)
        self.assertEqual(result["mode"], "incremental")
        self.assertEqual(result["files_modified"], 1)
        self.assertEqual(result["semantic_reparsed"], 1)
        self.assertGreaterEqual(result["semantic_reused"], 1)

        verify_code, verify_result = self.capture("verify")
        self.assertEqual(verify_code, 0)
        self.assertEqual(verify_result["freshness"], "fresh")

    # ---- ROG-039/015: explicit capability lifecycle via the CLI --------

    def test_build_then_status_reports_explicit_enabled(self) -> None:
        self.capture("build")
        code, result = self.capture("status")
        self.assertEqual(code, 0)
        self.assertEqual(result["capability_state"], "explicit_enabled")

    def test_disable_removes_graph_and_reports_explicit_disabled(self) -> None:
        self.capture("build")
        code, result = self.capture("disable")
        self.assertEqual(code, 0)
        self.assertTrue(result["disabled"])
        self.assertFalse((self.root / "rog").exists())

        status_code, status_result = self.capture("status")
        self.assertEqual(status_code, 0)
        self.assertEqual(status_result["freshness"], "absent")
        self.assertEqual(status_result["capability_state"], "explicit_disabled")

    def test_build_disable_rebuild_cycle_via_cli(self) -> None:
        self.capture("build")
        self.capture("disable")
        code, result = self.capture("build")
        self.assertEqual(code, 0)
        status_code, status_result = self.capture("status")
        self.assertEqual(status_code, 0)
        self.assertEqual(status_result["capability_state"], "explicit_enabled")

    def test_read_only_commands_never_enable_capability(self) -> None:
        # Before any build, capability must be legacy_absent; status/
        # verify are read-only and must never change that.
        self.capture("status")
        self.capture("verify")
        code, result = self.capture("status")
        self.assertEqual(code, 0)
        self.assertEqual(result["capability_state"], "legacy_absent")
        self.assertFalse((self.root / "rog").exists())
        self.assertFalse((self.root / "governance" / "rog-capability.json").exists())


class GraphReconcileMergeCliTests(unittest.TestCase):
    """ROG-029, Decision 0052 section 4: `repopact graph reconcile-merge`
    through the real Python CLI. The deep branch/merge/authoritative-
    conflict/capability-conflict proofs live in
    rust/crates/repopact-graph/src/merge_reconcile.rs (real git
    subprocess tests); this class proves the CLI itself is wired
    correctly end to end."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="repopact-reconcile-cli-")
        self.root = Path(self._tmp.name)
        import subprocess

        self._git_available = (
            subprocess.run(["git", "--version"], capture_output=True).returncode == 0
        )
        if not self._git_available:
            return
        (self.root / "src").mkdir()
        (self.root / "src" / "lib.rs").write_text("pub fn x() {}\n", encoding="utf-8")
        (self.root / "Cargo.toml").write_text(
            '[package]\nname = "fixture"\n', encoding="utf-8"
        )

        def git(*args: str) -> None:
            subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True)

        self._git = git
        git("init", "--quiet")
        git("config", "user.email", "test@example.invalid")
        git("config", "user.name", "Test")
        git("add", "-A")
        git("commit", "--quiet", "-m", "source")

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def run_cli(self, *args: str) -> int:
        return graph_cli.main([*args, "--root", str(self.root), "--json"])

    def capture(self, *args: str) -> tuple[int, dict]:
        import io
        import contextlib

        buffer = io.StringIO()
        with contextlib.redirect_stdout(buffer):
            code = self.run_cli(*args)
        return code, json.loads(buffer.getvalue())

    def test_no_conflicts_reports_nothing_to_reconcile(self) -> None:
        if not self._git_available:
            self.skipTest("git unavailable")
        code, result = self.capture("reconcile-merge")
        self.assertEqual(code, 0)
        self.assertEqual(result["outcome"], "nothing_to_reconcile")

    def test_authoritative_conflict_is_refused_with_a_nonzero_exit(self) -> None:
        if not self._git_available:
            self.skipTest("git unavailable")
        import subprocess

        self._git("checkout", "-b", "branch-a")
        (self.root / "src" / "lib.rs").write_text("pub fn x() { /* a */ }\n", encoding="utf-8")
        self._git("commit", "-a", "--quiet", "-m", "a")
        if subprocess.run(["git", "checkout", "main"], cwd=self.root, capture_output=True).returncode != 0:
            self._git("checkout", "master")
        self._git("checkout", "-b", "branch-b")
        (self.root / "src" / "lib.rs").write_text("pub fn x() { /* b */ }\n", encoding="utf-8")
        self._git("commit", "-a", "--quiet", "-m", "b")
        self._git("checkout", "branch-a")
        import subprocess

        subprocess.run(
            ["git", "merge", "--no-edit", "branch-b"], cwd=self.root, capture_output=True
        )
        code = self.run_cli("reconcile-merge")
        self.assertEqual(code, 1)


class GraphQueryCliTests(unittest.TestCase):
    """WI063 bounded-query-and-orientation checkpoint (ROG-023/024/026):
    the Python CLI is a thin typed-selector-to-engine-JSON adapter -- it
    never scrapes presentation text. Every assertion here reads the
    engine's own typed JSON fields directly."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="repopact-graph-query-cli-")
        self.root = Path(self._tmp.name)
        (self.root / "src").mkdir()
        (self.root / "src" / "lib.rs").write_text("pub fn hello() {}\n", encoding="utf-8")
        work_dir = self.root / "work" / "active" / "900"
        work_dir.mkdir(parents=True)
        (work_dir / "work-item.json").write_text(
            json.dumps(
                {
                    "id": "900",
                    "title": "Fixture item",
                    "status": "active",
                    "owner_scope": "work",
                    "affected_scopes": [],
                    "depends_on": [],
                    "acceptance_criteria": [
                        {"id": "AC-1", "text": "prove", "state": "pending", "evidence": []}
                    ],
                    "created": "2026-01-01",
                    "updated": "2026-01-01",
                }
            ),
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def run_cli(self, *args: str) -> int:
        return graph_cli.main([*args, "--root", str(self.root), "--json"])

    def capture(self, *args: str) -> tuple[int, dict]:
        import io
        import contextlib

        buffer = io.StringIO()
        with contextlib.redirect_stdout(buffer):
            code = self.run_cli(*args)
        return code, json.loads(buffer.getvalue())

    def test_resolve_before_a_durable_graph_exists_reports_graph_absent(self) -> None:
        code, result = self.capture("resolve", "--work-item", "900")
        self.assertEqual(code, 0)
        self.assertTrue(result["graph_absent"])

    def test_resolve_work_item_by_typed_selector(self) -> None:
        self.capture("build")
        code, result = self.capture("resolve", "--work-item", "900")
        self.assertEqual(code, 0)
        self.assertEqual(result["result"]["outcome"], "exact")
        self.assertEqual(result["result"]["fact"]["id"], "work:900")

    def test_search_finds_a_work_item_by_bounded_text(self) -> None:
        self.capture("build")
        code, result = self.capture("search", "900")
        self.assertEqual(code, 0)
        matches = result["result"]["matches"]
        self.assertTrue(any(match["node"]["id"] == "work:900" for match in matches))

    def test_resolve_repository_path(self) -> None:
        self.capture("build")
        code, result = self.capture("resolve", "--path", "src/lib.rs")
        self.assertEqual(code, 0)
        self.assertEqual(result["result"]["fact"]["id"], "file:src/lib.rs")

    def test_resolve_not_found_is_distinct_from_a_crash(self) -> None:
        self.capture("build")
        code, result = self.capture("resolve", "--work-item", "does-not-exist")
        self.assertEqual(code, 0)
        self.assertEqual(result["result"]["outcome"], "not_found")

    def test_dependencies_uses_resolved_node_id(self) -> None:
        self.capture("build")
        code, result = self.capture("context", "--work-item", "900")
        self.assertEqual(code, 0)
        self.assertEqual(result["result"]["identity"]["id"], "work:900")

    def test_orient_returns_facts_and_navigation_hints_separately(self) -> None:
        self.capture("build")
        code, result = self.capture("orient", "--work-item", "900")
        self.assertEqual(code, 0)
        self.assertEqual(result["result"]["outcome"], "resolved")
        self.assertIn("navigation_hints", result["result"])
        self.assertIn("direct_dependencies", result["result"])

    def test_tests_command_discloses_fixture_coverage_warning(self) -> None:
        self.capture("build")
        code, result = self.capture("tests", "--work-item", "900")
        self.assertEqual(code, 0)
        self.assertTrue(any("fixture" in warning for warning in result["warnings"]))

    def test_query_contract_version_is_present(self) -> None:
        self.capture("build")
        code, result = self.capture("resolve", "--work-item", "900")
        self.assertEqual(code, 0)
        self.assertEqual(result["query_contract_version"], 1)


if __name__ == "__main__":
    unittest.main()
