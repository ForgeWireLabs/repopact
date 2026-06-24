"""Import a `tracking/`-style governance system into RepoPact records.

Some teams plan not with a todo list but with a governance folder: a decision log, a
risk register, a milestone list, a status snapshot, a work log. These map to *different*
RepoPact record types, not just `work/`:

* `tracking/decisions.md`  (DEC-NNN blocks)        -> `decisions/NNNN-*.md`
* `tracking/risks.md`      (RISK-NNN table)         -> `audits/findings/RISK-NNN-*.json`
* `tracking/milestones.md` (M-N, Status/Evidence)   -> `work/<status>/` items

`status.md`, `work-log.md`, and per-initiative trackers are a derived snapshot, a
history log, and companions to existing todos respectively; they are **not** fabricated
into records — they are reported for review/archive. Imports are non-destructive (the
source is preserved, origin recorded) and idempotent.
"""

from __future__ import annotations

import json
import re
from datetime import date
from pathlib import Path

import adopt_repo

TRACKING_DIR_NAMES = ("tracking", "track")
DECISION_STATUSES = ("proposed", "accepted", "rejected", "deferred", "superseded", "deprecated")
_SEVERITY = {"p1": "high", "p2": "medium", "p3": "low", "high": "high", "medium": "medium", "low": "low"}
_FINDING_STATE = {"open": "open", "accepted": "accepted", "reconciled": "reconciled",
                  "closed": "reconciled", "resolved": "reconciled", "mitigated": "reconciled"}


def _read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def _field(block: str, name: str) -> str | None:
    m = re.search(rf"(?im)^\s*{name}\s*:\s*(.+?)\s*$", block)
    return m.group(1).strip().strip("`") if m else None


def _blocks(text: str, id_pattern: str) -> list[tuple[str, str, str]]:
    """Split on `## <ID>: <title>` headings; return (id, title, block_body)."""
    out = []
    matches = list(re.finditer(rf"(?m)^##\s+({id_pattern})\s*:?\s*(.*?)\s*$", text))
    for i, m in enumerate(matches):
        end = matches[i + 1].start() if i + 1 < len(matches) else len(text)
        out.append((m.group(1), m.group(2).strip(), text[m.start():end]))
    return out


def _find_tracking_dir(root: Path) -> Path | None:
    for name in TRACKING_DIR_NAMES:
        if (root / name).is_dir():
            return root / name
    return None


def import_tracking(root: Path, rep: adopt_repo.Report, today: date | None = None) -> None:
    today = today or date.today()
    tracking = _find_tracking_dir(root)
    if not tracking:
        return
    rel = tracking.name
    _import_decisions(root, tracking / "decisions.md", rel, rep, today)
    _import_risks(root, tracking / "risks.md", rel, rep, today)
    _import_milestones(root, tracking / "milestones.md", rel, rep, today)


def _import_decisions(root: Path, src: Path, rel: str, rep: adopt_repo.Report, today: date) -> None:
    if not src.is_file():
        return
    existing = {p.read_text(encoding="utf-8", errors="replace") for p in (root / "decisions").glob("*.md")} \
        if (root / "decisions").is_dir() else set()
    used = set()
    for p in (root / "decisions").glob("*.md") if (root / "decisions").is_dir() else []:
        m = re.match(r"(\d{4,})-", p.name)
        if m:
            used.add(m.group(1))
    for dec_id, title, body in _blocks(_read(src), r"DEC-\d+"):
        marker = f"<!-- repopact-source: {rel}/decisions.md#{dec_id} -->"
        if any(marker in e for e in existing):
            rep.skipped.append(f"decisions/* ({dec_id} already imported)")
            continue
        num = _alloc4(used)
        status = (_field(body, "Status") or "accepted").lower()
        if status not in DECISION_STATUSES:
            status = "accepted"
        d = _field(body, "Date") or today.isoformat()
        try:
            date.fromisoformat(d)
        except ValueError:
            d = today.isoformat()
        slug = adopt_repo._slug(title)[:50] or dec_id.lower()
        front = f"---\nid: {num}\ntitle: {json.dumps(title or dec_id)}\nstatus: {status}\ndate: {d}\nsupersedes: []\n---\n"
        rep.write(root / "decisions" / f"{num}-{slug}.md",
                  f"{front}\n{marker}\n\n# {num}: {title or dec_id}\n\n> Imported from `{rel}/decisions.md` ({dec_id}).\n\n{body.strip()}\n", root)


def _import_risks(root: Path, src: Path, rel: str, rep: adopt_repo.Report, today: date) -> None:
    if not src.is_file():
        return
    findings = root / "audits" / "findings"
    used_ids: set[str] = set()
    imported_sources: set[str] = set()
    if findings.is_dir():
        for p in findings.glob("*.json"):
            m = re.match(r"(\d{3,})-", p.name)
            if m:
                used_ids.add(m.group(1))
            try:
                imported_sources.add(json.loads(p.read_text(encoding="utf-8")).get("source", ""))
            except (OSError, ValueError):
                pass
    for raw in _read(src).splitlines():
        m = re.match(r"\s*\|\s*(RISK-\d+)\s*\|(.+)$", raw)
        if not m:
            continue
        rid = m.group(1)
        source = f"{rel}/risks.md#{rid}"
        cells = [c.strip() for c in m.group(2).split("|")]
        if source in imported_sources:           # idempotent: provenance already present
            rep.skipped.append(f"audits/findings/* ({rid} already imported)")
            continue
        # the finding schema requires a numeric id; keep the original RISK-NNN in `source`.
        fid = _alloc_finding(used_ids)
        observed = cells[0] if cells else rid
        severity = _SEVERITY.get((cells[1] if len(cells) > 1 else "").lower(), "medium")
        state = _FINDING_STATE.get((cells[2] if len(cells) > 2 else "").lower(), "open")
        mitigation = cells[4] if len(cells) > 4 else (cells[-1] if cells else "")
        slug = adopt_repo._slug(observed)[:40] or "risk"
        rep.json(findings / f"{fid}-{slug}.json", {
            "$schema": "../../schemas/audit-finding.schema.json",
            "id": fid, "scope": "governance", "observed": f"[{rid}] {observed}", "risk": severity,
            "reconciliation": mitigation or "See source risk register.", "state": state,
            "created": today.isoformat(), "source": source,
        }, root)


def _import_milestones(root: Path, src: Path, rel: str, rep: adopt_repo.Report, today: date) -> None:
    if not src.is_file():
        return
    used_ids, used_slugs = _existing_work(root)
    for mid, title, body in _blocks(_read(src), r"M\d+"):
        slug = adopt_repo._slug(f"milestone-{title or mid}")[:55]
        if slug in used_slugs:
            rep.skipped.append(f"work/* /{slug} (milestone already imported)")
            continue
        used_slugs.add(slug)
        status_text = (_field(body, "Status") or "").lower()
        done = any(k in status_text for k in ("shipped", "done", "complete", "delivered"))
        status, ac_state = ("completed", "waived") if done else ("active", "pending")
        item_id = _alloc_work(used_ids)
        source = f"{rel}/milestones.md#{mid}"
        ac = (f"Milestone delivered prior to/at RepoPact adoption (imported from `{source}`); waived: no RepoPact evidence captured."
              if done else f"Achieve the milestone described in the imported narrative (`{source}`).")
        rep.json(root / "work" / status / f"{item_id}-{slug}" / "work-item.json", {
            "$schema": "../../../schemas/work-item.schema.json",
            "id": item_id, "title": f"{mid}: {title}" if title else mid, "status": status,
            "owner_scope": "governance", "affected_scopes": [], "depends_on": [],
            "acceptance_criteria": [{"id": "AC-1", "text": ac, "state": ac_state, "evidence": []}],
            "created": today.isoformat(), "updated": today.isoformat(), "source": source,
        }, root)
        rep.write(root / "work" / status / f"{item_id}-{slug}" / "README.md",
                  f"# {item_id} - {mid}: {title}\n\n> Imported milestone from `{source}`; source preserved.\n\n{body.strip()}\n", root)


def _alloc_finding(used: set[str]) -> str:
    n = 1
    while f"{n:03d}" in used:
        n += 1
    used.add(f"{n:03d}")
    return f"{n:03d}"


def _alloc4(used: set[str]) -> str:
    n = 1
    while f"{n:04d}" in used:
        n += 1
    used.add(f"{n:04d}")
    return f"{n:04d}"


def _existing_work(root: Path) -> tuple[set[str], set[str]]:
    ids, slugs = set(), set()
    for manifest in root.glob("work/*/*/work-item.json"):
        try:
            ids.add(str(json.loads(manifest.read_text(encoding="utf-8")).get("id", "")))
        except (OSError, ValueError):
            pass
        m = re.match(r"\d+-(.*)$", manifest.parent.name)
        slugs.add(m.group(1) if m else manifest.parent.name)
    return ids, slugs


def _alloc_work(used: set[str]) -> str:
    n = 300
    while f"{n:03d}" in used:
        n += 1
    used.add(f"{n:03d}")
    return f"{n:03d}"
