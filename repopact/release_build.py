"""Build release artifacts from a clean committed Git tree.

Release artifacts are built twice from independent ``git archive`` exports and
must be byte-identical and structurally conformant before they are copied out.
The Maturin build includes the Python package and the platform-native engine
without relying on an in-place build cache.
"""

from __future__ import annotations

import copy
import base64
import gzip
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import zipfile
from pathlib import Path
from typing import Any

from .package_version import package_version


EXPECTED_SCHEMAS = 15
EXPECTED_TEMPLATES = 7
_SBOM_SOURCE_PATH = re.compile(
    r"path\+file:///[^\" ]+?/(repopact-[^/\" ]+)/"
)


def _wheel_version(version: str) -> str:
    """Use the PEP 440 spelling emitted by Maturin for a source identity."""
    for suffix in ("alpha", "a", "beta", "b", "rc", "dev"):
        marker = suffix
        if marker in version and version.rsplit(marker, 1)[1].isdigit():
            prefix, number = version.rsplit(marker, 1)
            return version if prefix.endswith(".") else f"{prefix}.{marker}{number}"
    return version


class ReleaseBuildError(RuntimeError):
    """Raised when a release artifact is dirty, ambiguous, or structurally wrong."""


def _run(
    command: list[str], *, cwd: Path, env: dict[str, str] | None = None,
    timeout: float | None = None,
) -> str:
    run_env = env
    if command and command[0].lower() == "git":
        run_env = dict(os.environ) if env is None else dict(env)
        run_env["GIT_TERMINAL_PROMPT"] = "0"
        run_env["GIT_OPTIONAL_LOCKS"] = "0"
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=run_env,
            text=True,
            encoding="utf-8",
            errors="replace",
            capture_output=True,
            check=False,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        raise ReleaseBuildError(
            f"command timed out after {timeout}s ({' '.join(command)})"
        ) from exc
    if result.returncode != 0:
        output = ((result.stdout or "") + (result.stderr or "")).strip()
        raise ReleaseBuildError(f"command failed ({' '.join(command)}): {output}")
    return result.stdout.strip()


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inspect_wheel(path: Path, version: str) -> dict[str, Any]:
    with zipfile.ZipFile(path) as archive:
        names = sorted(archive.namelist())
        metadata_entries = [name for name in names if name.endswith(".dist-info/METADATA")]
        wheel_entries = [name for name in names if name.endswith(".dist-info/WHEEL")]
        if len(metadata_entries) != 1 or len(wheel_entries) != 1:
            raise ReleaseBuildError("wheel must contain exactly one METADATA and WHEEL record")
        metadata = archive.read(metadata_entries[0]).decode("utf-8")
        wheel_metadata = archive.read(wheel_entries[0]).decode("utf-8")
        top_entries = [name for name in names if name.endswith(".dist-info/top_level.txt")]
        top_level = archive.read(top_entries[0]).decode("utf-8").strip().splitlines() if top_entries else []
    root_modules = sorted(name for name in names if "/" not in name and name.endswith(".py"))
    import_roots = sorted({
        name.split("/", 1)[0]
        for name in names
        if ".dist-info/" not in name and ".data/" not in name
    })
    schemas = sorted(name for name in names if name.startswith("repopact/schemas/") and name.endswith(".json"))
    templates = sorted(name for name in names if name.startswith("repopact/templates/") and not name.endswith("/"))
    data_files = sorted(name for name in names if ".data/data/" in name)
    expected_version = _wheel_version(version)
    expected_prefix = f"repopact-{expected_version}-"
    wheel_stem = path.name.removesuffix(".whl")
    tag_text = wheel_stem[len(expected_prefix):] if wheel_stem.startswith(expected_prefix) else ""
    tags = tag_text.split("-") if tag_text else []
    engine_scripts = [
        name for name in names
        if name.endswith(".data/scripts/repopact-engine.exe")
        or name.endswith(".data/scripts/repopact-engine")
    ]
    launcher_scripts = [
        name for name in names
        if name.endswith(".data/scripts/repopact.exe")
        or name.endswith(".data/scripts/repopact")
    ]
    desktop_payload = sorted(
        name for name in names
        if "src-tauri" in name.lower() or "repopact-desktop" in name.lower()
    )
    errors: list[str] = []
    if len(tags) != 3:
        errors.append(f"wheel filename has no valid python/abi/platform tags: {path.name}")
    else:
        python_tag, abi_tag, platform_tag = tags
        if python_tag != "py3":
            errors.append(f"wheel Python tag is {python_tag!r}, expected 'py3'")
        if abi_tag != "none":
            errors.append(f"wheel ABI tag is {abi_tag!r}, expected 'none'")
        if platform_tag == "any":
            errors.append("wheel must be platform-specific because it contains native binaries")
        if python_tag.startswith("cp") or abi_tag.startswith("cp"):
            errors.append("wheel must not depend on a CPython extension ABI")
    metadata_fields = {
        line.split(":", 1)[0]: line.split(":", 1)[1].strip()
        for line in metadata.splitlines()
        if ":" in line
    }
    if metadata_fields.get("Name") != "repopact":
        errors.append(f"wheel metadata name is {metadata_fields.get('Name')!r}, expected 'repopact'")
    if metadata_fields.get("Version") != expected_version:
        errors.append(
            f"wheel metadata version is {metadata_fields.get('Version')!r}, expected {expected_version!r}"
        )
    if "Root-Is-Purelib: true" in wheel_metadata:
        errors.append("native wheel must not be marked Root-Is-Purelib")
    if top_level and top_level != ["repopact"]:
        errors.append(f"top_level.txt is {top_level!r}, expected ['repopact'] when present")
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
    if not any(name == "repopact/__init__.py" for name in names):
        errors.append("wheel is missing repopact/__init__.py")
    if len(engine_scripts) != 1:
        errors.append(f"wheel contains {len(engine_scripts)} repopact-engine scripts, expected one")
    if len(launcher_scripts) != 1:
        errors.append(f"wheel contains {len(launcher_scripts)} repopact launchers, expected one")
    if desktop_payload:
        errors.append(f"wheel contains desktop payload: {', '.join(desktop_payload)}")
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
        "python_tag": tags[0] if len(tags) == 3 else None,
        "abi_tag": tags[1] if len(tags) == 3 else None,
        "platform_tag": tags[2] if len(tags) == 3 else None,
        "engine_scripts": engine_scripts,
        "launcher_scripts": launcher_scripts,
    }


def inspect_sdist(path: Path, version: str) -> dict[str, Any]:
    expected_version = _wheel_version(version)
    expected_name = f"repopact-{expected_version}.tar.gz"
    if path.name != expected_name:
        raise ReleaseBuildError(f"sdist name is {path.name}, expected {expected_name}")
    prefix = f"repopact-{expected_version}/"
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
    required = {
        f"{prefix}pyproject.toml",
        f"{prefix}rust/Cargo.toml",
        f"{prefix}rust/Cargo.lock",
        f"{prefix}rust/apps/repopact-engine/Cargo.toml",
        f"{prefix}rust/apps/repopact-engine/src/main.rs",
        f"{prefix}rust/crates/repopact-protocol/src/lib.rs",
        f"{prefix}repopact/__init__.py",
        f"{prefix}repopact/engine_client.py",
    }
    missing = sorted(required - set(names))
    if missing:
        raise ReleaseBuildError(f"sdist is missing required source-build files: {', '.join(missing)}")
    return {
        "path": path.name,
        "sha256": _sha256(path),
        "root_modules": root_modules,
        "rust_sources": sum(name.startswith(f"{prefix}rust/") for name in names),
    }


def _normalize_sdist(path: Path, epoch: int) -> None:
    """Rewrite the timestamp-bearing sdist as a canonical tar.gz."""
    target = path.with_suffix(path.suffix + ".tmp")
    with tarfile.open(path, "r:gz") as source:
        members = sorted(source.getmembers(), key=lambda member: member.name)
        with target.open("wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=epoch) as compressed:
                with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as output:
                    for member in members:
                        normalized = copy.copy(member)
                        normalized.mtime = epoch
                        normalized.uid = 0
                        normalized.gid = 0
                        normalized.uname = ""
                        normalized.gname = ""
                        normalized.pax_headers = {}
                        payload = source.extractfile(member) if member.isfile() else None
                        output.addfile(normalized, payload)
    target.replace(path)


def _normalize_wheel(path: Path, epoch: int) -> None:
    """Canonicalize generated SBOM paths and the dependent RECORD hashes.

    Maturin's Cargo SBOM contains the temporary directory used for each
    source export.  That directory is not part of the artifact's identity,
    so retain the source-distribution root name and remove only that
    environment-specific prefix.  Rebuilding RECORD after this transformation
    keeps the wheel internally valid while leaving all package payload bytes
    untouched.
    """
    with zipfile.ZipFile(path) as source:
        entries = {name: source.read(name) for name in source.namelist()}
        infos = {name: copy.copy(source.getinfo(name)) for name in source.namelist()}
    record_names = [name for name in entries if name.endswith(".dist-info/RECORD")]
    if len(record_names) != 1:
        raise ReleaseBuildError("wheel must contain exactly one RECORD file")
    record_name = record_names[0]
    for name in list(entries):
        if name.endswith(".dist-info/sboms/repopact-engine.cyclonedx.json"):
            text = entries[name].decode("utf-8")
            entries[name] = _SBOM_SOURCE_PATH.sub(r"path+file:///\1/", text).encode("utf-8")

    record_lines: list[str] = []
    for name in sorted(entries):
        if name == record_name:
            continue
        payload = entries[name]
        digest = base64.urlsafe_b64encode(hashlib.sha256(payload).digest()).rstrip(b"=").decode()
        record_lines.append(f"{name},sha256={digest},{len(payload)}")
    record_lines.append(f"{record_name},,")
    entries[record_name] = ("\n".join(record_lines) + "\n").encode("utf-8")

    target = path.with_suffix(path.suffix + ".tmp")
    epoch_time = time.gmtime(epoch)[:6]
    with zipfile.ZipFile(target, "w") as output:
        for name in sorted(entries):
            info = infos[name]
            info.date_time = epoch_time
            output.writestr(info, entries[name])
    target.replace(path)


def _export(root: Path, revision: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
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
    artifact_version = package_version(source)
    epoch = _run(["git", "show", "-s", "--format=%ct", revision], cwd=root, timeout=5)
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = epoch
    env["PYTHONHASHSEED"] = "0"
    if sys.platform == "win32":
        # cargo/rustc otherwise leaves PE timestamps and CodeView identifiers
        # dependent on the individual source-export build directory.
        env["RUSTFLAGS"] = "-C debuginfo=0 -C link-arg=/DEBUG:NONE -C link-arg=/Brepro"
    _run(
        [
            "cargo",
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--manifest-path",
            str(source / "rust" / "Cargo.toml"),
        ],
        cwd=source,
        env=env,
        timeout=30,
    )
    _run(
        [
            sys.executable,
            "-m",
            "maturin",
            "build",
            "--sdist",
            "--out",
            str(output),
        ],
        cwd=source,
        env=env,
    )
    wheels = sorted(output.glob("*.whl"))
    sdists = sorted(output.glob("*.tar.gz"))
    if len(wheels) != 1 or len(sdists) != 1:
        raise ReleaseBuildError(
            f"build produced {len(wheels)} wheel(s) and {len(sdists)} sdist(s); expected one each"
        )
    _normalize_wheel(wheels[0], int(epoch))
    _normalize_sdist(sdists[0], int(epoch))
    return {
        "version": version,
        "artifact_version": artifact_version,
        "wheel": inspect_wheel(wheels[0], artifact_version),
        "sdist": inspect_sdist(sdists[0], artifact_version),
        "wheel_path": wheels[0],
        "sdist_path": sdists[0],
    }


def build_release(root: Path, outdir: Path, revision: str = "HEAD") -> dict[str, Any]:
    root = root.resolve()
    outdir = outdir.resolve()
    dirty = _run(
        ["git", "status", "--porcelain", "--untracked-files=all"], cwd=root, timeout=5
    )
    if dirty:
        raise ReleaseBuildError("release build requires a clean Git worktree")
    commit = _run(["git", "rev-parse", f"{revision}^{{commit}}"], cwd=root, timeout=5)
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
        "artifact_version": first["artifact_version"],
        "reproducible": True,
        "wheel": first["wheel"],
        "sdist": first["sdist"],
    }


def render_json(report: dict[str, Any]) -> str:
    return json.dumps(report, indent=2, sort_keys=True) + "\n"
