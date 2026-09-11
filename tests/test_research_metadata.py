from __future__ import annotations

import json
import unittest
from datetime import date
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]

from repopact import validate_research  # noqa: E402
from repopact.dev_fixtures import open_fixture_repo  # noqa: E402
from repopact.validate_repo import validate as validate_repo  # noqa: E402


class ResearchMetadataTests(unittest.TestCase):
    def setUp(self) -> None:
        self.root = open_fixture_repo(self, prefix="repopact-research-metadata-", init_git=False)

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
        problems = validate_research.validate(self.root, today=date(2026, 9, 19))
        self.assertTrue(any("freshness expired on 2026-09-18" in p.message for p in problems))

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

    def _set_metadata_field(self, path: str, value) -> None:
        metadata_path = self.root / "research" / "metadata.json"
        data = json.loads(metadata_path.read_text(encoding="utf-8"))
        node = data
        *parents, leaf = path.split(".")
        for key in parents:
            node = node[key]
        node[leaf] = value
        metadata_path.write_text(json.dumps(data), encoding="utf-8")

    def test_freshness_policy_escaping_repository_is_rejected(self) -> None:
        outside = self.root.parent / "outside-policy.md"
        outside.write_text("# Not part of this repository\n", encoding="utf-8")
        self.addCleanup(outside.unlink, missing_ok=True)
        self._set_metadata_field("claim_freshness.policy", "../outside-policy.md")
        messages = self.messages()
        self.assertTrue(any("escapes the repository" in message for message in messages))
        self.assertFalse(any("Not part of this repository" in message for message in messages))

    def test_benchmark_source_escaping_repository_is_rejected(self) -> None:
        self._set_metadata_field("benchmark.pactbench.source", "../../etc/outside-source.json")
        messages = self.messages()
        self.assertTrue(any("escapes the repository" in message for message in messages))

    def test_trace_target_escaping_repository_is_rejected(self) -> None:
        self._set_metadata_field("proposed_state_trace.work_item", "../outside-work-item.json")
        messages = self.messages()
        self.assertTrue(any("escapes the repository" in message for message in messages))

    def test_lifecycle_figure_without_proposed_is_rejected(self) -> None:
        self.replace("research/figures.md", "│ proposed │", "│ candidate │")
        self.assertTrue(any("missing state(s): proposed" in message for message in self.messages()))

    def test_stale_pactbench_task_count_is_rejected(self) -> None:
        self.replace("research/figures.md", "24 pre-registered PactBench tasks", "21 pre-registered PactBench tasks")
        self.assertTrue(any("expected 24; observed 21" in message for message in self.messages()))

    def test_stale_hypothesis_range_is_rejected(self) -> None:
        self.replace("research/benchmark-protocol.md", "H8–H14", "H8–H10")
        self.assertTrue(any("expected H8–H14; observed H8–H10" in message for message in self.messages()))

    def test_future_provenance_wording_is_rejected(self) -> None:
        self.replace(
            "research/figures.md",
            "RepoPact 2.0 shipped the resolution",
            "Provenance-typed records are the principled future escape",
        )
        self.assertTrue(any("not future work" in message for message in self.messages()))


if __name__ == "__main__":
    unittest.main()
