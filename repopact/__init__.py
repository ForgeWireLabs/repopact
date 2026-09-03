"""RepoPact — a repository-native governance kernel for durable agent work.

The public entry point is the ``repopact`` console script (``repopact.cli``).
Submodules are importable but are not a supported API surface; they moved under
this package in 3.0.0 so that installing RepoPact claims exactly one name in the
environment instead of seventeen (decision 0029).
"""

from __future__ import annotations

__all__ = ["__version__"]


def _version() -> str:
    """The release line, preferring a checkout's VERSION over installed metadata.

    Order matters. When running from a checkout, a differently-versioned RepoPact
    may also be installed in the environment, and `importlib.metadata` would
    report *that* one — so the checkout's own `VERSION` record wins when present.
    An installed package has no sibling `VERSION`, so it falls through to its
    distribution metadata.
    """
    from pathlib import Path
    root = Path(__file__).resolve().parent.parent
    candidate = root / "VERSION"
    if candidate.is_file():
        from .package_version import package_version
        return package_version(root)
    from importlib.metadata import PackageNotFoundError, version
    try:
        return version("repopact")
    except PackageNotFoundError:
        return "unknown"


__version__ = _version()
