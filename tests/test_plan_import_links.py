"""Unit tests for import-plan narrative link rewriting (`_rewrite_narrative_links`).

Regression guard for the failure mode where `import-plan` lifted a plan README into
`work/<status>/<id>/` verbatim: relative links no longer resolved at the new depth,
and cross-item links kept pointing at the legacy plan tree that `takeover` deletes —
shipping dangling links throughout `work/`.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from plan_import import _rewrite_narrative_links, _first_heading  # noqa: E402

# A realistic slice of the plan->work map an import pass would build.
PLAN_TO_WORK = {
    "todos/114-forgewire-fabric": "work/active/114-forgewire-fabric",
    "todos/completed/104-execution-graph": "work/completed/104-execution-graph",
    "todos/completed/111-ncs-shard-runtime": "work/completed/111-ncs-shard-runtime",
}
SRC = "todos/114-forgewire-fabric"          # source item (a dir item)
WI = "work/active/114-forgewire-fabric"      # its new work home


def rewrite(narrative: str) -> str:
    return _rewrite_narrative_links(narrative, SRC, WI, PLAN_TO_WORK)


class RewriteNarrativeLinks(unittest.TestCase):
    def test_cross_item_link_remapped_and_rebased(self) -> None:
        # link to a sibling completed item resolves to its work README at correct depth
        out = rewrite("see [104](../completed/104-execution-graph/README.md)")
        self.assertIn("(../../completed/104-execution-graph/README.md)", out)
        self.assertNotIn("todos/", out)

    def test_cross_item_detail_file_collapses_to_readme(self) -> None:
        # a detail file inside another item is not copied to work/ -> collapse to README
        out = rewrite("schema: [d](../completed/111-ncs-shard-runtime/milestone-0.md)")
        self.assertIn("(../../completed/111-ncs-shard-runtime/README.md)", out)

    def test_same_item_detail_link_becomes_self_readme(self) -> None:
        out = rewrite("detail: [p1](phase-1-lan-cluster.md)")
        self.assertIn("(README.md)", out)
        self.assertNotIn("phase-1-lan-cluster.md", out)

    def test_non_plan_repo_link_depth_rebased(self) -> None:
        # a link to a repo file outside the plan tree keeps its target but is rebased
        # for the deeper work/<status>/<id>/ location (../../ -> ../../../).
        out = rewrite("code: [bus](../../forgewire_core/bus/channels.py)")
        self.assertIn("(../../../forgewire_core/bus/channels.py)", out)

    def test_urls_and_anchors_untouched(self) -> None:
        for target in ("https://example.com/x", "#a-section", "mailto:x@y.z"):
            src = f"[x]({target})"
            self.assertEqual(rewrite(src), src)

    def test_anchor_preserved_on_rewrite(self) -> None:
        out = rewrite("[x](../completed/104-execution-graph/README.md#design)")
        self.assertIn("(../../completed/104-execution-graph/README.md#design)", out)

    def test_no_links_left_pointing_at_legacy_plan_dir(self) -> None:
        narrative = (
            "Depends on [104](../completed/104-execution-graph/) and "
            "[111](../completed/111-ncs-shard-runtime/README.md); "
            "see [self](phase-0-foundations.md)."
        )
        out = rewrite(narrative)
        self.assertNotIn("todos/", out)
        self.assertNotIn("../completed/104-execution-graph/)", out)  # got rebased


class FirstHeadingStripsTrackerNumber(unittest.TestCase):
    """Heading titles must not carry the tracker number; the importer adds the id."""

    def test_strips_leading_number_em_dash(self) -> None:
        self.assertEqual(_first_heading("# 107 — FCR LLM Runtime\n", "fb"), "FCR LLM Runtime")

    def test_strips_leading_number_colon_and_hyphen(self) -> None:
        self.assertEqual(_first_heading("# 24: Recursive LLM\n", "fb"), "Recursive LLM")
        self.assertEqual(_first_heading("# 5 - Phase Plan\n", "fb"), "Phase Plan")

    def test_keeps_titles_without_a_tracker_number(self) -> None:
        self.assertEqual(_first_heading("# FCR LLM Runtime\n", "fb"), "FCR LLM Runtime")

    def test_does_not_eat_slug_or_year_titles(self) -> None:
        # no whitespace after the separator -> not the "NNN — Title" convention
        self.assertEqual(_first_heading("# 21-multi-agent\n", "fb"), "21-multi-agent")
        # number followed by a space (no separator) is a real title word
        self.assertEqual(_first_heading("# 2026 Roadmap\n", "fb"), "2026 Roadmap")

    def test_fallback_unchanged_when_no_heading(self) -> None:
        self.assertEqual(_first_heading("no heading here\n", "Fallback Title"), "Fallback Title")


if __name__ == "__main__":
    unittest.main()
