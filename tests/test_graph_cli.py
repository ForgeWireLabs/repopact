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
