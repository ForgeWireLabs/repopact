"""Build release artifacts from a clean committed Git tree.

Setuptools does not remove files that are no longer part of a distribution from
an existing ``build/lib`` directory. After RepoPact moved seventeen flat modules
under the ``repopact`` package, ``python -m build --wheel`` could therefore
silently repackage obsolete modules from an ignored checkout cache. Release
artifacts are instead built twice from independent ``git archive`` exports and
must be byte-identical and structurally conformant before they are copied out.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path
from typing import Any


EXPECTED_SCHEMAS = 8
EXPECTED_TEMPLATES = 6


class ReleaseBuildError(RuntimeError):
    """Raised when a release artifact is dirty, ambiguous, or structurally wrong."""


def _run(command: list[str], *, cwd: Path, env: dict[str, str] | None = None) -> str:
    result = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        output = (result.stdout + result.stderr).strip()
        raise ReleaseBuildError(f"command failed ({' '.join(command)}): {output}")
    return result.stdout.strip()


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inspect_wheel(path: Path, version: str) -> dict[str, Any]:
    with zipfile.ZipFile(path) as archive:
        names = sorted(archive.namelist())
        top_entries = [name for name in names if name.endswith(".dist-info/top_level.txt")]
        if len(top_entries) != 1:
            raise ReleaseBuildError("wheel must contain exactly one top_level.txt")
        top_level = archive.read(top_entries[0]).decode("utf-8").strip().splitlines()
    root_modules = sorted(name for name in names if "/" not in name and name.endswith(".py"))
    import_roots = sorted({
        name.split("/", 1)[0]
        for name in names
        if ".dist-info/" not in name and ".data/" not in name
    })
    schemas = sorted(name for name in names if name.startswith("repopact/schemas/") and name.endswith(".json"))
    templates = sorted(name for name in names if name.startswith("repopact/templates/") and not name.endswith("/"))
    data_files = sorted(name for name in names if ".data/data/" in name)
    expected_name = f"repopact-{version}-py3-none-any.whl"
    errors: list[str] = []
    if path.name != expected_name:
        errors.append(f"wheel name is {path.name}, expected {expected_name}")
    if top_level != ["repopact"]:
        errors.append(f"top_level.txt is {top_level!r}, expected ['repopact']")
    if import_roots != ["repopact"]:
        errors.append(f"wheel import roots are {import_roots!r}, expected ['repopact']")
    if root_modules:
        errors.append(f"wheel contains flat root modules: {', '.join(root_modules)}")
    if len(schemas) != EXPECTED_SCHEMAS:
        errors.append(f"wheel contains {len(schemas)} schemas, expected {EXPECTED_SCHEMAS}")
    if len(templates) != EXPECTED_TEMPLATES:
        errors.append(f"wheel contains {len(templates)} templates, expected {EXPECTED_TEMPLATES}")
    if data_files:
        errors.append(f"wheel contains deprecated data-files entries: {', '.join(data_files)}")
    if errors:
        raise ReleaseBuildError("; ".join(errors))
    return {
        "path": path.name,
        "sha256": _sha256(path),
        "top_level": top_level,
        "import_roots": import_roots,
        "root_modules": root_modules,
        "schemas": len(schemas),
        "templates": len(templates),
        "data_files": len(data_files),
    }


def inspect_sdist(path: Path, version: str) -> dict[str, Any]:
    expected_name = f"repopact-{version}.tar.gz"
    if path.name != expected_name:
        raise ReleaseBuildError(f"sdist name is {path.name}, expected {expected_name}")
    prefix = f"repopact-{version}/"
    with tarfile.open(path, "r:gz") as archive:
        names = sorted(member.name for member in archive.getmembers() if member.isfile())
    root_modules = sorted(
        name[len(prefix):]
        for name in names
        if name.startswith(prefix)
        and "/" not in name[len(prefix):]
        and name.endswith(".py")
    )
    if root_modules:
        raise ReleaseBuildError(f"sdist contains flat root modules: {', '.join(root_modules)}")
    return {
        "path": path.name,
        "sha256": _sha256(path),
        "root_modules": root_modules,
    }


def _export(root: Path, revision: str, destination: Path) -> None:
    archive_path = destination.parent / "source.zip"
    _run(
        ["git", "archive", "--format=zip", f"--output={archive_path}", revision],
        cwd=root,
    )
    destination.mkdir(parents=True)
    with zipfile.ZipFile(archive_path) as archive:
        archive.extractall(destination)


def _build_once(root: Path, revision: str, destination: Path) -> dict[str, Any]:
    source = destination / "source"
    output = destination / "dist"
    _export(root, revision, source)
    output.mkdir()
    version = (source / "VERSION").read_text(encoding="utf-8").strip()
    epoch = _run(["git", "show", "-s", "--format=%ct", revision], cwd=root)
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = epoch
    env["PYTHONHASHSEED"] = "0"
    _run(
        [sys.executable, "-m", "build", "--outdir", str(output)],
        cwd=source,
        env=env,
    )
    wheels = sorted(output.glob("*.whl"))
    sdists = sorted(output.glob("*.tar.gz"))
    if len(wheels) != 1 or len(sdists) != 1:
        raise ReleaseBuildError(
            f"build produced {len(wheels)} wheel(s) and {len(sdists)} sdist(s); expected one each"
        )
    return {
        "version": version,
        "wheel": inspect_wheel(wheels[0], version),
        "sdist": inspect_sdist(sdists[0], version),
        "wheel_path": wheels[0],
        "sdist_path": sdists[0],
    }


def build_release(root: Path, outdir: Path, revision: str = "HEAD") -> dict[str, Any]:
    root = root.resolve()
    outdir = outdir.resolve()
    dirty = _run(["git", "status", "--porcelain", "--untracked-files=all"], cwd=root)
    if dirty:
        raise ReleaseBuildError("release build requires a clean Git worktree")
    commit = _run(["git", "rev-parse", f"{revision}^{{commit}}"], cwd=root)
    if outdir.exists() and any(outdir.iterdir()):
        raise ReleaseBuildError(f"release output directory is not empty: {outdir}")
    outdir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="repopact-release-") as temporary:
        temporary_root = Path(temporary)
        first = _build_once(root, commit, temporary_root / "first")
        second = _build_once(root, commit, temporary_root / "second")
        for kind in ("wheel", "sdist"):
            if first[kind]["sha256"] != second[kind]["sha256"]:
                raise ReleaseBuildError(
                    f"{kind} is not reproducible: "
                    f"{first[kind]['sha256']} != {second[kind]['sha256']}"
                )
        wheel_target = outdir / first["wheel"]["path"]
        sdist_target = outdir / first["sdist"]["path"]
        shutil.copy2(first["wheel_path"], wheel_target)
        shutil.copy2(first["sdist_path"], sdist_target)
    return {
        "commit": commit,
        "version": first["version"],
        "reproducible": True,
        "wheel": first["wheel"],
        "sdist": first["sdist"],
    }


def render_json(report: dict[str, Any]) -> str:
    return json.dumps(report, indent=2, sort_keys=True) + "\n"
