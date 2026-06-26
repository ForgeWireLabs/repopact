"""S5 drift-detection harness (H12).

Builds a governed RepoPact repo, applies pre-registered drift mutations, and records
whether each is detected and how quickly. The `repopact` arm detects structural drift at
the next `validate` (latency = 1 step / the commit checkpoint); the `baseline`
convention-file arm has no detection mechanism (latency = infinity) - that asymmetry is the
measurement. Mutations marked blind_spot confirm where RepoPact itself does NOT catch drift
(honest reporting, per MUTATION-SET.md M4/M5/M7/M9).

Deterministic: no model required. Run:

    python benchmarks/drift/harness.py            # report
    python benchmarks/drift/harness.py --selftest # assert the asymmetry holds
"""
from __future__ import annotations

import argparse
import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import init_repo                       # noqa: E402
from validate_repo import validate     # noqa: E402


def _base_repo(dst: Path) -> Path:
    """A valid governed repo to drift from."""
    init_repo.bootstrap(dst)
    assert not validate(dst), "base repo must be valid before mutation"
    return dst


def _add_item(repo: Path, item_id: str, status: str, **over) -> Path:
    d = repo / "work" / status / f"{item_id}-probe"
    d.mkdir(parents=True, exist_ok=True)
    (d / "README.md").write_text("# probe\n", encoding="utf-8")
    data = {
        "id": item_id, "title": "probe", "status": status,
        "owner_scope": "governance", "affected_scopes": [], "depends_on": [],
        "provenance": "concrete",
        "acceptance_criteria": [{"id": "AC-1", "text": "x", "state": "pending", "evidence": []}],
        "created": "2020-01-01", "updated": "2020-01-01",  # pre-epoch -> preflight-exempt
    }
    data.update(over)
    (d / "work-item.json").write_text(json.dumps(data), encoding="utf-8")
    return d


# Each mutation takes a valid repo and introduces drift. Returns a label for the firing
# check, or None if it is a known blind spot (validate will NOT catch it).
def m08_unregistered_contract(repo: Path) -> None:
    (repo / "mod").mkdir()
    (repo / "mod" / "AGENTS.md").write_text("# nested\n", encoding="utf-8")


def m11_missing_evidence(repo: Path) -> None:
    _add_item(repo, "010", "active",
              acceptance_criteria=[{"id": "AC-1", "text": "x", "state": "satisfied",
                                    "evidence": ["does-not-exist"]}])


def m12_dependency_cycle(repo: Path) -> None:
    _add_item(repo, "020", "active", depends_on=["021"])
    _add_item(repo, "021", "active", depends_on=["020"])


def m15_status_dir_mismatch(repo: Path) -> None:
    # The JSON status says completed but the item sits in work/active/.
    d = _add_item(repo, "030", "active",
                  acceptance_criteria=[{"id": "AC-1", "text": "x", "state": "waived", "evidence": []}])
    mf = d / "work-item.json"
    data = json.loads(mf.read_text(encoding="utf-8"))
    data["status"] = "completed"
    mf.write_text(json.dumps(data), encoding="utf-8")


def m07_acceptance_regress(repo: Path) -> None:
    # Blind spot: a 'completed' item is valid as a record, but the code it asserted may have
    # since regressed. validate cannot see code, so it does not (and cannot) detect this.
    _add_item(repo, "040", "completed",
              acceptance_criteria=[{"id": "AC-1", "text": "x", "state": "waived", "evidence": []}])


MUTATIONS = [
    ("M8", "unregistered nested contract", m08_unregistered_contract, False),
    ("M11", "satisfied criterion -> missing evidence", m11_missing_evidence, False),
    ("M12", "dependency cycle", m12_dependency_cycle, False),
    ("M15", "status/dir mismatch", m15_status_dir_mismatch, False),
    ("M7", "completed item, code regressed (blind spot)", m07_acceptance_regress, True),
]


def run() -> list[dict]:
    results = []
    for mid, label, fn, blind in MUTATIONS:
        with tempfile.TemporaryDirectory() as tmp:
            repo = _base_repo(Path(tmp) / "repo")
            fn(repo)
            problems = validate(repo)
            detected = bool(problems)
            results.append({
                "id": mid, "label": label, "blind_spot": blind,
                "repopact_detected": detected,
                "repopact_latency": 1 if detected else "inf",
                "baseline_detected": False, "baseline_latency": "inf",
                "firing": problems[0].message if problems else None,
            })
    return results


def render(results: list[dict]) -> str:
    lines = ["# S5 drift-detection report", "",
             "| mutation | blind spot | repopact detected | latency | baseline detected | firing check |",
             "|---|---|---|---|---|---|"]
    for r in results:
        lines.append(f"| {r['id']} {r['label']} | {r['blind_spot']} | {r['repopact_detected']} | "
                     f"{r['repopact_latency']} | {r['baseline_detected']} | "
                     f"{(r['firing'] or '-')[:60]} |")
    det = sum(1 for r in results if r["repopact_detected"])
    nonblind = sum(1 for r in results if not r["blind_spot"])
    lines += ["", f"repopact detected {det}/{len(results)} mutations "
              f"({nonblind} non-blind-spot); baseline (convention files) detected 0."]
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description="S5 drift-detection harness")
    p.add_argument("--selftest", action="store_true")
    args = p.parse_args(argv)
    results = run()
    if args.selftest:
        # every non-blind-spot mutation must be detected by repopact and none by baseline;
        # the blind-spot mutation must NOT be detected (honest claim).
        for r in results:
            if r["blind_spot"]:
                assert not r["repopact_detected"], f"{r['id']} should be a blind spot"
            else:
                assert r["repopact_detected"], f"{r['id']} should be detected"
            assert not r["baseline_detected"]
        print(f"SELFTEST OK: repopact detected all non-blind-spot drift; "
              f"baseline detected none; blind spots confirmed.")
        return 0
    print(render(results))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
