from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import adopt_repo  # noqa: E402
import plan_import  # noqa: E402
import check_frozen_surface  # noqa: E402
import generate_spec  # noqa: E402
import init_repo  # noqa: E402
import repopact_cli  # noqa: E402
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

    # --- SPEC generator determinism (004) ----------------------------------

    def test_spec_generation_is_idempotent(self) -> None:
        text = (ROOT / "SPEC.md").read_text(encoding="utf-8")
        once = generate_spec.render(text)
        twice = generate_spec.render(once)
        self.assertEqual(once, twice)
        self.assertIn("work-item.schema.json", once)
        self.assertIn("INV-1", once)

    # --- CLI dispatch (005) -------------------------------------------------

    def test_cli_validate_returns_zero_on_valid_repo(self) -> None:
        self.assertEqual(0, repopact_cli.main(["validate", "--root", str(self.root)]))

    def test_cli_new_stamps_a_valid_record(self) -> None:
        rc = repopact_cli.main(["new", "work-item", "Cli Probe", "--root", str(self.root)])
        self.assertEqual(0, rc)
        stamped = list((self.root / "work" / "active").glob("*-cli-probe/work-item.json"))
        self.assertEqual(1, len(stamped))
        self.assertEqual([], [p.message for p in validate(self.root)])

    # --- proving-ground hardening (007) -------------------------------------

    def test_cli_spec_fails_cleanly_without_spec_file(self) -> None:
        """F-001: `spec` must not traceback on a repo that has no SPEC.md."""
        target = Path(self.temp.name) / "no-spec"
        init_repo.bootstrap(target)
        self.assertFalse((target / "SPEC.md").exists())
        self.assertEqual(1, repopact_cli.main(["spec", "--root", str(target)]))

    def test_check_frozen_detects_working_tree_change(self) -> None:
        """F-002: an uncommitted change to a protected path must be detected."""
        import subprocess

        repo = Path(self.temp.name) / "frz"
        (repo / "governance").mkdir(parents=True)
        (repo / "governance" / "frozen-surface.json").write_text(
            json.dumps({"version": 1, "protected": [
                {"glob": "governance/invariants.json", "reason": "the pact"}]}),
            encoding="utf-8")
        (repo / "governance" / "invariants.json").write_text(
            json.dumps({"version": 1, "invariants": []}), encoding="utf-8")
        try:
            run = lambda *a: subprocess.run(["git", *a], cwd=repo, check=True,
                                            capture_output=True, text=True)
            run("init"); run("config", "user.email", "t@t"); run("config", "user.name", "t")
            run("add", "-A"); run("commit", "-m", "init")
        except (OSError, subprocess.CalledProcessError):
            self.skipTest("git unavailable")
        # modify the protected file in the working tree only, no commit
        (repo / "governance" / "invariants.json").write_text(
            json.dumps({"version": 1, "invariants": [{"changed": True}]}), encoding="utf-8")
        hits = check_frozen_surface.violations(repo, "HEAD")
        self.assertEqual([("governance/invariants.json", "the pact")], hits)

    # --- brownfield adoption (008) ------------------------------------------

    def _seed_existing_repo(self) -> Path:
        """A minimal pre-existing project: CODEOWNERS, a CI workflow, a nested contract."""
        repo = Path(self.temp.name) / "existing"
        (repo / ".github" / "workflows").mkdir(parents=True)
        (repo / "core").mkdir()
        (repo / "docs" / "_audit").mkdir(parents=True)
        (repo / "README.md").write_text("# Existing\n", encoding="utf-8")
        (repo / "CODEOWNERS").write_text(
            "# owners\n/core/   @backend-team\n/docs/   @docs-team\n", encoding="utf-8")
        (repo / ".github" / "workflows" / "ci.yml").write_text(
            "name: Python CI\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n", encoding="utf-8")
        (repo / "docs" / "AGENTS.md").write_text("# Docs contract\n", encoding="utf-8")
        (repo / "docs" / "_audit" / "inventory.md").write_text("# Inventory\n", encoding="utf-8")
        return repo

    def test_adopt_existing_repo_validates(self) -> None:
        repo = self._seed_existing_repo()
        adopt_repo.adopt(repo)
        self.assertEqual([], [p.message for p in validate(repo)])

    def test_adopt_is_non_destructive(self) -> None:
        repo = self._seed_existing_repo()
        original = (repo / "README.md").read_text(encoding="utf-8")
        adopt_repo.adopt(repo)
        # existing files are preserved verbatim
        self.assertEqual(original, (repo / "README.md").read_text(encoding="utf-8"))
        self.assertEqual("# Docs contract\n", (repo / "docs" / "AGENTS.md").read_text(encoding="utf-8"))

    def test_adopt_maps_workflows_and_codeowners(self) -> None:
        repo = self._seed_existing_repo()
        adopt_repo.adopt(repo)
        owners = json.loads((repo / "governance" / "owners.json").read_text(encoding="utf-8"))
        scope_ids = {s["id"] for s in owners["scopes"]}
        self.assertIn("backend-team", scope_ids)
        self.assertIn("docs-team", scope_ids)
        # the workflow becomes a binding-gate policy and a frozen path
        policies = list((repo / "governance" / "policies").glob("*-ci-*.md"))
        self.assertEqual(1, len(policies))
        frozen = json.loads((repo / "governance" / "frozen-surface.json").read_text(encoding="utf-8"))
        self.assertIn(".github/workflows/**", [p["glob"] for p in frozen["protected"]])

    def test_adopt_dry_run_writes_nothing(self) -> None:
        repo = self._seed_existing_repo()
        rep = adopt_repo.adopt(repo, dry_run=True)
        self.assertFalse((repo / "governance").exists())
        self.assertTrue(rep.created)

    def test_adopt_warns_on_gitignored_records(self) -> None:
        """F-008: a .gitignore that swallows evidence records must be flagged."""
        import subprocess
        repo = self._seed_existing_repo()
        try:
            run = lambda *a: subprocess.run(["git", *a], cwd=repo, check=True, capture_output=True, text=True)
            run("init")
            # the classic collision: a broad `runs/` rule that also matches evidence/runs/
            (repo / ".gitignore").write_text("runs/\n", encoding="utf-8")
            rep = adopt_repo.adopt(repo)
        except (OSError, subprocess.CalledProcessError):
            self.skipTest("git unavailable")
        self.assertTrue(any("evidence/runs/" in r for r in rep.gitignored),
                        f"expected an evidence record flagged as gitignored, got {rep.gitignored}")

    # --- plan import (011) --------------------------------------------------

    def _seed_adopted_repo_with_plans(self) -> Path:
        repo = Path(self.temp.name) / "planned"
        init_repo.bootstrap(repo)
        # a todos/ tree: one active item, one completed item, one deferred item
        (repo / "todos" / "12-search").mkdir(parents=True)
        (repo / "todos" / "12-search" / "README.md").write_text("# Add search\nPlan body.\n", encoding="utf-8")
        (repo / "todos" / "completed" / "03-login").mkdir(parents=True)
        (repo / "todos" / "completed" / "03-login" / "README.md").write_text("# Login\nDone earlier.\n", encoding="utf-8")
        (repo / "todos" / "deferred" / "20-i18n").mkdir(parents=True)
        (repo / "todos" / "deferred" / "20-i18n" / "README.md").write_text("# i18n\nLater.\n", encoding="utf-8")
        # a flat checklist file
        (repo / "TODO.md").write_text("- [ ] wire up metrics\n- [x] choose a license\n", encoding="utf-8")
        return repo

    def test_import_plan_populates_and_validates(self) -> None:
        repo = self._seed_adopted_repo_with_plans()
        plan_import.import_plan(repo)
        self.assertEqual([], [p.message for p in validate(repo)])
        # directory items mapped to the right lifecycle
        self.assertTrue((repo / "work" / "active" / "012-search").is_dir())
        self.assertTrue((repo / "work" / "completed" / "003-login").is_dir())
        self.assertTrue((repo / "work" / "deferred" / "020-i18n").is_dir())
        # checklist items mapped by checkbox state
        self.assertTrue(list((repo / "work" / "active").glob("*-wire-up-metrics")))
        self.assertTrue(list((repo / "work" / "completed").glob("*-choose-a-license")))

    def test_import_plan_completed_items_are_waived_not_fabricated(self) -> None:
        repo = self._seed_adopted_repo_with_plans()
        plan_import.import_plan(repo)
        manifest = json.loads((repo / "work" / "completed" / "003-login" / "work-item.json").read_text(encoding="utf-8"))
        self.assertEqual("waived", manifest["acceptance_criteria"][0]["state"])
        self.assertEqual([], manifest["acceptance_criteria"][0]["evidence"])  # no fabricated evidence
        self.assertEqual("todos/completed/03-login", manifest["source"])

    def test_import_plan_is_idempotent(self) -> None:
        repo = self._seed_adopted_repo_with_plans()
        plan_import.import_plan(repo)
        before = len(list(repo.glob("work/*/*/work-item.json")))
        rep = plan_import.import_plan(repo)  # second run
        after = len(list(repo.glob("work/*/*/work-item.json")))
        self.assertEqual(before, after)
        self.assertEqual([], rep.created)
        self.assertEqual([], [p.message for p in validate(repo)])

    def test_import_plan_dry_run_writes_nothing(self) -> None:
        repo = self._seed_adopted_repo_with_plans()
        rep = plan_import.import_plan(repo, dry_run=True)
        self.assertFalse(list(repo.glob("work/active/*-search")))
        self.assertTrue(rep.created)


if __name__ == "__main__":
    unittest.main()
