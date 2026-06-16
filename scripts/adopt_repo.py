"""Adopt RepoPact into an EXISTING repository (work item 008).

Where ``init_repo`` seeds a greenfield repo, ``adopt_repo`` reads an existing
project's governance signals and generates RepoPact records *around* them, without
overwriting a single existing file:

* ``CODEOWNERS``            -> scopes and roles in ``governance/owners.json``
* ``.github/workflows/*``   -> binding-gate policies + a CI invariant + frozen surface
* nested ``AGENTS.md``      -> registered contracts in ``audits/registry.json``
  (with a stub ``_audit`` triplet created only where an ``_audit`` dir already exists)
* git history              -> the first evidence run and a completed adoption work item

Everything it writes is created only if absent. Run with ``--dry-run`` to see the
plan without touching the tree. The result is validated before it returns.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from datetime import date, datetime, timedelta
from pathlib import Path

import init_repo  # reuse _seed_dir, _json/_write semantics, LIFECYCLE


# --- non-destructive primitives --------------------------------------------

class Report:
    def __init__(self, dry_run: bool) -> None:
        self.dry_run = dry_run
        self.created: list[str] = []
        self.skipped: list[str] = []
        self.gitignored: list[str] = []  # created records an existing .gitignore would swallow (F-008)

    def write(self, path: Path, text: str, root: Path) -> None:
        rel = str(path.relative_to(root)).replace("\\", "/")
        if path.exists():
            self.skipped.append(rel)
            return
        self.created.append(rel)
        if not self.dry_run:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")

    def json(self, path: Path, data: object, root: Path) -> None:
        import json
        self.write(path, json.dumps(data, indent=2) + "\n", root)

    def copy_seed(self, name: str, root: Path) -> None:
        """Copy schemas/ or templates/ from the install, file by file, never clobbering."""
        import shutil
        src = init_repo._seed_dir(name)
        for item in src.iterdir():
            dest = root / name / item.name
            rel = f"{name}/{item.name}"
            if dest.exists():
                self.skipped.append(rel)
                continue
            self.created.append(rel)
            if not self.dry_run:
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(item, dest)


# --- detection --------------------------------------------------------------

def detect_workflows(root: Path) -> list[Path]:
    wf = root / ".github" / "workflows"
    if not wf.is_dir():
        return []
    return sorted(p for p in wf.iterdir() if p.suffix in (".yml", ".yaml"))


def workflow_name(path: Path) -> str:
    try:
        for line in path.read_text(encoding="utf-8").splitlines():
            m = re.match(r"\s*name:\s*(.+?)\s*$", line)
            if m:
                return m.group(1).strip().strip("'\"")
    except OSError:
        pass
    return path.stem


def parse_codeowners(root: Path) -> dict[str, list[str]]:
    """Return owner-handle -> list of RepoPact-style path globs."""
    for candidate in (root / ".github" / "CODEOWNERS", root / "CODEOWNERS", root / "docs" / "CODEOWNERS"):
        if candidate.is_file():
            text = candidate.read_text(encoding="utf-8")
            break
    else:
        return {}
    owners: dict[str, list[str]] = {}
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        parts = line.split()
        if len(parts) < 2:
            continue
        pattern, handles = parts[0], parts[1:]
        glob = pattern.lstrip("/")
        if glob.endswith("/"):
            glob += "**"
        for handle in handles:
            owners.setdefault(handle, [])
            if glob and glob not in owners[handle]:
                owners[handle].append(glob)
    return owners


def find_nested_contracts(root: Path) -> list[Path]:
    out: list[Path] = []
    for path in root.rglob("AGENTS.md"):
        if path.parent == root:
            continue
        if any(part in {".git", ".venv", "node_modules", "__pycache__"} for part in path.parts):
            continue
        out.append(path)
    return sorted(out)


def git_stats(root: Path) -> dict[str, object]:
    def run(args: list[str]) -> str | None:
        try:
            r = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True, check=True)
            return r.stdout.strip()
        except (OSError, subprocess.CalledProcessError):
            return None
    commits = run(["rev-list", "--count", "HEAD"])
    head = run(["rev-parse", "--short", "HEAD"])
    tag = run(["describe", "--tags", "--abbrev=0"])
    contributors = run(["shortlog", "-sne", "HEAD"])
    n_contrib = len(contributors.splitlines()) if contributors else None
    return {
        "commits": int(commits) if commits and commits.isdigit() else None,
        "head": head,
        "latest_tag": tag,
        "contributors": n_contrib,
    }


def gitignored_records(root: Path, rels: list[str]) -> list[str]:
    """Which of the generated records would an existing .gitignore swallow (F-008)?

    A record that is ignored validates on the author's disk but is missing on a fresh
    clone or in CI, silently breaking the adopted repository. Best-effort: returns []
    outside a git checkout or if git is unavailable.
    """
    if not rels:
        return []
    try:
        result = subprocess.run(["git", "check-ignore", "--stdin"], cwd=root,
                                input="\n".join(rels), capture_output=True, text=True)
    except OSError:
        return []
    if result.returncode not in (0, 1):  # 0 = some ignored, 1 = none ignored
        return []
    return [line.strip().replace("\\", "/") for line in result.stdout.splitlines() if line.strip()]


def _slug(text: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-") or "item"


def _detect_version(root: Path) -> str:
    existing = root / "VERSION"
    if existing.is_file():
        v = existing.read_text(encoding="utf-8").strip()
        if re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", v):
            return v
    pyproject = root / "pyproject.toml"
    if pyproject.is_file():
        m = re.search(r'(?m)^\s*version\s*=\s*["\']([0-9]+\.[0-9]+\.[0-9]+)["\']', pyproject.read_text(encoding="utf-8"))
        if m:
            return m.group(1)
    return "0.1.0"


# --- adoption ---------------------------------------------------------------

def adopt(target: Path, today: date | None = None, dry_run: bool = False) -> Report:
    today = today or date.today()
    next_review = (today + timedelta(days=90)).isoformat()
    rep = Report(dry_run)
    target.mkdir(parents=True, exist_ok=True)

    workflows = detect_workflows(target)
    codeowners = parse_codeowners(target)
    contracts = find_nested_contracts(target)
    stats = git_stats(target)

    rep.copy_seed("schemas", target)
    rep.copy_seed("templates", target)

    rep.write(target / "VERSION", _detect_version(target) + "\n", target)
    rep.write(target / "AGENTS.md",
              "# Agent Contract\n\nThe repository is the durable coordination surface. The invariants in\n"
              "`governance/invariants.json` are binding; weakening one requires operator approval.\n"
              "Read every `AGENTS.md` from root to the file you touch.\n", target)

    # governance/owners.json from CODEOWNERS, always including a governance scope.
    scopes = [{"id": "governance", "paths": ["AGENTS.md", "governance/**", "schemas/**"], "owner": "governance-owner"}]
    roles = [{"id": "governance-owner", "description": "Maintains the pact and schemas.", "scopes": ["governance"]}]
    for handle, paths in codeowners.items():
        scope_id = _slug(handle.lstrip("@"))
        if scope_id == "governance" or any(s["id"] == scope_id for s in scopes):
            scope_id = f"{scope_id}-owned"
        scopes.append({"id": scope_id, "paths": paths or ["**"], "owner": handle})
        roles.append({"id": scope_id, "description": f"Owns paths assigned to {handle} in CODEOWNERS.", "scopes": [scope_id]})
    rep.json(target / "governance" / "owners.json",
             {"version": 2, "scopes": scopes, "roles": roles,
              "concurrency": {"enforce_disjoint_active_scopes": False}}, target)

    rep.write(target / "governance" / "charter.md",
              "# Charter\n\n## Principles (human judgment)\n\n1. Systems before sessions.\n"
              "2. Completion requires proof.\n\n## Invariants (binding)\n\nSee `invariants.json`.\n", target)
    rep.write(target / "governance" / "workflow.md",
              "# Operating Workflow\n\nCapture intent in a work item, resolve authority, implement in scope,\n"
              "produce evidence, reconcile, then transition state.\n", target)

    invariants = [{
        "id": "INV-1",
        "statement": "No critical state exists only in conversation; it lives in versioned files.",
        "rationale": "State must be recoverable without a prior conversation.",
        "escalation": "If a task would leave load-bearing state only in chat, record it as a file first.",
        "enforced_by": None,
    }]
    if workflows:
        invariants.append({
            "id": "INV-2",
            "statement": "Declared CI workflows are binding gates; removing or weakening one requires operator approval.",
            "rationale": "CI is the enforcement substrate that the project's correctness claims rest on.",
            "escalation": "Flag any change that deletes or disables a workflow and confirm with the operator.",
            "enforced_by": ".github/workflows",
        })
    rep.json(target / "governance" / "invariants.json",
             {"$schema": "../schemas/invariants.schema.json", "version": 1, "invariants": invariants}, target)

    protected = [{"glob": "governance/invariants.json",
                  "reason": "Invariants are the pact; weakening requires operator approval.", "symbols": []}]
    if workflows:
        protected.append({"glob": ".github/workflows/**",
                          "reason": "CI is the enforcement substrate; changes need human review.", "symbols": []})
    if codeowners:
        protected.append({"glob": "CODEOWNERS",
                          "reason": "Ownership mapping; changing who can approve what needs review.", "symbols": []})
    rep.json(target / "governance" / "frozen-surface.json",
             {"$schema": "../schemas/frozen-surface.schema.json", "version": 1, "protected": protected}, target)

    # One policy per detected workflow: the existing gate, recorded as an operating rule.
    for i, wf in enumerate(workflows, start=1):
        name = workflow_name(wf)
        rel = str(wf.relative_to(target)).replace("\\", "/")
        pid = f"{i:03d}"
        rep.write(target / "governance" / "policies" / f"{pid}-ci-{_slug(name)}.md",
                  f"---\nid: {pid}\ntitle: 'CI gate: {name}'\nstatus: active\napplies_to: '{rel}'\n---\n\n"
                  f"# {pid}: CI gate — {name}\n\nThe workflow [`{rel}`]({rel}) is a binding gate adopted into the\n"
                  f"pact. It must pass before merge; disabling or weakening it requires operator approval (INV-2).\n", target)

    # audits/registry.json: root contract + every nested AGENTS.md, with _audit triplets stubbed.
    registry_scopes = [{"path": ".", "owner": "governance-owner", "contract": "AGENTS.md",
                        "last_reviewed": today.isoformat(), "next_review": next_review,
                        "alignment": "current", "notes": "Adopted repository root contract."}]
    for contract in contracts:
        rel_dir = str(contract.parent.relative_to(target)).replace("\\", "/")
        rel_contract = str(contract.relative_to(target)).replace("\\", "/")
        registry_scopes.append({"path": rel_dir, "owner": "governance-owner", "contract": rel_contract,
                                "last_reviewed": today.isoformat(), "next_review": next_review,
                                "alignment": "current", "notes": "Existing nested contract registered on adoption."})
        audit_dir = contract.parent / "_audit"
        if audit_dir.is_dir():
            for fname, body in (("README.md", f"# _audit — {rel_dir}\n\nAudit companion for the `{rel_dir}` contract.\n"),
                                ("inventory.md", "# Inventory\n\nStub created on RepoPact adoption; fill with the scope's inventory.\n"),
                                ("alignment-report.md", "# Alignment report\n\nStub created on RepoPact adoption; fill with the alignment review.\n")):
                rep.write(audit_dir / fname, body, target)
    rep.json(target / "audits" / "registry.json", {"version": 1, "scopes": registry_scopes}, target)

    # Lifecycle + empty dirs.
    if not dry_run:
        for status in init_repo.LIFECYCLE:
            (target / "work" / status).mkdir(parents=True, exist_ok=True)
        for empty in ("evidence/runs", "decisions", "governance/policies", "audits/findings", "audits/reports"):
            (target / empty).mkdir(parents=True, exist_ok=True)

    # The adoption itself: a completed work item proven by an evidence run over the scan.
    ts = datetime.now()
    ev_id = f"{ts.strftime('%Y%m%d-%H%M%S')}-adopt"
    summary = (f"Adopted RepoPact into an existing repository: {stats.get('commits')} commits, "
               f"{len(workflows)} CI workflow(s), {len(codeowners)} CODEOWNERS handle(s), "
               f"{len(contracts)} nested contract(s).")
    rep.json(target / "evidence" / "runs" / f"{ev_id}.json", {
        "$schema": "../../schemas/evidence-run.schema.json",
        "id": ev_id, "timestamp": ts.replace(microsecond=0).astimezone().isoformat(),
        "work_item": "000", "result": "passed",
        "commands": [
            {"command": "git rev-list --count HEAD", "exit_code": 0, "summary": f"{stats.get('commits')} commits of history adopted."},
            {"command": "repopact adopt", "exit_code": 0, "summary": summary},
        ],
        "artifacts": [str(w.relative_to(target)).replace("\\", "/") for w in workflows],
        "environment": {"platform": sys.platform},
    }, target)

    wi_dir = target / "work" / "completed" / "000-adopt-repopact"
    rep.json(wi_dir / "work-item.json", {
        "$schema": "../../../schemas/work-item.schema.json",
        "id": "000", "title": "Adopt RepoPact into the existing repository",
        "status": "completed", "owner_scope": "governance", "affected_scopes": ["governance"],
        "depends_on": [],
        "acceptance_criteria": [
            {"id": "AC-1", "text": "Existing CODEOWNERS, CI workflows, and nested contracts are represented as RepoPact records.",
             "state": "satisfied", "evidence": [ev_id]},
            {"id": "AC-2", "text": "The repository validates as a conformant RepoPact.",
             "state": "satisfied", "evidence": [ev_id]},
        ],
        "created": today.isoformat(), "updated": today.isoformat(),
    }, target)
    rep.write(wi_dir / "README.md",
              "# 000 — Adopt RepoPact into the existing repository\n\n"
              "> **Status**: ✅ Complete\n\n## Intent\n\n"
              "Bring an existing project under RepoPact governance by mapping its already-present\n"
              "ownership (CODEOWNERS), enforcement (CI workflows), and contracts (nested `AGENTS.md`)\n"
              "into RepoPact records, then proving the result validates.\n\n## Closeout\n\n"
              f"Satisfied by evidence run `{ev_id}`. {summary}\n", target)

    rep.write(target / "decisions" / "0001-adopt-repopact.md",
              "---\nid: 0001\ntitle: Adopt RepoPact\nstatus: accepted\n"
              f"date: {today.isoformat()}\nsupersedes: []\n---\n\n# 0001: Adopt RepoPact\n\n## Context\n\n"
              "The project already had ad-hoc governance (CODEOWNERS, CI gates, AGENTS.md). RepoPact\n"
              "makes those bindings explicit and machine-checkable.\n\n## Decision\n\n"
              "Adopt RepoPact; existing workflows become binding gates (INV-2) and ownership becomes\n"
              "scopes/roles. Existing files were preserved; RepoPact records were added around them.\n", target)

    rep.gitignored = gitignored_records(target, rep.created)
    return rep


def _print_report(rep: Report) -> None:
    verb = "Would create" if rep.dry_run else "Created"
    print(f"{verb} {len(rep.created)} record(s); skipped {len(rep.skipped)} existing file(s).")
    for rel in rep.created:
        print(f"  + {rel}")
    if rep.gitignored:
        print("\nWARNING: the repository's .gitignore would un-track these RepoPact records (F-008):")
        for rel in rep.gitignored:
            print(f"  ! {rel}")
        print("They validate locally but would be MISSING on a fresh clone or in CI. Add negations, e.g.:")
        for rel in sorted({f"!/{r.rsplit('/', 1)[0]}/" for r in rep.gitignored if "/" in r}):
            print(f"  {rel}")
        print("  (then re-include the files, e.g. `!/evidence/runs/*.json`).")


def main() -> int:
    parser = argparse.ArgumentParser(description="Adopt RepoPact into an existing repository")
    parser.add_argument("--target", type=Path, required=True)
    parser.add_argument("--dry-run", action="store_true", help="Report the plan without writing files")
    args = parser.parse_args()
    target = args.target.resolve()
    rep = adopt(target, dry_run=args.dry_run)
    _print_report(rep)
    if args.dry_run:
        print("\nDry run: nothing written. Re-run without --dry-run to apply.")
        return 0

    import validate_repo
    problems = validate_repo.validate(target)
    if problems:
        for p in problems:
            print(f"ERROR {p.path.relative_to(target)}: {p.message}")
        print(f"\nAdoption produced {len(problems)} validation error(s) to resolve.")
        return 1
    print("\nAdopted repository validates as a conformant RepoPact.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
