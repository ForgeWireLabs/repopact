"""`repopact doctor` — diagnose RepoPact drift and optionally repair it.

Adoption is not one-shot: as a project and the standard evolve, an adopted repository
drifts (stale `registry.json` paths, a missing root contract, unregistered nested
contracts, incomplete `_audit` companions, schemas older than the installed RepoPact,
governance records swallowed by `.gitignore`). The reference validator reports *what*
is invalid; `doctor` frames drift as fixable findings and, with ``--fix``, applies the
safe, non-destructive repairs — then re-validates.

    repopact doctor [--root PATH] [--fix]

Read-only by default; exits non-zero if any error-level findings remain.
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from dataclasses import dataclass
from datetime import date, timedelta
from pathlib import Path

import adopt_repo
import validate_repo


@dataclass
class Finding:
    severity: str       # "error" | "warn"
    code: str
    message: str
    fixable: bool


# --- individual diagnostics -------------------------------------------------

def _schema_skew(root: Path) -> list[Finding]:
    try:
        seed = adopt_repo.init_repo._seed_dir("schemas")
    except FileNotFoundError:
        return []
    out: list[Finding] = []
    for src in sorted(seed.glob("*.json")):
        dest = root / "schemas" / src.name
        if not dest.is_file():
            out.append(Finding("warn", "schema-missing", f"schema {src.name} is missing (installed RepoPact ships it)", True))
        elif dest.read_bytes() != src.read_bytes():
            out.append(Finding("warn", "schema-differs",
                               f"schema {src.name} differs from the installed RepoPact - review by hand "
                               "(may be an intentional local extension; not auto-fixed)", False))
    return out


def _stale_registry(root: Path) -> tuple[list[Finding], list[dict]]:
    """Findings + the cleaned scope list (entries whose path/contract exist)."""
    path = root / "audits" / "registry.json"
    if not path.is_file():
        return [], []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return [], []
    kept, findings = [], []
    for entry in data.get("scopes", []):
        scope_path = root if entry.get("path") == "." else root / str(entry.get("path", ""))
        contract = entry.get("contract")
        contract_ok = (root / str(contract)).is_file() if contract else True
        if not scope_path.exists() or not contract_ok:
            findings.append(Finding("error", "registry-stale",
                                    f"registry scope '{entry.get('path')}' points at a path/contract that does not exist", True))
        else:
            kept.append(entry)
    return findings, kept


def _missing_root_contract(root: Path) -> list[Finding]:
    if not (root / "AGENTS.md").is_file():
        return [Finding("error", "no-root-contract", "root AGENTS.md is missing (required contract)", True)]
    return []


def _unregistered_contracts(root: Path) -> tuple[list[Finding], list[Path]]:
    covered = validate_repo.registered_contracts(root)
    out, missing = [], []
    for contract in adopt_repo.find_nested_contracts(root):
        if contract.parent.resolve() not in covered:
            rel = str(contract.relative_to(root)).replace("\\", "/")
            out.append(Finding("error", "contract-unregistered", f"nested contract {rel} is not registered in audits/registry.json", True))
            missing.append(contract)
    return out, missing


def _incomplete_audits(root: Path) -> list[tuple[Finding, Path]]:
    out: list[tuple[Finding, Path]] = []
    for contract in [root / "AGENTS.md", *adopt_repo.find_nested_contracts(root)]:
        audit = contract.parent / "_audit"
        if audit.is_dir():
            for name in ("README.md", "inventory.md", "alignment-report.md"):
                if not (audit / name).is_file():
                    rel = str((audit / name).relative_to(root)).replace("\\", "/")
                    out.append((Finding("error", "audit-incomplete", f"incomplete _audit companion: missing {rel}", True), audit / name))
    return out


def _gitignored_records(root: Path) -> list[Finding]:
    rels: list[str] = []
    for pattern in ("governance/**/*", "schemas/*", "evidence/runs/*.json",
                    "audits/**/*", "decisions/*", "work/**/*"):
        for p in root.glob(pattern):
            if p.is_file():
                rels.append(str(p.relative_to(root)).replace("\\", "/"))
    swallowed = adopt_repo.gitignored_records(root, rels)
    return [Finding("warn", "gitignored-record", f"governance record is git-ignored (missing on clone/CI): {r}", True) for r in swallowed]


def _dead_source_of_truth(root: Path) -> list[Finding]:
    """Warn when a record's `source_of_truth` frontmatter points at a missing path.

    Records (work item READMEs, _audit companions, decisions, policies) often pin
    a `source_of_truth:` to the files that authoritatively back them. When those
    files are renamed, moved, or deleted the pointer dangles silently — the kind
    of drift that lets stale records reference a long-gone tree. Only path-like,
    single-token targets are checked, so prose entries do not produce false
    positives. Not auto-fixed: the correct target needs judgment.
    """
    out: list[Finding] = []
    patterns = ("work/**/*.md", "decisions/*.md", "governance/**/*.md",
                "audits/**/*.md", "*.md")
    seen: set[Path] = set()
    for pattern in patterns:
        for path in sorted(root.glob(pattern)):
            if not path.is_file() or path in seen:
                continue
            seen.add(path)
            try:
                text = path.read_text(encoding="utf-8")
            except OSError:
                continue
            if not text.startswith("---"):
                continue
            end = text.find("\n---", 3)
            block = text[3:end] if end != -1 else ""
            for line in block.splitlines():
                stripped = line.strip()
                if not stripped.startswith("source_of_truth:"):
                    continue
                value = stripped.split(":", 1)[1].strip()
                for token in (t.strip() for t in value.split(";")):
                    looks_like_path = "/" in token or token.endswith((".md", ".json"))
                    if token and " " not in token and looks_like_path and not (root / token).exists():
                        rel = str(path.relative_to(root)).replace("\\", "/")
                        out.append(Finding("warn", "source-of-truth-stale",
                                           f"{rel}: source_of_truth points at missing path '{token}'", False))
    return out


# --- orchestration ----------------------------------------------------------

def diagnose(root: Path) -> list[Finding]:
    findings: list[Finding] = []
    findings += _missing_root_contract(root)
    findings += _stale_registry(root)[0]
    findings += [f for f, _ in _incomplete_audits(root)]
    findings += _unregistered_contracts(root)[0]
    findings += _schema_skew(root)
    findings += _gitignored_records(root)
    findings += _dead_source_of_truth(root)
    return findings


def fix(root: Path, today: date | None = None) -> list[str]:
    """Apply safe repairs. Returns a list of actions taken."""
    today = today or date.today()
    next_review = (today + timedelta(days=90)).isoformat()
    actions: list[str] = []

    # 1. add MISSING schemas from the installed RepoPact. A schema that merely *differs*
    #    is left alone: the repo may have intentionally extended it (e.g. an added
    #    property), and overwriting would clobber that. Differences stay a warning for
    #    human review rather than an automatic overwrite.
    try:
        seed = adopt_repo.init_repo._seed_dir("schemas")
        for src in seed.glob("*.json"):
            dest = root / "schemas" / src.name
            if not dest.is_file():
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(src, dest)
                actions.append(f"added missing schemas/{src.name}")
    except FileNotFoundError:
        pass

    # 2. create a root contract if missing
    if not (root / "AGENTS.md").is_file():
        (root / "AGENTS.md").write_text(
            "# Agent Contract\n\nThe repository is the durable coordination surface. The invariants in\n"
            "`governance/invariants.json` are binding; weakening one requires operator approval.\n"
            "Read every `AGENTS.md` from root to the file you touch.\n", encoding="utf-8")
        actions.append("created root AGENTS.md")

    # 3. drop stale registry entries; register unregistered nested contracts
    reg_path = root / "audits" / "registry.json"
    if reg_path.is_file():
        try:
            data = json.loads(reg_path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            data = {"version": 1, "scopes": []}
        _, kept = _stale_registry(root)
        dropped = len(data.get("scopes", [])) - len(kept)
        data["scopes"] = kept
        covered = {(root / str(e.get("contract"))).resolve().parent for e in kept if e.get("contract")}
        covered.add(root.resolve())
        for contract in adopt_repo.find_nested_contracts(root):
            if contract.parent.resolve() not in covered:
                rel_dir = str(contract.parent.relative_to(root)).replace("\\", "/")
                rel_contract = str(contract.relative_to(root)).replace("\\", "/")
                data["scopes"].append({"path": rel_dir, "owner": "governance-owner", "contract": rel_contract,
                                       "last_reviewed": today.isoformat(), "next_review": next_review,
                                       "alignment": "current", "notes": "Registered by repopact doctor."})
                covered.add(contract.parent.resolve())
                actions.append(f"registered contract {rel_contract}")
        reg_path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
        if dropped > 0:
            actions.append(f"dropped {dropped} stale registry scope(s)")

    # 4. stub missing _audit triplet files
    for finding, target in _incomplete_audits(root):
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(f"# {target.stem}\n\nStub created by repopact doctor; fill in.\n", encoding="utf-8")
        actions.append(f"stubbed {target.relative_to(root)}".replace("\\", "/"))

    # 5. add .gitignore negations for swallowed records
    swallowed = [f.message.split(": ", 1)[1] for f in _gitignored_records(root)]
    if swallowed:
        dirs = sorted({f"/{r.rsplit('/', 1)[0]}/" for r in swallowed if "/" in r})
        gi = root / ".gitignore"
        existing = gi.read_text(encoding="utf-8") if gi.is_file() else ""
        lines = ["", "# RepoPact governance records — keep tracked (added by repopact doctor)"]
        for d in dirs:
            lines += [f"!{d}", f"!{d}*"]
        gi.write_text(existing + "\n".join(lines) + "\n", encoding="utf-8")
        actions.append(f"added .gitignore negations for {len(dirs)} path(s)")

    return actions


def main() -> int:
    parser = argparse.ArgumentParser(description="Diagnose (and optionally fix) RepoPact drift")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--fix", action="store_true", help="Apply safe, non-destructive repairs")
    args = parser.parse_args()
    root = args.root.resolve()

    if args.fix:
        actions = fix(root)
        if actions:
            print("repopact doctor applied:")
            for a in actions:
                print(f"  ~ {a}")
        else:
            print("repopact doctor: nothing to fix.")

    findings = diagnose(root)
    problems = validate_repo.validate(root)
    errors = [f for f in findings if f.severity == "error"]
    warns = [f for f in findings if f.severity == "warn"]

    for f in errors:
        print(f"ERROR  [{f.code}] {f.message}" + ("  (fixable: --fix)" if f.fixable and not args.fix else ""))
    for f in warns:
        print(f"WARN   [{f.code}] {f.message}" + ("  (fixable: --fix)" if f.fixable and not args.fix else ""))
    for p in problems:
        print(f"INVALID {p.path.relative_to(root)}: {p.message}")

    if not errors and not warns and not problems:
        print("repopact doctor: healthy - no drift detected; repository validates.")
        return 0
    print(f"\n{len(errors)} error(s), {len(warns)} warning(s), {len(problems)} validation issue(s).")
    return 1 if errors or problems else 0


if __name__ == "__main__":
    sys.exit(main())
