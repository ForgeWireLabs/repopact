"""Derive distribution identity without weakening the VERSION compatibility core."""

from __future__ import annotations

import re
from pathlib import Path


SEMVER_IDENT = r"(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
RELEASE_LABEL_RE = re.compile(
    rf"(?P<base>[0-9]+\.[0-9]+\.[0-9]+)"
    rf"-(?P<prerelease>{SEMVER_IDENT}(?:\.{SEMVER_IDENT})*)"
    rf"(?:\+(?P<build>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
)


class PackageVersionError(ValueError):
    """Raised when governed source identity cannot form package metadata."""


def semver_label_to_pep440(label: str) -> str:
    """Map a valid RepoPact SemVer pre-release identity deterministically to PEP 440.

    Conventional alpha/beta/rc/dev labels stay readable. Other legal SemVer
    pre-releases use a lossless hexadecimal local segment so setuptools never gets
    to apply an incidental, potentially ambiguous normalization.
    """
    match = RELEASE_LABEL_RE.fullmatch(label)
    if not match:
        raise PackageVersionError(f"invalid SemVer RELEASE_LABEL: {label!r}")
    base = match.group("base")
    prerelease = match.group("prerelease")
    build = match.group("build")
    conventional = re.fullmatch(r"(?P<kind>alpha|a|beta|b|rc|dev)\.(?P<num>0|[1-9][0-9]*)", prerelease)
    if conventional:
        kind = {"alpha": "a", "a": "a", "beta": "b", "b": "b", "rc": "rc", "dev": "dev"}[
            conventional.group("kind")
        ]
        rendered = f"{base}{kind}{conventional.group('num')}"
        if build:
            rendered += "+" + build.replace("-", ".")
        return rendered
    encoded = label[len(base) + 1 :].encode("utf-8").hex()
    return f"{base}.dev0+semver.{encoded}"


def package_version(root: Path) -> str:
    """Return stable VERSION or the explicit development distribution identity."""
    version_path = root / "VERSION"
    if not version_path.is_file():
        from importlib.metadata import PackageNotFoundError, version as distribution_version
        try:
            return distribution_version("repopact")
        except PackageNotFoundError:
            return "unknown"
    version = version_path.read_text(encoding="utf-8").strip()
    label_path = root / "RELEASE_LABEL"
    if not label_path.is_file():
        return version
    label = label_path.read_text(encoding="utf-8").strip()
    match = RELEASE_LABEL_RE.fullmatch(label)
    if not match or match.group("base") != version:
        raise PackageVersionError("RELEASE_LABEL must be a SemVer pre-release whose core equals VERSION")
    return semver_label_to_pep440(label)


# setuptools' dynamic attr reader obtains this value while building from the source tree.
__version__ = package_version(Path(__file__).resolve().parent.parent)
