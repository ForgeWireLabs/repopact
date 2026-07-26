from __future__ import annotations

import shutil
import tempfile
import unittest
from datetime import date
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]

from repopact import validate_research  # noqa: E402
from repopact.validate_repo import validate as validate_repo  # noqa: E402


class ResearchMetadataTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / "repo"
        shutil.copytree(
            ROOT,
            self.root,
            ignore=shutil.ignore_patterns(
                ".git", ".venv", ".pytest_cache", "__pycache__", "build", "dist", "*.egg-info"
            ),
        )

    def tearDown(self) -> None:
        self.temp.cleanup()

    def messages(self) -> list[str]:
        return [problem.message for problem in validate_research.validate(self.root)]

    def replace(self, relative: str, old: str, new: str) -> None:
        path = self.root / relative
        text = path.read_text(encoding="utf-8")
        self.assertIn(old, text)
        path.write_text(text.replace(old, new, 1), encoding="utf-8")

    def test_canonical_research_records_validate(self) -> None:
        self.assertEqual([], self.messages())

    def test_expired_research_claim_contract_is_rejected(self) -> None:
        problems = validate_research.validate(self.root, today=date(2026, 8, 10))
        self.assertTrue(any("freshness expired on 2026-08-09" in p.message for p in problems))

    def test_unregistered_research_claim_document_is_rejected(self) -> None:
        (self.root / "research" / "new-claim.md").write_text(
            "# New current claim\n",
            encoding="utf-8",
        )
        self.assertTrue(any("missing research/new-claim.md" in message for message in self.messages()))

    def test_repeated_threat_identifier_is_rejected_by_repo_gate(self) -> None:
        self.replace("research/threats-to-validity.md", "## T10 —", "## T7 —")
        messages = [problem.message for problem in validate_repo(self.root)]
        self.assertTrue(any("repeated T7" in message for message in messages))
        self.assertTrue(any("missing T10" in message for message in messages))

    def test_lifecycle_figure_without_proposed_is_rejected(self) -> None:
        self.replace("research/figures.md", "│ proposed │", "│ candidate │")
        self.assertTrue(any("missing state(s): proposed" in message for message in self.messages()))

    def test_stale_pactbench_task_count_is_rejected(self) -> None:
        self.replace("research/figures.md", "24 pre-registered PactBench tasks", "21 pre-registered PactBench tasks")
        self.assertTrue(any("expected 24; observed 21" in message for message in self.messages()))

    def test_stale_hypothesis_range_is_rejected(self) -> None:
        self.replace("research/benchmark-protocol.md", "H8–H13", "H8–H10")
        self.assertTrue(any("expected H8–H13; observed H8–H10" in message for message in self.messages()))

    def test_future_provenance_wording_is_rejected(self) -> None:
        self.replace(
            "research/figures.md",
            "RepoPact 2.0 shipped the resolution",
            "Provenance-typed records are the principled future escape",
        )
        self.assertTrue(any("not future work" in message for message in self.messages()))


if __name__ == "__main__":
    unittest.main()
