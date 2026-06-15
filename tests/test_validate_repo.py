from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import check_frozen_surface  # noqa: E402
import init_repo  # noqa: E402
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

    def write_json(self, path: Path, mutate) -> None:
        data = json.loads(path.read_text(encoding="utf-8"))
        mutate(data)
        path.write_text(json.dumps(data), encoding="utf-8")

    def add_active_item(self, item_id: str, owner_scope: str = "work") -> None:
        directory = self.root / "work" / "active" / f"{item_id}-probe"
        directory.mkdir(parents=True)
        (directory / "README.md").write_text("# probe\n", encoding="utf-8")
        (directory / "work-item.json").write_text(json.dumps({
            "id": item_id,
            "title": "probe",
            "status": "active",
            "owner_scope": owner_scope,
            "affected_scopes": [],
            "depends_on": [],
            "acceptance_criteria": [{"id": "AC-1", "text": "x", "state": "pending", "evidence": []}],
            "created": "2026-06-15",
            "updated": "2026-06-15",
        }), encoding="utf-8")

    # --- baseline -----------------------------------------------------------

    def test_repository_is_valid(self) -> None:
        self.assertEqual([], self.problems())

    # --- work lifecycle -----------------------------------------------------

    def test_status_must_match_directory(self) -> None:
        path = self.manifest()
        self.write_json(path, lambda d: d.__setitem__(
            "status", "active" if path.parent.parent.name == "completed" else "completed"))
        self.assertTrue(any("does not match directory" in v for v in self.problems()))

    def test_satisfied_criterion_requires_evidence(self) -> None:
        path = self.manifest()
        self.write_json(path, lambda d: (d["acceptance_criteria"][0].update({"state": "satisfied", "evidence": []})))
        self.assertTrue(any("satisfied without evidence" in v for v in self.problems()))

    def test_completed_item_cannot_have_pending_criteria(self) -> None:
        path = self.manifest()
        self.write_json(path, lambda d: (d["acceptance_criteria"][0].update({"state": "pending", "evidence": []})))
        self.assertTrue(any("completed item has pending criterion" in v for v in self.problems()))

    def test_dependency_must_reference_known_work_item(self) -> None:
        path = self.manifest()
        self.write_json(path, lambda d: d.__setitem__("depends_on", ["999"]))
        self.assertTrue(any("unknown dependency" in v for v in self.problems()))

    def test_unknown_affected_scope_is_rejected(self) -> None:
        path = self.manifest()
        self.write_json(path, lambda d: d.__setitem__("affected_scopes", ["nope"]))
        self.assertTrue(any("unknown affected_scope" in v for v in self.problems()))

    # --- evidence -----------------------------------------------------------

    def test_evidence_must_reference_known_work_item(self) -> None:
        path = next((self.root / "evidence" / "runs").glob("*.json"))
        self.write_json(path, lambda d: d.__setitem__("work_item", "999"))
        self.assertTrue(any("unknown work_item" in v for v in self.problems()))

    # --- contracts and audit coverage --------------------------------------

    def test_nested_contract_must_be_registered(self) -> None:
        extra = self.root / "extra"
        extra.mkdir()
        (extra / "AGENTS.md").write_text("# unregistered\n", encoding="utf-8")
        self.assertTrue(any("not registered in audits/registry.json" in v for v in self.problems()))

    def test_existing_audit_companion_must_be_complete(self) -> None:
        (self.root / "scripts" / "_audit" / "inventory.md").unlink()
        self.assertTrue(any("incomplete _audit companion" in v for v in self.problems()))

    # --- invariants ---------------------------------------------------------

    def test_invariant_requires_escalation(self) -> None:
        path = self.root / "governance" / "invariants.json"
        self.write_json(path, lambda d: d["invariants"][0].__setitem__("escalation", ""))
        self.assertTrue(any("is missing escalation" in v for v in self.problems()))

    # --- frozen surface -----------------------------------------------------

    def test_frozen_entry_requires_reason(self) -> None:
        path = self.root / "governance" / "frozen-surface.json"
        self.write_json(path, lambda d: d["protected"][0].__setitem__("reason", ""))
        self.assertTrue(any("needs a reason" in v for v in self.problems()))

    # --- roles --------------------------------------------------------------

    def test_role_must_reference_known_scope(self) -> None:
        path = self.root / "governance" / "owners.json"
        self.write_json(path, lambda d: d["roles"][0].__setitem__("scopes", ["ghost"]))
        self.assertTrue(any("references unknown scope" in v for v in self.problems()))

    # --- decisions and policies --------------------------------------------

    def test_decision_status_must_be_valid(self) -> None:
        path = next((self.root / "decisions").glob("0001-*.md"))
        text = path.read_text(encoding="utf-8").replace("status: accepted", "status: maybe")
        path.write_text(text, encoding="utf-8")
        self.assertTrue(any("status 'maybe' must be one of" in v for v in self.problems()))

    def test_policy_requires_front_matter(self) -> None:
        path = next((self.root / "governance" / "policies").glob("001-*.md"))
        path.write_text("# no front matter\n", encoding="utf-8")
        self.assertTrue(any("front-matter" in v or "front matter" in v for v in self.problems()))

    # --- optional disjoint-scope rule --------------------------------------

    def test_disjoint_scopes_off_by_default(self) -> None:
        self.add_active_item("900")
        self.add_active_item("901")
        self.assertFalse(any("active scope conflict" in v for v in self.problems()))

    def test_disjoint_scopes_enforced_when_enabled(self) -> None:
        owners = self.root / "governance" / "owners.json"
        self.write_json(owners, lambda d: d["concurrency"].__setitem__("enforce_disjoint_active_scopes", True))
        self.add_active_item("900")
        self.add_active_item("901")
        self.assertTrue(any("active scope conflict" in v for v in self.problems()))

    # --- schema layer (decision 0003) --------------------------------------

    def test_schema_rejects_bad_invariant_id(self) -> None:
        path = self.root / "governance" / "invariants.json"
        self.write_json(path, lambda d: d["invariants"][0].__setitem__("id", "BAD"))
        self.assertTrue(any(v.startswith("schema ") for v in self.problems()))

    # --- audit findings -----------------------------------------------------

    def test_audit_finding_state_must_be_valid(self) -> None:
        path = next((self.root / "audits" / "findings").glob("*.json"))
        self.write_json(path, lambda d: d.__setitem__("state", "nope"))
        self.assertTrue(any("schema" in v and "state" in v for v in self.problems()))

    # --- spec version -------------------------------------------------------

    def test_version_must_be_semver(self) -> None:
        (self.root / "VERSION").write_text("v1\n", encoding="utf-8")
        self.assertTrue(any("must be semantic" in v for v in self.problems()))

    # --- dependency cycles --------------------------------------------------

    def test_dependency_cycle_detected(self) -> None:
        self.add_active_item("900")
        self.add_active_item("901")
        a = self.manifest("900")
        b = self.manifest("901")
        self.write_json(a, lambda d: d.__setitem__("depends_on", ["901"]))
        self.write_json(b, lambda d: d.__setitem__("depends_on", ["900"]))
        self.assertTrue(any("dependency cycle" in v for v in self.problems()))

    # --- frozen-surface symbol matching (pure function) --------------------

    def test_symbol_hits_detects_protected_symbol(self) -> None:
        protected = [{"glob": "x", "reason": "r", "symbols": ["SecretToken"]}]
        patch = "+    value = SecretToken()\n-    old = 1"
        hits = check_frozen_surface.symbol_hits(patch, protected)
        self.assertEqual([("SecretToken", "r")], hits)

    def test_symbol_hits_ignores_context_lines(self) -> None:
        protected = [{"glob": "x", "reason": "r", "symbols": ["SecretToken"]}]
        patch = "     unchanged = SecretToken()"  # context line, not +/-
        self.assertEqual([], check_frozen_surface.symbol_hits(patch, protected))

    # --- bootstrap (003 B1) -------------------------------------------------

    def test_bootstrap_produces_valid_repo(self) -> None:
        target = Path(self.temp.name) / "seeded"
        init_repo.bootstrap(target)
        self.assertEqual([], [p.message for p in validate(target)])


if __name__ == "__main__":
    unittest.main()
