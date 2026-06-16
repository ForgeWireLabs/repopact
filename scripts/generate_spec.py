"""Generate the derived blocks of SPEC.md (work item 004, decision 0004).

The record-type catalog and the invariant list are generated from
``schemas/*.json`` and ``governance/invariants.json`` so the normative reference
cannot drift from what the validator enforces (policy 001). Hand-written normative
prose lives between the generated blocks and is never touched here.
"""

from __future__ import annotations

import re
from pathlib import Path

from repo_model import load_json

ROOT = Path(__file__).resolve().parents[1]

# Display metadata the schemas do not carry: where each record lives and its job.
RECORDS = [
    ("work-item.schema.json", "Work item", "work/<status>/NNN-slug/work-item.json",
     "Unit of work: intent, scope, dependencies, acceptance criteria."),
    ("evidence-run.schema.json", "Evidence run", "evidence/runs/<id>.json",
     "Immutable record of a validation run and its results."),
    ("audit-finding.schema.json", "Audit finding", "audits/findings/NNN-slug.json",
     "Observed drift or risk, with its reconciliation."),
    ("invariants.schema.json", "Invariants", "governance/invariants.json",
     "Binding guarantees with rationale and escalation."),
    ("frozen-surface.schema.json", "Frozen surface", "governance/frozen-surface.json",
     "Protected paths and symbols requiring operator approval."),
]


def _required(schema: dict) -> list[str]:
    return list(schema.get("required", []))


def catalog_block(root: Path = ROOT) -> str:
    lines = [
        "| Record | Location | Schema | Required fields |",
        "| --- | --- | --- | --- |",
    ]
    for filename, name, location, _purpose in RECORDS:
        schema = load_json(root / "schemas" / filename)
        required = ", ".join(f"`{f}`" for f in _required(schema)) or "—"
        lines.append(f"| {name} | `{location}` | [`{filename}`](schemas/{filename}) | {required} |")
    # Decision and policy front matter come from the shared definitions schema.
    fm = load_json(root / "schemas" / "record-frontmatter.schema.json")
    for kind, location in (("decision", "decisions/NNNN-slug.md"),
                           ("policy", "governance/policies/NNN-slug.md")):
        required = ", ".join(f"`{f}`" for f in fm["definitions"][kind].get("required", [])) or "—"
        lines.append(
            f"| {kind.capitalize()} (front matter) | `{location}` | "
            f"[`record-frontmatter.schema.json`](schemas/record-frontmatter.schema.json) | {required} |"
        )
    return "\n".join(lines)


def invariants_block(root: Path = ROOT) -> str:
    data = load_json(root / "governance" / "invariants.json")
    lines = ["| ID | Statement | Enforced by |", "| --- | --- | --- |"]
    for inv in data.get("invariants", []):
        enforced = inv.get("enforced_by") or "human review (escalation)"
        lines.append(f"| {inv['id']} | {inv['statement']} | {enforced} |")
    return "\n".join(lines)


def replace_block(text: str, name: str, content: str) -> str:
    pattern = re.compile(
        rf"(<!-- generated:{name} -->\n).*?(\n<!-- /generated:{name} -->)",
        re.DOTALL,
    )
    if not pattern.search(text):
        raise ValueError(f"SPEC.md is missing the generated:{name} markers")
    return pattern.sub(lambda m: m.group(1) + content + m.group(2), text)


def render(text: str, root: Path = ROOT) -> str:
    version = (root / "VERSION").read_text(encoding="utf-8").strip()
    text = replace_block(text, "version", f"This document specifies **RepoPact {version}**.")
    text = replace_block(text, "catalog", catalog_block(root))
    text = replace_block(text, "invariants", invariants_block(root))
    return text


def main() -> None:
    spec = ROOT / "SPEC.md"
    spec.write_text(render(spec.read_text(encoding="utf-8"), ROOT), encoding="utf-8")
    print("Generated SPEC.md derived blocks")


if __name__ == "__main__":
    main()
