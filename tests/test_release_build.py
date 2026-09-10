from __future__ import annotations

import gzip
import hashlib
import io
import json
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

from repopact import release_build
from repopact.package_version import semver_label_to_pep440


ROOT = Path(__file__).resolve().parents[1]


class ReleaseBuildTests(unittest.TestCase):
    def test_release_label_maps_deterministically_to_pep440(self) -> None:
        self.assertEqual("3.0.1rc1", semver_label_to_pep440("3.0.1-rc.1"))
        first = semver_label_to_pep440("3.0.1-preview.1+windows")
        self.assertEqual(first, semver_label_to_pep440("3.0.1-preview.1+windows"))
        self.assertNotEqual("3.0.1", first)
    def _wheel(self, root: Path, *, flat_module: bool = False) -> Path:
        path = root / "repopact-3.0.1-py3-none-win_amd64.whl"
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr("repopact/__init__.py", "")
            for index in range(release_build.EXPECTED_SCHEMAS):
                archive.writestr(f"repopact/schemas/{index}.json", "{}")
            for index in range(release_build.EXPECTED_TEMPLATES):
                archive.writestr(f"repopact/templates/{index}.txt", "")
            archive.writestr("repopact-3.0.1.data/scripts/repopact.exe", "launcher")
            archive.writestr("repopact-3.0.1.data/scripts/repopact-engine.exe", "engine")
            archive.writestr("repopact-3.0.1.dist-info/METADATA", "Name: repopact\nVersion: 3.0.1\n")
            archive.writestr(
                "repopact-3.0.1.dist-info/WHEEL",
                "Wheel-Version: 1.0\nRoot-Is-Purelib: false\nTag: py3-none-win_amd64\n",
            )
            if flat_module:
                archive.writestr("frontmatter.py", "")
        return path

    def test_clean_package_wheel_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            report = release_build.inspect_wheel(
                self._wheel(Path(temporary)),
                "3.0.1",
            )
        self.assertEqual(["repopact"], report["import_roots"])
        self.assertEqual(0, report["data_files"])
        self.assertEqual("win_amd64", report["platform_tag"])
        self.assertEqual(1, len(report["engine_scripts"]))

    def test_stale_flat_module_is_rejected_even_when_top_level_txt_is_clean(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = self._wheel(Path(temporary), flat_module=True)
            with self.assertRaisesRegex(
                release_build.ReleaseBuildError,
                "import roots.*frontmatter.py",
            ):
                release_build.inspect_wheel(path, "3.0.1")

    def test_export_creates_nested_temporary_parent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "first" / "source"
            release_build._export(ROOT, "HEAD", destination)
            self.assertTrue((destination / "VERSION").is_file())

    def test_sdist_normalization_removes_archive_timestamp_drift(self) -> None:
        def make_sdist(path: Path, timestamp: int) -> None:
            with path.open("wb") as raw:
                with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=timestamp) as compressed:
                    with tarfile.open(fileobj=compressed, mode="w") as archive:
                        member = tarfile.TarInfo("repopact-3.0.1/README.md")
                        member.size = 5
                        member.mtime = timestamp
                        archive.addfile(member, io.BytesIO(b"hello"))

        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first.tar.gz"
            second = Path(temporary) / "second.tar.gz"
            make_sdist(first, 1)
            make_sdist(second, 2)
            release_build._normalize_sdist(first, 42)
            release_build._normalize_sdist(second, 42)
            self.assertEqual(
                hashlib.sha256(first.read_bytes()).digest(),
                hashlib.sha256(second.read_bytes()).digest(),
            )

    def test_wheel_normalization_removes_maturin_sbom_export_path_drift(self) -> None:
        sbom_name = "repopact-3.0.1.dist-info/sboms/repopact-engine.cyclonedx.json"

        def make_wheel(path: Path, temporary_name: str) -> None:
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("repopact/__init__.py", "")
                archive.writestr(
                    sbom_name,
                    json.dumps(
                        {
                            "bom-ref": (
                                "path+file:///C:/Temp/"
                                f"{temporary_name}/repopact-3.0.1/rust/apps/repopact-engine"
                            )
                        }
                    ),
                )
                archive.writestr("repopact-3.0.1.dist-info/RECORD", "stale\n")

        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first.whl"
            second = Path(temporary) / "second.whl"
            make_wheel(first, ".tmpfirst")
            make_wheel(second, ".tmpsecond")
            release_build._normalize_wheel(first, 1789081237)
            release_build._normalize_wheel(second, 1789081237)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            with zipfile.ZipFile(first) as archive:
                sbom = archive.read(sbom_name)
                record = archive.read("repopact-3.0.1.dist-info/RECORD").decode()
            self.assertNotIn(".tmp", sbom.decode())
            self.assertIn("repopact-3.0.1.dist-info/sboms/repopact-engine.cyclonedx.json,sha256=", record)


if __name__ == "__main__":
    unittest.main()
