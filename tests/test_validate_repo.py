from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from validate_repo import validate  # noqa: E402


class RepositoryValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / "repo"
        shutil.copytree(ROOT, self.root, ignore=shutil.ignore_patterns(".git", "__pycache__"))

    def tearDown(self) -> None:
        self.temp.cleanup()

    def problems(self) -> list[str]:
        return [problem.message for problem in validate(self.root)]

    def manifest(self, item_id: str = "000") -> Path:
        matches = [
            path
            for path in (self.root / "work").glob("*/*/work-item.json")
            if json.loads(path.read_text(encoding="utf-8")).get("id") == item_id
        ]
        self.assertEqual(1, len(matches))
        return matches[0]

    def test_repository_is_valid(self) -> None:
        self.assertEqual([], self.problems())

    def test_status_must_match_directory(self) -> None:
        path = self.manifest()
        data = json.loads(path.read_text(encoding="utf-8"))
        data["status"] = "active" if path.parent.parent.name == "completed" else "completed"
        path.write_text(json.dumps(data), encoding="utf-8")
        self.assertTrue(any("does not match directory" in value for value in self.problems()))

    def test_nested_contract_requires_audit_companions(self) -> None:
        (self.root / "scripts" / "_audit" / "inventory.md").unlink()
        self.assertTrue(any("missing audit companion" in value for value in self.problems()))

    def test_satisfied_criterion_requires_evidence(self) -> None:
        path = self.manifest()
        data = json.loads(path.read_text(encoding="utf-8"))
        data["acceptance_criteria"][0]["state"] = "satisfied"
        data["acceptance_criteria"][0]["evidence"] = []
        path.write_text(json.dumps(data), encoding="utf-8")
        self.assertTrue(any("satisfied without evidence" in value for value in self.problems()))

    def test_completed_item_cannot_have_pending_criteria(self) -> None:
        path = self.manifest()
        data = json.loads(path.read_text(encoding="utf-8"))
        data["acceptance_criteria"][0]["state"] = "pending"
        data["acceptance_criteria"][0]["evidence"] = []
        path.write_text(json.dumps(data), encoding="utf-8")
        self.assertTrue(any("completed item has pending criterion" in value for value in self.problems()))

    def test_dependency_must_reference_known_work_item(self) -> None:
        path = self.manifest()
        data = json.loads(path.read_text(encoding="utf-8"))
        data["depends_on"] = ["999"]
        path.write_text(json.dumps(data), encoding="utf-8")
        self.assertTrue(any("unknown dependency" in value for value in self.problems()))

    def test_evidence_must_reference_known_work_item(self) -> None:
        path = next((self.root / "evidence" / "runs").glob("*.json"))
        data = json.loads(path.read_text(encoding="utf-8"))
        data["work_item"] = "999"
        path.write_text(json.dumps(data), encoding="utf-8")
        self.assertTrue(any("unknown work_item" in value for value in self.problems()))


if __name__ == "__main__":
    unittest.main()
