from __future__ import annotations

import gzip
import hashlib
import io
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
        path = root / "repopact-3.0.1-py3-none-any.whl"
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr("repopact/__init__.py", "")
            for index in range(release_build.EXPECTED_SCHEMAS):
                archive.writestr(f"repopact/schemas/{index}.json", "{}")
            for index in range(release_build.EXPECTED_TEMPLATES):
                archive.writestr(f"repopact/templates/{index}.txt", "")
            archive.writestr("repopact-3.0.1.dist-info/top_level.txt", "repopact\n")
            archive.writestr("repopact-3.0.1.dist-info/METADATA", "Version: 3.0.1\n")
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


if __name__ == "__main__":
    unittest.main()
