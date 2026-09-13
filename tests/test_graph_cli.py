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


if __name__ == "__main__":
    unittest.main()
