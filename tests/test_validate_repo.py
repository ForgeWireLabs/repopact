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
import takeover  # noqa: E402
import doctor  # noqa: E402
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

    # --- tracking import (015) ----------------------------------------------

    def _seed_repo_with_tracking(self) -> Path:
        repo = Path(self.temp.name) / "tracked"
        init_repo.bootstrap(repo)
        (repo / "tracking").mkdir()
        (repo / "tracking" / "decisions.md").write_text(
            "# Decision Log\n\n## DEC-001: Use Markdown Foundation\nDate: 2026-06-16  \nStatus: accepted  \n\n"
            "Decision: keep it repo-native.\n", encoding="utf-8")
        (repo / "tracking" / "risks.md").write_text(
            "# Risk Register\n\n| ID | Risk | Severity | Status | Owner | Mitigation |\n"
            "| --- | --- | --- | --- | --- | --- |\n"
            "| RISK-001 | Tracking can drift from repo state. | P2 | open | Steward | Validate regularly. |\n",
            encoding="utf-8")
        (repo / "tracking" / "milestones.md").write_text(
            "# Milestones\n\n## M0: MVP\nStatus: shipped\n\nEvidence: it works.\n\n"
            "## M1: Next thing\nStatus: in progress\n\nRemaining: lots.\n", encoding="utf-8")
        return repo

    def test_tracking_import_maps_to_record_types_and_validates(self) -> None:
        repo = self._seed_repo_with_tracking()
        plan_import.import_plan(repo)
        self.assertEqual([], [p.message for p in validate(repo)])
        # decision record created (DEC-001 -> a 4-digit id, 0001 is unused in a fresh bootstrap)
        self.assertTrue(list((repo / "decisions").glob("*-use-markdown-foundation.md")))
        # risk -> audit finding: numeric id (schema), RISK-001 kept in source, P2 -> medium, open
        finding = list((repo / "audits" / "findings").glob("*.json"))
        self.assertEqual(1, len(finding))
        data = json.loads(finding[0].read_text(encoding="utf-8"))
        self.assertRegex(data["id"], r"^[0-9]{3,}$")
        self.assertEqual(("medium", "open", "governance"), (data["risk"], data["state"], data["scope"]))
        self.assertTrue(data["source"].endswith("RISK-001"))
        self.assertIn("[RISK-001]", data["observed"])
        # milestones -> work items (shipped -> completed/waived, else active/pending)
        self.assertTrue(list((repo / "work" / "completed").glob("*-milestone-mvp")))
        self.assertTrue(list((repo / "work" / "active").glob("*-milestone-next-thing")))

    def test_tracking_import_is_idempotent(self) -> None:
        repo = self._seed_repo_with_tracking()
        plan_import.import_plan(repo)
        before = len(list(repo.glob("decisions/*.md"))) + len(list(repo.glob("audits/findings/*.json")))
        plan_import.import_plan(repo)
        after = len(list(repo.glob("decisions/*.md"))) + len(list(repo.glob("audits/findings/*.json")))
        self.assertEqual(before, after)
        self.assertEqual([], [p.message for p in validate(repo)])

    # --- takeover (015) -----------------------------------------------------

    def test_takeover_archives_fully_migrated_plan_dir(self) -> None:
        repo = Path(self.temp.name) / "tk"
        init_repo.bootstrap(repo)
        (repo / "todos" / "12-search").mkdir(parents=True)
        (repo / "todos" / "12-search" / "README.md").write_text("# Search\n", encoding="utf-8")
        plan_import.import_plan(repo)                       # migrate todos -> work/
        report = takeover.takeover(repo)                    # archive (default)
        self.assertIn("todos", report["retired"])
        self.assertFalse((repo / "todos").exists())
        self.assertTrue((repo / "archive" / "todos" / "12-search" / "README.md").is_file())
        self.assertEqual([], [p.message for p in validate(repo)])

    def test_takeover_refuses_unmigrated_dir(self) -> None:
        repo = Path(self.temp.name) / "tk2"
        init_repo.bootstrap(repo)
        (repo / "todos" / "12-search").mkdir(parents=True)
        (repo / "todos" / "12-search" / "README.md").write_text("# Search\n", encoding="utf-8")
        # do NOT import; takeover must not retire an un-migrated source
        report = takeover.takeover(repo)
        self.assertEqual([], report["retired"])
        self.assertTrue((repo / "todos").exists())
        self.assertTrue(any(s["dir"] == "todos" for s in report["skipped"]))

    def test_takeover_aborts_when_invalid(self) -> None:
        repo = Path(self.temp.name) / "tk3"
        init_repo.bootstrap(repo)
        (repo / "AGENTS.md").unlink()                       # make it invalid
        report = takeover.takeover(repo)
        self.assertFalse(report["validated"])
        self.assertEqual([], report["retired"])

    # --- doctor (013) -------------------------------------------------------

    def _seed_drifted_repo(self) -> Path:
        repo = Path(self.temp.name) / "drifted"
        init_repo.bootstrap(repo)
        (repo / "AGENTS.md").unlink()                          # missing root contract
        reg = json.loads((repo / "audits" / "registry.json").read_text(encoding="utf-8"))
        reg["scopes"].append({"path": "docs", "owner": "x", "contract": "docs/AGENTS.md",
                              "last_reviewed": "2026-06-16", "next_review": "2026-09-16",
                              "alignment": "current", "notes": "stale"})
        (repo / "audits" / "registry.json").write_text(json.dumps(reg), encoding="utf-8")
        (repo / "service").mkdir()
        (repo / "service" / "AGENTS.md").write_text("# service\n", encoding="utf-8")  # unregistered
        return repo

    def test_doctor_diagnoses_drift(self) -> None:
        codes = {f.code for f in doctor.diagnose(self._seed_drifted_repo())}
        self.assertIn("no-root-contract", codes)
        self.assertIn("registry-stale", codes)
        self.assertIn("contract-unregistered", codes)

    def test_doctor_fix_makes_repo_valid(self) -> None:
        repo = self._seed_drifted_repo()
        doctor.fix(repo)
        self.assertEqual([], [f.code for f in doctor.diagnose(repo) if f.severity == "error"])
        self.assertEqual([], [p.message for p in validate(repo)])

    def test_doctor_healthy_on_clean_repo(self) -> None:
        repo = Path(self.temp.name) / "clean"
        init_repo.bootstrap(repo)
        self.assertEqual([], [f for f in doctor.diagnose(repo) if f.severity == "error"])

    # --- orphan work directories & dead source_of_truth pointers -------------

    def test_orphan_work_directory_without_manifest_is_rejected(self) -> None:
        # A directory under work/ that carries planning content but no work-item.json
        # is invisible to discover_work_items, so validate must flag it.
        orphan = self.root / "work" / "active" / "099-ghost"
        orphan.mkdir(parents=True)
        (orphan / "README.md").write_text("# ghost plan\n", encoding="utf-8")
        self.assertTrue(any("no work-item.json" in v or "work-item.json" in v
                            for v in self.problems()))

    def test_orphan_check_ignores_tracked_items_and_audit_companions(self) -> None:
        # A properly tracked item — even with an _audit/ companion — is not an orphan.
        self.add_active_item("099")
        item = self.root / "work" / "active" / "099-probe"
        audit = item / "_audit"
        audit.mkdir()
        (audit / "README.md").write_text("# audit\n", encoding="utf-8")
        self.assertEqual([], self.problems())

    def test_doctor_flags_dead_source_of_truth_pointer(self) -> None:
        decision = self.root / "decisions" / "9999-probe.md"
        decision.write_text(
            "---\nid: 9999\ntitle: Probe\nstatus: accepted\ndate: 2026-06-17\n"
            "source_of_truth: docs/gone.md; AGENTS.md\n---\n\n# 9999: Probe\n",
            encoding="utf-8")
        findings = doctor._dead_source_of_truth(self.root)
        msgs = [f.message for f in findings]
        self.assertTrue(any("docs/gone.md" in m for m in msgs))
        self.assertFalse(any("AGENTS.md" in m for m in msgs))

    def test_import_plan_section_roadmap_without_checkboxes(self) -> None:
        repo = Path(self.temp.name) / "roadmapped"
        init_repo.bootstrap(repo)
        (repo / "ROADMAP.md").write_text(
            "# Roadmap\n\n## Now\n- Ship the API\n\n## Later\n- Mobile app\n\n"
            "## Done\n- Initial release\n", encoding="utf-8")
        plan_import.import_plan(repo)
        self.assertEqual([], [p.message for p in validate(repo)])
        self.assertTrue(list((repo / "work" / "active").glob("*-ship-the-api")))
        self.assertTrue(list((repo / "work" / "deferred").glob("*-mobile-app")))
        self.assertTrue(list((repo / "work" / "completed").glob("*-initial-release")))

    def test_split_num_strips_tracker_prefix(self) -> None:
        self.assertEqual(("001", "true-cert-factory"), plan_import._split_num("TODO-001-true-cert-factory"))
        self.assertEqual(("12", "search"), plan_import._split_num("12-search"))
        self.assertEqual((None, "freeform-note"), plan_import._split_num("freeform note"))

    def test_section_lifecycle_keywords(self) -> None:
        self.assertEqual("active", plan_import._section_lifecycle("Now — in progress"))
        self.assertEqual("deferred", plan_import._section_lifecycle("Later / future"))
        self.assertEqual("completed", plan_import._section_lifecycle("Shipped"))
        self.assertIsNone(plan_import._section_lifecycle("Overview"))


if __name__ == "__main__":
    unittest.main()
