"""Tests for the takeover inbound-reference guard (`rewrite_inbound_references`).

Regression guard for the failure mode where `takeover --delete` removed a heavily
referenced plan directory and left every inbound reference (docs links, frontmatter,
code paths) dangling — RepoPact validation does not check arbitrary link targets, so
nothing caught it.
"""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import takeover  # noqa: E402


def _work_item(root: Path, status: str, dir_name: str) -> None:
    d = root / "work" / status / dir_name
    d.mkdir(parents=True)
    (d / "work-item.json").write_text(json.dumps({"id": dir_name.split("-")[0]}), encoding="utf-8")
    (d / "README.md").write_text(f"# {dir_name}\n", encoding="utf-8")


class MapRef(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp())
        _work_item(self.tmp, "completed", "107-fcr-llm-awq-runtime")
        _work_item(self.tmp, "active", "114-forgewire-fabric")
        self.idx = takeover._work_index(self.tmp)

    def _map(self, tok: str) -> str | None:
        return takeover._map_ref(tok, {"todos"}, self.idx, self.tmp)

    def test_preserves_relative_depth(self) -> None:
        self.assertEqual(self._map("../../todos/completed/107-fcr-llm-awq-runtime/README.md"),
                         "../../work/completed/107-fcr-llm-awq-runtime/README.md")

    def test_lifecycle_segment_optional(self) -> None:
        # reference omits the completed/ segment -> matched by item basename
        self.assertEqual(self._map("todos/107-fcr-llm-awq-runtime/README.md"),
                         "work/completed/107-fcr-llm-awq-runtime/README.md")

    def test_unmigrated_detail_file_collapses_to_readme(self) -> None:
        self.assertEqual(self._map("todos/active/114-forgewire-fabric/phase-1-lan-cluster.md"),
                         "work/active/114-forgewire-fabric/README.md")

    def test_non_retired_dir_untouched(self) -> None:
        self.assertIsNone(self._map("tasks/107-fcr-llm-awq-runtime/README.md"))

    def test_unknown_item_returns_none(self) -> None:  # e.g. a merged-away dir with no work item
        self.assertIsNone(self._map("todos/completed/999-ghost/README.md"))


class RewriteInboundReferences(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp())
        _work_item(self.tmp, "completed", "107-fcr-llm-awq-runtime")
        (self.tmp / "docs").mkdir()
        (self.tmp / "docs" / "guide.md").write_text(
            "---\nsource_of_truth: todos/completed/107-fcr-llm-awq-runtime/README.md\n---\n"
            "See [107](../todos/completed/107-fcr-llm-awq-runtime/README.md).\n",
            encoding="utf-8")
        # a code path that must NOT be auto-rewritten, only reported
        (self.tmp / "test_x.py").write_text(
            'P = Path("todos/completed/107-fcr-llm-awq-runtime/README.md")\n', encoding="utf-8")

    def test_rewrites_link_and_frontmatter_reports_code(self) -> None:
        report: dict = {}
        takeover.rewrite_inbound_references(self.tmp, {"todos"}, dry_run=False, report=report)
        guide = (self.tmp / "docs" / "guide.md").read_text(encoding="utf-8")
        self.assertIn("(../work/completed/107-fcr-llm-awq-runtime/README.md)", guide)
        self.assertIn("source_of_truth: work/completed/107-fcr-llm-awq-runtime/README.md", guide)
        self.assertNotIn("todos/", guide)
        self.assertIn("docs/guide.md", report["refs_rewritten"])
        # the code Path() is reported, not silently rewritten
        self.assertTrue(any("test_x.py" in u for u in report["refs_unresolved"]))
        self.assertIn('todos/', (self.tmp / "test_x.py").read_text(encoding="utf-8"))

    def test_dry_run_writes_nothing(self) -> None:
        before = (self.tmp / "docs" / "guide.md").read_text(encoding="utf-8")
        report: dict = {}
        takeover.rewrite_inbound_references(self.tmp, {"todos"}, dry_run=True, report=report)
        self.assertEqual((self.tmp / "docs" / "guide.md").read_text(encoding="utf-8"), before)
        self.assertIn("docs/guide.md", report["refs_rewritten"])  # still reports what it would do


if __name__ == "__main__":
    unittest.main()
