"""Minimal front-matter parser for durable narrative records.

Records (decisions, policies) carry a leading ``---`` block of provenance fields.
A full YAML dependency is unnecessary for this controlled subset, so we parse a
deliberately small grammar: ``key: value`` scalars and inline ``[a, b]`` lists.
Anything outside that grammar is reported rather than silently coerced.
"""

from __future__ import annotations

from pathlib import Path


class FrontMatterError(ValueError):
    """Raised when a record is missing or has malformed front matter."""


def _coerce(value: str) -> object:
    value = value.strip()
    if value.startswith("[") and value.endswith("]"):
        inner = value[1:-1].strip()
        if not inner:
            return []
        return [item.strip().strip("'\"") for item in inner.split(",")]
    return value.strip("'\"")


def parse(text: str) -> dict[str, object]:
    """Parse the front-matter block from Markdown ``text``."""
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        raise FrontMatterError("missing leading '---' front-matter fence")
    body: dict[str, object] = {}
    for index in range(1, len(lines)):
        line = lines[index]
        if line.strip() == "---":
            return body
        if not line.strip():
            continue
        if ":" not in line:
            raise FrontMatterError(f"front-matter line is not 'key: value': {line!r}")
        key, _, raw = line.partition(":")
        body[key.strip()] = _coerce(raw)
    raise FrontMatterError("missing closing '---' front-matter fence")


def parse_file(path: Path) -> dict[str, object]:
    return parse(path.read_text(encoding="utf-8"))
